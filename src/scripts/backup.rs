use crate::child::Child;
use crate::mount::mount_source;
use anyhow::{Context, ensure};
use chrono::Utc;
use std::fs;

pub fn backup() -> anyhow::Result<()> {
    mount_source("protected").context("protected disk does not seem to be mounted")?;

    let now = format!("{}\n", Utc::now());
    tracing::info!("Starting backup at {}", now.trim());
    fs::write("protected/last-backup-attempt.txt", &now)
        .context("failed to write protected/last-backup-attempt.txt")?;

    let backup_mount = match mount_source("backup-1") {
        Ok(backup_mount) => backup_mount,
        Err(_) => {
            tracing::info!("Will try to mount backup-1");
            Child::new("sudo", &["./bare/mount-backup-1.sh"]).run()?;
            mount_source("backup-1").context("failed to mount backup-1")?
        }
    };

    tracing::info!("Backing up into {}", backup_mount);
    Child::new(
        "rsync",
        &[
            "bare",
            "protected",
            "backup-1",
            "--archive",
            "--delete",
            "--verbose",
        ],
    )
    .run()?;

    fs::copy(
        "backup-1/protected/last-backup-attempt.txt",
        "protected/last-successful-backup.txt",
    )
    .context("failed to copy backup-1/protected/last-backup-attempt.txt")?;
    let witness = fs::read_to_string("protected/last-successful-backup.txt")
        .context("failed to read protected/last-successful-backup.txt")?;
    ensure!(
        witness == now,
        "the backup witness file content is not the expected one"
    );
    tracing::info!("Witness file has expected content, backup is up to date");

    tracing::info!("Will unmount backup");
    Child::new("sudo", &["./bare/umount-backup-1.sh"]).run()?;

    Ok(())
}
