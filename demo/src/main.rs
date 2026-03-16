mod client;
mod config;
mod resource_server;
mod types;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use casper_types::crypto::{PublicKey, SecretKey};

use config::Config;
use types::{PaymentPayload, PaymentRequired};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();

    let config = Config::from_env()?;
    let (secret_key, public_key) = load_demo_keys_from_file(&config.secret_key_path)?;

    // Payment requirements for the demo resource server
    let requirements = PaymentRequired {
        x402_version: 1,
        scheme: "casper-exact".to_string(),
        network: "casper-test".to_string(),
        asset: "CEP18".to_string(),
        amount: config.payment_amount,
        pay_to: config.pay_to.clone(),
        max_timeout_secs: 300,
        resource: format!("{}/api/data", config.resource_url),
    };

    // Start the mock resource server in a background task
    let server_state = resource_server::ResourceServerState {
        payment_requirements: requirements.clone(),
        facilitator_url: config.facilitator_url.clone(),
        http_client: reqwest::Client::new(),
    };
    let router = resource_server::build_router(server_state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", config.resource_port))
        .await
        .expect("Failed to bind resource server");
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let http = reqwest::Client::new();
    let data_url = format!("{}/api/data", config.resource_url);

    // --- Step 1: Unauthenticated request → expect 402 ---
    println!("[client] GET {} (unauthenticated)", data_url);
    let resp = http.get(&data_url).send().await?;
    assert_eq!(resp.status(), 402, "Expected 402 Payment Required");

    let payment_required_b64 = resp
        .headers()
        .get("x-payment-required")
        .ok_or_else(|| anyhow!("Missing X-PAYMENT-REQUIRED header"))?
        .to_str()?
        .to_string();

    let payment_required_json = STANDARD.decode(&payment_required_b64)?;
    let payment_required: PaymentRequired = serde_json::from_slice(&payment_required_json)?;

    println!(
        "[client] Got 402. Payment required: {} tokens to {}",
        payment_required.amount, payment_required.pay_to
    );

    // --- Step 2: Sign authorization ---
    println!("[client] Signing authorization...");
    let authorization = client::sign_authorization(&secret_key, &public_key, &payment_required)?;

    let payload = PaymentPayload {
        x402_version: payment_required.x402_version,
        scheme: payment_required.scheme.clone(),
        network: payment_required.network.clone(),
        asset: payment_required.asset.clone(),
        authorization,
    };
    let payload_json = serde_json::to_string(&payload)?;
    let payment_header = STANDARD.encode(payload_json.as_bytes());

    // --- Step 3: Retry with X-PAYMENT header ---
    println!("[client] Retrying with X-PAYMENT header...");
    let resp = http
        .get(&data_url)
        .header("x-payment", &payment_header)
        .send()
        .await?;

    let status = resp.status();
    let tx_hash = resp
        .headers()
        .get("x-payment-response")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("(none)")
        .to_string();
    let body = resp.text().await?;

    if status == 200 {
        println!("[client] SUCCESS! Body: {}", body);
        println!("[client] Transaction: {}", tx_hash);
        println!("=== Flow complete ===");
    } else {
        eprintln!("[client] Unexpected status {}: {}", status, body);
        std::process::exit(1);
    }

    Ok(())
}


fn load_demo_keys_from_file(file_path: &str) -> Result<(SecretKey, PublicKey)> {
    // In a real implementation this would read from a PEM file specified in the
    // environment.  Here we generate a fresh ephemeral key so the demo works
    // out-of-the-box without any setup.
    let secret_key = SecretKey::from_file(&file_path)
        .map_err(|e| anyhow!("Failed to parse PEM: {:?}", e))?;
    let public_key = PublicKey::from(&secret_key);
    Ok((secret_key, public_key))
}
