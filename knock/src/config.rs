use crate::network::Network;
use crate::parse_duration::parse_duration;
use crate::template_renderer::TemplateRenderer;
use anyhow::Context;
use chrono::TimeDelta;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use totp_rs::{Rfc6238, Secret, TOTP};

pub struct Config {
    pub allowed_networks: Vec<Network>,
    pub app_token_expiration: TimeDelta,
    pub cookie_domain: String,
    pub data_file: PathBuf,
    pub data_persistence_interval: TimeDelta,
    pub failed_login_ban: TimeDelta,
    pub failed_login_max_attempts_per_ip: u16,
    pub failed_login_max_attempts_per_user: u16,
    pub forward_auth_bind: String,
    pub forward_auth_log_file: Option<PathBuf>,
    pub forward_auth_port: u16,
    pub guest_link_max_expiration: TimeDelta,
    pub guest_session_cookie: String,
    pub guest_session_expiration: TimeDelta,
    pub ip_session_expiration: TimeDelta,
    pub login_bind: String,
    pub login_hostname: String,
    pub login_port: u16,
    pub login_session_cookie: String,
    pub login_session_expiration: TimeDelta,
    pub login_throttle: TimeDelta,
    pub portal_bind: String,
    pub portal_port: u16,
    pub renderer: TemplateRenderer,
    pub totps_by_user: HashMap<String, Vec<TOTP>>,
    pub unlock_api_host: String,
    pub unlock_api_status_timeout: TimeDelta,
    pub unlock_api_unlock_throttle: TimeDelta,
    pub unlock_api_unlock_timeout: TimeDelta,
    pub valid_hosts: HashSet<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::from_path("default.env").context("failed to read default.env")?;

        let config: EnvConfig = envy::from_env().context("failed to load configuration. \
        Please make sure that all requires environment variables described in the documentation are set")?;

        let i18n_contents = fs::read_to_string(&config.i18n_file)
            .with_context(|| format!("failed to read i18n file: {}", config.i18n_file.display()))?;
        let renderer = TemplateRenderer::new(&i18n_contents, &config.i18n_language)?;

        let users_str = fs::read_to_string(&config.users_file).with_context(|| {
            format!("failed to read users file: {}", config.users_file.display())
        })?;

        let mut totps_by_user: HashMap<_, Vec<_>> = HashMap::new();
        for user_str in users_str.lines() {
            let (name, totp) = parse_user(user_str).context("failed to parse user")?;
            totps_by_user.entry(name).or_default().push(totp);
        }

        let allowed_networks = config
            .allowed_networks
            .split(',')
            .filter(|network| !network.is_empty())
            .map(|network| network.parse())
            .collect::<anyhow::Result<_, _>>()?;

        let valid_hosts = config
            .valid_hosts
            .split(',')
            .map(|host| host.to_string())
            .collect();

        Ok(Config {
            allowed_networks,
            app_token_expiration: parse_duration(&config.app_token_expiration)?,
            cookie_domain: config.cookie_domain,
            data_file: config.data_file,
            data_persistence_interval: parse_duration(&config.data_persistence_interval)?,
            failed_login_ban: parse_duration(&config.failed_login_ban)?,
            failed_login_max_attempts_per_ip: config.failed_login_max_attempts_per_ip,
            failed_login_max_attempts_per_user: config.failed_login_max_attempts_per_user,
            forward_auth_bind: config.forward_auth_bind,
            forward_auth_log_file: config.forward_auth_log_file,
            forward_auth_port: config.forward_auth_port,
            guest_link_max_expiration: parse_duration(&config.guest_link_max_expiration)?,
            guest_session_cookie: config.guest_session_cookie,
            guest_session_expiration: parse_duration(&config.guest_session_expiration)?,
            ip_session_expiration: parse_duration(&config.ip_session_expiration)?,
            login_bind: config.login_bind,
            login_hostname: config.login_hostname,
            login_port: config.login_port,
            login_session_cookie: config.login_session_cookie,
            login_session_expiration: parse_duration(&config.login_session_expiration)?,
            login_throttle: parse_duration(&config.login_throttle)?,
            portal_bind: config.portal_bind,
            portal_port: config.portal_port,
            renderer,
            totps_by_user,
            unlock_api_host: config.unlock_api_host,
            unlock_api_status_timeout: parse_duration(&config.unlock_api_status_timeout)?,
            unlock_api_unlock_throttle: parse_duration(&config.unlock_api_unlock_throttle)?,
            unlock_api_unlock_timeout: parse_duration(&config.unlock_api_unlock_timeout)?,
            valid_hosts,
        })
    }
}

#[derive(Deserialize)]
struct EnvConfig {
    allowed_networks: String,
    app_token_expiration: String,
    cookie_domain: String,
    data_file: PathBuf,
    data_persistence_interval: String,
    failed_login_ban: String,
    failed_login_max_attempts_per_ip: u16,
    failed_login_max_attempts_per_user: u16,
    forward_auth_bind: String,
    forward_auth_log_file: Option<PathBuf>,
    forward_auth_port: u16,
    guest_link_max_expiration: String,
    guest_session_cookie: String,
    guest_session_expiration: String,
    i18n_file: PathBuf,
    i18n_language: String,
    ip_session_expiration: String,
    login_bind: String,
    login_hostname: String,
    login_port: u16,
    login_session_cookie: String,
    login_session_expiration: String,
    login_throttle: String,
    portal_bind: String,
    portal_port: u16,
    unlock_api_host: String,
    unlock_api_status_timeout: String,
    unlock_api_unlock_throttle: String,
    unlock_api_unlock_timeout: String,
    users_file: PathBuf,
    valid_hosts: String,
}

fn parse_user(s: &str) -> anyhow::Result<(String, TOTP)> {
    let (name, secret) = s.split_once(',').context("missing comma")?;

    let secret_bytes = Secret::Encoded(secret.trim().to_string())
        .to_bytes()
        .context(
            "failed to decode TOTP secret. \
            Make sure it's saved one secret per line and encoded in base32",
        )?;
    let totp = TOTP::from_rfc6238(
        Rfc6238::with_defaults(secret_bytes).context("failed to create TOTP instance")?,
    )
    .context("failed to create TOTP instance")?;

    let name = name.trim().to_string();
    Ok((name, totp))
}
