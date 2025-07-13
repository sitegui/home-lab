use crate::config::Config;
use crate::serialize_to_string::serialize_to_string;
use crate::servers::login::LoginMessage;
use anyhow::{Context, anyhow, ensure};
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::Cookie;
use chrono::TimeDelta;
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

pub fn build_login_redirection(
    config: &Config,
    callback: &str,
    message: Option<LoginMessage>,
) -> Response {
    let encoded_callback = urlencoding::encode(callback);

    let url = match message {
        None => format!("{}/?callback={}", config.login_hostname, encoded_callback),
        Some(message) => {
            let message = serialize_to_string(message)
                .inspect_err(|error| {
                    tracing::warn!("failed to serialize login message: {}", error);
                })
                .unwrap_or_default();

            format!(
                "{}/?callback={}&message={}",
                config.login_hostname, encoded_callback, message,
            )
        }
    };

    Redirect::temporary(&url).into_response()
}

pub fn random_string() -> anyhow::Result<String> {
    let mut random_bytes = [0u8; 16];
    getrandom::fill(&mut random_bytes)
        .map_err(|error| anyhow!("failed to generate random bytes: {}", error))?;
    Ok(hex::encode(random_bytes))
}

pub fn create_cookie(
    name: String,
    value: String,
    domain: String,
    expiration: TimeDelta,
) -> Cookie<'static> {
    let max_age = ::time::Duration::try_from(expiration.to_std().unwrap()).unwrap();
    Cookie::build((name, value))
        .domain(domain)
        .path("/")
        .max_age(max_age)
        .secure(true)
        .http_only(true)
        .build()
}

pub async fn handle_static_file(Path(file_name): Path<String>) -> Response {
    let file_name = file_name.as_str();
    match file_name {
        "style.css" => (
            [("content-type", "text/css")],
            include_str!("../web/static/style.css"),
        )
            .into_response(),
        "portal.js" => (
            [("content-type", "application/javascript")],
            include_str!("../web/static/portal.js"),
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn check_valid_host(config: &Config, url: &str) -> anyhow::Result<()> {
    let uri: Uri = url.parse()?;
    let host = uri.host().context("missing host")?;
    ensure!(config.valid_hosts.contains(host));
    Ok(())
}
