use crate::data::Data;
use anyhow::Context;
use chrono::TimeDelta;
use parking_lot::Mutex;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub fn load_and_spawn_persist_loop(path: PathBuf, flush_interval: TimeDelta) -> Arc<Mutex<Data>> {
    let data = match fs::read_to_string(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("No previous data found");
            Data::default()
        }
        Err(error) => {
            tracing::error!("Failed to read previous data: {:?}", error);
            Data::default()
        }
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|error| {
            tracing::error!("Failed to parse previous data: {:?}", error);
            Data::default()
        }),
    };

    let data = Arc::new(Mutex::new(data));

    tokio::spawn(persist_loop(data.clone(), path, flush_interval));

    data
}

async fn persist_loop(data: Arc<Mutex<Data>>, path: PathBuf, flush_interval: TimeDelta) {
    loop {
        tokio::time::sleep(flush_interval.to_std().unwrap()).await;

        let persisted = serde_json::to_string(&*data.lock())
            .context("failed to serialize")
            .and_then(|contents| fs::write(&path, contents).context("failed to persist"));

        match persisted {
            Err(error) => {
                tracing::error!("Failed to persist data: {:?}", error);
            }
            Ok(()) => {
                tracing::debug!("Data persisted");
            }
        }
    }
}
