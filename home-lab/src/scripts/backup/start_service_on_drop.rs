use crate::child::Child;

pub struct StartServiceOnDrop(Option<String>);

impl StartServiceOnDrop {
    pub fn start(&mut self) -> anyhow::Result<()> {
        let Some(service) = self.0.take() else {
            return Ok(());
        };

        tracing::info!("Starting service {}", service);

        Child::new("systemctl")
            .args(["--user", "start", &service])
            .run()?;

        Ok(())
    }
}

impl Drop for StartServiceOnDrop {
    fn drop(&mut self) {
        if let Err(error) = self.start() {
            tracing::error!("Failed to start service: {:?}", error);
        }
    }
}

pub fn stop_service(name: String) -> anyhow::Result<StartServiceOnDrop> {
    tracing::info!("Stopping {}", name);
    Child::new("systemctl")
        .args(["--user", "stop", &name])
        .run()?;

    Ok(StartServiceOnDrop(Some(name)))
}
