use crate::child::Child;
use crate::mount::mount_source;
use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

#[derive(Debug, Copy, Clone)]
pub enum BackupDisk {
    Backup1,
    Backup2,
}

impl BackupDisk {
    pub fn mount_one(home: &Path) -> anyhow::Result<Self> {
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

    pub fn source_dirs(self, home: &Path) -> Vec<PathBuf> {
        vec![home.join("bare"), home.join("protected")]
    }

    pub fn exclude_dirs(self, home: &Path) -> Vec<PathBuf> {
        match self {
            BackupDisk::Backup1 => vec![home.join(
                "protected/nextcloud/volumes/nextcloud_aio_nextcloud_data/sitegui/files/Jellyfin",
            )],
            BackupDisk::Backup2 => vec![],
        }
    }

    pub fn backup_dir(self, home: &Path) -> PathBuf {
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

    pub fn unmount_script(self, home: &Path) -> PathBuf {
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
}
