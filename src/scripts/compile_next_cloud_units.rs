mod compose_schema;
mod quadlet_schema;

use crate::scripts::compile_next_cloud_units::compose_schema::{
    Compose, ComposeService, DynamicString, Environment, Volume,
};
use crate::scripts::compile_next_cloud_units::quadlet_schema::{
    Container, Install, Quadlet, Service, Unit,
};
use anyhow::Context;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn compile_next_cloud_units() -> anyhow::Result<()> {
    let nextcloud_dir = Path::new("config/nextcloud");
    let vars: BTreeMap<_, _> =
        dotenvy::from_path_iter(nextcloud_dir.join("vars.conf"))?.try_collect()?;

    let compose_source = fs::read_to_string(nextcloud_dir.join("vendor/latest.yml"))?;
    let compose: Compose = serde_yml::from_str(&compose_source)?;

    eprintln!("compose = {:#?}", compose);

    for (service_name, service) in compose.services {
        let quadlet = compile_service(&vars, &service_name, &service)
            .with_context(|| format!("failed to compile {}", service_name))?;

        let contents = quadlet.to_string();
        fs::write(
            nextcloud_dir.join(format!("{}.container", service_name)),
            contents,
        )?;
    }

    Ok(())
}

fn compile_service(
    vars: &BTreeMap<String, String>,
    service_name: &str,
    service: &ComposeService,
) -> anyhow::Result<Quadlet> {
    let unit = Unit {
        after: service.depends_on.keys().cloned().collect(),
    };

    let userns = vars
        .replace_opt(&service.user)?
        .map(|user| format!("keep-id:uid={}", user));

    let tmpfs = vars.replace_vec(&service.tmpfs)?;
    let add_capability = vars.replace_vec(&service.cap_add)?;
    let drop_capability = vars.replace_vec(&service.cap_drop)?;

    let mut environment = vec![];
    for environment_item in &service.environment {
        let environment_item: Environment = vars.replace(environment_item)?.parse()?;
        environment.push(format!(
            "{}={}",
            environment_item.name, environment_item.value
        ));
    }

    let mut volume = vec![];
    for volume_item in &service.volumes {
        let volume_item: Volume = vars.replace(volume_item)?.parse()?;
        volume.push(format!(
            "{}:{}:{}",
            volume_item.volume, volume_item.container_path, volume_item.access_mode
        ));
    }

    let container = Container {
        container_name: service_name.to_string(),
        image: vars.replace(&service.image)?,
        userns,
        run_init: service.init,
        health_start_period: vars.replace(&service.healthcheck.start_period)?,
        health_cmd: vars.replace(&service.healthcheck.test)?,
        health_interval: vars.replace(&service.healthcheck.interval)?,
        health_timeout: vars.replace(&service.healthcheck.timeout)?,
        health_retries: service.healthcheck.retries.to_string(),
        environment_file: "%h/protected/nextcloud/secrets.env".to_string(),
        environment,
        volume,
        read_only: service.read_only,
        tmpfs,
        add_capability,
        drop_capability,
        exec: vars.replace_opt(&service.command)?,
        stop_timeout: vars.replace_opt(&service.stop_grace_period)?,
        shm_size: vars.replace_opt(&service.shm_size)?,
    };

    Ok(Quadlet {
        unit,
        service: Service {
            restart: "always".to_string(),
            restart_sec: "10".to_string(),
        },
        container,
        install: Install {
            wanted_by: "protected.target".to_string(),
        },
    })
}

trait Replace {
    fn replace(&self, value: &DynamicString) -> anyhow::Result<String>;
    fn replace_vec(&self, value: &[DynamicString]) -> anyhow::Result<Vec<String>>;
    fn replace_opt(&self, value: &Option<DynamicString>) -> anyhow::Result<Option<String>>;
}

impl Replace for BTreeMap<String, String> {
    fn replace(&self, value: &DynamicString) -> anyhow::Result<String> {
        value.replaced(self)
    }

    fn replace_vec(&self, value: &[DynamicString]) -> anyhow::Result<Vec<String>> {
        value.iter().map(|s| s.replaced(self)).collect()
    }

    fn replace_opt(&self, value: &Option<DynamicString>) -> anyhow::Result<Option<String>> {
        value.as_ref().map(|s| s.replaced(self)).transpose()
    }
}
