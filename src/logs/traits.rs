use super::log_entry::RawLogEntry;

pub trait FromLogMsg
where
    Self: Sized,
{
    type Err;
    fn from_log_msg(msg: &str) -> Result<Self, Self::Err>;
}

pub trait FromLogEntry
where
    Self: Sized,
{
    type Err;
    fn from_log_entry(entry: &RawLogEntry) -> Result<Self, Self::Err>;
}
