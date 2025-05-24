use crate::child::Child;
use crate::home::home;
use clap::ValueEnum;

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum UpdateKind {
    /// Update podman images, pulling them. This may restart podman containers
    PodmanImages,
    /// Update packages with apt-get. This may reboot the system
    SystemPackages,
    /// Update only security packages with apt-get. This may reboot the system
    SystemPackagesForSecurity,
}

pub fn update(kind: UpdateKind) -> anyhow::Result<()> {
    let home = home()?;

    match kind {
        UpdateKind::PodmanImages => {
            todo!()
        }
        UpdateKind::SystemPackages => {
            tracing::info!("You can check the logs with `sudo journalctl -u apt-daily -r`");
            Child::new("sudo")
                .arg(home.join("sudo-scripts/update-system-packages.sh"))
                .run()?;
        }
        UpdateKind::SystemPackagesForSecurity => {
            tracing::info!("You can check the logs with `sudo journalctl -u apt-daily -r`");
            Child::new("sudo")
                .arg(home.join("sudo-scripts/update-system-packages-for-security.sh"))
                .run()?;
        }
    }

    Ok(())
}
