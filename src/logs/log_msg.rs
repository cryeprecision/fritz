use std::num::ParseIntError;
use std::str::FromStr;

use thiserror::Error;

use super::log_entry::RawLogEntry;
use super::message::InternetMsg;
use super::message::PhoneMsg;
use super::message::SystemMsg;
use super::message::UsbMsg;
use super::message::WlanMsg;
use super::traits::{FromLogEntry, FromLogMsg};

/// Only the message part of the log entry
#[derive(Debug)]
pub enum LogMsg {
    /// German: `System`
    System(SystemMsg),
    /// German: `Internetverbindung`
    Internet(InternetMsg),
    /// German: `Telefonie`
    Phone(PhoneMsg),
    /// German: `WLAN`
    Wlan(WlanMsg),
    /// German: `USB-Geräte`
    Usb(UsbMsg),
}

#[derive(Debug, Error)]
pub enum ParseLogMsgError {
    #[error("couldn't parse msg kind number")]
    CategoryParse(ParseIntError),
    #[error("message kind number `{0}` is out of range")]
    CategoryOutOfRange(u32),
    #[error("couldn't parse system message")]
    SystemMsgError,
    #[error("couldn't parse internet message")]
    InternetMsgError,
    #[error("couldn't parse phone message")]
    PhoneMsgError,
    #[error("couldn't parse wlan message")]
    WlanMsgError,
    #[error("couldn't parse usb message")]
    UsbMsgError,
}
type ParseLogMsgResult<T> = std::result::Result<T, ParseLogMsgError>;

impl FromLogEntry for LogMsg {
    type Err = ParseLogMsgError;
    fn from_log_entry(entry: &RawLogEntry) -> Result<Self, Self::Err> {
        match u32::from_str(&entry.category).map_err(ParseLogMsgError::CategoryParse)? {
            1 => Ok(LogMsg::System(
                SystemMsg::from_log_msg(&entry.msg)
                    .map_err(|_| ParseLogMsgError::SystemMsgError)?,
            )),
            2 => Ok(LogMsg::Internet(
                InternetMsg::from_log_msg(&entry.msg)
                    .map_err(|_| ParseLogMsgError::InternetMsgError)?,
            )),
            3 => Ok(LogMsg::Phone(
                PhoneMsg::from_log_msg(&entry.msg).map_err(|_| ParseLogMsgError::PhoneMsgError)?,
            )),
            4 => Ok(LogMsg::Wlan(
                WlanMsg::from_log_msg(&entry.msg).map_err(|_| ParseLogMsgError::WlanMsgError)?,
            )),
            5 => Ok(LogMsg::Usb(
                UsbMsg::from_log_msg(&entry.msg).map_err(|_| ParseLogMsgError::UsbMsgError)?,
            )),
            num => Err(ParseLogMsgError::CategoryOutOfRange(num)),
        }
    }
}

impl LogMsg {
    pub fn is_system(&self) -> bool {
        matches!(self, LogMsg::System(_))
    }
    pub fn is_internet(&self) -> bool {
        matches!(self, LogMsg::Internet(_))
    }
    pub fn is_phone(&self) -> bool {
        matches!(self, LogMsg::Phone(_))
    }
    pub fn is_wlan(&self) -> bool {
        matches!(self, LogMsg::Wlan(_))
    }
    pub fn is_usb(&self) -> bool {
        matches!(self, LogMsg::Usb(_))
    }

    pub fn system(&self) -> Option<&SystemMsg> {
        match self {
            Self::System(msg) => Some(msg),
            _ => None,
        }
    }
    pub fn internet(&self) -> Option<&InternetMsg> {
        match self {
            Self::Internet(msg) => Some(msg),
            _ => None,
        }
    }
    pub fn phone(&self) -> Option<&PhoneMsg> {
        match self {
            Self::Phone(msg) => Some(msg),
            _ => None,
        }
    }
    pub fn wlan(&self) -> Option<&WlanMsg> {
        match self {
            Self::Wlan(msg) => Some(msg),
            _ => None,
        }
    }
    pub fn usb(&self) -> Option<&UsbMsg> {
        match self {
            Self::Usb(msg) => Some(msg),
            _ => None,
        }
    }
}
