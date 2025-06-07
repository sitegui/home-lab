use crate::servers::forward_auth::access_level::AccessLevel;
use crate::servers::forward_auth::request_info::RequestInfo;
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct Logger {
    sender: UnboundedSender<String>,
    writer_task: JoinHandle<anyhow::Result<()>>,
}

#[derive(Serialize)]
struct Log<'a> {
    arrival: DateTime<Utc>,
    headers: HashMap<&'a str, &'a str>,
    access_level: &'a AccessLevel<'a>,
}

impl Logger {
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await
            .with_context(|| format!("failed to open file: {}", path.display()))?;

        let writer_task = tokio::spawn(Logger::writer_task(file, receiver));

        Ok(Self {
            sender,
            writer_task,
        })
    }

    async fn writer_task(
        mut file: File,
        mut receiver: UnboundedReceiver<String>,
    ) -> anyhow::Result<()> {
        while let Some(line) = receiver.recv().await {
            file.write_all(line.as_bytes())
                .await
                .context("failed to write to log file")?;
        }

        file.sync_all().await.context("failed to sync log file")?;

        Ok(())
    }

    pub fn log(&self, request: &RequestInfo, access_level: &AccessLevel<'_>) -> anyhow::Result<()> {
        let headers = request
            .headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("<INVALID UTF-8>")))
            .collect();

        let log = Log {
            arrival: request.arrival,
            headers,
            access_level,
        };

        let mut log_str = serde_json::to_string(&log)?;
        log_str.push('\n');

        let _ = self.sender.send(log_str);
        Ok(())
    }

    pub async fn shutdown(self) -> anyhow::Result<()> {
        drop(self.sender);
        self.writer_task.await.expect("writer_task panicked")
    }
}
