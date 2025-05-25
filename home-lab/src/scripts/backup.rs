use crate::child::Child;
use crate::home::home;
use crate::mount::mount_source;
use anyhow::{Context, ensure};
use chrono::Utc;
use itertools::Itertools;
use std::fs;
use std::path::{Path, PathBuf};

pub fn backup() -> anyhow::Result<()> {
    let home = home()?;
    let protected_dir = home.join("protected");

    mount_source(&protected_dir).context("protected disk does not seem to be mounted")?;

    let now = format!("{}\n", Utc::now());
    tracing::info!("Starting backup at {}", now.trim());
    fs::write(protected_dir.join("last-backup-attempt.txt"), &now)
        .context("failed to write last-backup-attempt.txt")?;

    let backup_dir = ensure_backup_mounted(&home)?;
    let stopped_services = stop_containers(&home)?;

    Child::new("rsync")
        .args([
            home.join("bare"),
            home.join("protected"),
            home.join("backup-1"),
        ])
        .args([
            "--archive",
            "--delete",
            "--verbose",
            "--exclude",
            "/protected/nextcloud/volumes/nextcloud_aio_nextcloud_data/sitegui/files/Jellyfin",
        ])
        .run()?;

    drop(stopped_services);

    let last_successful_backup = protected_dir.join("last-successful-backup.txt");
    fs::copy(
        backup_dir.join("protected/last-backup-attempt.txt"),
        &last_successful_backup,
    )
    .with_context(|| format!("failed to copy {}", last_successful_backup.display()))?;
    let witness = fs::read_to_string(&last_successful_backup)
        .with_context(|| format!("failed to read {}", last_successful_backup.display()))?;
    ensure!(
        witness == now,
        "the backup witness file content is not the expected one"
    );
    tracing::info!("Witness file has expected content, backup is up to date");

    tracing::info!("Will unmount backup");
    Child::new("sudo")
        .arg(home.join("sudo-scripts/umount-backup-1.sh"))
        .run()?;

    Ok(())
}

fn ensure_backup_mounted(home: &Path) -> anyhow::Result<PathBuf> {
    let backup_dir = home.join("backup-1");

    let backup_mount = match mount_source(&backup_dir) {
        Ok(backup_mount) => backup_mount,
        Err(_) => {
            tracing::info!("Will try to mount backup-1");
            Child::new("sudo")
                .arg(home.join("sudo-scripts/mount-backup-1.sh"))
                .run()?;
            mount_source(&backup_dir).context("failed to mount backup-1")?
        }
    };
    tracing::info!("Backing up into {}", backup_mount);

    Ok(backup_dir)
}

struct StartServicesOnDrop(Vec<String>);

fn stop_containers(home: &Path) -> anyhow::Result<StartServicesOnDrop> {
    let mut container_services = vec![];
    for item in fs::read_dir(home.join(".config/containers/systemd"))? {
        let item = item?;
        if item.file_type()?.is_file() {
            let file_name = item
                .file_name()
                .into_string()
                .ok()
                .context("failed to get file name")?;
            let Some(name) = file_name.strip_suffix(".container") else {
                continue;
            };
            container_services.push(name.to_owned());
        }
    }

    tracing::info!(
        "Stopping services: {}",
        container_services.iter().format(", ")
    );
    Child::new("systemctl")
        .args(["--user", "stop"])
        .args(&container_services)
        .run()?;

    Ok(StartServicesOnDrop(container_services))
}

impl Drop for StartServicesOnDrop {
    fn drop(&mut self) {
        tracing::info!("Starting services: {}", self.0.iter().format(", "));

        if let Err(error) = Child::new("systemctl")
            .args(["--user", "start"])
            .args(&self.0)
            .run()
        {
            tracing::error!("Failed to start services: {:?}", error);
        }
    }
}
