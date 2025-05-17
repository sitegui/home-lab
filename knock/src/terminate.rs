use std::sync::{Arc, LazyLock};
use tokio::signal::unix;
use tokio::signal::unix::SignalKind;
use tokio::sync::Notify;

pub static TERMINATE: LazyLock<Terminate> = LazyLock::new(Terminate::new);

pub struct Terminate(Arc<Notify>);

impl Terminate {
    fn new() -> Self {
        let notify = Arc::new(Notify::new());

        tokio::spawn(Self::monitor(notify.clone(), SignalKind::interrupt()));
        tokio::spawn(Self::monitor(notify.clone(), SignalKind::terminate()));

        Self(notify)
    }

    async fn monitor(notify: Arc<Notify>, signal_kind: SignalKind) {
        unix::signal(signal_kind)
            .expect("failed to install signal handler")
            .recv()
            .await;

        tracing::warn!("Shutting down...");
        notify.notify_waiters();
    }

    pub async fn wait(&self) {
        self.0.notified().await;
    }
}
