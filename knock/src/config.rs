use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::time::Duration;
use totp_rs::{Rfc6238, Secret, TOTP};

pub struct Config {
    pub auth_host: String,
    pub cookie_name: String,
    pub cookie_session_domain: String,
    pub cookie_session_interval: Duration,
    pub forward_auth_bind: String,
    pub forward_auth_port: u16,
    pub ip_session_interval: Duration,
    pub login_bin: String,
    pub login_port: u16,
    pub login_sleep: Duration,
    pub max_failed_attempts_per_ip: u32,
    pub totps: Vec<TOTP>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::from_path("default.env").context("failed to read default.env")?;

        let config: EnvConfig = envy::from_env().context("failed to load configuration. \
        Please make sure that all requires environment variables described in the documentation are set")?;

        let totp_secrets = fs::read_to_string(&config.totp_secrets_file).with_context(|| {
            format!(
                "failed to read TOTP secrets file: {}",
                config.totp_secrets_file
            )
        })?;

        let mut totps = vec![];
        for secret in totp_secrets.lines() {
            let secret_bytes = Secret::Encoded(secret.to_string()).to_bytes().context(
                "failed to decode TOTP secret. \
            Make sure it's saved one secret per line and encoded in base32",
            )?;
            let totp = TOTP::from_rfc6238(
                Rfc6238::with_defaults(secret_bytes).context("failed to create TOTP instance")?,
            )
            .context("failed to create TOTP instance")?;
            totps.push(totp);
        }

        Ok(Config {
            auth_host: config.auth_host,
            cookie_name: config.cookie_name,
            cookie_session_domain: config.cookie_session_domain,
            cookie_session_interval: Duration::from_secs_f64(
                config.cookie_session_interval_seconds,
            ),
            forward_auth_bind: config.forward_auth_bind,
            forward_auth_port: config.forward_auth_port,
            ip_session_interval: Duration::from_secs_f64(config.ip_session_interval_seconds),
            login_bin: config.login_bind,
            login_port: config.login_port,
            login_sleep: Duration::from_secs_f64(config.login_sleep_seconds),
            max_failed_attempts_per_ip: config.max_failed_attempts_per_ip,
            totps,
        })
    }
}

#[derive(Deserialize)]
struct EnvConfig {
    auth_host: String,
    cookie_name: String,
    cookie_session_domain: String,
    cookie_session_interval_seconds: f64,
    forward_auth_bind: String,
    forward_auth_port: u16,
    ip_session_interval_seconds: f64,
    login_bind: String,
    login_port: u16,
    login_sleep_seconds: f64,
    max_failed_attempts_per_ip: u32,
    totp_secrets_file: String,
}
