use crate::child::Child;
use crate::home::home;
use crate::list_files::list_files;
use anyhow::Context;
use itertools::Itertools;
use std::fs;
use std::path::{Path, PathBuf};

pub fn install_user_units(force: bool, path: Option<PathBuf>) -> anyhow::Result<()> {
    let path = path.as_deref().unwrap_or(Path::new("config"));
    let files = list_files(path)?;

    let home = home()?;
    let containers_dir = home.clone().join(".config/containers/systemd");
    let user_dir = home.join(".config/systemd/user");
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
                || extension == "timer"
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
        Child::new("systemctl")
            .args(["--user", "daemon-reload"])
            .run()?;

        for unit in updated_units {
            if let Some(enable_name) = unit.enable_name {
                tracing::info!("Enabling {}", enable_name);
                Child::new("systemctl")
                    .args(["--user", "enable", &enable_name])
                    .run()?;
            }
            if let Some(restart_name) = unit.restart_name {
                tracing::info!("Restarting {}", restart_name);
                Child::new("systemctl")
                    .args(["--user", "restart", &restart_name])
                    .ignore_status()
                    .run()?;
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct UnitFile {
    target_path: PathBuf,
    enable_name: Option<String>,
    restart_name: Option<String>,
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

        let enable_name = if extension == "service" || extension == "socket" || extension == "timer"
        {
            Some(name.to_string())
        } else {
            None
        };

        let restart_name = if extension == "service" || extension == "socket" {
            Some(name.to_string())
        } else if extension == "container" {
            let base_name = name
                .strip_suffix(".container")
                .context("invalid container name")?;
            Some(format!("{}.service", base_name))
        } else {
            None
        };

        let contents = fs::read_to_string(&source_path)?;

        Ok(Self {
            enable_name,
            restart_name,
            target_path,
            contents,
        })
    }
}
