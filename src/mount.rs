use crate::child::Child;
use anyhow::bail;
use std::path::Path;

/// Return the mount source (like "/dev/mapper/backup-1") if the given path is its root mount
pub fn mount_source(path: &Path) -> anyhow::Result<String> {
    let output = Child::new(
        "findmnt",
        &[
            "--output".as_ref(),
            "SOURCE".as_ref(),
            "--noheadings".as_ref(),
            "--mountpoint".as_ref(),
            path.as_os_str(),
        ],
    )
    .capture_stdout()
    .ignore_status()
    .run()?;

    if output.status().code() == Some(1) {
        bail!("{} is not mounted", path.display())
    } else if output.status().success() {
        let source = output.stdout()?.trim().to_string();
        if source.is_empty() {
            bail!("{} is not mounted", path.display())
        } else {
            Ok(source)
        }
    } else {
        bail!("unexpected findmnt status: {}", output.status())
    }
}
