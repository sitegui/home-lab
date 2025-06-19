mod map;

use crate::ban_timer::BanTimer;
use crate::common::random_string;
use crate::data::map::{Map, MapItem};
use crate::string_hash::StringHash;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Default)]
pub struct Data {
    pub users: Map<User>,
    pub login_sessions: Map<LoginSession>,
    pub guest_links: Map<GuestLink>,
    pub guest_sessions: Map<GuestSession>,
    pub ips: Map<Ip>,
    pub app_tokens: Map<AppToken>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub ban_timer: BanTimer,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginSession {
    pub value_hash: StringHash,
    pub user_name: String,
    pub origin_ip: IpAddr,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuestLink {
    pub url_hash: StringHash,
    pub suffix_length: usize,
    pub created_at: DateTime<Utc>,
    pub created_by_login_session: StringHash,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuestSession {
    pub value_hash: StringHash,
    pub hosts: HashSet<String>,
    pub guest_link_hashes: HashSet<StringHash>,
    pub origin_ip: IpAddr,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ip {
    pub ip_addr: IpAddr,
    pub ban_timer: BanTimer,
    /// If this ip is authorized
    pub session: Option<IpSession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpSession {
    pub login_sessions: HashSet<StringHash>,
    pub app_tokens: HashSet<StringHash>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppToken {
    pub value_hash: StringHash,
    pub host: String,
    pub login_sessions: HashSet<StringHash>,
    pub ips: HashSet<IpAddr>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum GuestLinkResult<'a> {
    None,
    Expired,
    Ok(&'a GuestLink),
}

macro_rules! impl_map_item {
    ($struct_name:ty => $field:ident: $field_type:ty) => {
        impl MapItem for $struct_name {
            type Key = $field_type;

            fn key(&self) -> &$field_type {
                &self.$field
            }
        }
    };
}

impl_map_item! {User => name: String}
impl_map_item! {LoginSession => value_hash: StringHash}
impl_map_item! {GuestLink => url_hash: StringHash}
impl_map_item! {GuestSession => value_hash: StringHash}
impl_map_item! {Ip => ip_addr: IpAddr}
impl_map_item! {AppToken => value_hash: StringHash}

impl Data {
    pub fn valid_login_session(
        &self,
        now: DateTime<Utc>,
        value_hash: StringHash,
    ) -> Option<&LoginSession> {
        let session = self.login_sessions.get(&value_hash)?;

        (session.expires_at > now).then_some(session)
    }

    pub fn valid_guest_link(&self, now: DateTime<Utc>, url: &str) -> GuestLinkResult {
        if !url.ends_with('k') {
            return GuestLinkResult::None;
        }

        match self.guest_links.get(&StringHash::new(url)) {
            None => GuestLinkResult::None,
            Some(guest_link) if guest_link.expires_at > now => GuestLinkResult::Ok(guest_link),
            _ => GuestLinkResult::Expired,
        }
    }

    pub fn valid_guest_session(
        &self,
        now: DateTime<Utc>,
        host: &str,
        value_hash: StringHash,
    ) -> Option<&GuestSession> {
        let session = self.guest_sessions.get(&value_hash)?;

        (session.expires_at > now && session.hosts.contains(host)).then_some(session)
    }

    pub fn valid_ip(&self, now: DateTime<Utc>, ip_addr: IpAddr) -> Option<&IpSession> {
        let ip_session = self.ips.get(&ip_addr)?.session.as_ref()?;

        (ip_session.expires_at > now).then_some(ip_session)
    }

    pub fn valid_app_token(
        &self,
        now: DateTime<Utc>,
        app_token_hash: StringHash,
    ) -> Option<&AppToken> {
        let app_token = self.app_tokens.get(&app_token_hash)?;

        (app_token.expires_at > now).then_some(app_token)
    }

    pub fn create_login_session(
        &mut self,
        user_name: String,
        origin_ip: IpAddr,
        expiration: TimeDelta,
    ) -> anyhow::Result<(String, StringHash)> {
        let value = random_string()?;
        let value_hash = StringHash::new(&value);
        let created_at = Utc::now();
        self.login_sessions.insert(LoginSession {
            value_hash,
            user_name,
            origin_ip,
            created_at,
            expires_at: created_at + expiration,
        });

        Ok((value, value_hash))
    }

    pub fn create_guest_link(
        &mut self,
        login_session_hash: StringHash,
        url: String,
        expiration: TimeDelta,
    ) -> anyhow::Result<String> {
        let token = random_string()?;

        let new_url = format!("{}{}k", url, token);
        let url_hash = StringHash::new(&new_url);

        let created_at = Utc::now();
        self.guest_links.insert(GuestLink {
            url_hash,
            created_by_login_session: login_session_hash,
            suffix_length: token.len() + 1,
            created_at,
            expires_at: created_at + expiration,
        });

        Ok(new_url)
    }

    pub fn create_guest_session(
        &mut self,
        url_hash: StringHash,
        host: String,
        origin_ip: IpAddr,
        expiration: TimeDelta,
    ) -> anyhow::Result<String> {
        let value = random_string()?;
        let value_hash = StringHash::new(&value);

        let created_at = Utc::now();
        self.guest_sessions.insert(GuestSession {
            value_hash,
            hosts: HashSet::from_iter([host]),
            guest_link_hashes: HashSet::from_iter([url_hash]),
            origin_ip,
            created_at,
            expires_at: created_at + expiration,
        });

        Ok(value)
    }

    pub fn update_guest_session(
        &mut self,
        value_hash: StringHash,
        host: String,
        guest_link_hash: StringHash,
    ) {
        let Some(session) = self.guest_sessions.get_mut(&value_hash) else {
            return;
        };

        session.hosts.insert(host);
        session.guest_link_hashes.insert(guest_link_hash);
    }

    pub fn update_ip_session(
        &mut self,
        ip_addr: IpAddr,
        login_session: Option<StringHash>,
        app_token: Option<StringHash>,
        expiration: TimeDelta,
    ) {
        let ip = self.ips.get_or_insert_with(&ip_addr, || Ip {
            ip_addr,
            ban_timer: Default::default(),
            session: None,
        });

        let now = Utc::now();
        let ip_session = ip.session.get_or_insert_with(|| IpSession {
            login_sessions: Default::default(),
            app_tokens: Default::default(),
            created_at: now,
            expires_at: now + expiration,
        });

        if let Some(login_session) = login_session {
            ip_session.login_sessions.insert(login_session);
        }
        if let Some(app_token) = app_token {
            ip_session.app_tokens.insert(app_token);
        }
        ip_session.expires_at = now + expiration;
    }

    pub fn update_app_token(
        &mut self,
        value_hash: StringHash,
        host: &str,
        login_session: Option<StringHash>,
        ip: IpAddr,
        expiration: TimeDelta,
    ) {
        let app_token = self.app_tokens.get_or_insert_with(&value_hash, || {
            let created_at = Utc::now();
            AppToken {
                value_hash,
                host: host.to_owned(),
                login_sessions: Default::default(),
                ips: Default::default(),
                created_at,
                expires_at: created_at + expiration,
            }
        });

        if let Some(login_session) = login_session {
            app_token.login_sessions.insert(login_session);
        }
        app_token.ips.insert(ip);
    }
}

impl GuestLink {
    pub fn original_url<'a>(&self, url: &'a str) -> &'a str {
        &url[..url.len() - self.suffix_length]
    }
}

impl<'a> GuestLinkResult<'a> {
    pub fn ok(self) -> Option<&'a GuestLink> {
        match self {
            GuestLinkResult::Ok(link) => Some(link),
            _ => None,
        }
    }
}
