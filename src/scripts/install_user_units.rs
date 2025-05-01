use crate::child::Child;
use crate::list_files::list_files;
use anyhow::Context;
use itertools::Itertools;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub fn install_user_units(force: bool) -> anyhow::Result<()> {
    let mut files = vec![];
    list_files(Path::new("config/caddy"), &mut files)?;

    let home = env::var_os("HOME").context("missing HOME env var")?;
    let containers_dir = PathBuf::from(&home).join(".config/containers/systemd");
    let user_dir = PathBuf::from(home).join(".config/systemd/user");
    fs::create_dir_all(&containers_dir)?;
    fs::create_dir_all(&user_dir)?;

    let units: Vec<_> = files
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
        .map(|file| UnitFile::new(&containers_dir, &user_dir, file))
        .try_collect()?;

    tracing::info!("Detected {} units", units.len());

    let mut updated_units = vec![];
    for unit in units {
        let should_update = force
            || match fs::read_to_string(&unit.target_path) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
                Err(err) => return Err(err.into()),
                Ok(contents) => contents != unit.contents,
            };

        if should_update {
            tracing::info!("Copying {}", unit.target_path.display());
            fs::write(&unit.target_path, &unit.contents)?;

            updated_units.push(unit);
        }
    }

    if !updated_units.is_empty() {
        Child::new("systemctl", &["--user", "daemon-reload"]).run()?;

        for unit in updated_units {
            tracing::info!("Restarting {}", unit.name);

            Child::new("systemctl", &["--user", "enable", &unit.name]).run()?;
            Child::new("systemctl", &["--user", "restart", &unit.name]).run()?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct UnitFile {
    name: String,
    target_path: PathBuf,
    contents: String,
}

impl UnitFile {
    fn new(containers_dir: &Path, user_dir: &Path, source_path: PathBuf) -> anyhow::Result<Self> {
        let extension = source_path.extension().context("missing extension")?;
        let name = source_path
            .file_name()
            .context("missing file name")?
            .to_str()
            .context("invalid file name")?;
        let target_path = if extension == "network" || extension == "container" {
            containers_dir.join(name)
        } else {
            user_dir.join(name)
        };

        let contents = fs::read_to_string(&source_path)?;

        Ok(Self {
            name: name.to_string(),
            target_path,
            contents,
        })
    }
}
