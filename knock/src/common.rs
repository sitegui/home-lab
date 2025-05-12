use crate::AppState;
use anyhow::Context;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use std::net::IpAddr;

pub fn read_client_ip(headers: &HeaderMap) -> anyhow::Result<IpAddr> {
    let client_ips = headers
        .get("x-forwarded-for")
        .context("missing x-forwarded-for")?;
    let client_ips = client_ips.to_str().context("invalid x-forwarded-for")?;
    tracing::debug!("x-forwarded-for = {}", client_ips);

    let client_ip = client_ips
        .split_once(',')
        .map(|(first, _)| first)
        .unwrap_or(client_ips)
        .trim();
    let client_ip: IpAddr = client_ip.parse().context("invalid client ip")?;

    Ok(client_ip)
}

pub fn build_redirection(state: &AppState, uri: &str) -> Response {
    Redirect::temporary(&format!("{}/?callback={}", state.config.auth_host, uri)).into_response()
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
