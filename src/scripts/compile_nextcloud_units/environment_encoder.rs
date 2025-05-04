use crate::scripts::compile_nextcloud_units::compose_schema::{DynamicString, Environment};
use anyhow::{Context, bail};
use itertools::Itertools;
use regex::{Captures, Regex};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

/// A helper struct to translate the environment variables for the containers
#[derive(Debug)]
pub struct EnvironmentEncoder {
    secret_vars: BTreeMap<String, String>,
    vars: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct ServiceEnvironmentEncoder<'a> {
    encoder: &'a EnvironmentEncoder,
    secrets: BTreeMap<String, String>,
}

#[derive(Debug)]
enum Encoded {
    Public(String),
    Secret(String),
}

impl EnvironmentEncoder {
    pub fn new(secret_vars_path: &Path, vars_path: &Path) -> anyhow::Result<Self> {
        let secret_vars = dotenvy::from_path_iter(secret_vars_path)
            .with_context(|| format!("failed to read {}", secret_vars_path.display()))?
            .try_collect()?;
        let vars = dotenvy::from_path_iter(vars_path)
            .with_context(|| format!("failed to read {}", vars_path.display()))?
            .try_collect()?;

        Ok(Self { secret_vars, vars })
    }
}

impl<'a> ServiceEnvironmentEncoder<'a> {
    pub fn new(encoder: &'a EnvironmentEncoder) -> Self {
        Self {
            encoder,
            secrets: BTreeMap::new(),
        }
    }

    pub fn encode_public(&self, data: &DynamicString) -> anyhow::Result<String> {
        match self.encode(data)? {
            Encoded::Public(value) => Ok(value),
            Encoded::Secret(_) => bail!("secret variable is not allowed in public information",),
        }
    }

    pub fn encode_public_opt(
        &self,
        data: &Option<DynamicString>,
    ) -> anyhow::Result<Option<String>> {
        data.as_ref()
            .map(|data| self.encode_public(data))
            .transpose()
    }

    pub fn encode_public_vec(&self, data: &[DynamicString]) -> anyhow::Result<Vec<String>> {
        data.iter().map(|data| self.encode_public(data)).collect()
    }

    pub fn encode_environment(
        &mut self,
        data: &Environment,
    ) -> anyhow::Result<Option<(String, String)>> {
        let name = self.encode(&data.name)?;
        let value = self.encode(&data.value)?;

        match (name, value) {
            (Encoded::Public(name), Encoded::Public(value)) => Ok(Some((name, value))),
            (name, value) => {
                self.secrets.insert(name.into_string(), value.into_string());
                Ok(None)
            }
        }
    }

    fn encode(&self, data: &DynamicString) -> anyhow::Result<Encoded> {
        static INTERPOLATION_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"\$([_a-zA-Z][_a-zA-Z0-9]*)|\$\{([^}]+)}"#).unwrap());

        let mut is_secret = false;
        let mut error = None;

        let replaced = INTERPOLATION_REGEX.replace_all(&data.0, |captures: &Captures| -> &str {
            let name = captures
                .get(1)
                .or_else(|| captures.get(2))
                .expect("the regex has two capturing groups")
                .as_str();

            if let Some(secret_value) = self.encoder.secret_vars.get(name) {
                is_secret = true;
                secret_value
            } else if let Some(value) = self.encoder.vars.get(name) {
                value
            } else {
                error = Some(anyhow::anyhow!(
                    "variable {} not found when interpolating: {}",
                    name,
                    data.0
                ));
                ""
            }
        });

        match error {
            Some(error) => Err(error),
            None => {
                let value = replaced.to_string();
                let value = if is_secret {
                    Encoded::Secret(value)
                } else {
                    Encoded::Public(value)
                };

                Ok(value)
            }
        }
    }

    pub fn secret_env_contents(&self) -> Option<String> {
        let contents = self
            .secrets
            .iter()
            .format_with("\n", |(k, v), f| f(&format_args!("{}={}", k, v)))
            .to_string();

        (!contents.is_empty()).then_some(contents)
    }
}

impl Encoded {
    fn into_string(self) -> String {
        match self {
            Encoded::Public(value) => value,
            Encoded::Secret(value) => value,
        }
    }
}
