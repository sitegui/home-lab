use crate::child::Child;
use crate::error::error_messages;
use crate::home::home;
use crate::mount::mount_source;
use crate::notifications::{Notifier, Priority};
use crate::scripts::backup::check_files::CheckStats;
use crate::scripts::backup::do_backup::{backup_other_files, backup_service};
use anyhow::{Context, bail, ensure};
use backup_disk::BackupDisk;
use chrono::Utc;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, thread};

mod backup_disk;
mod check_files;
mod do_backup;
mod list_services;
mod start_service_on_drop;

pub fn backup(check_percentage: f64, target_service: Option<String>) -> anyhow::Result<()> {
    let start = Instant::now();
    let home = home()?;

    let result = backup_inner(&home, check_percentage, target_service);
    let elapsed_minutes = start.elapsed().as_secs_f64() / 60.0;

    let (title, message, priority) = match &result {
        Ok(stats) => {
            if stats.bad == 0 {
                (
                    format!("Backup successful ({} files checked)", stats.good),
                    format!("It took {:.1} minutes", elapsed_minutes),
                    Priority::Low,
                )
            } else {
                (
                    format!("Backup executed, but {} files failed check", stats.bad),
                    format!("It took {:.1} minutes", elapsed_minutes),
                    Priority::Low,
                )
            }
        }
        Err(error) => (
            "Backup failed".to_string(),
            error_messages(error),
            Priority::High,
        ),
    };

    if let Err(notification_error) = send_notification(&home, title, message, priority) {
        tracing::warn!("Failed to send notification: {:?}", notification_error);
    }

    result.map(|_| ())
}

fn backup_inner(
    home: &Path,
    check_percentage: f64,
    target_service: Option<String>,
) -> anyhow::Result<CheckStats> {
    let now = format!("{}\n", Utc::now());
    let protected_dir = home.join("protected");
    tracing::info!("Starting backup at {}", now.trim());
    fs::write(protected_dir.join("last-backup-attempt.txt"), &now)
        .context("failed to write last-backup-attempt.txt")?;

    mount_source(&protected_dir).context("protected disk does not seem to be mounted")?;
    let backup_disk = BackupDisk::mount_one(home)?;

    let mut check_stats = CheckStats::default();

    let services = list_services::list_services()?;
    let mut one_error = Ok(());
    for service in &services {
        let service_bare_name = service
            .name
            .strip_suffix(".service")
            .unwrap_or(&service.name);
        if let Some(target_service) = &target_service
            && target_service != service_bare_name
        {
            continue;
        }

        if let Err(error) = backup_service(
            home,
            backup_disk,
            &mut check_stats,
            check_percentage,
            service,
        ) {
            tracing::error!("Failed to backup service {}: {:?}", service.name, error);
            one_error = Err(error);
        }
    }
    one_error?;

    if target_service.is_none() {
        backup_other_files(
            home,
            backup_disk,
            &mut check_stats,
            check_percentage,
            &services,
        )?;
    }

    tracing::info!("Backup check stats: {:?}", check_stats);

    let last_successful_backup = protected_dir.join("last-successful-backup.txt");
    let backup_dir = backup_disk.backup_dir(home);
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
        .arg(backup_disk.unmount_script(home))
        .run()?;

    Ok(check_stats)
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
