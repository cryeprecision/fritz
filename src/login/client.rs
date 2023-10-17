use std::time::Instant;

use anyhow::Context;
use parking_lot::Mutex;
use reqwest::tls::Version;

use crate::logs::LogEntry;
use crate::{LoginChallenge, SessionId};

pub struct Client {
    /// Use to make REST requests
    client: reqwest::Client,
    /// Example: `192.168.178.1` or `fritz.box`
    domain: String,
    /// This is set once logged in
    session_id: Mutex<Option<SessionId>>,
    /// Username to log in with
    username: String,
    /// Password to log in with
    password: String,
}

impl Client {
    pub async fn new(
        domain: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        root_cert: Option<&[u8]>,
    ) -> anyhow::Result<Client> {
        fn resolve_var(key: &str, default: Option<&str>) -> anyhow::Result<String> {
            default.map(|s| Ok(s.to_string())).unwrap_or_else(|| {
                dotenv::var(key).with_context(|| format!("couldn't find env var {}", key))
            })
        }

        fn resolve_root_cert(
            key: &str,
            default: Option<&[u8]>,
        ) -> anyhow::Result<reqwest::Certificate> {
            let bytes = default.map(|b| Ok(b.to_vec())).unwrap_or_else(|| {
                let path = dotenv::var(key).unwrap_or("./cert.pem".to_string());
                std::fs::read(&path).with_context(|| format!("couldn't find root cert at {}", path))
            })?;
            reqwest::Certificate::from_pem(&bytes).context("certificate is invalid")
        }

        let domain = resolve_var("FRITZBOX_DOMAIN", domain)?;
        let username = resolve_var("FRITZBOX_USERNAME", username)?;
        let password = resolve_var("FRITZBOX_PASSWORD", password)?;

        let mut builder = reqwest::Client::builder()
            .https_only(true)
            .min_tls_version(Version::TLS_1_2);

        match resolve_root_cert("FRITZBOX_ROOT_CERT_PATH", root_cert) {
            Err(_) => {
                log::warn!("couldn't load root cert, accepting invalid certs");
                builder = builder.danger_accept_invalid_certs(true);
            }
            Ok(root_cert) => {
                builder = builder.add_root_certificate(root_cert);
            }
        };

        let client = builder
            .build()
            .context("invalid http client configuration")?;

        Ok(Client {
            client,
            domain,
            session_id: Mutex::new(None),
            username,
            password,
        })
    }

    /// Example: `client.make_url("/cgi-bin/firmwarecfg")` will produce
    /// `https://{host}/cgi-bin/firmwarecfg`
    fn make_url(&self, path: &str) -> String {
        format!("https://{}{}", self.domain, path)
    }

    async fn check_or_renew_session_id(&self) -> anyhow::Result<SessionId> {
        match self.check_session_id().await? {
            None => self.login().await,
            Some(session_id) => Ok(session_id),
        }
    }

    async fn check_session_id(&self) -> anyhow::Result<Option<SessionId>> {
        // We don't have a SessionId yet
        let Some(session_id) = self.session_id.lock().clone() else {
            return Ok(None);
        };

        // We have a session id, verify it
        let url = self.make_url("/login_sid.lua?version=2");
        let form: [(&str, &str); 1] = [("sid", &session_id.to_string())];

        let now = Instant::now();
        let req = self.client.post(&url).form(&form);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        let elapsed_ms = now.elapsed().as_secs_f64() * 1e3;
        log::debug!("check_session_id ({:.2}ms):\n{}", elapsed_ms, text.trim());

        let challenge =
            LoginChallenge::from_xml_text(&text).context("couldn't parse response xml")?;

        Ok(challenge
            .session_id
            .map(|id| if id == session_id { Some(id) } else { None })
            .flatten())
    }

    /// Get the login challenge
    pub async fn login_challenge(&self) -> anyhow::Result<LoginChallenge> {
        let url = self.make_url("/login_sid.lua?version=2");

        let now = Instant::now();
        let req = self.client.get(&url);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        let elapsed_ms = now.elapsed().as_secs_f64() * 1e3;
        log::debug!("login_challenge ({:.2}ms):\n{}", elapsed_ms, text);

        let xml = roxmltree::Document::parse(&text)
            .context("login challenge response returned invalid xml")?;

        LoginChallenge::from_xml(&xml).context("couldn't parse login challenge xml")
    }

    /// Login by sending the correct response for the given challenge
    pub async fn login_response(
        &self,
        challenge: &LoginChallenge,
    ) -> anyhow::Result<LoginChallenge> {
        // check for username present in users
        if !challenge.users.iter().any(|user| user == &self.username) {
            anyhow::bail!(
                "trying to login with invalid user ({} not in {:?})",
                self.username,
                challenge.users
            )
        }

        let url = self.make_url("/login_sid.lua?version=2");
        let response = challenge.make_response(&self.password).to_string();
        let form: [(&str, &str); 2] = [("username", &self.username), ("response", &response)];

        let now = Instant::now();
        let req = self.client.post(&url).form(&form);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        let elapsed_ms = now.elapsed().as_secs_f64() * 1e3;
        log::debug!("login_response ({:.2}ms):\n{}", elapsed_ms, text);

        LoginChallenge::from_xml_text(&text).context("couldn't parse response xml")
    }

    pub async fn login(&self) -> anyhow::Result<SessionId> {
        // get the challenge
        let login_challenge = self.login_challenge().await?;

        // respond with the correct response
        let response = self.login_response(&login_challenge).await?;

        // get the session id
        let session_id = response
            .session_id
            .ok_or(anyhow::anyhow!("missing session id"))?;

        *self.session_id.lock() = Some(session_id.clone());
        Ok(session_id)
    }

    pub async fn box_cert(&self) -> anyhow::Result<String> {
        let url = self.make_url("/cgi-bin/firmwarecfg");
        let session_id = self.check_or_renew_session_id().await?.to_string();
        let form = reqwest::multipart::Form::new()
            .text("sid", session_id)
            .text("BoxCertExport", "");

        let now = Instant::now();
        let req = self.client.post(&url).multipart(form);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        let elapsed_ms = now.elapsed().as_secs_f64() * 1e3;
        log::debug!("box_cert ({:.2}ms):\n{}", elapsed_ms, text);

        Ok(text)
    }

    pub async fn logout(&self) -> anyhow::Result<()> {
        let Some(session_id) = self.check_session_id().await? else {
            return Ok(());
        };

        let url = self.make_url("/login_sid.lua?version=2");
        let form: [(&str, &str); 2] = [("logout", "1"), ("sid", &session_id.to_string())];

        let now = Instant::now();
        let req = self.client.post(&url).form(&form);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        let elapsed_ms = now.elapsed().as_secs_f64() * 1e3;
        log::debug!("logout ({:.2}ms):\n{}", elapsed_ms, text);

        Ok(())
    }

    pub async fn logs(&self) -> anyhow::Result<Vec<LogEntry>> {
        let url = self.make_url("/data.lua");
        let session_id = self.check_or_renew_session_id().await?.to_string();
        let form: [(&str, &str); 4] = [
            ("page", "log"),
            ("lang", "de"),
            ("filter", "0"),
            ("sid", &session_id),
        ];

        let now = Instant::now();
        let req = self.client.post(&url).form(&form);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        let elapsed_ms = now.elapsed().as_secs_f64() * 1e3;
        log::debug!("logs ({:.2}ms):\n{}", elapsed_ms, text);

        LogEntry::from_json(&text)
    }
}
