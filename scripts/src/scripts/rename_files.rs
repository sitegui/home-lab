use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn rename_files(path: &Path, input: &Path) -> anyhow::Result<()> {
    let contents = fs::read_to_string(input)?;
    let names: BTreeMap<String, String> = serde_json::from_str(&contents)?;

    for (old_name, new_name) in names {
        if new_name != old_name {
            tracing::info!("Renaming {} to {}", old_name, new_name);
            fs::rename(path.join(old_name), path.join(new_name))?;
        }
    }

    Ok(())
}
