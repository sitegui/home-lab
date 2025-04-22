use anyhow::Context;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn prepare_rename_files(path: &Path, output: &Path) -> anyhow::Result<()> {
    let mut names = BTreeMap::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let name = entry
                .file_name()
                .into_string()
                .ok()
                .context("Invalid file name")?;

            names.insert(name.clone(), name);
        }
    }

    let contents = serde_json::to_string_pretty(&names)?;
    fs::write(output, contents)?;

    Ok(())
}
