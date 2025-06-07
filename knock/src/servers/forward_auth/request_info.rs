use crate::common::{read_client_ip, read_header};
use crate::config::Config;
use crate::string_hash::StringHash;
use anyhow::Context;
use axum::http::HeaderMap;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use std::net::IpAddr;

pub struct RequestInfo {
    pub arrival: DateTime<Utc>,
    pub headers: HeaderMap,
    pub client_ip: IpAddr,
    pub uri: String,
    pub proto: String,
    pub host: String,
    pub login_session_hash: Option<StringHash>,
    pub guest_session_hash: Option<StringHash>,
    pub app_token_hash: Option<StringHash>,
}

impl RequestInfo {
    pub fn new(config: &Config, cookies: &CookieJar, headers: HeaderMap) -> anyhow::Result<Self> {
        let client_ip = read_client_ip(&headers).context("failed to read client ip")?;
        let uri = read_header(&headers, "x-forwarded-uri")?.to_owned();
        let proto = read_header(&headers, "x-forwarded-proto")?.to_owned();
        let host = read_header(&headers, "x-forwarded-host")?.to_owned();
        let login_session_hash = cookies
            .get(&config.login_session_cookie)
            .map(|cookie| StringHash::new(cookie.value()));
        let guest_session_hash = cookies
            .get(&config.guest_session_cookie)
            .map(|cookie| StringHash::new(cookie.value()));
        let app_token_hash = read_header(&headers, "authentication")
            .ok()
            .map(|auth| StringHash::new(&format!("{},{}", host, auth)));

        Ok(Self {
            arrival: Utc::now(),
            headers,
            client_ip,
            uri,
            proto,
            host,
            login_session_hash,
            guest_session_hash,
            app_token_hash,
        })
    }

    pub fn url(&self) -> String {
        format!("{}://{}{}", self.proto, self.host, self.uri)
    }
}
