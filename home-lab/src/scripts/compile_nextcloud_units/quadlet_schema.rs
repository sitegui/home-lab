use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Quadlet {
    pub unit: Unit,
    pub service: Service,
    pub container: Container,
    pub install: Install,
}

#[derive(Debug)]
pub struct Unit {
    pub after: Vec<String>,
}

#[derive(Debug)]
pub struct Service {
    pub restart: String,
    pub restart_sec: String,
}

#[derive(Debug)]
pub struct Container {
    pub container_name: String,
    pub image: String,
    pub pull: String,
    pub auto_update: String,
    pub user_ns: String,
    pub run_init: bool,
    pub health_start_period: String,
    pub health_cmd: String,
    pub health_interval: String,
    pub health_timeout: String,
    pub health_retries: String,
    pub environment_file: Option<String>,
    pub environment: BTreeMap<String, String>,
    pub volume: Vec<String>,
    pub read_only: bool,
    pub tmpfs: Vec<String>,
    pub add_capability: Vec<String>,
    pub drop_capability: Vec<String>,
    pub exec: Option<String>,
    pub stop_timeout_s: Option<i32>,
    pub shm_size: Option<String>,
    pub network: Vec<String>,
    pub add_host: Option<String>,
}

#[derive(Debug)]
pub struct Install {
    pub wanted_by: String,
}

impl Display for Quadlet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.unit)?;
        writeln!(f, "{}", self.service)?;
        writeln!(f, "{}", self.container)?;
        writeln!(f, "{}", self.install)?;

        Ok(())
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[Unit]")?;

        for after in &self.after {
            writeln!(f, "After = {}", after)?;
        }

        Ok(())
    }
}

impl Display for Service {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[Service]")?;
        writeln!(f, "Restart = {}", self.restart)?;
        writeln!(f, "RestartSec = {}", self.restart_sec)?;

        Ok(())
    }
}

impl Display for Container {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[Container]")?;
        writeln!(f, "ContainerName = {}", self.container_name)?;
        writeln!(f, "Image = {}", self.image)?;
        writeln!(f, "Pull = {}", self.pull)?;
        writeln!(f, "AutoUpdate = {}", self.auto_update)?;
        writeln!(f, "UserNS = {}", self.user_ns)?;
        writeln!(f, "RunInit = {}", self.run_init)?;
        writeln!(f, "HealthStartPeriod = {}", self.health_start_period)?;
        writeln!(f, "HealthCmd = {}", self.health_cmd)?;
        writeln!(f, "HealthInterval = {}", self.health_interval)?;
        writeln!(f, "HealthTimeout = {}", self.health_timeout)?;
        writeln!(f, "HealthRetries = {}", self.health_retries)?;
        if let Some(environment_file) = &self.environment_file {
            writeln!(f, "EnvironmentFile = {}", environment_file)?;
        }
        for (key, value) in &self.environment {
            writeln!(
                f,
                "Environment = {}",
                systemd_quote(format!("{}={}", key, value))
            )?;
        }
        for volume in &self.volume {
            writeln!(f, "Volume = {}", volume)?;
        }
        writeln!(f, "ReadOnly = {}", self.read_only)?;
        for tmpfs in &self.tmpfs {
            writeln!(f, "Tmpfs = {}", tmpfs)?;
        }
        for add_capability in &self.add_capability {
            writeln!(f, "AddCapability = {}", add_capability)?;
        }
        for drop_capability in &self.drop_capability {
            writeln!(f, "DropCapability = {}", drop_capability)?;
        }
        if let Some(exec) = &self.exec {
            writeln!(f, "Exec = {}", exec)?;
        }
        if let Some(stop_timeout_s) = &self.stop_timeout_s {
            // Note: this is a hack around the fact that the version I'm using does not support
            // `StopTimeout` yet. See https://github.com/containers/podman/issues/21134
            writeln!(f, "PodmanArgs = --stop-timeout {}", stop_timeout_s)?;
        }
        if let Some(shm_size) = &self.shm_size {
            writeln!(f, "ShmSize = {}", shm_size)?;
        }
        for network in &self.network {
            writeln!(f, "Network = {}", network)?;
        }
        if let Some(add_host) = &self.add_host {
            // Note: this is a hack around the fact that the version I'm using does not support
            // `AddHost` yet.
            writeln!(f, "PodmanArgs = --add-host {}", add_host)?;
        }

        Ok(())
    }
}

impl Display for Install {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[Install]")?;
        writeln!(f, "WantedBy = {}", self.wanted_by)?;

        Ok(())
    }
}

fn systemd_quote(s: String) -> String {
    let mut quoted = "\"".to_string();
    for c in s.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}
