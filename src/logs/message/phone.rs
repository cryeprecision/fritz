use crate::logs::traits::FromLogMsg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneMsg {
    Unknown,
}

impl FromLogMsg for PhoneMsg {
    type Err = ();
    fn from_log_msg(_msg: &str) -> Result<Self, Self::Err> {
        Ok(Self::Unknown)
    }
}
