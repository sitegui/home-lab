mod child;
mod list_files;
mod mount;
mod scripts;

use crate::scripts::backup::backup;
use crate::scripts::detect_duplicates::detect_duplicates;
use crate::scripts::detect_films::detect_films;
use crate::scripts::hash_files::hash_files;
use crate::scripts::install_user_units::install_user_units;
use crate::scripts::move_films::move_films;
use crate::scripts::prepare_rename_files::prepare_rename_files;
use crate::scripts::rename_files::rename_files;
use crate::scripts::unlock::unlock;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
enum Cli {
    /// Unlock the internal disk and start up the other services
    Unlock,
    Backup,
    HashFiles {
        path: PathBuf,
        output: PathBuf,
    },
    DetectDuplicates {
        path: PathBuf,
    },
    PrepareRenameFiles {
        path: PathBuf,
        output: PathBuf,
    },
    RenameFiles {
        path: PathBuf,
        input: PathBuf,
    },
    DetectFilms {
        path: PathBuf,
        output: PathBuf,
    },
    MoveFilms {
        path: PathBuf,
    },
    /// Copy all systemd unit files to the user folder, enable them and restart the impacted
    /// services.
    InstallUserUnits {
        /// Force copying and restarting the services even when the contents are the same
        #[clap(long)]
        force: bool,
        /// Look for units to install in this directory
        path: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt().with_target(false).init();

    match Cli::parse() {
        Cli::Unlock => unlock()?,
        Cli::Backup => backup()?,
        Cli::HashFiles { path, output } => hash_files(&path, &output)?,
        Cli::DetectDuplicates { path } => detect_duplicates(&path)?,
        Cli::PrepareRenameFiles { path, output } => prepare_rename_files(&path, &output)?,
        Cli::RenameFiles { path, input } => rename_files(&path, &input)?,
        Cli::DetectFilms { path, output } => detect_films(&path, &output)?,
        Cli::MoveFilms { path } => move_films(&path)?,
        Cli::InstallUserUnits { force, path } => install_user_units(force, path)?,
    }

    tracing::info!("Done");
    Ok(())
}
