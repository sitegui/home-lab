mod child;
mod home;
mod list_files;
mod mount;
mod scripts;

use crate::scripts::backup::backup;
use crate::scripts::compile_nextcloud_units::compile_nextcloud_units;
use crate::scripts::detect_duplicates::detect_duplicates;
use crate::scripts::detect_films::detect_films;
use crate::scripts::generate_totp_secret::generate_totp_secret;
use crate::scripts::hash_files::hash_files;
use crate::scripts::install_sudo_scripts::install_sudo_scripts;
use crate::scripts::install_user_units::install_user_units;
use crate::scripts::match_deleted_films::match_deleted_films;
use crate::scripts::monitor_host::monitor_host;
use crate::scripts::move_films::move_films;
use crate::scripts::patch_takeout_exif::patch_takeout_exif;
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
    /// Copy all sudo scripts to ~/sudo-scripts and edit the sudoers file to enable running them
    InstallSudoScripts,
    /// Copy all systemd unit files to the user folder, enable them and restart the impacted
    /// services.
    InstallUserUnits {
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
    MonitorHost {
        /// The hostname or IP
        host: String,
        /// The path for a .jsonl file with the logs
        output: PathBuf,
        #[clap(long, default_value_t = 10)]
        interval_seconds: u64,
    },
    /// Patch the EXIF information of the photos, using the Google's takeout '*.json' files.
    ///
    /// The files are patched in-place, so please keep a backup
    PatchTakeoutExif {
        /// The folder to recursively patch
        input: PathBuf,
    },
    /// Print a secret (in base 32) for TOTP
    GenerateTotpSecret,
    /// I've removed the movies that were already organized on the server. This command will try
    ///
    /// rsync's verbose output.
    MatchDeletedFilms {
        #[clap(long)]
        rsync_log: PathBuf,
        #[clap(long = "source")]
        sources: Vec<PathBuf>,
        #[clap(long)]
        already_matched: Option<PathBuf>,
        #[clap(long)]
        output: PathBuf,
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
        Cli::InstallSudoScripts => install_sudo_scripts()?,
        Cli::InstallUserUnits { force, path } => install_user_units(force, path)?,
        Cli::CompileNextcloudUnits {
            input_secrets,
            output_secrets_dir,
            volumes_dir,
            profiles,
        } => compile_nextcloud_units(input_secrets, output_secrets_dir, volumes_dir, profiles)?,
        Cli::MonitorHost {
            host,
            output,
            interval_seconds,
        } => monitor_host(host, output, interval_seconds)?,
        Cli::PatchTakeoutExif { input } => patch_takeout_exif(input)?,
        Cli::GenerateTotpSecret => generate_totp_secret()?,
        Cli::MatchDeletedFilms {
            rsync_log,
            sources,
            already_matched,
            output,
        } => match_deleted_films(rsync_log, sources, already_matched, output)?,
    }

    tracing::info!("Done");
    Ok(())
}
