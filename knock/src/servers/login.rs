use crate::AppState;
use crate::common::{check_valid_host, create_cookie, read_client_ip};
use crate::config::Config;
use crate::data::{Ip, User};
use anyhow::Context;
use axum::Form;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use minijinja::context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use totp_rs::TOTP;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum LoginMessage {
    ExpiredLoginSession,
    ExpiredGuestSession,
    ExpiredGuestLink,
    InvalidCredentials,
}

#[derive(Deserialize)]
pub struct LoginPageQuery {
    callback: String,
    message: Option<LoginMessage>,
}

#[derive(Deserialize)]
pub struct LoginPageForm {
    callback: String,
    username: String,
    token: String,
}

pub async fn handle_login_page(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginPageQuery>,
) -> Response {
    render_login_page(&state.config, &query.callback, query.message)
}

pub async fn handle_login_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    cookies: CookieJar,
    Form(LoginPageForm {
        callback,
        username,
        token,
    }): Form<LoginPageForm>,
) -> Response {
    let config = &state.config;

    state.throttle.wait(config.login_throttle).await;

    unwrap_or_400!(check_valid_host(config, &callback).context("callback is invalid"));

    let username = username.trim().to_string();

    let client_ip = unwrap_or_403!(read_client_ip(&headers));
    let now = Utc::now();
    let mut data = state.data.lock();
    let data = &mut *data;
    let ip_attempt = data
        .ips
        .get_or_insert_with(&client_ip, || Ip {
            ip_addr: client_ip,
            session: None,
            ban_timer: Default::default(),
        })
        .ban_timer
        .attempt(
            now,
            config.failed_login_max_attempts_per_ip,
            config.failed_login_ban,
        );
    let user_attempt = data
        .users
        .get_or_insert_with(&username, || User {
            name: username.clone(),
            ban_timer: Default::default(),
        })
        .ban_timer
        .attempt(
            now,
            config.failed_login_max_attempts_per_user,
            config.failed_login_ban,
        );

    let Some((ip_attempt, user_attempt)) = ip_attempt.zip(user_attempt) else {
        tracing::info!(
            "FAILED: too many failed attempts for ip {} or user {}",
            client_ip,
            username
        );
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Some(totps) = config.totps_by_user.get(&username) else {
        tracing::info!("FAILED: unknown user");
        return StatusCode::UNAUTHORIZED.into_response();
    };

    if !unwrap_or_500!(check_token(totps, &token)) {
        tracing::info!("FAILED: invalid token");
        return render_login_page(config, &callback, Some(LoginMessage::InvalidCredentials));
    }

    ip_attempt.report_success();
    user_attempt.report_success();
    tracing::info!("SUCCESS: {} login at {}", username, client_ip);

    let (value, value_hash) = unwrap_or_500!(data.create_login_session(
        username,
        client_ip,
        config.login_session_expiration
    ));
    data.upsert_ip_session(
        client_ip,
        Some(value_hash),
        None,
        config.ip_session_expiration,
    );

    let cookie = create_cookie(
        config.login_session_cookie.clone(),
        value,
        config.cookie_domain.clone(),
        config.login_session_expiration,
    );
    let cookies = cookies.add(cookie);

    (cookies, Redirect::temporary(&callback)).into_response()
}

fn render_login_page(config: &Config, callback: &str, message: Option<LoginMessage>) -> Response {
    let message_key = match message {
        None => "You need to authenticate",
        Some(LoginMessage::ExpiredLoginSession) => "Your login session has expired",
        Some(LoginMessage::ExpiredGuestSession) => {
            "Your guest session has expired. Please click the shared link again"
        }
        Some(LoginMessage::ExpiredGuestLink) => "This link has expired",
        Some(LoginMessage::InvalidCredentials) => "Invalid credentials, try again",
    };
    let login_html = unwrap_or_500!(
        config
            .renderer
            .render("login.html", context!(callback, message_key))
    );

    ([("content-type", "text/html")], login_html).into_response()
}

fn check_token(totps: &[TOTP], token: &str) -> anyhow::Result<bool> {
    for topt in totps {
        if topt.check_current(token).context("failed to check token")? {
            return Ok(true);
        }
    }

    Ok(false)
}
