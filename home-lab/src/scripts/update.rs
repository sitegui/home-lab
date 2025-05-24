use crate::child::Child;
use crate::home::home;
use clap::ValueEnum;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum UpdateKind {
    /// Update podman images, pulling them. This may restart podman containers
    PodmanImages,
    /// Update packages with apt-get. This may reboot the system
    SystemPackages,
    /// Update only security packages with apt-get. This may reboot the system
    SystemPackagesForSecurity,
}

pub fn update(kind: UpdateKind) -> anyhow::Result<()> {
    match kind {
        UpdateKind::PodmanImages => {
            update_podman_images()?;
        }
        UpdateKind::SystemPackages => {
            update_system_packages("update-system-packages.sh")?;
        }
        UpdateKind::SystemPackagesForSecurity => {
            update_system_packages("update-system-packages-for-security.sh")?;
        }
    }

    Ok(())
}

fn update_podman_images() -> anyhow::Result<()> {
    #[derive(Debug, Deserialize)]
    struct Container {
        #[serde(rename = "Names")]
        names: Vec<String>,
        #[serde(rename = "Image")]
        image: String,
        #[serde(rename = "ImageID")]
        image_id: String,
    }

    let containers_contents = Child::new("podman")
        .args(["ps", "--format", "json"])
        .capture_stdout()
        .run()?
        .stdout()?;

    let containers: Vec<Container> = serde_json::from_str(&containers_contents)?;

    let mut images = BTreeMap::new();
    for container in containers {
        images
            .entry((container.image, container.image_id))
            .or_insert_with(Vec::new)
            .extend(container.names);
    }

    tracing::info!("Detected images:");
    let mut image_names = BTreeSet::new();
    for ((image, image_id), containers) in images {
        tracing::info!(
            "- {}@{}: used by {}",
            image,
            image_id,
            containers.iter().format(", ")
        );
        image_names.insert(image);
    }

    for image in image_names {
        let new_image_id = Child::new("podman")
            .args(["pull", "--quiet", &image])
            .capture_stdout()
            .run()?
            .stdout()?;

        tracing::info!("Pulled image {}@{}", image, new_image_id);
    }

    Child::new("podman").arg("auto-update").run()?;

    Ok(())
}

fn update_system_packages(script_name: &str) -> anyhow::Result<()> {
    let home = home()?;
    tracing::info!(
        "You can check the logs with `sudo journalctl -u apt-daily -r` and `less /var/log/unattended-upgrades/unattended-upgrades.log`"
    );
    Child::new("sudo")
        .arg(home.join("sudo-scripts").join(script_name))
        .run()?;
    Ok(())
}
