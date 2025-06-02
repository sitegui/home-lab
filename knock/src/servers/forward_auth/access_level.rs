use crate::AppState;
use crate::servers::forward_auth::request_info::RequestInfo;
use crate::string_hash::StringHash;
use serde::Serialize;

/// Represents which kind of access validates this request
#[derive(Debug, Clone, Serialize)]
pub enum AccessLevel {
    /// Access is denied
    None,
    /// Access is ensured by a session cookie, created by an explicit login or an invitation
    Session(StringHash),
    /// Access is ensured by an exact invitation link
    InviteLink {
        generated_by: StringHash,
        original_length: usize,
    },
    /// Access is ensured by a previously approved IP
    Ip,
    /// Access is ensured by the IP being part of an allowed network
    AllowedNetwork,
}

impl AccessLevel {
    pub fn new(state: &AppState, request: &RequestInfo) -> Self {
        let data = state.data.lock();

        if let Some(session_hash) = request.session_hash() {
            if let Some(session) = data.sessions.get(&session_hash) {
                if session.expires_at > request.arrival() {
                    return AccessLevel::Session(session_hash);
                }
            }
        }

        if let Some(invite_link) = data.invite_links.get(&StringHash::new(&request.uri())) {
            if invite_link.expires_at > request.arrival() {
                return AccessLevel::InviteLink {
                    generated_by: invite_link.generated_by,
                    original_length: invite_link.original_length,
                };
            }
        }

        if let Some(ip) = data.ips.get(&request.client_ip()) {
            if let Some(ip_session) = &ip.session {
                if ip_session.expires_at > request.arrival() {
                    return AccessLevel::Ip;
                }
            }
        }

        if state
            .config
            .allowed_networks
            .iter()
            .any(|network| network.includes(request.client_ip()))
        {
            return AccessLevel::AllowedNetwork;
        }

        AccessLevel::None
    }
}
