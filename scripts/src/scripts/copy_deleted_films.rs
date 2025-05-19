use crate::scripts::match_deleted_films::Match;
use std::path::{Path, PathBuf};
use std::{fs, thread};

pub fn copy_deleted_films(matches: PathBuf, prefix: String) -> anyhow::Result<()> {
    let mut disk_1 = Vec::new();
    let mut disk_2 = Vec::new();
    let mut reader = csv::Reader::from_path(matches)?;
    for result in reader.deserialize() {
        let match_: Match = result?;

        if let Some(source) = match_.source {
            if source.contains("/disk-1/") {
                disk_1.push((source, format!("{}/{}", prefix, match_.target)));
            } else {
                disk_2.push((source, format!("{}/{}", prefix, match_.target)));
            }
        }
    }

    tracing::info!(
        "Will copy {} files from disk 1 and {} from disk 2",
        disk_1.len(),
        disk_2.len()
    );

    let thread_1 = thread::spawn(|| copy_files("disk_1", disk_1));
    let thread_2 = thread::spawn(|| copy_files("disk_2", disk_2));

    thread_1.join().unwrap();
    thread_2.join().unwrap();

    Ok(())
}

fn copy_files(name: &'static str, pairs: Vec<(String, String)>) {
    let len = pairs.len();
    for (index, (from, to)) in pairs.into_iter().enumerate() {
        if index % 10 == 0 {
            tracing::info!("{}: {}/{}", name, index + 1, len);
        }

        if let Err(error) = copy_file(&from, &to) {
            tracing::error!("Failed to copy {} -> {}: {:?}", from, to, error);
        }
    }
}

fn copy_file(from: &str, to: &str) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(to).parent() {
        fs::create_dir_all(parent)?;
    }

    // Skip files already copied
    let from_meta = Some(Path::new(from).metadata()?.len());
    let to_meta = match Path::new(to).metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Ok(meta) => Some(meta.len()),
        Err(error) => return Err(error.into()),
    };
    if from_meta == to_meta {
        return Ok(());
    };

    tracing::info!("Copying {} -> {}", from, to);
    fs::copy(from, to)?;

    Ok(())
}
