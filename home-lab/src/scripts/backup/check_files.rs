use crate::scripts::backup::backup_disk::BackupDisk;
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use rand::rng;
use sha1::digest::Output;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct CheckStats {
    pub good: usize,
    pub bad: usize,
}

pub fn check_files(
    home: &Path,
    check_percentage: f64,
    disk: BackupDisk,
) -> anyhow::Result<CheckStats> {
    let backup_dir = disk.backup_dir(home);
    let files = disk.list_files(home)?;

    let sample_size = (files.len() as f64 * check_percentage / 100.0).round() as usize;
    let selected_files = files.choose_multiple(&mut rng(), sample_size).collect_vec();
    tracing::info!("Will check the contents of {} files", selected_files.len());

    let mut good = 0;
    let mut bad = 0;
    for original in selected_files {
        let backup = backup_dir.join(original.strip_prefix(home)?);

        let original_hash = hash_file(original)?;
        let backup_hash = hash_file(&backup)?;
        if original_hash != backup_hash {
            tracing::warn!(
                "File {} does not have the same contents in the backup disk",
                original.display()
            );
            bad += 1;
        } else {
            good += 1;
        }
    }

    Ok(CheckStats { good, bad })
}

fn hash_file(file_path: &Path) -> anyhow::Result<Output<Sha1>> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha1::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize())
}
