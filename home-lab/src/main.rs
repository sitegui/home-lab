mod child;
mod error;
mod home;
mod list_files;
mod mount;
mod notifications;
mod scripts;

use crate::scripts::backup::backup;
use crate::scripts::compile_nextcloud_units::compile_nextcloud_units;
use crate::scripts::install_sudo_scripts::install_sudo_scripts;
use crate::scripts::install_units::install_units;
use crate::scripts::run_unlock_api::run_unlock_api;
use crate::scripts::unlock::unlock;
use crate::scripts::update::{UpdateKind, update};
use clap::Parser;
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
enum Cli {
    /// Unlock the internal disk and start up the other services
    Unlock,
    /// Start a simple HTTP server that can be used to unlock the internal disk
    RunUnlockApi {
        #[clap(long, default_value = "127.0.0.1")]
        bind: String,
        port: u16,
    },
    /// Run the backup, copying local files into one of the backup disks, then check a given
    /// percentage of the files
    Backup {
        #[clap(long, default_value_t = 1.0)]
        check_percentage: f64,
    },
    /// Copy all sudo scripts to ~/sudo-scripts and edit the sudoers file to enable running them
    InstallSudoScripts,
    /// Copy all systemd unit files to the user folder, enable them and restart the impacted
    /// services.
    InstallUnits {
        /// Force copying and restarting the services even when the contents are the same
        #[clap(long)]
        force: bool,
        /// Look for units to install in this directory
        path: Option<PathBuf>,
    },
    /// Convert the official docker compose file into podman systemd unit files
    CompileNextcloudUnits {
        #[clap(long)]
        input_secrets: PathBuf,
        #[clap(long)]
        output_secrets_dir: PathBuf,
        #[clap(long)]
        volumes_dir: PathBuf,
        /// Comma-separated list of profiles to enable
        #[clap(long, value_delimiter = ',')]
        profiles: Vec<String>,
    },
    /// Update the system
    Update { kind: UpdateKind },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .init();

    match Cli::parse() {
        Cli::Unlock => unlock()?,
        Cli::RunUnlockApi { bind, port } => run_unlock_api(bind, port)?,
        Cli::Backup { check_percentage } => backup(check_percentage)?,
        Cli::InstallSudoScripts => install_sudo_scripts()?,
        Cli::InstallUnits { force, path } => install_units(force, path)?,
        Cli::CompileNextcloudUnits {
            input_secrets,
            output_secrets_dir,
            volumes_dir,
            profiles,
        } => compile_nextcloud_units(input_secrets, output_secrets_dir, volumes_dir, profiles)?,
        Cli::Update { kind } => update(kind)?,
    }

    tracing::info!("Done");
    Ok(())
}
