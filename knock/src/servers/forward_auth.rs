mod access_level;
pub mod logger;
mod request_info;

use crate::AppState;
use crate::common::build_login_redirection;
use crate::data::Data;
use crate::servers::forward_auth::access_level::AccessLevel;
use crate::servers::forward_auth::request_info::RequestInfo;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use std::sync::Arc;

pub async fn handle_forward_auth(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
    headers: HeaderMap,
) -> Response {
    let config = &state.config;
    let request = unwrap_or_403!(RequestInfo::new(config, &cookies, headers));

    if let Some(logger) = &state.forward_auth_logger {
        unwrap_or_500!(logger.log(&request).await);
    }

    if tracing::enabled!(tracing::Level::DEBUG) {
        let callback = request.callback();
        let callback_without_params = match callback.split_once('?') {
            None => callback.clone(),
            Some((before, _)) => format!("{}?<REDACTED>", before),
        };
        tracing::debug!(
            "Original request by {} for {}",
            request.client_ip(),
            callback_without_params
        );
    }

    let access_level = AccessLevel::new(&state, &request);
    match access_level {
        AccessLevel::None => build_login_redirection(config, &request.callback()),
        AccessLevel::Session(session) => {
            state.data.lock().allow_ip(
                &state.audit,
                request.client_ip(),
                session,
                request.arrival() + config.ip_session_expiration,
            );

            StatusCode::OK.into_response()
        }
        AccessLevel::InviteLink(invited_by) => {
            let (session_hash, cookie) = unwrap_or_500!(Data::generate_session(
                config,
                config.invitee_session_expiration
            ));

            let mut data = state.data.lock();
            data.allow_ip(
                &state.audit,
                request.client_ip(),
                session_hash,
                request.arrival() + config.ip_session_expiration,
            );
            data.allow_invitee_session(
                &state.audit,
                &invited_by,
                session_hash,
                request.arrival() + config.invitee_session_expiration,
            );

            let cookies = cookies.add(cookie);
            (cookies, Redirect::temporary(&request.callback())).into_response()
        }
        AccessLevel::Ip | AccessLevel::AllowedNetwork => StatusCode::OK.into_response(),
    }
}
