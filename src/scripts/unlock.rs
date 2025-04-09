use crate::child::Child;
use crate::mount::mount_source;
use anyhow::ensure;
use std::io::{Write, stdin, stdout};

pub fn unlock() -> anyhow::Result<()> {
    if mount_source("data").is_ok() {
        tracing::info!("Data is already mounted: nothing to do");
        return Ok(());
    }

    print!("Please enter password: ");
    stdout().flush()?;

    let mut line = String::new();
    stdin().read_line(&mut line)?;

    let password = line.trim();
    ensure!(!password.is_empty());

    tracing::info!("Unlocking...");
    Child::new("sudo", &["./config/scripts/mount-data"])
        .stdin(password.to_string())
        .run()?;

    Ok(())
}
