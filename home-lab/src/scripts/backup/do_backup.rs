use crate::child::Child;
use crate::scripts::backup::backup_disk::BackupDisk;
use crate::scripts::backup::check_files::{CheckStats, check_files};
use crate::scripts::backup::list_services::Service;
use crate::scripts::backup::start_service_on_drop::stop_service;
use anyhow::{Context, bail};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

pub fn backup_service(
    home: &Path,
    backup_disk: BackupDisk,
    check_stats: &mut CheckStats,
    check_percentage: f64,
    service: &Service,
) -> anyhow::Result<()> {
    tracing::info!("Backing up service: {}", service.name);

    if service.writable_binds.is_empty() {
        tracing::info!("Nothing to do");
        return Ok(());
    }

    let mut stopped_service =
        stop_service(service.name.clone()).context("failed to stop service")?;
    let sources = backup_disk.source_dirs(home);
    let excludes = backup_disk.exclude_dirs(home);
    let backup_dir = backup_disk.backup_dir(home);

    for bind in &service.writable_binds {
        if sources.iter().all(|source| !bind.starts_with(source)) {
            tracing::warn!(
                "Ignoring {} because it's not nested inside a source directory",
                bind.display()
            );
            continue;
        }

        let bind_strip_home = bind
            .strip_prefix(home)
            .context("bind volume is not nested under home")?;
        let destination = backup_dir.join(bind_strip_home);

        let mut child = Child::new("rsync");

        let bind_metadata = bind
            .metadata()
            .context("failed to get bind volume metadata")?;
        if bind_metadata.is_dir() {
            // rsync $BIND/ $BACKUP/$BIND_STRIP_HOME --exclude /$EXCLUDE_STRIP_BIND/
            fs::create_dir_all(&destination).context("failed to create destination directory")?;
            let mut bind_arg = bind.as_os_str().to_owned();
            bind_arg.push("/");

            child = child.arg(bind_arg).arg(destination).arg("--delete");

            for exclude in &excludes {
                if let Ok(exclude_strip_bind) = exclude.strip_prefix(bind) {
                    let mut exclude_arg = OsString::from("/");
                    exclude_arg.push(exclude_strip_bind);
                    exclude_arg.push("/");
                    child = child.arg("--exclude").arg(exclude_arg);
                }
            }
        } else if bind_metadata.is_file() {
            // rsync $BIND $BACKUP/$BIND_STRIP_HOME
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).context("failed to create destination directory")?;
            }
            child = child.arg(bind).arg(destination);
        } else {
            bail!("bind volume is neither a directory nor a file")
        }

        child.arg("--archive").arg("--verbose").run()?;
    }

    check_files(
        home,
        check_percentage,
        check_stats,
        backup_disk,
        &service.writable_binds,
        &excludes,
    )
    .context("failed to check files")?;

    stopped_service.start().context("failed to start service")?;

    Ok(())
}

pub fn backup_other_files(
    home: &Path,
    backup_disk: BackupDisk,
    check_stats: &mut CheckStats,
    check_percentage: f64,
    services: &[Service],
) -> anyhow::Result<()> {
    tracing::info!("Backing up other files");

    let sources = backup_disk.source_dirs(home);
    let backup_dir = backup_disk.backup_dir(home);

    let mut excludes = backup_disk.exclude_dirs(home);
    for service in services {
        for bind in &service.writable_binds {
            excludes.push(bind.clone());
        }
    }

    // rsync $SOURCE_N $BACKUP --exclude /$SOURCE_LAST_NAME/$EXCLUDE_STRIP_SOURCE/
    let mut child = Child::new("rsync")
        .args(&sources)
        .arg(&backup_dir)
        .arg("--archive")
        .arg("--verbose")
        .arg("--delete");

    for source in &sources {
        for exclude in &excludes {
            if let Ok(exclude_strip_source) = exclude.strip_prefix(source) {
                let mut exclude_arg = OsString::from("/");
                exclude_arg.push(
                    source
                        .file_name()
                        .context("failed to get file_name from source")?,
                );
                exclude_arg.push("/");
                exclude_arg.push(exclude_strip_source);
                exclude_arg.push("/");
                child = child.arg("--exclude").arg(exclude_arg);
            }
        }
    }

    child.run()?;

    check_files(
        home,
        check_percentage,
        check_stats,
        backup_disk,
        &sources,
        &excludes,
    )
    .context("failed to check files")?;

    Ok(())
}
