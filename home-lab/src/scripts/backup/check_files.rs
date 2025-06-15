use crate::scripts::backup::backup_disk::BackupDisk;
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use rand::rng;
use std::path::Path;

pub fn check_files(home: &Path, check_percentage: f64, disk: BackupDisk) -> anyhow::Result<()> {
    let backup_dir = disk.backup_dir(home);
    let files = disk.list_files(home)?;

    let sample_size = (files.len() as f64 * check_percentage / 100.0).round() as usize;
    let selected_files = files.choose_multiple(&mut rng(), sample_size).collect_vec();
    tracing::info!("Will check the contents of {} files", selected_files.len());

    for selected_file in selected_files {
        let source = selected_file;
        let target = backup_dir.join(selected_file.strip_prefix(home)?);

        println!("{} -> {}", source.display(), target.display());
    }

    Ok(())
}
