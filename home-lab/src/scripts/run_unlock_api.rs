use crate::child::Child;
use crate::home::home;
use crate::mount::mount_source;
use anyhow::Context;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::task;
use web_error::WebError;

mod web_error;

#[tokio::main]
pub async fn run_unlock_api(bind: String, port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind((bind.as_str(), port)).await?;

    let router = Router::new()
        .route("/status", get(status_handler))
        .route("/unlock", post(unlock_handler));
    tracing::info!("Listening on {}", listener.local_addr()?);
    let server = tokio::spawn(axum::serve(listener, router).into_future());

    server.await.unwrap()?;

    Ok(())
}

#[derive(Serialize)]
struct StatusResponse {
    is_unlocked: bool,
}

async fn status_handler() -> Result<Json<StatusResponse>, WebError> {
    let home = home().context("failed to get home information")?;

    let is_unlocked = mount_source(&home.join("protected")).is_ok();

    Ok(Json(StatusResponse { is_unlocked }))
}

#[derive(Deserialize)]
struct UnlockRequest {
    password: String,
}

#[derive(Serialize)]
struct UnlockResponse {
    is_unlocked: bool,
}

async fn unlock_handler(
    Json(request): Json<UnlockRequest>,
) -> Result<Json<UnlockResponse>, WebError> {
    let home = home()?;
    let protected_dir = home.join("protected");

    let is_unlocked = if mount_source(&protected_dir).is_ok() {
        tracing::info!("Protected disk is already mounted: nothing to do");
        true
    } else {
        tracing::info!("Unlocking...");
        let unlock_result = task::spawn_blocking(move || {
            Child::new("sudo")
                .arg(home.join("sudo-scripts/mount-protected.sh"))
                .stdin(request.password)
                .ignore_status()
                .run()
                .context("failed to run mount script")
        })
        .await
        .unwrap()?;

        if !unlock_result.status().success() {
            false
        } else {
            tracing::info!("Starting protected services");
            task::spawn_blocking(move || {
                Child::new("systemctl")
                    .args(["--user", "start", "protected.target"])
                    .run()
                    .context("failed to start services")
            })
            .await
            .unwrap()?;

            true
        }
    };

    Ok(Json(UnlockResponse { is_unlocked }))
}
