mod access_level;
pub mod logger;
mod request_info;

use crate::AppState;
use crate::common::{build_login_redirection, create_cookie};
use crate::data::GuestLink;
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

    let mut data = state.data.lock();
    let access_level = AccessLevel::new(config, &data, &request);

    if let Some(logger) = state.forward_auth_logger.lock().as_ref() {
        unwrap_or_500!(logger.log(&request, &access_level));
    }

    match access_level {
        AccessLevel::None => build_login_redirection(config, &request.url()),
        AccessLevel::LoginSession(session, guest_link) => {
            let session_hash = session.value_hash;
            let response = ok_or_redirect(&request, guest_link);

            data.update_ip_session(
                request.client_ip,
                Some(session_hash),
                request.app_token_hash,
                config.login_session_expiration,
            );
            if let Some(app_token_hash) = request.app_token_hash {
                data.update_app_token(
                    app_token_hash,
                    &request.host,
                    Some(session_hash),
                    request.client_ip,
                    config.app_token_expiration,
                );
            }

            response
        }
        AccessLevel::GuestSession(guest_session, guest_link) => {
            let response = ok_or_redirect(&request, guest_link);

            if let Some(guest_link) = guest_link {
                let value_hash = guest_session.value_hash;
                let guest_link_hash = guest_link.url_hash;

                data.update_guest_session(value_hash, request.host, guest_link_hash);
            }

            response
        }
        AccessLevel::GuestLink(guest_link) => {
            let url = request.url();
            let original_url = guest_link.original_url(&url);
            let guest_link_hash = guest_link.url_hash;
            let token = unwrap_or_500!(data.create_guest_session(
                guest_link_hash,
                request.host.clone(),
                request.client_ip,
                config.guest_session_expiration,
            ));

            let cookie = create_cookie(
                config.guest_session_cookie.clone(),
                token,
                config.cookie_domain.clone(),
                config.guest_session_expiration,
            );
            let cookies = cookies.add(cookie);

            (cookies, Redirect::temporary(original_url)).into_response()
        }
        AccessLevel::AppToken(app_token) => {
            let app_token_hash = app_token.value_hash;

            data.update_app_token(
                app_token_hash,
                &request.host,
                None,
                request.client_ip,
                config.app_token_expiration,
            );

            data.update_ip_session(
                request.client_ip,
                None,
                Some(app_token_hash),
                config.ip_session_expiration,
            );

            StatusCode::OK.into_response()
        }
        AccessLevel::Ip(_) => {
            if let Some(app_token_hash) = request.app_token_hash {
                data.update_app_token(
                    app_token_hash,
                    &request.host,
                    None,
                    request.client_ip,
                    config.app_token_expiration,
                );
            }

            data.update_ip_session(
                request.client_ip,
                None,
                request.app_token_hash,
                config.ip_session_expiration,
            );

            StatusCode::OK.into_response()
        }
        AccessLevel::AllowedNetwork => StatusCode::OK.into_response(),
    }
}

fn ok_or_redirect(request: &RequestInfo, guest_link: Option<&GuestLink>) -> Response {
    match guest_link {
        None => StatusCode::OK.into_response(),
        Some(guest_link) => {
            let url = request.url();
            let original_url = guest_link.original_url(&url);
            Redirect::temporary(original_url).into_response()
        }
    }
}
