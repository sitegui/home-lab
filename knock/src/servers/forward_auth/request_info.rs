use crate::common::{read_client_ip, read_header};
use crate::config::Config;
use crate::string_hash::StringHash;
use anyhow::Context;
use axum::http::HeaderMap;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use std::net::IpAddr;

pub struct RequestInfo {
    arrival: DateTime<Utc>,
    headers: HeaderMap,
    client_ip: IpAddr,
    uri: String,
    proto: String,
    host: String,
    session_hash: Option<StringHash>,
}

impl RequestInfo {
    pub fn new(config: &Config, cookies: &CookieJar, headers: HeaderMap) -> anyhow::Result<Self> {
        let client_ip = read_client_ip(&headers).context("failed to read client ip")?;
        let uri = read_header(&headers, "x-forwarded-uri")?.to_owned();
        let proto = read_header(&headers, "x-forwarded-proto")?.to_owned();
        let host = read_header(&headers, "x-forwarded-host")?.to_owned();
        let session_hash = cookies
            .get(&config.knock_cookie_name)
            .map(|cookie| StringHash::new(cookie.value()));

        Ok(Self {
            arrival: Utc::now(),
            headers,
            client_ip,
            uri,
            proto,
            host,
            session_hash,
        })
    }

    pub fn arrival(&self) -> DateTime<Utc> {
        self.arrival
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn client_ip(&self) -> IpAddr {
        self.client_ip
    }

    pub fn session_hash(&self) -> Option<StringHash> {
        self.session_hash
    }

    pub fn uri(&self) -> String {
        format!("{}://{}{}", self.proto, self.host, self.uri)
    }
}
