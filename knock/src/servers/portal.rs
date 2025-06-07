use crate::AppState;
use crate::common::{escape_html, random_string};
use crate::parse_duration::parse_duration;
use crate::string_hash::StringHash;
use anyhow::{Context, bail};
use axum::Json;
use axum::extract::State;
use axum::http::Uri;
use axum::http::uri::Builder;
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
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(body): Json<InvitationLinkRequest>,
) -> Response {
    let token = unwrap_or_500!(random_string());
    let new_url = unwrap_or_400!(append_token_to_url(&body.url, &token));
    let expiration = unwrap_or_400!(parse_duration(&body.expiration));

    let session = unwrap_or_403!(ensure_valid_session(&state, &cookies));

    let url_hash = StringHash::new(&new_url);
    let now = Utc::now();
    state.data.lock().add_invite_link(
        &state.audit,
        url_hash,
        session,
        body.url.len(),
        now + expiration,
    );

    Json(InvitationLinkResponse { url: new_url }).into_response()
}

fn ensure_valid_session(state: &AppState, cookies: &CookieJar) -> anyhow::Result<StringHash> {
    let session = cookies
        .get(&state.config.knock_cookie_name)
        .context("missing knock cookie")?;
    let session_hash = StringHash::new(session.value());
    let data = state.data.lock();
    let session = data
        .sessions
        .get(&session_hash)
        .context("unknown session")?;

    if session.expires_at < Utc::now() {
        bail!("session expired");
    }

    Ok(session_hash)
}

fn append_token_to_url(url: &str, token: &str) -> anyhow::Result<String> {
    let url: Uri = url.parse()?;
    let parts = url.into_parts();
    let path_and_query = parts.path_and_query.context("missing path")?;
    let new_path_and_query = match path_and_query.query() {
        None => format!("{}?_knock={}", path_and_query, token),
        Some("") => format!("{}_knock={}", path_and_query, token),
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
