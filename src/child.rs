use anyhow::{Context, bail};
use itertools::Itertools;
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct Child<'a> {
    program: &'a str,
    args: Vec<&'a OsStr>,
    stdin: Option<String>,
    capture_stdout: bool,
    capture_stderr: bool,
    ignore_status: bool,
}

#[derive(Debug)]
pub struct ChildOutput {
    status: ExitStatus,
    stdout: Option<Vec<u8>>,
    stderr: Option<Vec<u8>>,
}

impl<'a> Child<'a> {
    pub fn new(program: &'a str, args: &'a [impl AsRef<OsStr>]) -> Self {
        Child {
            program,
            args: args.iter().map(|a| a.as_ref()).collect(),
            stdin: None,
            capture_stdout: false,
            capture_stderr: false,
            ignore_status: false,
        }
    }

    pub fn stdin(mut self, stdin: String) -> Self {
        self.stdin = Some(stdin);
        self
    }

    pub fn capture_stdout(mut self) -> Self {
        self.capture_stdout = true;
        self
    }

    pub fn capture_stderr(mut self) -> Self {
        self.capture_stderr = true;
        self
    }

    pub fn ignore_status(mut self) -> Self {
        self.ignore_status = true;
        self
    }

    pub fn run(self) -> anyhow::Result<ChildOutput> {
        tracing::debug!(
            "Run `{} {}`",
            self.program,
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy())
                .format(" ")
        );

        let mut child = Command::new(self.program)
            .args(self.args)
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let write_stdin = match (child.stdin.take(), self.stdin) {
            (None, None) => None,
            (Some(mut child_stdin), Some(stdin)) => Some(thread::spawn(move || {
                child_stdin.write_all(stdin.as_bytes())
            })),
            _ => bail!("unexpected"),
        };

        let stdout = child.stdout.take().context("missing stdout pipe")?;
        let stdout_thread = if self.capture_stdout {
            capture(stdout)
        } else {
            reprint("out".to_string(), stdout)
        };

        let stderr = child.stderr.take().context("missing stderr pipe")?;
        let stderr_thread = if self.capture_stderr {
            capture(stderr)
        } else {
            reprint("err".to_string(), stderr)
        };

        let status = child.wait()?;
        if let Some(write_stdin) = write_stdin {
            write_stdin.join().unwrap()?;
        }
        let stdout = stdout_thread.join().unwrap()?;
        let stderr = stderr_thread.join().unwrap()?;

        if !self.ignore_status && !status.success() {
            bail!("child process returned {}", status);
        }

        let command_output = ChildOutput {
            status,
            stderr,
            stdout,
        };

        Ok(command_output)
    }
}

impl ChildOutput {
    pub fn status(&self) -> ExitStatus {
        self.status
    }

    pub fn stdout(&self) -> anyhow::Result<String> {
        Ok(String::from_utf8(
            self.stdout.clone().context("missing stdout")?,
        )?)
    }

    pub fn stdout_bytes(&self) -> anyhow::Result<&[u8]> {
        self.stdout.as_deref().context("did not capture stdout")
    }

    pub fn stderr(&self) -> anyhow::Result<String> {
        Ok(String::from_utf8(
            self.stderr.clone().context("missing stderr")?,
        )?)
    }

    pub fn stderr_bytes(&self) -> anyhow::Result<&[u8]> {
        self.stderr.as_deref().context("did not capture stderr")
    }
}

fn capture(mut reader: impl Read + Send + 'static) -> JoinHandle<anyhow::Result<Option<Vec<u8>>>> {
    thread::spawn(move || {
        let mut output = vec![];
        reader.read_to_end(&mut output)?;
        Ok(Some(output))
    })
}

fn reprint(
    prefix: String,
    reader: impl Read + Send + 'static,
) -> JoinHandle<anyhow::Result<Option<Vec<u8>>>> {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let line = line?;
            tracing::info!("{}> {}", prefix, line);
        }
        Ok(None)
    })
}
