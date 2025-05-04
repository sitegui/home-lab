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
    pub userns: Option<String>,
    pub run_init: bool,
    pub health_start_period: String,
    pub health_cmd: String,
    pub health_interval: String,
    pub health_timeout: String,
    pub health_retries: String,
    pub environment_file: String,
    pub environment: Vec<String>,
    pub volume: Vec<String>,
    pub read_only: bool,
    pub tmpfs: Vec<String>,
    pub add_capability: Vec<String>,
    pub drop_capability: Vec<String>,
    pub exec: Option<String>,
    pub stop_timeout: Option<String>,
    pub shm_size: Option<String>,
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
        if let Some(userns) = &self.userns {
            writeln!(f, "Userns = {}", userns)?;
        }
        writeln!(f, "RunInit = {}", self.run_init)?;
        writeln!(f, "HealthStartPeriod = {}", self.health_start_period)?;
        writeln!(f, "HealthCmd = {}", self.health_cmd)?;
        writeln!(f, "HealthInterval = {}", self.health_interval)?;
        writeln!(f, "HealthTimeout = {}", self.health_timeout)?;
        writeln!(f, "HealthRetries = {}", self.health_retries)?;
        writeln!(f, "EnvironmentFile = {}", self.environment_file)?;
        for environment in &self.environment {
            writeln!(f, "Environment = {}", environment)?;
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
        if let Some(stop_timeout) = &self.stop_timeout {
            writeln!(f, "StopTimeout = {}", stop_timeout)?;
        }
        if let Some(shm_size) = &self.shm_size {
            writeln!(f, "ShmSize = {}", shm_size)?;
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
