use anyhow::{Context, bail};
use itertools::Itertools;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

#[derive(Debug)]
pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_command(
    program: &str,
    args: &[&str],
    stdin: Option<String>,
) -> anyhow::Result<CommandOutput> {
    fn inner(program: &str, args: &[&str], stdin: Option<String>) -> anyhow::Result<CommandOutput> {
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

        let command_output = CommandOutput {
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        };

        if !command_output.status.success() {
            bail!("{}", command_output);
        }

        Ok(command_output)
    }

    inner(program, args, stdin)
        .with_context(|| format!("failed to run `{} {}`", program, args.iter().format(" ")))
}

impl Display for CommandOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.status.success() {
            writeln!(f, "failed {}", self.status)?;
        }

        if !self.stderr.is_empty() {
            writeln!(f, "=== stderr ===")?;
            for line in self.stderr.lines() {
                writeln!(f, "> {}", line)?;
            }
        }

        if !self.stdout.is_empty() {
            writeln!(f, "=== stdout ===")?;
            for line in self.stdout.lines() {
                writeln!(f, "> {}", line)?;
            }
        }

        Ok(())
    }
}
