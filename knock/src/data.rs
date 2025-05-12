use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Default)]
pub struct Data {
    /// Store the sessions authenticated with a cookie token
    pub cookie_sessions: BTreeMap<String, CookieSessionInfo>,
    pub ip_infos: BTreeMap<IpAddr, IpInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct Key {
    pub valid_until: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct CookieSessionInfo {
    pub valid_until: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct IpInfo {
    /// If this ip is authorized, until when it is valid
    pub valid_until: Option<DateTime<Utc>>,
    pub invalid_logins: u32,
}
