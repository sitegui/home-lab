use anyhow::ensure;
use regex::{Captures, Regex};
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::LazyLock;

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
    pub environment: Vec<DynamicString>,
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
pub struct DynamicString(String);

#[derive(Debug)]
pub struct Volume {
    pub volume: String,
    pub container_path: String,
    pub access_mode: String,
}

#[derive(Debug)]
pub struct Environment {
    pub name: String,
    pub value: String,
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

impl FromStr for Environment {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.split_once('=') {
            None => Ok(Environment {
                name: text.to_string(),
                value: text.to_string(),
            }),
            Some((name, value)) => Ok(Environment {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
    }
}

static INTERPOLATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\$([_a-zA-Z][_a-zA-Z0-9]*)|\$\{([^}])}"#).unwrap());

impl DynamicString {
    pub fn vars(&self) -> BTreeSet<String> {
        let mut vars = BTreeSet::new();

        for captures in INTERPOLATION_REGEX.captures_iter(&self.0) {
            let name = captures
                .get(1)
                .or_else(|| captures.get(2))
                .expect("the regex has two capturing groups")
                .as_str();
            vars.insert(name.to_string());
        }

        vars
    }

    pub fn replaced(&self, vars: &BTreeMap<String, String>) -> anyhow::Result<String> {
        let mut error = None;

        let replaced = INTERPOLATION_REGEX.replace_all(&self.0, |captures: &Captures| -> &str {
            let name = captures
                .get(1)
                .or_else(|| captures.get(2))
                .expect("the regex has two capturing groups")
                .as_str();

            match vars.get(name) {
                Some(value) => value,
                None => {
                    error = Some(anyhow::anyhow!(
                        "variable {} not found when interpolating: {}",
                        name,
                        self.0
                    ));
                    ""
                }
            }
        });

        match error {
            Some(error) => Err(error),
            None => Ok(replaced.to_string()),
        }
    }
}
