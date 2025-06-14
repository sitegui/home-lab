use anyhow::Context;
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use std::path::Path;

pub struct Notifier {
    client: Client,
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
            client: Client::new(),
            token,
        })
    }

    pub fn send_notification(
        &self,
        title: String,
        message: String,
        priority: Priority,
    ) -> anyhow::Result<()> {
        self.client
            .post("https://notifications.sitegui.dev/message")
            .bearer_auth(&self.token)
            .json(&json!({
                "title": title,
                "message": message,
                "priority": priority.to_int()
            }))
            .send()?
            .error_for_status()?;

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
