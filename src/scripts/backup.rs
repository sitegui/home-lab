use crate::child::Child;
use crate::mount::mount_source;
use anyhow::{Context, ensure};
use chrono::Utc;
use std::fs;

pub fn backup() -> anyhow::Result<()> {
    mount_source("data").context("data does not seem to be mounted")?;

    let now = format!("{}\n", Utc::now());
    tracing::info!("Starting backup at {}", now.trim());
    fs::write("data/last-backup-attempt", &now).context("failed to write next backup witness")?;

    let backup_mount = match mount_source("backup") {
        Ok(backup_mount) => backup_mount,
        Err(_) => {
            tracing::info!("Will try to mount backup");
            Child::new("sudo", &["./config/scripts/mount-backup-1"]).run()?;
            mount_source("backup").context("failed to mount backup")?
        }
    };

    tracing::info!("Backing up into {}", backup_mount);
    Child::new(
        "rsync",
        &["data", "config", "backup", "--archive", "--delete"],
    )
    .run()?;

    fs::copy(
        "backup/data/last-backup-attempt",
        "data/last-successful-backup",
    )
    .context("failed to copy backup witness")?;
    let witness = fs::read_to_string("data/last-successful-backup")
        .context("failed to read backup witness")?;
    ensure!(
        witness == now,
        "the backup witness file content is not the expected one"
    );
    tracing::info!("Witness file has expected content, backup is up to date");

    tracing::info!("Will unmount backup");
    Child::new("sudo", &["./config/scripts/umount-backup-1"]).run()?;

    Ok(())
}
