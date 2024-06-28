use std::borrow::Cow;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::Context;
use serde::{Deserialize, Deserializer};

use super::challenge::{self, Challenge};

const INVALID_SESSION_ID: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct User {
    #[serde(rename = "@last")]
    #[serde(deserialize_with = "de::deserialize_last")]
    #[serde(default)]
    pub is_last: bool,
    #[serde(rename = "$text")]
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SessionInfo {
    #[serde(rename = "SID")]
    pub session_id: SessionId,
    #[serde(rename = "Challenge")]
    pub challenge: Challenge,
    #[serde(rename = "BlockTime")]
    pub block_time: u64,
    #[serde(rename = "Users")]
    #[serde(deserialize_with = "de::unwrap_users")]
    #[serde(default)]
    pub users: Vec<User>,
}

impl SessionInfo {
    pub fn make_response(&self, password: &str) -> challenge::Response {
        self.challenge.make_response(password)
    }
    pub fn from_xml(xml: &str) -> anyhow::Result<SessionInfo> {
        quick_xml::de::from_str(xml).context("parse session info xml")
    }
    pub fn has_user(&self, username: &str) -> bool {
        self.users.iter().any(|user| user.name == username)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SessionId {
    /// Actual SessionId
    pub id: [u8; 8],
}

impl SessionId {
    pub fn is_valid(&self) -> bool {
        self.id != INVALID_SESSION_ID
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.id))
    }
}

impl FromStr for SessionId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<SessionId> {
        let mut id = [0u8; 8];
        hex::decode_to_slice(s, &mut id).context("decode session id")?;
        Ok(SessionId { id })
    }
}
impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sid = Cow::<'de, str>::deserialize(deserializer)?;
        sid.parse().map_err(serde::de::Error::custom)
    }
}

mod de {
    use serde::{Deserialize, Deserializer};

    use super::{SessionId, User, INVALID_SESSION_ID};

    pub fn unwrap_users<'de, D>(deserializer: D) -> Result<Vec<User>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Users {
            #[serde(rename = "User")]
            users: Vec<User>,
        }

        Ok(Users::deserialize(deserializer)?.users)
    }
    pub fn deserialize_last<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match <Option<u8>>::deserialize(deserializer)? {
            None => false,
            Some(1) => true,
            Some(_) => false,
        })
    }
    #[allow(dead_code)]
    pub fn deserialize_session_id_opt<'de, D>(
        deserializer: D,
    ) -> Result<Option<SessionId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match SessionId::deserialize(deserializer)?.id {
            INVALID_SESSION_ID => None,
            id => Some(SessionId { id }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionInfo, User};

    #[test]
    fn parse_xml_error() {
        const XML: &str = r#"
<SessionInfo>
    <SID>0000000000000000</SID>
    <Challenge>2$60000$d4949767019d1e6eed27c27f404c7aa7$6000$4f3415a3b5396a9675d08906ee6a6933</Challenge>
    <BlockTime>12</BlockTime>
    <Rights></Rights>
    <Users>
        <User last="1">fritz3713</User>
    </Users>
</SessionInfo>
            "#;

        let resp: SessionInfo = quick_xml::de::from_str(XML).unwrap();

        assert!(!resp.session_id.is_valid());

        assert_eq!(resp.challenge.rounds_1, 60000);
        assert_eq!(resp.challenge.rounds_2, 6000);
        assert_eq!(
            resp.challenge.salt_1,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            resp.challenge.salt_2,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );

        assert_eq!(resp.block_time, 12);
        assert_eq!(
            resp.users,
            [User {
                is_last: true,
                name: "fritz3713".to_string()
            }]
        );
    }

    #[test]
    fn parse_xml_success() {
        const XML_SUCCESS: &str = r#"
<SessionInfo>
    <SID>0de8afc227e5abeb</SID>
    <Challenge>2$60000$d4949767019d1e6eed27c27f404c7aa7$6000$4f3415a3b5396a9675d08906ee6a6933</Challenge>
    <BlockTime>0</BlockTime>
    <Rights>
        <Name>Dial</Name>
        <Access>2</Access>
        <Name>App</Name>
        <Access>2</Access>
        <Name>HomeAuto</Name>
        <Access>2</Access>
        <Name>BoxAdmin</Name>
        <Access>2</Access>
        <Name>Phone</Name>
        <Access>2</Access>
        <Name>NAS</Name>
        <Access>2</Access>
    </Rights>
    <Users>
        <User last="1">fritz3713</User>
    </Users>
</SessionInfo>
        "#;

        let resp: SessionInfo = quick_xml::de::from_str(XML_SUCCESS).unwrap();

        assert_eq!(resp.session_id.id, [13, 232, 175, 194, 39, 229, 171, 235]);

        assert_eq!(resp.challenge.rounds_1, 60000);
        assert_eq!(resp.challenge.rounds_2, 6000);
        assert_eq!(
            resp.challenge.salt_1,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            resp.challenge.salt_2,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );

        assert_eq!(resp.block_time, 0);
        assert_eq!(
            resp.users,
            [User {
                is_last: true,
                name: "fritz3713".to_string()
            }]
        );
    }

    #[test]
    fn parse_xml_serde() {
        const XML_SUCCESS: &str = r#"
<SessionInfo>
    <SID>0de8afc227e5abeb</SID>
    <Challenge>2$60000$d4949767019d1e6eed27c27f404c7aa7$6000$4f3415a3b5396a9675d08906ee6a6933</Challenge>
    <BlockTime>0</BlockTime>
    <Rights>
        <Name>Dial</Name>
        <Access>2</Access>
        <Name>App</Name>
        <Access>2</Access>
        <Name>HomeAuto</Name>
        <Access>2</Access>
        <Name>BoxAdmin</Name>
        <Access>2</Access>
        <Name>Phone</Name>
        <Access>2</Access>
        <Name>NAS</Name>
        <Access>2</Access>
    </Rights>
    <Users>
        <User last="1">fritz3713</User>
        <User>fritz3714</User>
    </Users>
</SessionInfo>
        "#;

        let parsed: SessionInfo = quick_xml::de::from_str(XML_SUCCESS).unwrap();
        println!("{:#?}", parsed);
    }
}
