mod command;
mod scripts;

use crate::scripts::unlock::unlock;
use clap::Parser;

#[derive(Parser)]
enum Cli {
    /// Unlock the internal disk and start up the other services
    Unlock,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    match Cli::parse() {
        Cli::Unlock => unlock(),
    }
}
