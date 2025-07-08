use crate::child::Child;
use anyhow::Context;
use itertools::Itertools;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub writable_binds: Vec<PathBuf>,
}

pub fn list_services() -> anyhow::Result<Vec<Service>> {
    let mut services = vec![];

    let containers = Child::new("podman")
        .args(["container", "list", "--quiet"])
        .capture_stdout()
        .run()?
        .stdout()?
        .split('\n')
        .filter(|container| !container.is_empty())
        .map(|container| container.to_string())
        .collect_vec();

    tracing::debug!("Found containers: {:?}", containers);

    for container in containers {
        let service = parse_service(&container)
            .with_context(|| format!("failed to parse service for {}", container))?;

        if let Some(service) = service {
            services.push(service);
        }
    }

    Ok(services)
}

fn parse_service(container: &str) -> anyhow::Result<Option<Service>> {
    #[derive(Debug, Deserialize)]
    struct PodmanContainer {
        #[serde(rename = "Config")]
        config: PodmanConfig,
        #[serde(rename = "Mounts")]
        mounts: Vec<PodmanMount>,
    }
    #[derive(Debug, Deserialize)]
    struct PodmanConfig {
        #[serde(rename = "Labels")]
        labels: PodmanLabels,
    }
    #[derive(Debug, Deserialize)]
    struct PodmanLabels {
        #[serde(rename = "PODMAN_SYSTEMD_UNIT")]
        unit: Option<String>,
    }
    #[derive(Debug, Deserialize)]
    struct PodmanMount {
        #[serde(rename = "Type")]
        kind: String,
        #[serde(rename = "Source")]
        source: PathBuf,
        #[serde(rename = "rw")]
        rw: bool,
    }

    let inspect_str = Child::new("podman")
        .args(["container", "inspect", container])
        .capture_stdout()
        .run()?
        .stdout()?;

    let containers: [PodmanContainer; 1] = serde_json::from_str(&inspect_str)
        .with_context(|| format!("failed parse JSON: {}", inspect_str))?;
    let [container] = containers;

    let Some(name) = container.config.labels.unit else {
        return Ok(None);
    };

    let writable_binds = container
        .mounts
        .into_iter()
        .filter(|mount| mount.rw && mount.kind == "bind")
        .map(|mount| mount.source.canonicalize())
        .try_collect()?;

    Ok(Some(Service {
        name,
        writable_binds,
    }))
}
