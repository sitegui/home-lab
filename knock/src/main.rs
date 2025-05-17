#[macro_use]
mod macros;
mod alive_timer;
mod ban_timer;
mod common;
mod config;
mod data;
mod forward_auth;
mod i18n;
mod login;
mod network;
mod parse_duration;
mod persistence;
mod string_hash;
mod terminate;
mod throttle;

use crate::config::Config;
use crate::data::Data;
use crate::forward_auth::handle_forward_auth;
use crate::login::{handle_login_action, handle_login_page};
use crate::persistence::load_and_spawn_persist_loop;
use crate::terminate::TERMINATE;
use crate::throttle::Throttle;
use axum::Router;
use axum::routing::get;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::net::TcpListener;

struct AppState {
    data: Arc<Mutex<Data>>,
    config: Config,
    throttle: Throttle,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;

    let forward_auth_listener =
        TcpListener::bind((config.forward_auth_bind.as_str(), config.forward_auth_port)).await?;
    let login_listener = TcpListener::bind((config.login_bind.as_str(), config.login_port)).await?;

    let data =
        load_and_spawn_persist_loop(config.data_file.clone(), config.data_persistence_interval);
    let state = Arc::new(AppState {
        data,
        config,
        throttle: Throttle::default(),
    });

    let forward_auth_router = Router::new()
        .route("/", get(handle_forward_auth))
        .with_state(state.clone());
    tracing::info!(
        "Forward auth listening on {}",
        forward_auth_listener.local_addr()?
    );
    let forward_auth_server = tokio::spawn(
        axum::serve(forward_auth_listener, forward_auth_router)
            .with_graceful_shutdown(TERMINATE.wait())
            .into_future(),
    );

    let login_router = Router::new()
        .route("/", get(handle_login_page).post(handle_login_action))
        .with_state(state);
    tracing::info!("Login listening on {}", login_listener.local_addr()?);
    let login_server = tokio::spawn(
        axum::serve(login_listener, login_router)
            .with_graceful_shutdown(TERMINATE.wait())
            .into_future(),
    );

    forward_auth_server.await.unwrap()?;
    login_server.await.unwrap()?;

    Ok(())
}
