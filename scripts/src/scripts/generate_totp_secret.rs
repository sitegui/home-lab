use totp_rs::Secret;

pub fn generate_totp_secret() -> anyhow::Result<()> {
    let secret = Secret::generate_secret().to_encoded();

    tracing::info!("Generated secret: {}", secret);

    Ok(())
}
