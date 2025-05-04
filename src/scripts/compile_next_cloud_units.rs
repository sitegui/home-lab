mod compose_schema;
mod environment_encoder;
mod quadlet_schema;

use crate::scripts::compile_next_cloud_units::compose_schema::{Compose, ComposeService, Volume};
use crate::scripts::compile_next_cloud_units::environment_encoder::{
    EnvironmentEncoder, ServiceEnvironmentEncoder,
};
use crate::scripts::compile_next_cloud_units::quadlet_schema::{
    Container, Install, Quadlet, Service, Unit,
};
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

pub fn compile_next_cloud_units(
    input_secrets: PathBuf,
    output_secrets: PathBuf,
) -> anyhow::Result<()> {
    fs::create_dir_all(&output_secrets)?;
    let output_secrets = output_secrets
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", output_secrets.display()))?;

    let nextcloud_dir = Path::new("config/nextcloud");
    let encoder = EnvironmentEncoder::new(&input_secrets, &nextcloud_dir.join("vars.conf"))?;

    let compose_source = fs::read_to_string(nextcloud_dir.join("vendor/latest.yml"))?;
    let compose: Compose = serde_yml::from_str(&compose_source)?;

    for (service_name, service) in compose.services {
        let service_name = service_name.replace("-aio-", "-");

        let mut service_encoder = ServiceEnvironmentEncoder::new(&encoder);
        let service_secrets_path = output_secrets.join(format!("{}.env", service_name));

        let quadlet = compile_service(
            &mut service_encoder,
            &service_secrets_path,
            &service_name,
            &service,
        )
        .with_context(|| format!("failed to compile {}", service_name))?;

        let contents = quadlet.to_string();
        fs::write(
            nextcloud_dir.join(format!("units/{}.container", service_name)),
            contents,
        )?;
        fs::write(service_secrets_path, service_encoder.secret_env_contents())?;
    }

    Ok(())
}

fn compile_service(
    encoder: &mut ServiceEnvironmentEncoder,
    service_secrets_path: &Path,
    name: &str,
    service: &ComposeService,
) -> anyhow::Result<Quadlet> {
    let unit = Unit {
        after: service.depends_on.keys().cloned().collect(),
    };

    let userns = encoder
        .encode_public_opt(&service.user)?
        .map(|user| format!("keep-id:uid={}", user));

    let tmpfs = encoder.encode_public_vec(&service.tmpfs)?;
    let add_capability = encoder.encode_public_vec(&service.cap_add)?;
    let drop_capability = encoder.encode_public_vec(&service.cap_drop)?;

    let environment_file = service_secrets_path
        .to_str()
        .context("failed to represent secrets path")?
        .to_string();

    let mut environment = vec![];
    for environment_item in &service.environment {
        if let Some((name, value)) = encoder.encode_environment(environment_item)? {
            environment.push(format!("{}={}", name, value));
        }
    }

    let mut volume = vec![];
    for volume_item in &service.volumes {
        let volume_item: Volume = encoder.encode_public(volume_item)?.parse()?;
        volume.push(format!(
            "{}:{}:{}",
            volume_item.volume, volume_item.container_path, volume_item.access_mode
        ));
    }

    let container = Container {
        container_name: name.to_string(),
        image: encoder.encode_public(&service.image)?,
        userns,
        run_init: service.init,
        health_start_period: encoder.encode_public(&service.healthcheck.start_period)?,
        health_cmd: encoder.encode_public(&service.healthcheck.test)?,
        health_interval: encoder.encode_public(&service.healthcheck.interval)?,
        health_timeout: encoder.encode_public(&service.healthcheck.timeout)?,
        health_retries: service.healthcheck.retries.to_string(),
        environment_file,
        environment,
        volume,
        read_only: service.read_only,
        tmpfs,
        add_capability,
        drop_capability,
        exec: encoder.encode_public_opt(&service.command)?,
        stop_timeout: encoder.encode_public_opt(&service.stop_grace_period)?,
        shm_size: encoder.encode_public_opt(&service.shm_size)?,
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
