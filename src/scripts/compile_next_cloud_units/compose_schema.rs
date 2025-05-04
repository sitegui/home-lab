use anyhow::ensure;
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Note: this is not meant to be a complete schema, just the parts we need.
#[derive(Debug, Deserialize)]
pub struct Compose {
    pub services: BTreeMap<String, ComposeService>,
}

#[derive(Debug, Deserialize)]
pub struct ComposeService {
    #[serde(default)]
    pub depends_on: BTreeMap<String, IgnoredAny>,
    pub image: DynamicString,
    pub user: Option<DynamicString>,
    #[serde(default)]
    pub init: bool,
    pub healthcheck: Healthcheck,
    #[serde(default)]
    pub environment: Vec<Environment>,
    #[serde(default)]
    pub volumes: Vec<DynamicString>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub tmpfs: Vec<DynamicString>,
    #[serde(default)]
    pub cap_drop: Vec<DynamicString>,
    #[serde(default)]
    pub cap_add: Vec<DynamicString>,
    pub command: Option<DynamicString>,
    #[serde(default)]
    pub profiles: Vec<DynamicString>,
    pub stop_grace_period: Option<DynamicString>,
    pub shm_size: Option<DynamicString>,
}

#[derive(Debug, Deserialize)]
pub struct Healthcheck {
    pub start_period: DynamicString,
    pub test: DynamicString,
    pub interval: DynamicString,
    pub timeout: DynamicString,
    pub retries: i32,
}

#[derive(Debug, Deserialize)]
pub struct DynamicString(pub String);

#[derive(Debug)]
pub struct Volume {
    pub volume: String,
    pub container_path: String,
    pub access_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(from = "String")]
pub struct Environment {
    pub name: DynamicString,
    pub value: DynamicString,
}

impl FromStr for Volume {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = text.split(':').collect();
        ensure!(parts.len() == 3);

        Ok(Volume {
            volume: parts[0].to_string(),
            container_path: parts[1].to_string(),
            access_mode: parts[2].to_string(),
        })
    }
}

impl From<String> for Environment {
    fn from(text: String) -> Self {
        match text.split_once('=') {
            None => Environment {
                value: DynamicString(format!("${{{}}}", text)),
                name: DynamicString(text),
            },
            Some((name, value)) => Environment {
                name: DynamicString(name.to_string()),
                value: DynamicString(value.to_string()),
            },
        }
    }
}
