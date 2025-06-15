use crate::child::Child;
use crate::error::error_messages;
use crate::home::home;
use crate::mount::mount_source;
use crate::notifications::{Notifier, Priority};
use crate::scripts::backup::check_files::check_files;
use anyhow::{Context, bail, ensure};
use backup_disk::BackupDisk;
use chrono::Utc;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, thread};

mod backup_disk;
mod check_files;
mod start_services_on_drop;

pub fn backup(check_percentage: f64, check_only: bool) -> anyhow::Result<()> {
    let start = Instant::now();
    let home = home()?;

    let result = backup_inner(&home, check_percentage, check_only);
    let elapsed_minutes = start.elapsed().as_secs_f64() / 60.0;

    let (title, message, priority) = match &result {
        Ok(_) => (
            "Backup successful".to_string(),
            format!("It took {:.1} minutes", elapsed_minutes),
            Priority::Low,
        ),
        Err(error) => (
            "Backup failed".to_string(),
            error_messages(error),
            Priority::High,
        ),
    };

    if let Err(notification_error) = send_notification(&home, title, message, priority) {
        tracing::warn!("Failed to send notification: {:?}", notification_error);
    }

    result
}

fn backup_inner(home: &Path, check_percentage: f64, check_only: bool) -> anyhow::Result<()> {
    let protected_dir = home.join("protected");

    mount_source(&protected_dir).context("protected disk does not seem to be mounted")?;
    let backup_disk = BackupDisk::mount_one(home)?;

    if !check_only {
        let now = format!("{}\n", Utc::now());
        tracing::info!("Starting backup at {}", now.trim());
        fs::write(protected_dir.join("last-backup-attempt.txt"), &now)
            .context("failed to write last-backup-attempt.txt")?;

        let backup_dir = backup_disk.backup_dir(home);
        let stopped_services = start_services_on_drop::stop_containers(home)?;

        let mut child = Child::new("rsync")
            .args(backup_disk.source_dirs(home))
            .arg(&backup_dir)
            .arg("--archive")
            .arg("--delete")
            .arg("--verbose");

        for exclude in backup_disk.excludes() {
            child = child.arg("--exclude");
            child = child.arg(exclude);
        }

        child.run()?;

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
    }

    check_files(home, check_percentage, backup_disk)?;

    tracing::info!("Will unmount backup");
    Child::new("sudo")
        .arg(backup_disk.unmount_script(home))
        .run()?;

    Ok(())
}

fn send_notification(
    home: &Path,
    title: String,
    message: String,
    priority: Priority,
) -> anyhow::Result<()> {
    let notifier = Notifier::new(home)?;

    // Since the backup logic had to stop all services, we'll wait a bit for the notification server
    // to be healthy
    let give_up_at = Instant::now() + Duration::from_secs(60);
    let pooling = Duration::from_secs(2);
    loop {
        thread::sleep(pooling);
        if Instant::now() > give_up_at {
            bail!("Gave up waiting for notification service to be healthy");
        }

        match notifier.is_healthy() {
            Err(error) => {
                tracing::info!("Notification service is not yet available: {}", error)
            }
            Ok(_) => break,
        }
    }

    notifier.send_notification(title, message, priority)
}
