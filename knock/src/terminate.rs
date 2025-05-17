use std::sync::{Arc, LazyLock};
use tokio::sync::Notify;

pub static TERMINATE: LazyLock<Terminate> = LazyLock::new(Terminate::new);

pub struct Terminate(Arc<Notify>);

impl Terminate {
    fn new() -> Self {
        let notify = Arc::new(Notify::new());

        let notify_clone = notify.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C handler");

            tracing::warn!("Shutting down...");
            notify_clone.notify_waiters();
        });

        Self(notify)
    }

    pub async fn wait(&self) {
        self.0.notified().await;
    }
}
