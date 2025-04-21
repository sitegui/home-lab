mod child;
mod mount;
mod scripts;

use crate::scripts::backup::backup;
use crate::scripts::detect_duplicates::detect_duplicates;
use crate::scripts::unlock::unlock;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
enum Cli {
    /// Unlock the internal disk and start up the other services
    Unlock,
    Backup,
    DetectDuplicates {
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt().with_target(false).init();

    match Cli::parse() {
        Cli::Unlock => unlock()?,
        Cli::Backup => backup()?,
        Cli::DetectDuplicates { path } => detect_duplicates(&path)?,
    }

    tracing::info!("Done");
    Ok(())
}
