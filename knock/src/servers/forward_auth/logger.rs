use crate::file_appender::FileAppender;
use crate::servers::forward_auth::request_info::RequestInfo;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug)]
pub struct Logger(FileAppender);

#[derive(Serialize)]
struct Log<'a> {
    arrival: DateTime<Utc>,
    headers: BTreeMap<&'a str, &'a str>,
}

impl Logger {
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        Ok(Self(FileAppender::new(path).await?))
    }

    pub async fn log(&self, request: &RequestInfo) -> anyhow::Result<()> {
        let headers = request
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("<INVALID UTF-8>")))
            .collect();

        let log = Log {
            arrival: request.arrival(),
            headers,
        };

        let log_str = serde_json::to_string(&log)?;
        self.0.append(log_str.as_bytes()).await?;
        self.0.append(b"\n").await
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        self.0.flush().await
    }
}
