mod child;
mod list_files;
mod scripts;

use crate::scripts::copy_deleted_films::copy_deleted_films;
use crate::scripts::detect_duplicates::detect_duplicates;
use crate::scripts::detect_films::detect_films;
use crate::scripts::generate_totp_secret::generate_totp_secret;
use crate::scripts::hash_files::hash_files;
use crate::scripts::match_deleted_films::match_deleted_films;
use crate::scripts::merge_contacts::merge_contacts;
use crate::scripts::monitor_host::monitor_host;
use crate::scripts::move_films::move_films;
use crate::scripts::patch_takeout_exif::patch_takeout_exif;
use crate::scripts::prepare_rename_files::prepare_rename_files;
use crate::scripts::rename_files::rename_files;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
enum Cli {
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
    /// to use rsync's verbose output to find where the files were in the original disks.
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
    /// Copy over the files from the disks.
    CopyDeletedFilms {
        #[clap(long)]
        matches: PathBuf,
        #[clap(long)]
        prefix: String,
    },
    /// Merge and deduplicate contacts from VCF files
    MergeContacts {
        #[clap(long)]
        output: PathBuf,
        inputs: Vec<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt().with_target(false).init();

    match Cli::parse() {
        Cli::HashFiles { path, output } => hash_files(&path, &output)?,
        Cli::DetectDuplicates { path } => detect_duplicates(&path)?,
        Cli::PrepareRenameFiles { path, output } => prepare_rename_files(&path, &output)?,
        Cli::RenameFiles { path, input } => rename_files(&path, &input)?,
        Cli::DetectFilms { path, output } => detect_films(&path, &output)?,
        Cli::MoveFilms { path } => move_films(&path)?,
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
        Cli::CopyDeletedFilms { matches, prefix } => copy_deleted_films(matches, prefix)?,
        Cli::MergeContacts { inputs, output } => merge_contacts(inputs, output)?,
    }

    tracing::info!("Done");
    Ok(())
}
