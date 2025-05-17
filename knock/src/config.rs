use crate::i18n::I18n;
use crate::network::Network;
use crate::parse_duration::parse_duration;
use anyhow::Context;
use chrono::TimeDelta;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use totp_rs::{Rfc6238, Secret, TOTP};

pub struct Config {
    pub allowed_networks: Vec<Network>,
    pub data_file: PathBuf,
    pub data_persistence_interval: TimeDelta,
    pub failed_login_ban: TimeDelta,
    pub failed_login_max_attempts_per_ip: u16,
    pub failed_login_max_attempts_per_user: u16,
    pub forward_auth_bind: String,
    pub forward_auth_port: u16,
    pub i18n: I18n,
    pub i18n_language: String,
    pub ip_session_max_inactivity: TimeDelta,
    pub knock_cookie_domain: String,
    pub knock_cookie_name: String,
    pub login_bind: String,
    pub login_hostname: String,
    pub login_port: u16,
    pub login_throttle: TimeDelta,
    pub session_max_inactivity: TimeDelta,
    pub session_max_lifetime: TimeDelta,
    pub totps_by_user: BTreeMap<String, Vec<TOTP>>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::from_path("default.env").context("failed to read default.env")?;

        let config: EnvConfig = envy::from_env().context("failed to load configuration. \
        Please make sure that all requires environment variables described in the documentation are set")?;

        let i18n_contents = fs::read_to_string(&config.i18n_file)
            .with_context(|| format!("failed to read i18n file: {}", config.i18n_file.display()))?;
        let i18n = I18n::new(&i18n_contents)?;

        let users_str = fs::read_to_string(&config.users_file).with_context(|| {
            format!("failed to read users file: {}", config.users_file.display())
        })?;

        let mut totps_by_user: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for user_str in users_str.lines() {
            let (name, totp) = parse_user(user_str).context("failed to parse user")?;
            totps_by_user.entry(name).or_default().push(totp);
        }

        let allowed_networks = config
            .allowed_networks
            .split(',')
            .map(|network| network.parse())
            .collect::<anyhow::Result<_, _>>()?;

        Ok(Config {
            allowed_networks,
            data_file: config.data_file,
            data_persistence_interval: parse_duration(&config.data_persistence_interval)?,
            failed_login_ban: parse_duration(&config.failed_login_ban)?,
            failed_login_max_attempts_per_ip: config.failed_login_max_attempts_per_ip,
            failed_login_max_attempts_per_user: config.failed_login_max_attempts_per_user,
            forward_auth_bind: config.forward_auth_bind,
            forward_auth_port: config.forward_auth_port,
            i18n,
            i18n_language: config.i18n_language,
            ip_session_max_inactivity: parse_duration(&config.ip_session_max_inactivity)?,
            knock_cookie_domain: config.knock_cookie_domain,
            knock_cookie_name: config.knock_cookie_name,
            login_bind: config.login_bind,
            login_hostname: config.login_hostname,
            login_port: config.login_port,
            login_throttle: parse_duration(&config.login_throttle)?,
            session_max_inactivity: parse_duration(&config.session_max_inactivity)?,
            session_max_lifetime: parse_duration(&config.session_max_lifetime)?,
            totps_by_user,
        })
    }
}

#[derive(Deserialize)]
struct EnvConfig {
    allowed_networks: String,
    data_file: PathBuf,
    data_persistence_interval: String,
    failed_login_ban: String,
    failed_login_max_attempts_per_ip: u16,
    failed_login_max_attempts_per_user: u16,
    forward_auth_bind: String,
    forward_auth_port: u16,
    i18n_file: PathBuf,
    i18n_language: String,
    ip_session_max_inactivity: String,
    knock_cookie_domain: String,
    knock_cookie_name: String,
    login_bind: String,
    login_hostname: String,
    login_port: u16,
    login_throttle: String,
    session_max_inactivity: String,
    session_max_lifetime: String,
    users_file: PathBuf,
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
