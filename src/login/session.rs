use std::fmt::Display;
use std::num::ParseIntError;
use std::str::FromStr;

use roxmltree::{Document, Node};
use thiserror::Error;

use super::challenge::Challenge;
use crate::xml::{find_node_by_tag, find_text_by_tag};
use crate::{ChallengeParseError, Response};

/// `<Access>`
#[derive(Debug, PartialEq, Eq)]
pub enum Permission {
    /// `1`
    ReadOnly,
    /// `2`
    ReadWrite,
}

#[derive(Debug, Error)]
pub enum PermissionParseError {
    #[error("couldn't parse integer number")]
    Parse(#[from] ParseIntError),
    #[error("number doesn't correspond to a permission")]
    OutOfRange,
}

impl FromStr for Permission {
    type Err = PermissionParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.parse::<i32>()? {
            1 => Ok(Permission::ReadOnly),
            2 => Ok(Permission::ReadWrite),
            _ => Err(PermissionParseError::OutOfRange),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Permissions {
    /// `<Name>Dial</Name>`
    dial: Permission,
    /// `<Name>App</Name>`
    app: Permission,
    /// `<Name>HomeAuto</Name>`
    home_auto: Permission,
    /// `<Name>BoxAdmin</Name>`
    box_admin: Permission,
    /// `<Name>Phone</Name>`
    phone: Permission,
    /// `<Name>NAS</Name>`
    nas: Permission,
}

#[derive(Debug, Error)]
pub enum PermissionsParseError {
    #[error("encountered a node without text")]
    NoText,
    #[error("unexpected number of nodes")]
    Length,
    #[error("unexpected permission name")]
    PermissionName,
    #[error("couldn't parse permission value")]
    PermissionValue(#[from] PermissionParseError),
}
type PermissionsParseResult<T> = std::result::Result<T, PermissionsParseError>;

impl Permissions {
    /// `node`: `<Rights>...</Rights>`
    pub fn from_rights_node(node: &Node) -> PermissionsParseResult<Option<Permissions>> {
        const EXPECTED_NODE_COUNT: usize = 12;
        const EXPECTED_NODE_NAMES: [&str; 6] =
            ["Dial", "App", "HomeAuto", "BoxAdmin", "Phone", "NAS"];

        if !node.has_children() {
            return Ok(None);
        }

        let values = node
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.text())
            .collect::<Option<Vec<_>>>()
            .ok_or(PermissionsParseError::NoText)?;

        if values.len() != EXPECTED_NODE_COUNT {
            return Err(PermissionsParseError::Length);
        }

        let mut result_iter = values.chunks_exact(2);
        let mut expected_name_iter = EXPECTED_NODE_NAMES.iter();
        let mut next = || -> PermissionsParseResult<Permission> {
            let kv = result_iter.next().unwrap();
            if kv[0] != *expected_name_iter.next().unwrap() {
                return Err(PermissionsParseError::PermissionName);
            }
            Ok(Permission::from_str(kv[1])?)
        };

        Ok(Some(Permissions {
            dial: next()?,
            app: next()?,
            home_auto: next()?,
            box_admin: next()?,
            phone: next()?,
            nas: next()?,
        }))
    }
}

#[derive(Debug)]
pub struct LoginChallenge {
    /// `<SID>`
    pub session_id: Option<SessionId>,
    /// `<Challenge>`
    pub challenge: Challenge,
    /// `<BlockTime>`
    pub block_time: u32,
    /// `<Rights>`
    pub permissions: Option<Permissions>,
    /// `<Users>`
    pub users: Vec<String>,
}

#[derive(Debug, Error)]
pub enum SessionResponseParseError {
    #[error("couldn't find node: {0}")]
    MissingNode(#[from] crate::xml::Error),
    #[error("user tag has no text content")]
    NoText,
    #[error("couldn't parse permissions: {0}")]
    Permissions(#[from] PermissionsParseError),
    #[error("couldn't parse session id: {0}")]
    SessionId(#[from] SessionIdParseError),
    #[error("couldn't parse challenge: {0}")]
    Challenge(#[from] ChallengeParseError),
    #[error("couldn't parse block time: {0}")]
    BlockTime(#[from] ParseIntError),
    #[error("text is not valid xml: {0}")]
    Xml(#[from] roxmltree::Error),
}
type SessionResponseParseResult<T> = std::result::Result<T, SessionResponseParseError>;

impl LoginChallenge {
    pub(crate) fn from_xml_text(xml: &str) -> SessionResponseParseResult<LoginChallenge> {
        Self::from_xml(&roxmltree::Document::parse(&xml)?)
    }

    pub(crate) fn from_xml(doc: &Document) -> SessionResponseParseResult<LoginChallenge> {
        let session_info = find_node_by_tag(doc.root(), "SessionInfo")?;
        let session_id = find_text_by_tag(session_info, "SID")?;
        let challenge = find_text_by_tag(session_info, "Challenge")?;
        let block_time = find_text_by_tag(session_info, "BlockTime")?;
        let rights = find_node_by_tag(session_info, "Rights")?;
        let users = find_node_by_tag(session_info, "Users")?;

        // Zero id corresponds to None
        let session_id = match SessionId::from_str(session_id) {
            Err(err) => match err {
                SessionIdParseError::Zero => Ok(None),
                err @ SessionIdParseError::Id(_) => Err(err),
            },
            Ok(id) => Ok(Some(id)),
        }?;

        let challenge = Challenge::from_str(challenge)?;
        let block_time = block_time.parse::<u32>()?;
        let permissions = Permissions::from_rights_node(&rights)?;
        let users = users
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("User"))
            .map(|n| n.text().map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or(SessionResponseParseError::NoText)?;

        Ok(LoginChallenge {
            session_id,
            challenge,
            block_time,
            permissions,
            users,
        })
    }

    pub fn make_response(&self, password: &str) -> Response {
        self.challenge.make_response(password)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SessionId {
    /// Actual SessionId
    id: [u8; 8],
}

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.id))
    }
}

#[derive(Debug, Error)]
pub enum SessionIdParseError {
    #[error("invalid id: {0}")]
    Id(#[from] hex::FromHexError),
    #[error("zero id is never valid")]
    Zero,
}

impl FromStr for SessionId {
    type Err = SessionIdParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut id = [0u8; 8];
        hex::decode_to_slice(s, &mut id)?;
        if id.iter().all(|&b| b == 0) {
            return Err(SessionIdParseError::Zero);
        }
        Ok(SessionId { id })
    }
}

#[cfg(test)]
mod tests {
    use roxmltree::Document;

    use crate::{LoginChallenge, Permission, Permissions};

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

        let doc = Document::parse(XML).unwrap();
        let resp = LoginChallenge::from_xml(&doc).unwrap();

        assert_eq!(resp.session_id, None);

        assert_eq!(resp.challenge.static_params.iterations, 60000);
        assert_eq!(resp.challenge.dynamic_params.iterations, 6000);
        assert_eq!(
            resp.challenge.static_params.salt,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            resp.challenge.dynamic_params.salt,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );

        assert_eq!(resp.block_time, 12);
        assert_eq!(resp.permissions, None);
        assert_eq!(resp.users, ["fritz3713"]);
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

        let doc = Document::parse(XML_SUCCESS).unwrap();
        let resp = LoginChallenge::from_xml(&doc).unwrap();

        assert_eq!(
            resp.session_id.map(|s| s.id),
            Some([13, 232, 175, 194, 39, 229, 171, 235])
        );

        assert_eq!(resp.challenge.static_params.iterations, 60000);
        assert_eq!(resp.challenge.dynamic_params.iterations, 6000);
        assert_eq!(
            resp.challenge.static_params.salt,
            [212, 148, 151, 103, 1, 157, 30, 110, 237, 39, 194, 127, 64, 76, 122, 167]
        );
        assert_eq!(
            resp.challenge.dynamic_params.salt,
            [79, 52, 21, 163, 181, 57, 106, 150, 117, 208, 137, 6, 238, 106, 105, 51]
        );

        assert_eq!(resp.block_time, 0);
        assert_eq!(
            resp.permissions,
            Some(Permissions {
                dial: Permission::ReadWrite,
                app: Permission::ReadWrite,
                home_auto: Permission::ReadWrite,
                box_admin: Permission::ReadWrite,
                phone: Permission::ReadWrite,
                nas: Permission::ReadWrite,
            })
        );
        assert_eq!(resp.users, ["fritz3713"]);
    }
}
