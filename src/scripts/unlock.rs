use crate::child::Child;
use anyhow::ensure;
use std::io::{Write, stdin, stdout};

pub fn unlock() -> anyhow::Result<()> {
    print!("Please enter password: ");
    stdout().flush()?;

    let mut line = String::new();
    stdin().read_line(&mut line)?;

    let password = line.trim();
    ensure!(!password.is_empty());

    println!("Unlocking...");
    Child::new("sudo", &["./config/scripts/mount-data"])
        .stdin(password.to_string())
        .run()?;

    Ok(())
}
