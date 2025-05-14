use crate::AppState;
use crate::common::{build_redirection, read_client_ip, read_header};
use crate::data::IpSession;
use crate::string_hash::StringHash;
use anyhow::Context;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use std::net::IpAddr;
use std::sync::Arc;

pub async fn handle_forward_auth(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
    headers: HeaderMap,
) -> Response {
    tracing::debug!("Received request with headers: {:#?}", headers);

    let client_ip = unwrap_or_403!(read_client_ip(&headers).context("failed to read client ip"));
    let uri = unwrap_or_403!(read_header(&headers, "x-forwarded-uri"));
    let proto = unwrap_or_403!(read_header(&headers, "x-forwarded-proto"));
    let host = unwrap_or_403!(read_header(&headers, "x-forwarded-host"));

    let callback = format!("{}://{}{}", proto, host, uri);
    tracing::debug!("Original request: {}", callback);

    match cookies.get(&state.config.knock_cookie_name) {
        Some(cookie) => {
            let session_hash = StringHash::new(cookie.value());
            handle_request_with_knock_session(&state, &callback, client_ip, session_hash)
        }
        None => handle_request_without_cookie(&state, &callback, client_ip),
    }
}

fn handle_request_with_knock_session(
    state: &AppState,
    callback: &str,
    client_ip: IpAddr,
    session_hash: StringHash,
) -> Response {
    let config = &state.config;
    let now = Utc::now();

    enum Decision {
        Redirect,
        Allow,
    }

    let decision = {
        let mut data = state.data.lock();
        let is_alive = data
            .knock_sessions
            .get_mut(&session_hash)
            .map(|session| {
                session.timer.check_alive(
                    now,
                    config.session_max_lifetime,
                    config.session_max_inactivity,
                )
            })
            .unwrap_or(false);

        if is_alive {
            data.ips.entry(client_ip).or_default().session = Some(IpSession {
                session: session_hash,
                last_activity: now,
            });
            Decision::Allow
        } else {
            drop(data);
            Decision::Redirect
        }
    };

    match decision {
        Decision::Redirect => {
            tracing::info!("BLOCKED: knock session is unknown or dead");
            build_redirection(config, callback)
        }
        Decision::Allow => {
            tracing::debug!("ALLOWED: knock session is valid");
            StatusCode::OK.into_response()
        }
    }
}

fn handle_request_without_cookie(state: &AppState, uri: &str, client_ip: IpAddr) -> Response {
    let config = &state.config;
    if config
        .allowed_networks
        .iter()
        .any(|network| network.includes(client_ip))
    {
        tracing::debug!("ALLOWED: IP is part of allowed networks");
        return StatusCode::OK.into_response();
    }

    let now = Utc::now();

    let is_alive = state
        .data
        .lock()
        .ips
        .get(&client_ip)
        .and_then(|info| info.session.as_ref())
        .map(|session| now < session.last_activity + config.ip_session_max_inactivity)
        .unwrap_or(false);

    if is_alive {
        tracing::debug!("ALLOWED: IP is part of an alive session");
        StatusCode::OK.into_response()
    } else {
        tracing::info!("BLOCKED: IP is unknown or expired");
        build_redirection(config, uri)
    }
}
