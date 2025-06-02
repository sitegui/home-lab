use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct FileAppender(Mutex<File>);

impl FileAppender {
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await?;

        Ok(Self(Mutex::new(file)))
    }

    pub async fn append(&self, bytes: &[u8]) -> anyhow::Result<()> {
        self.0.lock().await.write_all(bytes).await?;
        Ok(())
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        self.0.lock().await.sync_all().await?;
        Ok(())
    }
}
