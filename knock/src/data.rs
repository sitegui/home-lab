use crate::audit::{Audit, AuditEvent};
use crate::ban_timer::BanTimer;
use crate::common::generate_token;
use crate::config::Config;
use crate::string_hash::StringHash;
use axum_extra::extract::cookie::Cookie;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
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
    pub generated_by: StringHash,
    pub original_length: usize,
    pub expires_at: DateTime<Utc>,
}

impl Display for UserName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
        invited_by: StringHash,
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

    pub fn allow_login_session(
        &mut self,
        audit: &Audit,
        user: &UserName,
        session: StringHash,
        expires_at: DateTime<Utc>,
    ) {
        audit.report(AuditEvent::NewLoginSession {
            user,
            session,
            expires_at,
        });
        self.sessions.insert(session, Session { expires_at });
    }

    pub fn generate_session(
        config: &Config,
        expiration: TimeDelta,
    ) -> anyhow::Result<(StringHash, Cookie<'static>)> {
        let session = generate_token()?;
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

    pub fn add_invite_link(
        &mut self,
        audit: &Audit,
        link_hash: StringHash,
        generated_by: StringHash,
        original_length: usize,
        expires_at: DateTime<Utc>,
    ) {
        audit.report(AuditEvent::NewInviteLink {
            link_hash,
            generated_by,
            expires_at,
        });
        self.invite_links.insert(
            link_hash,
            InviteLink {
                generated_by,
                original_length,
                expires_at,
            },
        );
    }
}
