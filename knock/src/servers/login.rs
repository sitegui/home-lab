use crate::AppState;
use crate::common::{escape_html, read_client_ip};
use crate::config::Config;
use crate::data::{Data, UserName};
use anyhow::Context;
use axum::Form;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use totp_rs::TOTP;

#[derive(Deserialize)]
pub struct LoginPageQuery {
    callback: String,
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
    render_login_page(&state.config, &query.callback)
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
    // TODO: validate callback

    let config = &state.config;
    state.throttle.wait(config.login_throttle).await;

    let username = UserName(username.trim().to_string());

    let client_ip = unwrap_or_403!(read_client_ip(&headers));
    let now = Utc::now();
    {
        let mut data = state.data.lock();
        let data = &mut *data;
        let ip_attempt = data.ips.entry(client_ip).or_default().ban_timer.attempt(
            now,
            config.failed_login_max_attempts_per_ip,
            config.failed_login_ban,
        );
        let user_attempt = data
            .users
            .entry(username.clone())
            .or_default()
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
            return render_login_page(config, &callback);
        }

        ip_attempt.report_success();
        user_attempt.report_success();
    }

    let (session_hash, cookie) = unwrap_or_500!(Data::generate_session(
        config,
        config.login_session_expiration
    ));

    {
        let mut data = state.data.lock();

        data.allow_login_session(
            &state.audit,
            &username,
            session_hash,
            now + config.login_session_expiration,
        );
        data.allow_ip(
            &state.audit,
            client_ip,
            session_hash,
            now + config.ip_session_expiration,
        );
    }

    let cookies = cookies.add(cookie);

    tracing::info!("SUCCESS: {} login at {}", username, client_ip);
    (cookies, Redirect::temporary(&callback)).into_response()
}

fn render_login_page(config: &Config, callback: &str) -> Response {
    let login_html = unwrap_or_500!(
        config
            .i18n
            .translate(&config.i18n_language, include_str!("../../web/login.html"))
    );

    let login_html = login_html.replace("{{callback}}", &escape_html(callback));

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
