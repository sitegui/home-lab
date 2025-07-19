use crate::child::Child;
use crate::home::home;
use crate::list_files::list_files;
use anyhow::Context;
use itertools::Itertools;
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub fn install_units(force: bool, path: Option<PathBuf>) -> anyhow::Result<()> {
    let target_path = path.as_deref().unwrap_or(Path::new("config"));
    let files = if target_path.metadata()?.is_file() {
        vec![target_path.to_owned()]
    } else {
        list_files(target_path)?
    };

    let home = home()?;
    let containers_dir = home.join(".config/containers/systemd");
    let user_dir = home.join(".config/systemd/user");
    fs::create_dir_all(&containers_dir)?;
    fs::create_dir_all(&user_dir)?;

    let mut units = load_units(files, &containers_dir, &user_dir)?;
    tracing::info!("Detected {} units", units.len());

    pull_missing_images(&units)?;
    auto_declare_networks(&containers_dir, &user_dir, &mut units)?;

    let updated_units = copy_changed_units(force, &units)?;

    if !updated_units.is_empty() {
        enable_and_restart_services(&updated_units)?;
    }

    if path.is_none() {
        let desired_units: BTreeSet<_> = units.iter().map(|unit| &unit.target_path).collect();
        let present_units = list_files(&containers_dir)?
            .into_iter()
            .chain(list_files(&user_dir)?);

        for unit in present_units {
            if !desired_units.contains(&unit) {
                tracing::warn!("Extra file in target dirs detected: {}", unit.display());
            }
        }
    }

    Ok(())
}

fn load_units(
    files: Vec<PathBuf>,
    containers_dir: &Path,
    user_dir: &Path,
) -> anyhow::Result<Vec<UnitFile>> {
    let mut units = vec![];
    for file in &files {
        let kind = match file.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "target" => UnitKind::Target,
            "service" => UnitKind::Service,
            "network" => UnitKind::Network,
            "container" => UnitKind::Container,
            "socket" => UnitKind::Socket,
            "timer" => UnitKind::Timer,
            _ => continue,
        };

        let name = file
            .file_name()
            .context("missing file name")?
            .to_str()
            .context("invalid file name")?
            .to_owned();

        let contents = fs::read_to_string(file)?;

        units.push(UnitFile::new(
            containers_dir,
            user_dir,
            kind,
            name,
            contents,
        )?);
    }

    Ok(units)
}

fn pull_missing_images(units: &[UnitFile]) -> anyhow::Result<()> {
    let mut images = BTreeSet::new();
    for unit in units {
        if unit.kind == UnitKind::Container {
            static IMAGE_REGEX: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^Image *= *(.*)$").unwrap());

            for line in unit.contents.lines() {
                if let Some(capture) = IMAGE_REGEX.captures(line) {
                    images.insert(capture.get(1).unwrap().as_str());
                }
            }
        }
    }

    tracing::info!("Detected {} used images", images.len());
    let mut missing_images = vec![];
    for image in images {
        let exists = Child::new("podman")
            .args(["image", "exists", image])
            .ignore_status()
            .run()?
            .status()
            .success();

        if !exists {
            missing_images.push(image);
        }
    }

    if !missing_images.is_empty() {
        tracing::info!("Will pull {} missing images", missing_images.len());
        for image in missing_images {
            tracing::info!("Pulling {}", image);
            Child::new("podman").args(["pull", image]).run()?;
        }
    }

    Ok(())
}

/// Detect referenced networks and declare the missing ones automatically
fn auto_declare_networks(
    containers_dir: &Path,
    user_dir: &Path,
    units: &mut Vec<UnitFile>,
) -> anyhow::Result<()> {
    let mut mentioned_networks = BTreeSet::new();
    let mut existing_networks = BTreeSet::new();
    for unit in units.iter() {
        if unit.kind == UnitKind::Container {
            static NETWORK_REGEX: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^Network *= *(.*)\.network$").unwrap());

            for line in unit.contents.lines() {
                if let Some(capture) = NETWORK_REGEX.captures(line) {
                    mentioned_networks.insert(capture.get(1).unwrap().as_str());
                }
            }
        } else if unit.kind == UnitKind::Network {
            existing_networks.insert(unit.name.as_str());
        }
    }

    let missing_networks = mentioned_networks
        .difference(&existing_networks)
        .map(|name| name.to_string())
        .collect_vec();
    tracing::info!("Auto-declared {} networks", missing_networks.len());

    for name in missing_networks {
        units.push(UnitFile::new(
            containers_dir,
            user_dir,
            UnitKind::Network,
            name,
            "[Network]\n".to_owned(),
        )?);
    }

    Ok(())
}

fn copy_changed_units(force: bool, units: &[UnitFile]) -> anyhow::Result<Vec<&UnitFile>> {
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

    Ok(updated_units)
}

fn enable_and_restart_services(updated_units: &[&UnitFile]) -> anyhow::Result<()> {
    Child::new("systemctl")
        .args(["--user", "daemon-reload"])
        .run()?;

    for unit in updated_units {
        if let Some(enable_name) = &unit.enable_name {
            tracing::info!("Enabling {}", enable_name);
            Child::new("systemctl")
                .args(["--user", "enable", enable_name])
                .run()?;
        }
        if let Some(restart_name) = &unit.restart_name {
            tracing::info!("Restarting {}", restart_name);
            Child::new("systemctl")
                .args(["--user", "restart", restart_name])
                .ignore_status()
                .run()?;
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
    kind: UnitKind,
    name: String,
}

#[derive(Debug, Copy, Clone, PartialEq)]
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
        kind: UnitKind,
        name: String,
        contents: String,
    ) -> anyhow::Result<Self> {
        let target_path = match kind {
            UnitKind::Network | UnitKind::Container => containers_dir.join(&name),
            _ => user_dir.join(&name),
        };

        let enable_name = match kind {
            UnitKind::Service | UnitKind::Socket | UnitKind::Timer => Some(name.clone()),
            _ => None,
        };

        let restart_name = match kind {
            UnitKind::Service | UnitKind::Timer => Some(name.clone()),
            UnitKind::Container => {
                let base_name = name
                    .strip_suffix(".container")
                    .context("invalid container name")?;
                Some(format!("{}.service", base_name))
            }
            UnitKind::Network => {
                let base_name = name
                    .strip_suffix(".network")
                    .context("invalid network name")?;
                Some(format!("{}-network.service", base_name))
            }
            _ => None,
        };

        Ok(Self {
            name,
            enable_name,
            restart_name,
            target_path,
            contents,
            kind,
        })
    }
}
