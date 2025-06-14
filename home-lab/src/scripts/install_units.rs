use crate::child::Child;
use crate::home::home;
use crate::list_files::list_files;
use anyhow::Context;
use itertools::Itertools;
use std::fs;
use std::path::{Path, PathBuf};

pub fn install_units(force: bool, path: Option<PathBuf>) -> anyhow::Result<()> {
    // TODO: pull new container images when Pull = never

    let path = path.as_deref().unwrap_or(Path::new("config"));
    let files = if path.metadata()?.is_file() {
        vec![path.to_owned()]
    } else {
        list_files(path)?
    };

    let home = home()?;
    let containers_dir = home.clone().join(".config/containers/systemd");
    let user_dir = home.join(".config/systemd/user");
    fs::create_dir_all(&containers_dir)?;
    fs::create_dir_all(&user_dir)?;

    let units: Vec<_> = files
        .into_iter()
        .filter_map(|file| {
            let kind = match file.extension()?.to_str()? {
                "target" => UnitKind::Target,
                "service" => UnitKind::Service,
                "network" => UnitKind::Network,
                "container" => UnitKind::Container,
                "socket" => UnitKind::Socket,
                "timer" => UnitKind::Timer,
                _ => return None,
            };

            Some(UnitFile::new(&containers_dir, &user_dir, file, kind))
        })
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

#[derive(Debug, Copy, Clone)]
enum UnitKind {
    Target,
    Service,
    Network,
    Container,
    Socket,
    Timer,
}

impl UnitFile {
    fn new(
        containers_dir: &Path,
        user_dir: &Path,
        source_path: PathBuf,
        kind: UnitKind,
    ) -> anyhow::Result<Self> {
        let name = source_path
            .file_name()
            .context("missing file name")?
            .to_str()
            .context("invalid file name")?;
        let target_path = match kind {
            UnitKind::Network | UnitKind::Container => containers_dir.join(name),
            _ => user_dir.join(name),
        };

        let enable_name = match kind {
            UnitKind::Service | UnitKind::Socket | UnitKind::Timer => Some(name.to_string()),
            _ => None,
        };

        let restart_name = match kind {
            UnitKind::Service | UnitKind::Timer => Some(name.to_string()),
            UnitKind::Container => {
                let base_name = name
                    .strip_suffix(".container")
                    .context("invalid container name")?;
                Some(format!("{}.service", base_name))
            }
            UnitKind::Network => {
                let base_name = name
                    .strip_suffix(".container")
                    .context("invalid container name")?;
                Some(format!("{}-network.service", base_name))
            }
            _ => None,
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
