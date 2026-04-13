use anyhow::{anyhow, Result};
use casper_eip_712::DomainSeparator;
use casper_types::{
    account::AccountHash,
    bytesrepr::ToBytes,
    crypto::{PublicKey, SecretKey},
    U256,
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
    let pay_to_str = x402_eip712::format_casper_address(&requirements.pay_to);
    let to_hash = AccountHash::from_formatted_str(&pay_to_str)
        .map_err(|_| anyhow!("Invalid pay_to address: {}", pay_to_str))?;

    // Parse amount from string to U256
    let amount =
        U256::from_dec_str(&requirements.amount).map_err(|e| anyhow!("invalid amount: {}", e))?;

    // Generate random 32-byte nonce
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);

    // Time window
    let now = U256::from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
    let valid_after = now - U256::from(1);
    let valid_before = now + U256::from(requirements.max_timeout_seconds);
    let mut valid_after_bytes = [0u8; 32];
    let mut valid_before_bytes = [0u8; 32];
    valid_after.to_big_endian(&mut valid_after_bytes);
    valid_before.to_big_endian(&mut valid_before_bytes);

    // Build and sign message
    let mut value_bytes = [0u8; 32];
    amount.to_big_endian(&mut value_bytes);
    let from_addr = x402_eip712::casper_address_from_bytes(from_hash.0);
    let to_addr = x402_eip712::casper_address_from_bytes(to_hash.0);
    let transfer = x402_types::TransferAuthorization {
        from: from_addr,
        to: to_addr,
        value: value_bytes,
        valid_after: valid_after_bytes,
        valid_before: valid_before_bytes,
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
