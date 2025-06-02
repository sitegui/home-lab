use crate::data::UserName;
use crate::string_hash::StringHash;
use chrono::{DateTime, Utc};
use std::net::IpAddr;

pub struct Audit {}

pub enum AuditEvent {
    IpAllowed {
        ip: IpAddr,
        by_session: StringHash,
        until_at_least: DateTime<Utc>,
    },
    NewLoginSession {
        user: UserName,
        session: StringHash,
        expires_at: DateTime<Utc>,
    },
    NewInviteeSession {
        invited_by: UserName,
        session: StringHash,
        expires_at: DateTime<Utc>,
    },
}

impl Audit {
    pub fn report(&self, _event: AuditEvent) {
        todo!()
    }
}
