//! Exposes a `Client` struct to interact with the API.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Context;
use chrono::{Local, Utc};
use parking_lot::Mutex;
use reqwest::tls::Version;
use reqwest::{Method, RequestBuilder};

use super::{model, SessionId, SessionInfo};
use crate::{db, fritz};

fn elapsed_ms(start: &Instant) -> i64 {
    start.elapsed().as_millis().min(i64::MAX as u128) as i64
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
    /// Create a new client to interact with the FRITZ!Box API.
    ///
    /// Parameters that are `None` will be with their environment variables
    /// counterpart.
    pub async fn new(
        domain: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        root_cert: Option<&[u8]>,
        pool: Option<&db::Database>,
    ) -> anyhow::Result<Client> {
        fn resolve_var(key: &str, default: Option<&str>) -> anyhow::Result<String> {
            match default {
                None => dotenv::var(key).with_context(|| format!("couldn't find env var {}", key)),
                Some(s) => Ok(s.to_string()),
            }
        }

        fn resolve_root_cert(
            key: &str,
            default: Option<&[u8]>,
        ) -> anyhow::Result<reqwest::Certificate> {
            let bytes = match default {
                None => {
                    let path = dotenv::var(key)
                        .with_context(|| format!("couldn't find env var {}", key))?;
                    std::fs::read(&path)
                        .with_context(|| format!("couldn't find root cert at {}", path))
                }
                Some(b) => Ok(b.to_vec()),
            }?;
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
    async fn save_response_path() -> Option<PathBuf> {
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

    async fn save_response(&self, name: &str, text: &str) {
        let Some(mut path) = self.save_response_path.as_ref().cloned() else {
            return;
        };

        let now = Local::now().format("%Y-%m-%d_%H-%M-%S.%3f");
        path.push(format!("response_{}_{}.txt", now, name));

        if let Err(err) = tokio::fs::write(&path, text).await {
            log::warn!("couldn't save {}: {:?}", path.to_string_lossy(), err);
        }
    }

    async fn request_with_inner<F>(
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
        meta.datetime = Utc::now();
        meta.name = name.to_string();
        meta.url = url.to_string();
        meta.method = method.to_string();

        let now = Instant::now();
        let mut builder = self.client.request(method.clone(), url);
        builder = func(builder);

        let resp = builder.send().await.context("send request")?;
        meta.response_code = Some(resp.status().as_u16().into());

        if let Err(err) = resp.error_for_status_ref() {
            meta.duration_ms = elapsed_ms(&now);
            return Err(err).context("response status non 2XX");
        }

        let text = resp.text().await;
        meta.duration_ms = elapsed_ms(&now);
        meta.session_id = (*self.session_id.lock()).map(|id| id.to_string());
        let text = text.context("response code non 2XX")?;

        log::info!(
            "{} request to {} ({} - {}) took {}ms (session-id: {:?})",
            meta.name,
            meta.url,
            meta.method,
            meta.response_code.unwrap_or(-1),
            meta.duration_ms,
            meta.session_id,
        );

        self.save_response(name, &text).await;

        Ok(text)
    }

    async fn request_with<F>(
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
                log::warn!("couldn't insert request metadata: {}: {:#?}", err, meta);
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
        let resp_session_id = SessionInfo::from_xml(&text)?.session_id;

        Ok(
            if !resp_session_id.is_valid() || resp_session_id != session_id {
                None
            } else {
                Some(resp_session_id)
            },
        )
    }

    /// Get the login challenge
    async fn login_challenge(&self) -> anyhow::Result<SessionInfo> {
        let url = self.make_url("/login_sid.lua?version=2");

        let text = self
            .request_with("login-challenge", &url, Method::GET, |req| req)
            .await?;

        SessionInfo::from_xml(&text)
    }

    /// Login by sending the correct response for the given challenge
    async fn login_response(&self, challenge: &SessionInfo) -> anyhow::Result<SessionInfo> {
        // check for username present in users
        if !challenge.has_user(&self.username) {
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

        SessionInfo::from_xml(&text)
    }

    /// Create a new session, doesn't check for an existing one.
    pub async fn login(&self) -> anyhow::Result<SessionId> {
        // get the challenge
        let login_challenge = self.login_challenge().await?;
        // respond with the correct response
        let response = self.login_response(&login_challenge).await?;
        // check returned session id
        if !response.session_id.is_valid() {
            return Err(anyhow::anyhow!(
                "invalid session id after login ({})",
                response.session_id
            ));
        }

        *self.session_id.lock() = Some(response.session_id);
        Ok(response.session_id)
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
    pub async fn certificate(&self) -> anyhow::Result<String> {
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

        let logs: Vec<model::Log> = serde_json::from_str::<model::Response>(&text)
            .context("parse response json")?
            .data
            .logs;

        logs.into_iter().map(fritz::Log::try_from).collect()
    }
}
