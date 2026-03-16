use anyhow::{Context, Result};

pub struct Config {
    pub facilitator_url: String,
    pub resource_url: String,
    pub resource_port: u16,
    pub pay_to: String,
    pub payment_amount: u64,
    pub secret_key_path: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let facilitator_url =
            std::env::var("FACILITATOR_URL").context("Missing FACILITATOR_URL")?;
        let resource_url =
            std::env::var("RESOURCE_SERVER_URL").context("Missing RESOURCE_SERVER_URL")?;
        let resource_port: u16 = std::env::var("RESOURCE_SERVER_PORT")
            .context("Missing RESOURCE_SERVER_PORT")?
            .parse()
            .context("Invalid RESOURCE_SERVER_PORT")?;
        let pay_to = std::env::var("PAY_TO").context("Missing PAY_TO")?;
        let payment_amount: u64 = std::env::var("PAYMENT_AMOUNT")
            .unwrap_or_else(|_| "1000000".to_string())
            .parse()
            .context("Invalid PAYMENT_AMOUNT")?;
        let secret_key_path =
            std::env::var("SECRET_KEY_PATH").context("Missing SECRET_KEY_PATH")?;

        Ok(Self {
            facilitator_url,
            resource_url,
            resource_port,
            pay_to,
            payment_amount,
            secret_key_path,
        })
    }
}
