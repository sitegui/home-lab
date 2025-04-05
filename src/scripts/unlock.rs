use crate::command::run_command;
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
    let output = run_command(
        "cryptmount",
        &["data", "--passwd-fd", "0"],
        Some(password.to_string()),
    )?;
    println!("{}", output);

    Ok(())
}
