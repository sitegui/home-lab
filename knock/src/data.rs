use crate::audit::{Audit, AuditEvent};
use crate::ban_timer::BanTimer;
use crate::config::Config;
use crate::string_hash::StringHash;
use anyhow::anyhow;
use axum_extra::extract::cookie::Cookie;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Default)]
pub struct Data {
    pub users: BTreeMap<UserName, User>,
    pub sessions: BTreeMap<StringHash, Session>,
    pub ips: BTreeMap<IpAddr, Ip>,
    pub invite_links: BTreeMap<StringHash, InviteLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct UserName(pub String);

#[derive(Serialize, Deserialize, Default)]
pub struct User {
    pub ban_timer: BanTimer,
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Ip {
    pub ban_timer: BanTimer,
    /// If this ip is authorized
    pub session: Option<IpSession>,
}

#[derive(Serialize, Deserialize)]
pub struct IpSession {
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct InviteLink {
    pub host: String,
    pub generated_by: UserName,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Data {
    pub fn allow_ip(
        &mut self,
        audit: &Audit,
        ip: IpAddr,
        by_session: StringHash,
        expires_at: DateTime<Utc>,
    ) {
        self.ips
            .entry(ip)
            .or_insert_with(|| {
                audit.report(AuditEvent::IpAllowed {
                    ip,
                    by_session,
                    until_at_least: expires_at,
                });
                Ip::default()
            })
            .session = Some(IpSession { expires_at });
    }

    pub fn allow_invitee_session(
        &mut self,
        audit: &Audit,
        invited_by: UserName,
        session: StringHash,
        expires_at: DateTime<Utc>,
    ) {
        audit.report(AuditEvent::NewInviteeSession {
            invited_by,
            session,
            expires_at,
        });
        self.sessions.insert(session, Session { expires_at });
    }

    pub fn generate_session(
        config: &Config,
        expiration: TimeDelta,
    ) -> anyhow::Result<(StringHash, Cookie<'static>)> {
        let mut random_bytes = [0u8; 16];
        getrandom::fill(&mut random_bytes)
            .map_err(|error| anyhow!("failed to generate random bytes: {}", error))?;
        let session = hex::encode(random_bytes);
        let session_hash = StringHash::new(&session);

        let max_age = ::time::Duration::try_from(expiration.to_std()?)?;
        let cookie = Cookie::build((config.knock_cookie_name.clone(), session))
            .domain(config.knock_cookie_domain.clone())
            .max_age(max_age)
            .secure(true)
            .http_only(true)
            .build();

        Ok((session_hash, cookie))
    }
}
