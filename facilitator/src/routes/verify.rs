use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use casper_types::{
    account::AccountHash,
    bytesrepr::FromBytes,
    crypto::{verify, PublicKey, Signature},
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    types::{CasperAuthorization, PaymentRequired, VerifyRequest, VerifyResponse},
    AppState,
};

/// Build the 159-byte authorization message pre-image (same as contract).
pub fn build_message(
    from_hash: &[u8; 32],
    to_hash: &[u8; 32],
    amount: u64,
    valid_after: u64,
    valid_before: u64,
    nonce: &[u8],
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(159);
    msg.extend_from_slice(b"casper-x402-v1:");
    msg.extend_from_slice(from_hash);
    msg.extend_from_slice(to_hash);

    // amount as U256 little-endian 32 bytes
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

/// Validate a `CasperAuthorization` against payment requirements — off-chain only.
pub fn verify_authorization(
    auth: &CasperAuthorization,
    requirements: &PaymentRequired,
) -> Result<String, String> {
    // 1. Check destination and amount match requirements
    if auth.to != requirements.pay_to {
        return Err(format!(
            "payment destination mismatch: got {}, want {}",
            auth.to, requirements.pay_to
        ));
    }
    if auth.amount != requirements.amount {
        return Err(format!(
            "amount mismatch: got {}, want {}",
            auth.amount, requirements.amount
        ));
    }

    // 2. Time window check
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    if now <= auth.valid_after {
        return Err("authorization not yet valid".to_string());
    }
    if now >= auth.valid_before {
        return Err("authorization expired".to_string());
    }

    // 3. Decode public key
    let pk_bytes =
        hex::decode(&auth.public_key).map_err(|e| format!("invalid public_key hex: {}", e))?;
    let (public_key, _) = PublicKey::from_bytes(&pk_bytes)
        .map_err(|e| format!("cannot parse public key: {:?}", e))?;

    // 4. Verify public_key → from address
    let derived_hash = AccountHash::from(&public_key);
    let from_bytes = hex::decode(&auth.from).map_err(|e| format!("invalid from hex: {}", e))?;
    if from_bytes.len() != 32 {
        return Err("from address must be 32 bytes".to_string());
    }
    let mut from_arr = [0u8; 32];
    from_arr.copy_from_slice(&from_bytes);
    let from_hash = AccountHash(from_arr);
    if derived_hash != from_hash {
        return Err("public key does not match from address".to_string());
    }

    // 5. Decode to address
    let to_bytes = hex::decode(&auth.to).map_err(|e| format!("invalid to hex: {}", e))?;
    if to_bytes.len() != 32 {
        return Err("to address must be 32 bytes".to_string());
    }
    let mut to_arr = [0u8; 32];
    to_arr.copy_from_slice(&to_bytes);

    // 6. Decode nonce
    let nonce = hex::decode(&auth.nonce).map_err(|e| format!("invalid nonce hex: {}", e))?;

    // 7. Build message and verify signature
    let message = build_message(
        &from_arr,
        &to_arr,
        auth.amount,
        auth.valid_after,
        auth.valid_before,
        &nonce,
    );

    let sig_bytes =
        hex::decode(&auth.signature).map_err(|e| format!("invalid signature hex: {}", e))?;
    let (signature, _) = Signature::from_bytes(&sig_bytes)
        .map_err(|e| format!("cannot parse signature: {:?}", e))?;

    verify(&message, &signature, &public_key)
        .map_err(|e| format!("signature verification failed: {:?}", e))?;

    Ok(auth.from.clone())
}

pub async fn handle_verify(
    State(_state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    match verify_authorization(
        &req.payment_payload.authorization,
        &req.payment_requirements,
    ) {
        Ok(payer) => {
            println!(
                "Authorization valid for payer {}. Responding with 200 OK.",
                payer
            );
            (
                StatusCode::OK,
                Json(VerifyResponse {
                    is_valid: true,
                    invalid_reason: None,
                    payer: Some(payer),
                }),
            )
        }
        Err(reason) => {
            println!("Authorization invalid: {}. Responding with 200 OK.", reason);
            (
                StatusCode::OK,
                Json(VerifyResponse {
                    is_valid: false,
                    invalid_reason: Some(reason),
                    payer: None,
                }),
            )
        }
    }
}
