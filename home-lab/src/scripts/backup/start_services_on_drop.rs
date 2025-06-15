use crate::child::Child;
use anyhow::Context;
use itertools::Itertools;
use std::fs;
use std::path::Path;

pub struct StartServicesOnDrop(Vec<String>);

impl Drop for StartServicesOnDrop {
    fn drop(&mut self) {
        tracing::info!("Starting services: {}", self.0.iter().format(", "));

        if let Err(error) = Child::new("systemctl")
            .args(["--user", "start"])
            .args(&self.0)
            .run()
        {
            tracing::error!("Failed to start services: {:?}", error);
        }
    }
}

pub fn stop_containers(home: &Path) -> anyhow::Result<StartServicesOnDrop> {
    let mut container_services = vec![];
    for item in fs::read_dir(home.join(".config/containers/systemd"))? {
        let item = item?;
        if item.file_type()?.is_file() {
            let file_name = item
                .file_name()
                .into_string()
                .ok()
                .context("failed to get file name")?;
            let Some(name) = file_name.strip_suffix(".container") else {
                continue;
            };
            container_services.push(name.to_owned());
        }
    }

    tracing::info!(
        "Stopping services: {}",
        container_services.iter().format(", ")
    );
    Child::new("systemctl")
        .args(["--user", "stop"])
        .args(&container_services)
        .run()?;

    Ok(StartServicesOnDrop(container_services))
}
