use crate::AppState;
use crate::common::{build_login_redirection, escape_html, read_header};
use crate::config::Config;
use crate::data::Data;
use crate::parse_duration::parse_duration;
use crate::string_hash::StringHash;
use anyhow::Context;
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn handle_portal_page(
    cookies: CookieJar,
    headers: HeaderMap,
    url: Uri,
    State(state): State<Arc<AppState>>,
) -> Response {
    let config = &state.config;

    let data = state.data.lock();
    let _ = match valid_login_session(config, &data, &cookies, Utc::now()) {
        Ok(login_session_hash) => login_session_hash,
        Err(_) => {
            let host = unwrap_or_403!(read_header(&headers, "x-forwarded-host"));
            let proto = unwrap_or_403!(read_header(&headers, "x-forwarded-proto"));
            return build_login_redirection(
                config,
                &format!("{}://{}{}", proto, host, url),
            );
        }
    };

    let html = unwrap_or_500!(
        config
            .i18n
            .translate(&config.i18n_language, include_str!("../../web/portal.html"))
    );

    let data = serde_json::to_string_pretty(&*data).unwrap_or_default();

    let html = html.replace("{{data}}", &escape_html(&data));

    ([("content-type", "text/html")], html).into_response()
}

#[derive(Deserialize, Debug)]
pub struct GuestLinkRequest {
    url: String,
    expiration: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct GuestLinkResponse {
    url: String,
}

pub async fn post_guest_link(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(body): Json<GuestLinkRequest>,
) -> Response {
    let config = &state.config;
    let body_expiration = unwrap_or_400!(
        body.expiration
            .map(|expiration| parse_duration(&expiration))
            .transpose()
    );
    let expiration = match body_expiration {
        None => config.guest_link_max_expiration,
        Some(expiration) => expiration.min(config.guest_link_max_expiration),
    };

    let mut data = state.data.lock();
    let login_session_hash =
        unwrap_or_403!(valid_login_session(config, &data, &cookies, Utc::now()));

    let new_url = unwrap_or_500!(data.create_guest_link(login_session_hash, body.url, expiration));

    Json(GuestLinkResponse { url: new_url }).into_response()
}

fn valid_login_session(
    config: &Config,
    data: &Data,
    cookies: &CookieJar,
    now: DateTime<Utc>,
) -> anyhow::Result<StringHash> {
    let cookie = cookies
        .get(&config.login_session_cookie)
        .context("missing knock cookie")?;
    let value_hash = StringHash::new(cookie.value());

    let session = data
        .valid_login_session(now, value_hash)
        .context("invalid knock session")?;

    Ok(session.value_hash)
}
