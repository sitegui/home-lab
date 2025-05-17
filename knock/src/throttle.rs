use chrono::TimeDelta;
use tokio::sync::Mutex;

/// A helper struct that will impose a global waiting queue
#[derive(Debug, Default)]
pub struct Throttle(Mutex<()>);

impl Throttle {
    /// This call will block the current async task for at least `duration`. Other tasks calling
    /// this method on the same instance will also block and be queued, effectively limiting the
    /// maximum throughput
    pub async fn wait(&self, duration: TimeDelta) {
        let _guard = self.0.lock().await;
        tokio::time::sleep(duration.to_std().unwrap()).await;
    }
}
