use std::time::Duration;
use std::{fmt::Display, str::FromStr};

use anyhow::{Context, Result};
use log::warn;
use reqwest::Client;
use roxmltree::Document;
use thiserror::Error;

use super::{challenge::Challenge, xml};

#[derive(Debug)]
pub struct Session {
    pub id: [u8; 8],
}

impl Display for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.id))
    }
}

#[derive(Debug, Error)]
pub enum SessionParseError {
    #[error("invalid id: {0}")]
    Id(#[from] hex::FromHexError),
    #[error("zero id is never valid")]
    Zero,
}

impl FromStr for Session {
    type Err = SessionParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut id = [0u8; 8];
        hex::decode_to_slice(s, &mut id)?;
        if id.iter().all(|&b| b == 0) {
            return Err(SessionParseError::Zero);
        }
        Ok(Session { id })
    }
}

impl Session {
    fn xml_get_challenge(doc: &Document) -> Result<Challenge> {
        let session_info = xml::find_node_by_tag(doc.root(), "SessionInfo")?;
        let challenge = xml::find_text_by_tag(session_info, "Challenge")?;
        Challenge::from_str(challenge).context("couldn't parse challenge from xml")
    }
    fn xml_get_block_time(doc: &Document) -> Result<u32> {
        let session_info = xml::find_node_by_tag(doc.root(), "SessionInfo")?;
        let block_time = xml::find_text_by_tag(session_info, "BlockTime")?;
        block_time
            .parse::<u32>()
            .context("couldn't parse block time from xml")
    }
    fn xml_get_session_id(doc: &Document) -> Result<Session> {
        let session_info = xml::find_node_by_tag(doc.root(), "SessionInfo")?;
        let session_id = xml::find_text_by_tag(session_info, "SID")?;
        Session::from_str(session_id).context("couldn't parse session id from xml")
    }
    fn xml_get_users(doc: &Document) -> Result<Vec<String>> {
        let session_info = xml::find_node_by_tag(doc.root(), "SessionInfo")?;
        let users = xml::find_node_by_tag(session_info, "Users")?;
        Ok(users
            .children()
            .filter(|n| n.has_tag_name("User"))
            .map(|n| n.text().unwrap().to_string())
            .collect())
    }

    pub async fn get_challenge(client: &Client) -> Result<Challenge> {
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";
        let req = client.get(URL);
        let resp = req.send().await?.text().await?;
        let xml = Document::parse(&resp).context("couldn't parse challenge response xml")?;
        Self::xml_get_challenge(&xml)
    }
    pub async fn get_challenge_and_users(client: &Client) -> Result<(Challenge, Vec<String>)> {
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";
        let req = client.get(URL);
        let resp = req.send().await?.text().await?;
        let xml = Document::parse(&resp).context("couldn't parse challenge response xml")?;
        Ok((Self::xml_get_challenge(&xml)?, Self::xml_get_users(&xml)?))
    }
    pub async fn get_session_id(
        client: &Client,
        username: &str,
        response: &str,
    ) -> Result<Session> {
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";

        let form = [
            ("username", username.to_string()),
            ("response", response.to_string()),
        ];

        let req = client.post(URL).form(&form);
        // request body is not a stream so it should always be cloneable
        let resp = req.try_clone().unwrap().send().await?.text().await?;
        let xml = Document::parse(&resp).context("couldn't parse session id response xml")?;

        let block_time = Self::xml_get_block_time(&xml)?;
        Ok(if block_time > 0 {
            warn!("sleeping for {block_time}s");
            tokio::time::sleep(Duration::from_secs(block_time as u64)).await;
            let resp = req.send().await?.text().await?;
            let xml = Document::parse(&resp).context("couldn't parse session id response xml")?;
            Self::xml_get_session_id(&xml).context("received invalid session id")?
        } else {
            Self::xml_get_session_id(&xml).context("received invalid session id")?
        })
    }
    pub async fn is_valid(&self, client: &Client) -> Result<bool> {
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";

        let form = [("sid", hex::encode(self.id))];

        let req = client.post(URL).form(&form);
        let _resp = req.send().await?.text().await?;
        unimplemented!()
    }
    pub async fn login(client: &Client, username: &str, password: &[u8]) -> Result<Session> {
        let ch = Self::get_challenge(client).await?;
        Self::get_session_id(client, username, &ch.response(password).to_string()).await
    }
    pub async fn logout(self, client: &Client) -> Result<()> {
        const URL: &str = "https://fritz.box/login_sid.lua?version=2";

        let form = [("logout", "1".to_string()), ("sid", hex::encode(self.id))];

        let req = client.post(URL).form(&form);
        Ok(req.send().await?.text().await.map(|_| ())?)
    }
}

#[cfg(test)]
mod tests {
    use roxmltree::Document;

    use crate::login::session::Session;

    const XML: &str = r#"
<SessionInfo>
    <SID>0de8afc227e5abeb</SID>
    <Challenge>2$60000$d4949767019d1e6eed27c27f404c7aa7$6000$4f3415a3b5396a9675d08906ee6a6933</Challenge>
    <BlockTime>32</BlockTime>
    <Users>
        <User last="1">fritz3713</User>
    </Users>
</SessionInfo>
    "#;

    #[test]
    fn parse_challenge_xml() {
        let doc = Document::parse(XML).unwrap();
        let ch = Session::xml_get_challenge(&doc).unwrap();

        assert_eq!(ch.statick.iterations, 60000);
        assert_eq!(ch.dynamic.iterations, 6000);

        assert_eq!(
            ch.statick.salt,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            ch.dynamic.salt,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );
    }

    #[test]
    fn parse_session_id_xml() {
        let doc = Document::parse(XML).unwrap();
        let ch = Session::xml_get_session_id(&doc).unwrap();

        assert_eq!(ch.id, [13, 232, 175, 194, 39, 229, 171, 235]);
    }

    #[test]
    fn parse_block_time_xml() {
        let doc = Document::parse(XML).unwrap();
        let block_time = Session::xml_get_block_time(&doc).unwrap();

        assert_eq!(block_time, 32);
    }

    #[test]
    fn parse_users_xml() {
        let doc = Document::parse(XML).unwrap();
        let users = Session::xml_get_users(&doc).unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0], "fritz3713");
    }
}
