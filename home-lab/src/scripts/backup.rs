use crate::child::Child;
use crate::home::home;
use crate::mount::mount_source;
use anyhow::{Context, bail, ensure};
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

    let backup_disk = BackupDisk::mount_one(&home)?;
    let backup_dir = backup_disk.backup_dir(&home);
    let stopped_services = stop_containers(&home)?;

    let mut child = Child::new("rsync")
        .arg(home.join("bare"))
        .arg(home.join("protected"))
        .arg(&backup_dir)
        .arg("--archive")
        .arg("--delete")
        .arg("--verbose");

    for exclude in backup_disk.exclude() {
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

    tracing::info!("Will unmount backup");
    Child::new("sudo")
        .arg(backup_disk.unmount_script(&home))
        .run()?;

    Ok(())
}

#[derive(Debug, Copy, Clone)]
enum BackupDisk {
    Backup1,
    Backup2,
}

impl BackupDisk {
    fn mount_one(home: &Path) -> anyhow::Result<Self> {
        match BackupDisk::Backup1.ensure_mounted(home) {
            Ok(()) => Ok(BackupDisk::Backup1),
            Err(error_1) => match BackupDisk::Backup2.ensure_mounted(home) {
                Ok(()) => Ok(BackupDisk::Backup2),
                Err(error_2) => {
                    tracing::warn!("Failed to mount backup-1: {:?}", error_1);
                    tracing::warn!("Failed to mount backup-2: {:?}", error_2);
                    bail!("Failed to mount backup-1 or backup-2");
                }
            },
        }
    }

    fn backup_dir(self, home: &Path) -> PathBuf {
        match self {
            BackupDisk::Backup1 => home.join("backup-1"),
            BackupDisk::Backup2 => home.join("backup-2"),
        }
    }

    fn mount_script(self, home: &Path) -> PathBuf {
        match self {
            BackupDisk::Backup1 => home.join("sudo-scripts/mount-backup-1.sh"),
            BackupDisk::Backup2 => home.join("sudo-scripts/mount-backup-2.sh"),
        }
    }

    fn unmount_script(self, home: &Path) -> PathBuf {
        match self {
            BackupDisk::Backup1 => home.join("sudo-scripts/umount-backup-1.sh"),
            BackupDisk::Backup2 => home.join("sudo-scripts/umount-backup-2.sh"),
        }
    }

    fn ensure_mounted(self, home: &Path) -> anyhow::Result<()> {
        let backup_dir = self.backup_dir(home);
        let backup_mount = match mount_source(&backup_dir) {
            Ok(backup_mount) => backup_mount,
            Err(_) => {
                tracing::info!("Will try to mount {}", backup_dir.display());
                Child::new("sudo").arg(self.mount_script(home)).run()?;
                mount_source(&backup_dir).context("failed to mount backup")?
            }
        };
        tracing::info!("Backing up into {}", backup_mount);

        Ok(())
    }

    fn exclude(self) -> Vec<PathBuf> {
        match self {
            BackupDisk::Backup1 => vec![PathBuf::from(
                "/protected/nextcloud/volumes/nextcloud_aio_nextcloud_data/sitegui/files/Jellyfin",
            )],
            BackupDisk::Backup2 => vec![],
        }
    }
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
