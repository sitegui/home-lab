use crate::AppState;
use crate::common::{escape_html, read_client_ip};
use anyhow::Context;
use axum::Form;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use serde::Deserialize;
use std::sync::Arc;
use tokio::time;
use totp_rs::TOTP;

#[derive(Deserialize)]
pub struct LoginPageQuery {
    callback: String,
}

#[derive(Deserialize)]
pub struct LoginPageForm {
    callback: String,
    token: String,
}

pub async fn handle_login_page(Query(query): Query<LoginPageQuery>) -> Response {
    let login_html =
        include_str!("../web/login.html").replace("{{callback}}", &escape_html(&query.callback));

    ([("content-type", "text/html")], login_html).into_response()
}

pub async fn handle_login_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    cookies: CookieJar,
    Form(form): Form<LoginPageForm>,
) -> Response {
    // TODO: validate callback

    let config = &state.config;
    time::sleep(config.login_sleep).await;

    let client_ip = unwrap_or_403!(read_client_ip(&headers));
    let invalid_logins = state
        .data
        .lock()
        .ip_infos
        .get(&client_ip)
        .map(|info| info.invalid_logins)
        .unwrap_or(0);
    if invalid_logins > config.max_failed_attempts_per_ip {
        tracing::debug!("Too many failed attempts for ip {}", client_ip);
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if !unwrap_or_500!(check_token(&state.config.totps, &form.token)) {
        state
            .data
            .lock()
            .ip_infos
            .entry(client_ip)
            .or_default()
            .invalid_logins += 1;
        tracing::debug!("Failed attempt for {}", client_ip);
        return handle_login_page(Query(LoginPageQuery {
            callback: form.callback,
        }))
        .await;
    }

    let mut random_bytes = [0u8; 16];
    unwrap_or_500!(getrandom::fill(&mut random_bytes));
    let session = hex::encode(random_bytes);

    let now = chrono::Utc::now();
    let valid_until = now + config.cookie_session_interval;

    state
        .data
        .lock()
        .cookie_sessions
        .insert(session.clone(), CookieSessionInfo { valid_until });

    let max_age = unwrap_or_500!(::time::Duration::try_from(config.cookie_session_interval));
    let session_cookie = Cookie::build((config.knock_cookie_name.clone(), session))
        .domain(config.knock_cookie_domain.clone())
        .max_age(max_age)
        .secure(true)
        .http_only(true);
    let cookies = cookies.add(session_cookie);

    tracing::info!("Successful attempt for {}", client_ip);
    (cookies, Redirect::temporary(&form.callback)).into_response()
}

fn check_token(totps: &[TOTP], token: &str) -> anyhow::Result<bool> {
    for topt in totps {
        if topt.check_current(token).context("failed to check token")? {
            return Ok(true);
        }
    }

    Ok(false)
}
