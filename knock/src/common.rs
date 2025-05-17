use crate::config::Config;
use anyhow::Context;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use std::net::IpAddr;

pub fn read_header<'a>(headers: &'a HeaderMap, name: &str) -> anyhow::Result<&'a str> {
    headers
        .get(name)
        .with_context(|| format!("missing {}", name))?
        .to_str()
        .with_context(|| format!("invalid {}", name))
}

pub fn read_client_ip(headers: &HeaderMap) -> anyhow::Result<IpAddr> {
    let client_ips = read_header(headers, "x-forwarded-for")?;

    let client_ip = client_ips
        .split_once(',')
        .map(|(first, _)| first)
        .unwrap_or(client_ips)
        .trim();
    let client_ip: IpAddr = client_ip.parse().context("invalid client ip")?;

    Ok(client_ip)
}

pub fn build_redirection(config: &Config, callback: &str) -> Response {
    Redirect::temporary(&format!(
        "{}/?callback={}",
        config.login_hostname,
        urlencoding::encode(callback)
    ))
    .into_response()
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
