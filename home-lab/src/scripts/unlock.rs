use crate::child::Child;
use crate::home::home;
use crate::mount::mount_source;
use anyhow::ensure;
use std::io::{Write, stdin, stdout};

pub fn unlock() -> anyhow::Result<()> {
    let home = home()?;
    let protected_dir = home.join("protected");

    if mount_source(&protected_dir).is_ok() {
        tracing::info!("Protected disk is already mounted: nothing to do");
        return Ok(());
    }

    print!("Please enter password: ");
    stdout().flush()?;

    let mut line = String::new();
    stdin().read_line(&mut line)?;

    let password = line.trim();
    ensure!(!password.is_empty());

    tracing::info!("Unlocking...");
    Child::new("sudo")
        .arg(home.join("sudo-scripts/mount-protected.sh"))
        .stdin(password.to_string())
        .run()?;

    tracing::info!("Starting protected services");
    Child::new("systemctl")
        .args(["--user", "start", "protected.target"])
        .run()?;

    Ok(())
}
