use crate::scripts::backup::backup_disk::BackupDisk;
use anyhow::{Context, ensure};
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use rand::rng;
use sha1::digest::Output;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Debug, Default)]
pub struct CheckStats {
    pub good: i32,
    pub bad: i32,
}

pub fn check_files(
    home: &Path,
    check_percentage: f64,
    check_stats: &mut CheckStats,
    disk: BackupDisk,
    volumes: &[PathBuf],
    excludes: &[PathBuf],
) -> anyhow::Result<()> {
    let backup_dir = disk.backup_dir(home);
    let files = list_files(volumes, excludes)?;

    let sample_size = (files.len() as f64 * check_percentage / 100.0).ceil() as usize;
    let selected_files = files.choose_multiple(&mut rng(), sample_size).collect_vec();
    tracing::info!("Will check the contents of {} files", selected_files.len());

    for original in selected_files {
        let backup = backup_dir.join(original.strip_prefix(home)?);

        if let Err(error) = check_file(original, &backup) {
            tracing::error!("File check failed for '{}': {}", original.display(), error);
            check_stats.bad += 1;
        } else {
            check_stats.good += 1;
        }
    }

    Ok(())
}

fn list_files(volumes: &[PathBuf], excludes: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = vec![];
    for volume in volumes {
        let metadata = volume.metadata()?;
        if metadata.is_dir() {
            collect_files(excludes, volume, &mut files)?;
        } else if metadata.is_file() {
            files.push(volume.clone());
        }
    }

    Ok(files)
}

fn collect_files(
    excludes: &[PathBuf],
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() && !excludes.contains(&path) {
            collect_files(excludes, &path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

fn check_file(original_path: &Path, backup_path: &Path) -> anyhow::Result<()> {
    let original_hash = hash_file(original_path).context("failed to hash original")?;
    let backup_hash = hash_file(backup_path).context("failed to hash backup")?;
    ensure!(original_hash == backup_hash, "contents are different");

    Ok(())
}

fn hash_file(file_path: &Path) -> anyhow::Result<Output<Sha1>> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha1::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize())
}
