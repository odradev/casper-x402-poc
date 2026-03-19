use anyhow::{anyhow, Result};
use casper_eip_712::DomainSeparator;
use casper_types::{
    U256, account::AccountHash, bytesrepr::ToBytes, crypto::{PublicKey, SecretKey}
};
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{CasperAuthorization, PaymentRequirements};

/// Sign a payment authorization and return a `CasperAuthorization`.
pub fn sign_authorization(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    domain: &DomainSeparator,
    requirements: &PaymentRequirements,
) -> Result<CasperAuthorization> {
    // Derive from AccountHash
    let from_hash = AccountHash::from(public_key);

    // Decode pay_to as account hash hex
    let to_hash = AccountHash::from_formatted_str(&format!("account-hash-{}", requirements.pay_to))
        .map_err(|_| anyhow!("Invalid pay_to address"))?;

    // Parse amount from string to U256
    let amount = U256::from_dec_str(&requirements.amount)
        .map_err(|e| anyhow!("invalid amount: {}", e))?;

    // Generate random 32-byte nonce
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);

    // Time window
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let valid_after = now - 1;
    let valid_before = now + requirements.max_timeout_seconds;

    // Build and sign message
    let mut value_bytes = [0u8; 32];
    amount.to_big_endian(&mut value_bytes);
    let transfer = x402_types::TransferAuthorization {
        from: from_hash.value(),
        to: to_hash.value(),
        value: value_bytes,
        valid_after,
        valid_before,
        nonce,
    };
    let message = casper_eip_712::hash_typed_data(&domain, &transfer);
    let signature = casper_types::crypto::sign(&message, secret_key, public_key);

    // Serialize with tag bytes intact
    let pk_bytes = public_key
        .to_bytes()
        .map_err(|e| anyhow!("pk serialization: {:?}", e))?;
    let sig_bytes = signature
        .to_bytes()
        .map_err(|e| anyhow!("sig serialization: {:?}", e))?;

    Ok(CasperAuthorization {
        transfer,
        public_key: hex::encode(pk_bytes),
        signature: hex::encode(sig_bytes),
    })
}
