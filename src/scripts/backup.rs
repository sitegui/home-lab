use crate::child::Child;
use crate::home::home;
use crate::mount::mount_source;
use anyhow::{Context, ensure};
use chrono::Utc;
use std::fs;

pub fn backup() -> anyhow::Result<()> {
    let home = home()?;
    let protected_dir = home.join("protected");

    mount_source(&protected_dir).context("protected disk does not seem to be mounted")?;

    let now = format!("{}\n", Utc::now());
    tracing::info!("Starting backup at {}", now.trim());
    fs::write(protected_dir.join("last-backup-attempt.txt"), &now)
        .context("failed to write last-backup-attempt.txt")?;

    let backup_dir = home.join("backup-1");
    let backup_mount = match mount_source(&backup_dir) {
        Ok(backup_mount) => backup_mount,
        Err(_) => {
            tracing::info!("Will try to mount backup-1");
            Child::new("sudo")
                .arg(home.join("home-lab/config/mount-backup-1.sh"))
                .run()?;
            mount_source(&backup_dir).context("failed to mount backup-1")?
        }
    };

    tracing::info!("Backing up into {}", backup_mount);
    Child::new("rsync")
        .args([
            home.join("bare"),
            home.join("protected"),
            home.join("backup-1"),
        ])
        .args(["--archive", "--delete", "--verbose"])
        .run()?;

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
        .arg(home.join("home-lab/config/umount-backup-1.sh"))
        .run()?;

    Ok(())
}
