use crate::throttle::Throttle;
use chrono::TimeDelta;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

pub struct UnlockApi {
    http_client: Client,
    host: String,
    status_timeout: Duration,
    unlock_timeout: Duration,
    unlock_throttle: TimeDelta,
    throttle: Throttle,
}

impl UnlockApi {
    pub fn new(
        host: String,
        status_timeout: TimeDelta,
        unlock_timeout: TimeDelta,
        unlock_throttle: TimeDelta,
    ) -> anyhow::Result<Self> {
        Ok(UnlockApi {
            http_client: Client::new(),
            host,
            status_timeout: status_timeout.to_std()?,
            unlock_timeout: unlock_timeout.to_std()?,
            unlock_throttle,
            throttle: Throttle::default(),
        })
    }

    pub async fn is_unlocked(&self) -> anyhow::Result<bool> {
        #[derive(Deserialize)]
        struct Response {
            is_unlocked: bool,
        }

        let response: Response = self
            .http_client
            .get(format!("{}/status", self.host))
            .timeout(self.status_timeout)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response.is_unlocked)
    }

    pub async fn unlock(&self, password: &str) -> anyhow::Result<bool> {
        self.throttle.wait(self.unlock_throttle).await;

        #[derive(Deserialize)]
        struct Response {
            is_unlocked: bool,
        }

        let response: Response = self
            .http_client
            .post(format!("{}/unlock", self.host))
            .json(&json!({
                "password": password,
            }))
            .timeout(self.unlock_timeout)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response.is_unlocked)
    }
}
