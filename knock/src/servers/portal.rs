use crate::AppState;
use crate::common::{build_login_redirection, check_valid_host, read_header};
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
use minijinja::context;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;

pub async fn handle_portal_page(
    cookies: CookieJar,
    headers: HeaderMap,
    url: Uri,
    State(state): State<Arc<AppState>>,
) -> Response {
    let config = &state.config;

    let is_unlocked = state
        .unlock_api
        .is_unlocked()
        .await
        .unwrap_or_else(|error| {
            tracing::warn!("Could not check unlock status: {}", error);
            true
        });

    let data = state.data.lock();
    let user_name = match valid_login_session(config, &data, &cookies, Utc::now()) {
        Ok((user_name, _)) => user_name,
        Err(_) => {
            let host = unwrap_or_403!(read_header(&headers, "x-forwarded-host"));
            let proto = unwrap_or_403!(read_header(&headers, "x-forwarded-proto"));
            return build_login_redirection(config, &format!("{}://{}{}", proto, host, url), None);
        }
    };

    #[derive(Serialize)]
    struct LoginSessionData {
        origin_ip: IpAddr,
        created_at: String,
        expires_at: String,
    }

    let mut login_sessions = Vec::new();
    for session in data.login_sessions.iter() {
        if session.user_name == user_name {
            login_sessions.push(LoginSessionData {
                origin_ip: session.origin_ip,
                created_at: session.created_at.format("%Y-%m-%d %H:%M %Z").to_string(),
                expires_at: session.expires_at.format("%Y-%m-%d %H:%M %Z").to_string(),
            });
        }
    }
    login_sessions.sort_by(|a, b| b.expires_at.cmp(&a.expires_at));

    let html = unwrap_or_500!(
        config
            .renderer
            .render("portal.html", context!(login_sessions, is_unlocked))
    );

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
    let (_, login_session_hash) =
        unwrap_or_403!(valid_login_session(config, &data, &cookies, Utc::now()));

    unwrap_or_400!(check_valid_host(config, &body.url).context("host is invalid"));
    let new_url = unwrap_or_500!(data.create_guest_link(login_session_hash, body.url, expiration));

    Json(GuestLinkResponse { url: new_url }).into_response()
}

#[derive(Deserialize, Debug)]
pub struct UnlockSystemRequest {
    password: String,
}

pub async fn post_unlock_system(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UnlockSystemRequest>,
) -> Response {
    let config = &state.config;

    let (user, _) = unwrap_or_403!(valid_login_session(
        config,
        &state.data.lock(),
        &cookies,
        Utc::now()
    ));

    tracing::info!("Unlocking system initialized by {}", user);
    let unlocked = unwrap_or_500!(state.unlock_api.unlock(&body.password).await);

    if unlocked {
        StatusCode::OK.into_response()
    } else {
        StatusCode::BAD_REQUEST.into_response()
    }
}

fn valid_login_session(
    config: &Config,
    data: &Data,
    cookies: &CookieJar,
    now: DateTime<Utc>,
) -> anyhow::Result<(String, StringHash)> {
    let cookie = cookies
        .get(&config.login_session_cookie)
        .context("missing knock cookie")?;
    let value_hash = StringHash::new(cookie.value());

    let session = data
        .valid_login_session(now, value_hash)
        .context("invalid knock session")?;

    Ok((session.user_name.clone(), session.value_hash))
}
