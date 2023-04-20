use std::net::Ipv4Addr;
use std::str::FromStr;

use anyhow::Result;
use lazy_regex::regex_captures;
use thiserror::Error;

use crate::logs::traits::FromLogMsg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDetails {
    pub ip: Ipv4Addr,
    pub dns: [Ipv4Addr; 2],
    pub gateway: Ipv4Addr,
}

#[derive(Debug, Error)]
pub enum ParseConnectedDetailsError {
    #[error("log message has invalid prefix")]
    Prefix,
    #[error("couldn't parse public ip (v4)")]
    PublicIp,
    #[error("couldn't parse dns ip (v4)")]
    Dns,
    #[error("couldn't parse gateway ip (v4)")]
    Gateway,
}

impl FromStr for ConnectedDetails {
    type Err = ParseConnectedDetailsError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("Internetverbindung wurde erfolgreich hergestellt.") {
            return Err(ParseConnectedDetailsError::Prefix);
        }

        let (_, ip) = regex_captures!(r#"IP-Adresse: ([0-9\.]+)"#, s)
            .ok_or(ParseConnectedDetailsError::PublicIp)?;
        let (_, dns_1, dns_2) = regex_captures!(r#"DNS-Server: ([0-9\.]+) und ([0-9\.]+)"#, s)
            .ok_or(ParseConnectedDetailsError::Dns)?;
        let (_, gateway) = regex_captures!(r#"Gateway: ([0-9\.]+)"#, s)
            .ok_or(ParseConnectedDetailsError::Gateway)?;

        Ok(ConnectedDetails {
            ip: Ipv4Addr::from_str(ip).map_err(|_| ParseConnectedDetailsError::PublicIp)?,
            dns: [
                Ipv4Addr::from_str(dns_1).map_err(|_| ParseConnectedDetailsError::Dns)?,
                Ipv4Addr::from_str(dns_2).map_err(|_| ParseConnectedDetailsError::Dns)?,
            ],
            gateway: Ipv4Addr::from_str(gateway)
                .map_err(|_| ParseConnectedDetailsError::Gateway)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslReadyDeatils {
    /// in `kbit/s`
    pub up: u32,
    /// in `kbit/s`
    pub down: u32,
}

#[derive(Debug, Error)]
pub enum ParseDslReadyDetailsError {
    #[error("log message has invalid prefix")]
    Prefix,
    #[error("couldn't parse up-/download")]
    Rate,
}

impl FromStr for DslReadyDeatils {
    type Err = ParseDslReadyDetailsError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if !s.starts_with("DSL ist verfügbar") {
            return Err(ParseDslReadyDetailsError::Prefix);
        }

        let (_, up, down) =
            regex_captures!(r#"DSL-Synchronisierung besteht mit (\d+)/(\d+) kbit/s"#, s)
                .ok_or(ParseDslReadyDetailsError::Rate)?;

        Ok(DslReadyDeatils {
            up: u32::from_str(up).map_err(|_| ParseDslReadyDetailsError::Rate)?,
            down: u32::from_str(down).map_err(|_| ParseDslReadyDetailsError::Rate)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternetMsg {
    /// German: `Internetverbindung wurde getrennt.`
    /// German: `Verbindung getrennt: ...`
    Disconnected,
    /// German: `Internetverbindung wurde erfolgreich hergestellt.`
    Connected(ConnectedDetails),
    /// German: `Zeitüberschreitung bei der PPP-Aushandlung.`
    /// German: `PPPoE-Fehler: Zeitüberschreitung.`
    PppTimeout,
    /// German: `PPPoE-Fehler: Unbekannter Fehler.`
    PppUnknown,
    /// German: `DSL-Synchronisierung beginnt (Training).`
    DslSyncBegin,
    /// German: `DSL antwortet nicht (Keine DSL-Synchronisierung).`
    DslNoAnswer,
    /// German: `DSL ist verfügbar (...).`
    DslReady(DslReadyDeatils),
    /// German: `Anmeldung beim Internetanbieter ist fehlgeschlagen.`
    SignInFailed,
    /// None of the above
    Unknown,
}

impl FromLogMsg for InternetMsg {
    type Err = ();
    fn from_log_msg(s: &str) -> std::result::Result<Self, ()> {
        let s = s.trim();
        if s.starts_with("Internetverbindung wurde getrennt.")
            || s.starts_with("Verbindung getrennt")
        {
            Ok(InternetMsg::Disconnected)
        } else if s.starts_with("Internetverbindung wurde erfolgreich hergestellt.") {
            // Already checked for required prefix
            Ok(InternetMsg::Connected(
                ConnectedDetails::from_str(s).unwrap(),
            ))
        } else if s.starts_with("Zeitüberschreitung bei der PPP-Aushandlung")
            || s.starts_with("PPPoE-Fehler: Zeitüberschreitung.")
        {
            Ok(InternetMsg::PppTimeout)
        } else if s.starts_with("PPPoE-Fehler: Unbekannter Fehler.") {
            Ok(InternetMsg::PppUnknown)
        } else if s.starts_with("DSL-Synchronisierung beginnt (Training).") {
            Ok(InternetMsg::DslSyncBegin)
        } else if s.starts_with("DSL antwortet nicht (Keine DSL-Synchronisierung).") {
            Ok(InternetMsg::DslNoAnswer)
        } else if s.starts_with("DSL ist verfügbar") {
            // Already checked for required prefix
            Ok(InternetMsg::DslReady(DslReadyDeatils::from_str(s).unwrap()))
        } else if s.starts_with("Anmeldung beim Internetanbieter ist fehlgeschlagen.") {
            Ok(InternetMsg::SignInFailed)
        } else {
            Ok(InternetMsg::Unknown)
        }
    }
}
