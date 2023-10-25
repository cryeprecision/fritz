use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context;
use chrono::Local;
use parking_lot::Mutex;
use reqwest::tls::Version;
use reqwest::{Method, RequestBuilder};

use super::{LoginChallenge, SessionId};
use crate::{api, db, fritz};

fn elapsed_ms(start: &Instant) -> i64 {
    start.elapsed().as_millis().max(i64::MAX as u128) as i64
}

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
    /// Path to save responses to
    save_response_path: Option<PathBuf>,
    /// Database
    database: Option<db::Database>,
}

impl Client {
    pub async fn new(
        domain: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        root_cert: Option<&[u8]>,
        pool: Option<&db::Database>,
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

        let save_response_path = Self::save_response_path().await;

        Ok(Client {
            client,
            domain,
            session_id: Mutex::new(None),
            username,
            password,
            save_response_path,
            database: pool.cloned(),
        })
    }

    /// Determine path to save responses to from environment variables.
    pub async fn save_response_path() -> Option<PathBuf> {
        let Ok(save_response) = dotenv::var("FRITZBOX_SAVE_RESPONSE") else {
            return None;
        };
        let Ok(save_response) = save_response.parse::<bool>() else {
            log::warn!("couldn't parse FRITZBOX_SAVE_RESPONSE as bool");
            return None;
        };
        if !save_response {
            return None;
        }

        let Ok(save_response_path) = dotenv::var("FRITZBOX_SAVE_RESPONSE_PATH") else {
            log::warn!("missing env var FRITZBOX_SAVE_RESPONSE_PATH");
            return None;
        };

        let save_response_path = PathBuf::from(save_response_path);
        match tokio::fs::metadata(&save_response_path).await {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    log::warn!("FRITZBOX_SAVE_RESPONSE_PATH does not point to a folder");
                    return None;
                }
                Some(save_response_path)
            }
            Err(_) => {
                if let Err(err) = tokio::fs::create_dir(&save_response_path).await {
                    log::warn!(
                        "couldn't create folder to FRITZBOX_SAVE_RESPONSE_PATH: {:?}",
                        err
                    );
                    None
                } else {
                    log::info!("created folder to FRITZBOX_SAVE_RESPONSE_PATH");
                    Some(save_response_path)
                }
            }
        }
    }

    pub async fn save_response(&self, name: &str, text: &str) {
        let Some(mut path) = self.save_response_path.as_ref().cloned() else {
            return;
        };

        let now = Local::now().format("%Y-%m-%d_%H-%M-%S.%3f");
        path.push(format!("response_{}_{}.txt", now, name));

        if let Err(err) = tokio::fs::write(&path, text).await {
            log::warn!("couldn't save {}: {:?}", path.to_string_lossy(), err);
        }
    }

    pub async fn request_with_inner<F>(
        &self,
        name: &str,
        url: &str,
        method: Method,
        func: F,
        meta: &mut db::Request,
    ) -> anyhow::Result<String>
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        meta.url = url.to_string();
        meta.method = method.to_string();
        meta.datetime = db::util::local_to_utc_timestamp(Local::now());

        let now = Instant::now();
        let mut builder = self.client.request(method.clone(), url);
        builder = func(builder);

        let resp = builder.send().await.context("send request")?;
        meta.response_code = Some(i64::from(resp.status().as_u16()));

        if let Err(err) = resp.error_for_status_ref() {
            meta.duration_ms = elapsed_ms(&now);
            return Err(err).context("response status non 2XX");
        }

        let text = resp.text().await;
        meta.duration_ms = elapsed_ms(&now);
        meta.session_id = (*self.session_id.lock()).map(|id| id.to_string());
        let text = text.context("response code non 2XX")?;

        log::info!(
            "{} request to {} ({:?} - {}) took {}ms (session-id: {:?})",
            name,
            meta.url,
            meta.method,
            meta.response_code.unwrap_or(0),
            meta.duration_ms,
            meta.session_id,
        );

        self.save_response(name, &text).await;

        Ok(text)
    }

    pub async fn request_with<F>(
        &self,
        name: &str,
        url: &str,
        method: Method,
        func: F,
    ) -> anyhow::Result<String>
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        let mut meta = db::Request::default();

        let resp = self
            .request_with_inner(name, url, method, func, &mut meta)
            .await;

        if let Some(database) = self.database.as_ref() {
            if let Err(err) = database.insert_request(&meta).await {
                log::warn!("couldn't insert request metadata: {}", err);
            }
        }

        resp
    }

    /// Example: `client.make_url("/cgi-bin/firmwarecfg")` will produce
    /// `https://{host}/cgi-bin/firmwarecfg`
    pub fn make_url(&self, path: &str) -> String {
        format!("https://{}{}", self.domain, path)
    }

    pub async fn check_or_renew_session_id(&self) -> anyhow::Result<SessionId> {
        match self.check_session_id().await? {
            None => self.login().await,
            Some(session_id) => Ok(session_id),
        }
    }

    async fn check_session_id(&self) -> anyhow::Result<Option<SessionId>> {
        // We don't have a SessionId yet
        let Some(session_id) = *self.session_id.lock() else {
            return Ok(None);
        };

        // We have a session id, verify it
        let url = self.make_url("/login_sid.lua?version=2");
        let form: [(&str, &str); 1] = [("sid", &session_id.to_string())];

        let text = self
            .request_with("check-session-id", &url, Method::POST, |req| {
                req.form(&form)
            })
            .await?;

        let challenge =
            LoginChallenge::from_xml_text(&text).context("couldn't parse response xml")?;
        Ok(challenge
            .session_id
            .and_then(|id| if id == session_id { Some(id) } else { None }))
    }

    /// Get the login challenge
    pub async fn login_challenge(&self) -> anyhow::Result<LoginChallenge> {
        let url = self.make_url("/login_sid.lua?version=2");
        let text = self
            .request_with("login-challenge", &url, Method::GET, |req| req)
            .await?;
        Ok(LoginChallenge::from_xml_text(&text)?)
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
        let response = challenge.make_response(&self.password).to_string();
        let url = self.make_url("/login_sid.lua?version=2");
        let form: [(&str, &str); 2] = [("username", &self.username), ("response", &response)];

        let text = self
            .request_with("login-response", &url, Method::POST, |req| req.form(&form))
            .await?;

        Ok(LoginChallenge::from_xml_text(&text)?)
    }

    /// Create a new session, doesn't check for an existing one.
    pub async fn login(&self) -> anyhow::Result<SessionId> {
        // get the challenge
        let login_challenge = self.login_challenge().await?;
        // respond with the correct response
        let response = self.login_response(&login_challenge).await?;
        // get the session id
        let session_id = response.session_id.context("missing session id")?;

        *self.session_id.lock() = Some(session_id);
        Ok(session_id)
    }

    /// Destroy the current session if there is one.
    pub async fn logout(&self) -> anyhow::Result<()> {
        let Some(session_id) = self.check_session_id().await? else {
            *self.session_id.lock() = None;
            return Ok(());
        };

        let url = self.make_url("/login_sid.lua?version=2");
        let form: [(&str, &str); 2] = [("logout", "1"), ("sid", &session_id.to_string())];

        let _ = self
            .request_with("logout", &url, Method::POST, |req| req.form(&form))
            .await?;

        *self.session_id.lock() = None;
        Ok(())
    }

    /// Get the current certificate from the FRITZ!Box.
    pub async fn box_cert(&self) -> anyhow::Result<String> {
        let url = self.make_url("/cgi-bin/firmwarecfg");
        let session_id = self.check_or_renew_session_id().await?.to_string();
        let form = reqwest::multipart::Form::new()
            .text("sid", session_id)
            .text("BoxCertExport", "");

        let text = self
            .request_with("box-cert", &url, Method::POST, |req| req.multipart(form))
            .await?;

        Ok(text)
    }

    /// Clear the logs on the FRITZ!Box.
    pub async fn clear_logs(&self) -> anyhow::Result<serde_json::Value> {
        let url = self.make_url("/data.lua");
        let session_id = self.check_or_renew_session_id().await?.to_string();
        let form: [(&str, &str); 6] = [
            ("xhr", "1"),
            ("sid", &session_id),
            ("page", "log"),
            ("lang", "de"),
            ("xhrId", "del"),
            ("del", "1"),
        ];

        let text = self
            .request_with("clear-logs", &url, Method::POST, |req| req.form(&form))
            .await?;

        serde_json::from_str(&text).context("parse json")
    }

    /// Fetch logs from the FRITZ!Box.
    ///
    /// API returns logs ordered from **new to old** so the **newest log is at index 0**.
    pub async fn logs(&self) -> anyhow::Result<Vec<fritz::Log>> {
        let url = self.make_url("/data.lua");
        let session_id = self.check_or_renew_session_id().await?.to_string();
        let form: [(&str, &str); 6] = [
            ("xhr", "1"),
            ("page", "log"),
            ("lang", "de"),
            ("filter", "0"),
            ("sid", &session_id),
            ("xhrId", "all"),
        ];

        let text = self
            .request_with("logs", &url, Method::POST, |req| req.form(&form))
            .await?;

        let logs: Vec<api::Log> = serde_json::from_str::<api::Response>(&text)
            .context("parse response json")?
            .data
            .logs;

        logs.into_iter().map(fritz::Log::try_from).collect()
    }
}
