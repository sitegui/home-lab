use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

pub fn detect_duplicates(path: &Path) -> anyhow::Result<()> {
    let mut files = Vec::new();
    find_files(path, &mut files)?;

    let total_files = files.len();
    tracing::info!("Found {} files", total_files);

    let mut paths_by_hash = BTreeMap::new();
    for (index, file_path) in files.into_iter().enumerate() {
        tracing::info!("Processing file {}/{}", index + 1, total_files);

        let mut file = File::open(&file_path)?;
        let mut hasher = Sha1::new();
        io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        paths_by_hash
            .entry(hash)
            .or_insert_with(Vec::new)
            .push(file_path);
    }

    for paths in paths_by_hash.into_values() {
        if paths.len() > 1 {
            println!("Got duplicates:");
            for path in paths {
                println!("- {}", path.display());
            }
        }
    }

    Ok(())
}

fn find_files(path: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            find_files(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }

    Ok(())
}
