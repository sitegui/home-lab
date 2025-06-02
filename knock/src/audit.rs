use crate::data::UserName;
use crate::file_appender::FileAppender;
use crate::string_hash::StringHash;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::Serialize;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct Audit(Mutex<Option<AuditInner>>);

#[derive(Debug)]
pub struct AuditInner {
    tx: UnboundedSender<String>,
    writer_task: JoinHandle<anyhow::Result<()>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum AuditEvent<'a> {
    IpAllowed {
        ip: IpAddr,
        by_session: StringHash,
        until_at_least: DateTime<Utc>,
    },
    NewLoginSession {
        user: &'a UserName,
        session: StringHash,
        expires_at: DateTime<Utc>,
    },
    NewInviteeSession {
        invited_by: StringHash,
        session: StringHash,
        expires_at: DateTime<Utc>,
    },
    NewInviteLink {
        link_hash: StringHash,
        generated_by: StringHash,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Serialize)]
struct Log<'a> {
    datetime: DateTime<Utc>,
    event: AuditEvent<'a>,
}

impl Audit {
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        let appender = Arc::new(FileAppender::new(path).await?);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let writer_task = tokio::spawn(async move {
            while let Some(log) = rx.recv().await {
                appender.append(log.as_bytes()).await?;
                appender.append(b"\n").await?;
                appender.soft_flush().await?;
            }

            appender.flush().await
        });

        let inner = AuditInner { tx, writer_task };
        Ok(Audit(Mutex::new(Some(inner))))
    }

    pub fn report(&self, event: AuditEvent<'_>) {
        let log = Log {
            datetime: Utc::now(),
            event,
        };

        let log_str = serde_json::to_string(&log).expect("json serialization should not fail");
        if let Some(inner) = &*self.0.lock() {
            let _ = inner.tx.send(log_str);
        }
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        let inner = self.0.lock().take();
        if let Some(inner) = inner {
            drop(inner.tx);
            inner.writer_task.await.unwrap()?;
        }

        Ok(())
    }
}
