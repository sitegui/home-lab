use crate::child::Child;
use anyhow::bail;

/// Return the mount source (like "/dev/mapper/backup-1") if the given path is its root mount
pub fn mount_source(path: &str) -> anyhow::Result<String> {
    let output = Child::new(
        "findmnt",
        &["--output", "SOURCE", "--noheadings", "--mountpoint", path],
    )
    .capture_stdout()
    .ignore_status()
    .run()?;

    if output.status().code() == Some(1) {
        bail!("{} is not mounted", path)
    } else if output.status().success() {
        let source = output.stdout()?.trim().to_string();
        if source.is_empty() {
            bail!("{} is not mounted", path)
        } else {
            Ok(source)
        }
    } else {
        bail!("unexpected findmnt status: {}", output.status())
    }
}
