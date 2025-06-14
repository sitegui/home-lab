use anyhow::Context;
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub struct Notifier {
    client: Client,
    host: String,
    token: String,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Notifier {
    /// Create a new instance, reading the token from `~/bare/home-lab-gotify-token.txt`
    pub fn new(home: &Path) -> anyhow::Result<Self> {
        let token = fs::read_to_string(home.join("bare/home-lab-gotify-token.txt"))
            .context("could not read token")?;
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(5)).build()?,
            host: "https://notifications.sitegui.dev".to_string(),
            token,
        })
    }

    pub fn is_healthy(&self) -> anyhow::Result<()> {
        self.client
            .get(format!("{}/health", self.host))
            .send()?
            .error_for_status()?;
        Ok(())
    }

    pub fn send_notification(
        &self,
        title: String,
        message: String,
        priority: Priority,
    ) -> anyhow::Result<()> {
        let response = self
            .client
            .post(format!("{}/message", self.host))
            .bearer_auth(&self.token)
            .json(&json!({
                "title": title,
                "message": message,
                "priority": priority.to_int()
            }))
            .send()?
            .error_for_status()?
            .text()?;

        tracing::info!("Notification sent: {}", response);

        Ok(())
    }
}

impl Priority {
    fn to_int(self) -> i32 {
        match self {
            Priority::Low => 0,
            Priority::Medium => 5,
            Priority::High => 10,
        }
    }
}
