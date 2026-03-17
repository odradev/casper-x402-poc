use anyhow::{anyhow, Result};
use casper_types::{
    account::AccountHash,
    bytesrepr::ToBytes,
    crypto::{PublicKey, SecretKey},
};
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{CasperAuthorization, PaymentRequirements};

/// Build the 159-byte authorization message pre-image.
fn build_message(
    from_hash: &[u8; 32],
    to_hash: &[u8; 32],
    amount: u64,
    valid_after: u64,
    valid_before: u64,
    nonce: &[u8],
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(159);
    msg.extend_from_slice(b"casper-x402-v2:");
    msg.extend_from_slice(from_hash);
    msg.extend_from_slice(to_hash);

    // amount as U256 little-endian (only lower 8 bytes used for u64)
    let mut amount_bytes = [0u8; 32];
    amount_bytes[..8].copy_from_slice(&amount.to_le_bytes());
    msg.extend_from_slice(&amount_bytes);

    msg.extend_from_slice(&valid_after.to_le_bytes());
    msg.extend_from_slice(&valid_before.to_le_bytes());

    let mut nonce_padded = [0u8; 32];
    let len = nonce.len().min(32);
    nonce_padded[..len].copy_from_slice(&nonce[..len]);
    msg.extend_from_slice(&nonce_padded);

    msg
}

/// Sign a payment authorization and return a `CasperAuthorization`.
pub fn sign_authorization(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    requirements: &PaymentRequirements,
) -> Result<CasperAuthorization> {
    // Derive from AccountHash
    let from_hash = AccountHash::from(public_key);

    // Decode pay_to as account hash hex
    let to_hash = AccountHash::from_formatted_str(&format!("account-hash-{}", requirements.pay_to))
        .map_err(|_| anyhow!("Invalid pay_to address"))?;

    // Parse amount fsom string to u64
    let amount: u64 = requirements
        .amount
        .parse()
        .map_err(|e| anyhow!("invalid amount: {}", e))?;

    // Generate random 32-byte nonce
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);

    // Time window
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let valid_after = now - 1;
    let valid_before = now + requirements.max_timeout_seconds;

    // Build and sign message
    let message = build_message(
        &from_hash.value(),
        &to_hash.value(),
        amount,
        valid_after,
        valid_before,
        &nonce,
    );

    let signature = casper_types::crypto::sign(&message, secret_key, public_key);

    // Serialize with tag bytes intact
    let pk_bytes = public_key
        .to_bytes()
        .map_err(|e| anyhow!("pk serialization: {:?}", e))?;
    let sig_bytes = signature
        .to_bytes()
        .map_err(|e| anyhow!("sig serialization: {:?}", e))?;

    Ok(CasperAuthorization {
        from: hex::encode(from_hash.value()),
        to: hex::encode(to_hash.value()),
        amount: requirements.amount.clone(),
        valid_after,
        valid_before,
        nonce: hex::encode(nonce),
        public_key: hex::encode(pk_bytes),
        signature: hex::encode(sig_bytes),
    })
}
