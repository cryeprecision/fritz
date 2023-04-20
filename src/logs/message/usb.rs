use crate::logs::traits::FromLogMsg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbMsg {
    Unknown,
}

impl FromLogMsg for UsbMsg {
    type Err = ();
    fn from_log_msg(_msg: &str) -> Result<Self, Self::Err> {
        Ok(Self::Unknown)
    }
}
