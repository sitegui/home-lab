use crate::list_files;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::{fs, io};

pub fn hash_files(path: &Path, output: &Path) -> anyhow::Result<()> {
    let mut files = Vec::new();
    list_files::list_files(path, &mut files)?;

    let total_files = files.len();
    tracing::info!("Found {} files", total_files);

    let mut paths_by_hash = BTreeMap::new();
    for (index, file_path) in files.into_iter().enumerate() {
        tracing::info!("Processing file {}/{}", index + 1, total_files);

        match hash_file(&file_path) {
            Err(err) => {
                tracing::error!("Failed to read file {}: {}", file_path.display(), err);
            }
            Ok(hash) => paths_by_hash
                .entry(hash)
                .or_insert_with(Vec::new)
                .push(file_path),
        };
    }

    let contents = serde_json::to_string_pretty(&paths_by_hash)?;
    fs::write(output, contents)?;

    Ok(())
}

fn hash_file(file_path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha1::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}
