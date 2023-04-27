use crate::{logs::LogEntry, Response, SessionId, SessionResponse};

use anyhow::anyhow;

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
    pub async fn is_session_id_valid(&self, session_id: &SessionId) -> reqwest::Result<bool> {
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";

        let form: [(&str, &str); 1] = [("sid", &session_id.to_string())];

        let req = self.inner.post(URL).form(&form);
        let _resp = req.send().await?.text().await?;

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
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";

        let form: [(&str, &str); 2] = [("logout", "1"), ("sid", &session_id.to_string())];

        let req = self.inner.post(URL).form(&form);
        req.send().await?.text().await.map(|_| ())
    }
    pub async fn reboot(&self, session_id: &SessionId) -> reqwest::Result<()> {
        const URL: &str = "https://fritz.box/reboot.lua";

        let form: [(&str, &str); 1] = [("sid", &session_id.to_string())];

        let req = self.inner.post(URL).form(&form);
        req.send().await?.text().await.map(|_| ())
    }
    pub async fn logs(&self, session_id: &SessionId) -> anyhow::Result<Vec<LogEntry>> {
        const URL: &str = "https://fritz.box/data.lua";

        let form: [(&str, &str); 4] = [
            ("page", "log"),
            ("lang", "de"),
            ("filter", "0"),
            ("sid", &session_id.to_string()),
        ];

        let req = self.inner.post(URL).form(&form);
        let resp = req.send().await?.text().await?;
        LogEntry::from_json(&resp)
    }
}
