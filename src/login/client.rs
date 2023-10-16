use std::path::Path;

use crate::{logs::LogEntry, Response, SessionId, SessionResponse};

use anyhow::{anyhow, Context};

pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Client {
        Client {
            // TODO: Add FRITZ!Box root cert
            inner: reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        }
    }
    pub async fn new_with_cert(cert: impl AsRef<Path>) -> anyhow::Result<Client> {
        let cert = tokio::fs::read_to_string(cert.as_ref())
            .await
            .context("couldn't read certificate")?;

        let cert =
            reqwest::Certificate::from_pem(cert.as_bytes()).context("certificate is invalid")?;

        Ok(Client {
            inner: reqwest::Client::builder()
                .add_root_certificate(cert)
                .build()
                .unwrap(),
        })
    }

    pub async fn certificate(&self, session_id: &SessionId) -> reqwest::Result<String> {
        const URL: &str = "https://fritzbox.home.arpa/cgi-bin/firmwarecfg";

        let form = reqwest::multipart::Form::new()
            .text("sid", session_id.to_string())
            .text("BoxCertExport", "");

        let req = self.inner.post(URL).multipart(form);
        let resp = req.send().await?;
        resp.error_for_status()?.text().await
    }
    pub async fn is_session_id_valid(&self, session_id: &SessionId) -> reqwest::Result<bool> {
        const URL: &str = "https://fritzbox.home.arpa/login_sid.lua?version=2";

        let form: [(&str, &str); 1] = [("sid", &session_id.to_string())];

        let req = self.inner.post(URL).form(&form);
        let resp = req.send().await?;
        let _text = resp.error_for_status()?.text().await?;

        unimplemented!()
    }
    pub async fn session_response(&self) -> anyhow::Result<SessionResponse> {
        SessionResponse::fetch_challenge(&self.inner).await
    }
    pub async fn login(&self, username: &str, password: &[u8]) -> anyhow::Result<SessionId> {
        let ch_resp = SessionResponse::fetch_challenge(&self.inner).await?;
        let response = ch_resp.challenge.response(password);
        let auth_resp = SessionResponse::fetch_session_id(&self.inner, username, response).await?;
        auth_resp
            .session_id
            .ok_or(anyhow!("no session id after authenticating"))
    }
    pub async fn session_id(
        &self,
        username: &str,
        response: Response,
    ) -> anyhow::Result<SessionId> {
        SessionResponse::fetch_session_id(&self.inner, username, response)
            .await?
            .session_id
            .ok_or(anyhow!("no session id after authenticating"))
    }
    pub async fn logout(&self, session_id: SessionId) -> reqwest::Result<()> {
        const URL: &str = "https://fritzbox.home.arpa/login_sid.lua?version=2";

        let form: [(&str, &str); 2] = [("logout", "1"), ("sid", &session_id.to_string())];

        let req = self.inner.post(URL).form(&form);
        let resp = req.send().await?;
        resp.error_for_status()?.text().await.map(|_| ())
    }
    pub async fn reboot(&self, session_id: &SessionId) -> reqwest::Result<()> {
        const URL: &str = "https://fritzbox.home.arpa/data.lua";

        let form: [(&str, &str); 2] = [("sid", &session_id.to_string()), ("reboot", "1")];

        let req = self.inner.post(URL).form(&form);
        let resp = req.send().await?;
        resp.error_for_status()?.text().await.map(|_| ())
    }
    pub async fn logs(&self, session_id: &SessionId) -> anyhow::Result<Vec<LogEntry>> {
        const URL: &str = "https://fritzbox.home.arpa/data.lua";

        let form: [(&str, &str); 4] = [
            ("page", "log"),
            ("lang", "de"),
            ("filter", "0"),
            ("sid", &session_id.to_string()),
        ];

        let req = self.inner.post(URL).form(&form);
        let resp = req.send().await?;
        let text = resp.error_for_status()?.text().await?;
        LogEntry::from_json(&text)
    }
}
