use crate::child::Child;
use crate::home::home;
use crate::list_files::list_files;
use anyhow::Context;
use itertools::Itertools;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub fn install_sudo_scripts() -> anyhow::Result<()> {
    let home = home()?;
    let scripts = list_files("config/sudo-scripts")?
        .into_iter()
        .filter(|file| file.extension() == Some(OsStr::new("sh")))
        .collect_vec();

    let target_dir = home.join("sudo-scripts");
    fs::create_dir_all(&target_dir)?;
    restrict_to_root(&target_dir)?;

    tracing::info!("Found {} scripts", scripts.len());
    let mut sudoers_contents = String::new();

    for script in scripts {
        let target = target_dir.join(script.file_name().context("invalid script name")?);

        tracing::info!("Prepare {}", target.display());
        writeln!(
            sudoers_contents,
            "sitegui ALL=(ALL) NOPASSWD: {}",
            target.display()
        )?;

        fs::copy(script, &target)?;
        restrict_to_root(&target)?;
    }

    println!("{}", sudoers_contents);
    let sudoers_path = Path::new("/etc/sudoers.d/sitegui");
    // fs::write(sudoers_path, sudoers_contents)?;
    // restrict_to_root(sudoers_path)?;

    Ok(())
}

fn restrict_to_root(path: &Path) -> anyhow::Result<()> {
    Child::new("chown").arg("root:root").arg(path).run()?;
    Child::new("chmod").arg("700").arg(path).run()?;
    Ok(())
}
