mod compose_schema;
mod environment_encoder;
mod path_from_home;
mod quadlet_schema;

use crate::child::Child;
use crate::scripts::compile_nextcloud_units::compose_schema::{Compose, ComposeService, Volume};
use crate::scripts::compile_nextcloud_units::environment_encoder::{
    EnvironmentEncoder, ServiceEnvironmentEncoder,
};
use crate::scripts::compile_nextcloud_units::path_from_home::PathFromHome;
use crate::scripts::compile_nextcloud_units::quadlet_schema::{
    Container, Install, Quadlet, Service, Unit,
};
use anyhow::Context;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn compile_nextcloud_units(
    input_secrets: PathBuf,
    output_secrets_dir: PathBuf,
    volumes_dir: PathBuf,
    profiles: Vec<String>,
) -> anyhow::Result<()> {
    let output_secrets_dir = PathFromHome::new(&output_secrets_dir)?;
    let volumes_dir = PathFromHome::new(&volumes_dir)?;

    let nextcloud_dir = Path::new("config/nextcloud");
    let units_dir = nextcloud_dir.join("units");
    fs::create_dir_all(&units_dir)?;

    let encoder = EnvironmentEncoder::new(&input_secrets, &nextcloud_dir.join("vars.conf"))?;

    let nc_domain = encoder
        .get_var("NC_DOMAIN")
        .context("failed to get NC_DOMAIN")?;

    let compose_source = fs::read_to_string(nextcloud_dir.join("vendor/latest.yml"))?;
    let compose: Compose = serde_yml::from_str(&compose_source)?;

    for (service_name, service) in compose.services {
        let mut service_encoder = ServiceEnvironmentEncoder::new(&encoder);
        let service_profiles = service_encoder.encode_public_vec(&service.profiles)?;

        if !service_profiles.is_empty()
            && service_profiles
                .iter()
                .all(|profile| !profiles.contains(profile))
        {
            continue;
        }

        let service_secrets_path = output_secrets_dir.join(format!("{}.env", service_name));

        let quadlet = compile_service(
            &mut service_encoder,
            &service_secrets_path,
            &volumes_dir,
            &service_name,
            nc_domain,
            &service,
        )
        .with_context(|| format!("failed to compile {}", service_name))?;

        let contents = quadlet.to_string();
        fs::write(
            units_dir.join(format!("{}.container", service_name)),
            contents,
        )?;
    }

    Ok(())
}

fn compile_service(
    encoder: &mut ServiceEnvironmentEncoder,
    service_secrets_path: &PathFromHome,
    volumes_dir: &PathFromHome,
    service_name: &str,
    nc_domain: &str,
    service: &ComposeService,
) -> anyhow::Result<Quadlet> {
    let unit = Unit {
        after: service
            .depends_on
            .keys()
            .map(|another_service| format!("{}.service", another_service))
            .collect(),
    };

    let user_ns = match &service.user {
        Some(user) => {
            let uid = encoder.encode_public(user)?;
            format!("keep-id:uid={},gid={}", uid, uid)
        }
        None => "keep-id".to_string(),
    };

    let tmpfs = encoder.encode_public_vec(&service.tmpfs)?;
    let add_capability = encoder.encode_public_vec(&service.cap_add)?;
    let drop_capability = encoder.encode_public_vec(&service.cap_drop)?;

    let mut environment = BTreeMap::new();
    for environment_item in &service.environment {
        if let Some((name, value)) = encoder.encode_environment(environment_item)? {
            environment.insert(name, value);
        }
    }

    let mut volume = vec![];
    for volume_item in &service.volumes {
        let volume_item: Volume = encoder.encode_public(volume_item)?.parse()?;
        if volume_item.volume.is_empty() {
            continue;
        }

        let volume_path = if volume_item.volume.contains('/') {
            volume_item.volume
        } else {
            let volume_dir = volumes_dir.join(volume_item.volume);
            fs::create_dir_all(&volume_dir)?;
            volume_dir.to_systemd_string()?
        };

        volume.push(format!(
            "{}:{}:{}",
            volume_path, volume_item.container_path, volume_item.access_mode
        ));
    }

    let environment_file = if let Some(contents) = encoder.secret_env_contents() {
        fs::write(service_secrets_path, contents)?;
        Child::new("chmod")
            .arg("600")
            .arg(service_secrets_path.as_ref())
            .run()?;
        Some(service_secrets_path.to_systemd_string()?)
    } else {
        None
    };

    let mut network = vec!["nextcloud.network".to_string()];
    if !service.ports.is_empty() {
        // We assume that containers that expose ports will be exposed through the reverse proxy
        network.push("caddy-nextcloud.network".to_string());
    }

    let stop_timeout_s = match encoder.encode_public_opt(&service.stop_grace_period)? {
        None => None,
        Some(stop_grace_period) => Some(
            stop_grace_period
                .strip_suffix("s")
                .context("failed to parse stop_grace_period as seconds")?
                .trim()
                .parse()
                .context("failed to parse stop_grace_period as seconds")?,
        ),
    };

    // Hack: the nextcloud-aio-nextcloud container needs to talk itself using the public domain
    // name. However, since I'm using a public ipv6-only domain (that is, no A record on the public
    // dns), and the podman container network doesn't have support for ipv6 (hopefully fixed in
    // podman v5), I need this hack. This will add an entry to /etc/hosts of the container telling
    // it that the public dns points to the host.
    let add_host = Some(format!("{}:host-gateway", nc_domain));

    let container = Container {
        container_name: service_name.to_string(),
        image: encoder.encode_public(&service.image)?,
        pull: "never".to_string(),
        auto_update: "local".to_string(),
        user_ns,
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
        stop_timeout_s,
        shm_size: encoder.encode_public_opt(&service.shm_size)?,
        network,
        add_host,
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
