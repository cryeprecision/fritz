use crate::logs::traits::FromLogMsg;

#[derive(Debug)]
pub enum SystemMsg {
    Unknown,
}

impl FromLogMsg for SystemMsg {
    type Err = ();
    fn from_log_msg(_msg: &str) -> Result<Self, Self::Err> {
        Ok(Self::Unknown)
    }
}
