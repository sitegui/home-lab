use crate::child::Child;
use crate::mount::mount_source;
use anyhow::{Context, ensure};
use chrono::Utc;
use std::fs;

pub fn backup() -> anyhow::Result<()> {
    mount_source("data").context("data does not seem to be mounted")?;

    let now = Utc::now().to_string();
    tracing::info!("Starting backup at {}", now);
    fs::write("data/backup-witness", &now).context("failed to write backup witness")?;

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
        &[
            "data",
            "config",
            "backup",
            "--verbose",
            "--archive",
            "--delete",
        ],
    )
    .run()?;

    let witness = fs::read_to_string("backup/data/backup-witness")
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
