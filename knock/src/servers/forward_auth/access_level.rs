use crate::config::Config;
use crate::data::{
    AppToken, Data, GuestLink, GuestLinkResult, GuestSession, IpSession, LoginSession,
};
use crate::servers::forward_auth::request_info::RequestInfo;
use serde::Serialize;

/// Represents which kind of access validates this request
#[derive(Debug, Clone, Serialize)]
pub enum AccessLevel<'a> {
    LoginSession(&'a LoginSession, Option<&'a GuestLink>),
    ExpiredLoginSession,
    GuestSession(&'a GuestSession, Option<&'a GuestLink>),
    ExpiredGuestSession,
    GuestLink(&'a GuestLink),
    ExpiredGuestLink,
    AppToken(&'a AppToken),
    Ip(&'a IpSession),
    AllowedNetwork,
    None,
}

impl<'a> AccessLevel<'a> {
    pub fn new(config: &Config, data: &'a Data, request: &RequestInfo) -> Self {
        let guest_link = data.valid_guest_link(request.arrival, &request.url());

        if let Some(login_session_hash) = request.login_session_hash {
            if let Some(login_session) =
                data.valid_login_session(request.arrival, login_session_hash)
            {
                return AccessLevel::LoginSession(login_session, guest_link.ok());
            } else {
                return AccessLevel::ExpiredLoginSession;
            }
        }

        if let Some(guest_session_hash) = request.guest_session_hash {
            if let Some(guest_session) =
                data.valid_guest_session(request.arrival, &request.host, guest_session_hash)
            {
                return AccessLevel::GuestSession(guest_session, guest_link.ok());
            } else {
                return AccessLevel::ExpiredGuestSession;
            }
        }

        match guest_link {
            GuestLinkResult::Expired => {
                return AccessLevel::ExpiredGuestLink;
            }
            GuestLinkResult::Ok(guest_link) => {
                return AccessLevel::GuestLink(guest_link);
            }
            _ => {}
        }

        if let Some(app_token_hash) = request.app_token_hash {
            if let Some(app_token) = data.valid_app_token(request.arrival, app_token_hash) {
                return AccessLevel::AppToken(app_token);
            }
        }

        if let Some(ip) = data.valid_ip(request.arrival, request.client_ip) {
            return AccessLevel::Ip(ip);
        }

        if config
            .allowed_networks
            .iter()
            .any(|network| network.includes(request.client_ip))
        {
            return AccessLevel::AllowedNetwork;
        }

        AccessLevel::None
    }
}
