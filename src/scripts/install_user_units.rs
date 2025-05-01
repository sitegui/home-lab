use crate::list_files::list_files;
use itertools::Itertools;
use std::path::Path;

pub fn install_user_units() -> anyhow::Result<()> {
    let mut files = vec![];
    list_files(Path::new("config"), &mut files)?;

    let units = files
        .into_iter()
        .filter(|file| {
            let Some(extension) = file.extension() else {
                return false;
            };

            extension == "target"
                || extension == "service"
                || extension == "network"
                || extension == "container"
                || extension == "socket"
        })
        .collect_vec();

    println!("Detected units: {:?}", units);

    Ok(())
}
