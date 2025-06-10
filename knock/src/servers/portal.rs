use crate::AppState;
use crate::common::{build_login_redirection, escape_html};
use crate::data::Data;
use crate::parse_duration::parse_duration;
use crate::servers::forward_auth::request_info::RequestInfo;
use crate::string_hash::StringHash;
use anyhow::Context;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn handle_portal_page(
    cookies: CookieJar,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Response {
    let config = &state.config;
    println!("{:#?}", headers);
    let request = unwrap_or_403!(RequestInfo::new(config, &cookies, headers));

    let data = state.data.lock();
    let _ = match valid_login_session(&data, &request) {
        Ok(login_session_hash) => login_session_hash,
        Err(_) => return build_login_redirection(config, &request.url()),
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
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(body): Json<GuestLinkRequest>,
) -> Response {
    let config = &state.config;
    let request = unwrap_or_403!(RequestInfo::new(config, &cookies, headers));
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
    let login_session_hash = unwrap_or_403!(valid_login_session(&data, &request));

    let new_url = unwrap_or_500!(data.create_guest_link(login_session_hash, body.url, expiration));

    Json(GuestLinkResponse { url: new_url }).into_response()
}

fn valid_login_session(data: &Data, request: &RequestInfo) -> anyhow::Result<StringHash> {
    let value_hash = request
        .login_session_hash
        .context("missing login session hash")?;

    let session = data
        .valid_login_session(request.arrival, value_hash)
        .context("invalid login session")?;

    Ok(session.value_hash)
}
