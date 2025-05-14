use crate::alive_timer::AliveTimer;
use crate::ban_timer::BanTimer;
use crate::string_hash::StringHash;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Default)]
pub struct Data {
    pub users: BTreeMap<String, User>,
    pub knock_sessions: BTreeMap<StringHash, Session>,
    pub ips: BTreeMap<IpAddr, IpInfo>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct User {
    pub ban_timer: BanTimer,
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub user_name: String,
    pub login_ip: IpAddr,
    pub timer: AliveTimer,
}

#[derive(Serialize, Deserialize, Default)]
pub struct IpInfo {
    pub ban_timer: BanTimer,
    /// If this ip is authorized
    pub session: Option<IpSession>,
}

#[derive(Serialize, Deserialize)]
pub struct IpSession {
    pub session: StringHash,
    pub last_activity: DateTime<Utc>,
}
