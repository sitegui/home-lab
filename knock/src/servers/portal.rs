use crate::AppState;
use crate::common::{escape_html, generate_token};
use crate::parse_duration::parse_duration;
use crate::servers::forward_auth::access_level::AccessLevel;
use crate::servers::forward_auth::request_info::RequestInfo;
use crate::string_hash::StringHash;
use anyhow::Context;
use axum::Json;
use axum::extract::State;
use axum::http::uri::Builder;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn handle_portal_page(State(state): State<Arc<AppState>>) -> Response {
    let config = &state.config;

    let html = unwrap_or_500!(
        config
            .i18n
            .translate(&config.i18n_language, include_str!("../../web/portal.html"))
    );

    let data = serde_json::to_string_pretty(&*state.data.lock()).unwrap_or_default();

    let html = html.replace("{{data}}", &escape_html(&data));

    ([("content-type", "text/html")], html).into_response()
}

#[derive(Deserialize, Debug)]
pub struct InvitationLinkRequest {
    url: String,
    expiration: String,
}

#[derive(Serialize, Debug)]
pub struct InvitationLinkResponse {
    url: String,
}

pub async fn post_invite_link(
    headers: HeaderMap,
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(body): Json<InvitationLinkRequest>,
) -> Response {
    let config = &state.config;
    let request = unwrap_or_500!(RequestInfo::new(config, &cookies, headers));

    let token = unwrap_or_500!(generate_token());
    let new_url = unwrap_or_400!(append_token_to_url(&body.url, &token));
    let expiration = unwrap_or_400!(parse_duration(&body.expiration));

    let session = match AccessLevel::new(&state, &request) {
        AccessLevel::Session(session) => session,
        _ => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let url_hash = StringHash::new(&new_url);
    let now = Utc::now();
    state
        .data
        .lock()
        .add_invite_link(&state.audit, url_hash, session, now + expiration);

    Json(InvitationLinkResponse { url: new_url }).into_response()
}

fn append_token_to_url(url: &str, token: &str) -> anyhow::Result<String> {
    let url: Uri = url.parse()?;
    let parts = url.into_parts();
    let path_and_query = parts.path_and_query.context("missing path")?;
    let new_path_and_query = match path_and_query.query() {
        None | Some("") => format!("{}_knock={}", path_and_query, token),
        Some(_) => format!("{}&_knock={}", path_and_query, token),
    };

    let new_url = Builder::new()
        .scheme(parts.scheme.context("missing scheme")?)
        .authority(parts.authority.context("missing authority")?)
        .path_and_query(new_path_and_query)
        .build()
        .context("invalid new url")?;

    Ok(new_url.to_string())
}
