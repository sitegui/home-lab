use std::fs;
use std::path::{Path, PathBuf};

pub fn list_files(path: impl AsRef<Path>) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = vec![];
    collect_files(path.as_ref(), &mut files)?;
    Ok(files)
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            collect_files(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }

    Ok(())
}
