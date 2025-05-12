use crate::AppState;
use crate::common::{build_redirection, read_client_ip};
use anyhow::Context;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use std::net::IpAddr;
use std::sync::Arc;

pub async fn handle_forward_auth(
    State(state): State<Arc<AppState>>,
    cookies: CookieJar,
    headers: HeaderMap,
) -> Response {
    let client_ip = unwrap_or_403!(read_client_ip(&headers).context("failed to read client ip"));
    let uri = unwrap_or_403!(
        headers
            .get("X-Forwarded-Uri")
            .and_then(|header| header.to_str().ok())
            .context("failed to read original uri")
    );
    let proto = unwrap_or_403!(
        headers
            .get("X-Forwarded-Proto")
            .and_then(|header| header.to_str().ok())
            .context("failed to read original protocol")
    );
    let host = unwrap_or_403!(
        headers
            .get("X-Forwarded-Host")
            .and_then(|header| header.to_str().ok())
            .context("failed to read original host")
    );

    let callback = format!("{}://{}{}", proto, host, uri);
    tracing::debug!("Original request: {}", callback);

    match cookies.get(&state.config.cookie_name) {
        Some(cookie) => handle_request_with_cookie(&state, &callback, client_ip, cookie.value()),
        None => handle_request_without_cookie(&state, &callback, client_ip),
    }
}

fn handle_request_with_cookie(
    state: &AppState,
    uri: &str,
    client_ip: IpAddr,
    cookie: &str,
) -> Response {
    tracing::debug!("Request has session cookie");
    let now = Utc::now();
    let ip_valid_until = now + state.config.ip_session_interval;

    {
        let mut data = state.data.lock();
        let valid_until = data
            .cookie_sessions
            .get(cookie)
            .map(|info| info.valid_until);

        if !is_still_valid(now, valid_until) {
            drop(data);
            tracing::debug!("Session is unknown or expired: redirecting to auth");

            return build_redirection(state, uri);
        }

        data.ip_infos.entry(client_ip).or_default().valid_until = Some(ip_valid_until);
    }

    StatusCode::OK.into_response()
}

fn handle_request_without_cookie(state: &AppState, uri: &str, client_ip: IpAddr) -> Response {
    tracing::debug!("Request does not have session cookie");
    let now = Utc::now();

    let valid_until = state
        .data
        .lock()
        .ip_infos
        .get(&client_ip)
        .and_then(|info| info.valid_until);

    if !is_still_valid(now, valid_until) {
        tracing::debug!("IP is unknown or expired: redirecting to auth");
        return build_redirection(state, uri);
    }

    StatusCode::OK.into_response()
}

fn is_still_valid(now: DateTime<Utc>, valid_until: Option<DateTime<Utc>>) -> bool {
    matches!(valid_until, Some(valid_until) if valid_until > now)
}
