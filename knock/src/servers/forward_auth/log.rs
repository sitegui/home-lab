use crate::servers::forward_auth::request_info::RequestInfo;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use tokio::fs::File;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Serialize)]
struct Log<'a> {
    arrival: DateTime<Utc>,
    headers: BTreeMap<&'a str, &'a str>,
}

pub async fn log(file: &mut impl AsyncWrite, request: &RequestInfo) -> anyhow::Result<()> {
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
    file.write_all(log_str.as_bytes()).await?;

    Ok(())
}
