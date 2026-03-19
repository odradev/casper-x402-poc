mod b64;
mod client;
mod config;
mod resource_server;
mod types;
mod ui;

use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::routing::{get, post};
use casper_types::crypto::{PublicKey, SecretKey};

use config::Config;
use types::{PaymentRequired, PaymentRequirements, ResourceInfo};
use x402_eip712::x402_domain;

pub const HEADER_PAYMENT_REQUIRED: &str = "PAYMENT-REQUIRED";
pub const HEADER_PAYMENT_SIGNATURE: &str = "PAYMENT-SIGNATURE";
pub const HEADER_PAYMENT_RESPONSE: &str = "PAYMENT-RESPONSE";

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();

    let config = Config::from_env()?;
    let (secret_key, public_key) = load_demo_keys_from_file(&config.secret_key_path)?;
    let resource_url = format!("{}/api/data", config.resource_url);

    let requirements = PaymentRequired {
        x402_version: 2,
        error: None,
        resource: ResourceInfo {
            url: resource_url.clone(),
            description: None,
            mime_type: None,
        },
        accepts: vec![PaymentRequirements {
            scheme: "exact".to_string(),
            network: "casper:test".to_string(),
            asset: "CEP18X402".to_string(),
            amount: config.payment_amount.to_string(),
            pay_to: config.pay_to.clone(),
            max_timeout_seconds: 300,
            extra: serde_json::Value::Null,
        }],
        extensions: None,
    };

    let resource_state = resource_server::ResourceServerState {
        payment_requirements: requirements,
        facilitator_url: config.facilitator_url.clone(),
        http_client: reqwest::Client::new(),
    };

    let ui_state = Arc::new(ui::UiState {
        secret_key,
        public_key,
        resource_url: config.resource_url.clone(),
        facilitator_url: config.facilitator_url.clone(),
        domain: x402_domain(&config.chain_name, config.x402_token_address)
    });

    // Resource server router (already has its own state applied)
    let resource_router = resource_server::build_router(resource_state);

    // UI router with its own state
    let ui_router = axum::Router::new()
        .route("/", get(ui::handle_index))
        .route("/api/run-flow", post(ui::handle_run_flow))
        .with_state(ui_state);

    // Merge both routers
    let router = resource_router.merge(ui_router);

    let addr = format!("127.0.0.1:{}", config.resource_port);
    println!("Demo server running at http://{}", addr);
    println!("Open in your browser to start the payment flow.");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

fn load_demo_keys_from_file(file_path: &str) -> Result<(SecretKey, PublicKey)> {
    let secret_key =
        SecretKey::from_file(file_path).map_err(|e| anyhow!("Failed to parse PEM: {:?}", e))?;
    let public_key = PublicKey::from(&secret_key);
    Ok((secret_key, public_key))
}
