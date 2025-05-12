use crate::child::Child;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub fn monitor_host(host: String, output: PathBuf, interval_seconds: u64) -> anyhow::Result<()> {
    let interval = Duration::from_secs(interval_seconds);
    let mut log_file = File::create(output)?;

    loop {
        let timestamp = Utc::now();
        let output = Child::new("ping")
            .args(["-c", "1", host.as_str()])
            .capture_stdout()
            .capture_stderr()
            .ignore_status()
            .run()?;

        let log_line = LogLine {
            timestamp,
            status: output.status().code(),
            stdout: output.stdout().unwrap_or_default(),
            stderr: output.stderr().unwrap_or_default(),
        };
        println!("{:#?}", log_line);

        writeln!(log_file, "{}", serde_json::to_string(&log_line)?)?;

        thread::sleep(interval);
    }
}

#[derive(Debug, Serialize)]
struct LogLine {
    timestamp: DateTime<Utc>,
    status: Option<i32>,
    stdout: String,
    stderr: String,
}
