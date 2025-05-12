use anyhow::Context;
use std::fs;
use std::path::Path;

pub fn move_films(path: &Path) -> anyhow::Result<()> {
    let mut files_to_move = vec![];

    for entry in fs::read_dir(path)? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            files_to_move.push(entry.path());
        }
    }

    for path in files_to_move {
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .context("invalid file name")?;
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("invalid file name")?;

        let new_folder = path.parent().context("missing parent")?.join(file_stem);
        let new_path = new_folder.join(file_name);

        println!("Moving {} to {}", path.display(), new_path.display());
        fs::create_dir_all(&new_folder)?;
        fs::rename(path, new_path)?;
    }

    Ok(())
}
