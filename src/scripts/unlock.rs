use crate::command::run_command;
use anyhow::ensure;
use std::io::stdin;

pub fn unlock() -> anyhow::Result<()> {
    print!("Please enter password: ");

    let mut line = String::new();
    stdin().read_line(&mut line)?;

    let password = line.trim();
    ensure!(password.is_empty());

    run_command(
        "cryptmount",
        &["data".to_string()],
        Some(password.to_string()),
    )?;

    Ok(())
}
