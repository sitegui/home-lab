use crate::config::Config;
use anyhow::{Context, anyhow};
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

pub fn build_login_redirection(config: &Config, callback: &str) -> Response {
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

pub fn generate_token() -> anyhow::Result<String> {
    let mut random_bytes = [0u8; 16];
    getrandom::fill(&mut random_bytes)
        .map_err(|error| anyhow!("failed to generate random bytes: {}", error))?;
    Ok(hex::encode(random_bytes))
}
