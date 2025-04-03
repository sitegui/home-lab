use anyhow::{Context, bail};
use itertools::Itertools;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

pub fn run_command(program: &str, args: &[String], stdin: Option<String>) -> anyhow::Result<()> {
    fn inner(program: &str, args: &[String], stdin: Option<String>) -> anyhow::Result<()> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let write_stdin = match (child.stdin.take(), stdin) {
            (None, None) => None,
            (Some(mut child_stdin), Some(stdin)) => Some(thread::spawn(move || {
                child_stdin.write_all(stdin.as_bytes())
            })),
            _ => bail!("unexpected"),
        };

        let output = child.wait_with_output()?;
        if let Some(write_stdin) = write_stdin {
            write_stdin.join().unwrap()?;
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            bail!(
                "got status {}\n=== stderr ===\n{}\n=== stdout ===\n{}",
                output.status,
                stderr,
                stdout
            );
        }

        Ok(())
    }

    inner(program, args, stdin)
        .with_context(|| format!("failed to run {} {}", program, args.iter().format(" ")))
}
