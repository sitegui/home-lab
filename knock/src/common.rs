use crate::config::Config;
use anyhow::Context;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use std::net::IpAddr;

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
