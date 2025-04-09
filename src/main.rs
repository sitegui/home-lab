mod child;
mod mount;
mod scripts;

use crate::scripts::backup::backup;
use crate::scripts::unlock::unlock;
use clap::Parser;

#[derive(Parser)]
enum Cli {
    /// Unlock the internal disk and start up the other services
    Unlock,
    Backup,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    match Cli::parse() {
        Cli::Unlock => unlock()?,
        Cli::Backup => backup()?,
    }

    println!("Done");
    Ok(())
}
