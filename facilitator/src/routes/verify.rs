use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use casper_types::{
    account::AccountHash,
    bytesrepr::FromBytes,
    crypto::{verify, PublicKey, Signature},
};
use tokio::sync::OnceCell;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    AppState, types::{CasperAuthorization, PaymentRequirements, VerifyRequest, VerifyResponse}
};

pub static X402_DOMAIN: OnceCell<casper_eip_712::DomainSeparator> = OnceCell::const_new();
pub async fn x402_domain() -> &'static casper_eip_712::DomainSeparator {
    X402_DOMAIN.get_or_init(|| async {
        let x402_token_address_str = std::env::var("X402_TOKEN_ADDRESS")
                .expect("Missing X402_TOKEN_ADDRESS env var");
        let x402_token_address_str = x402_token_address_str
                .strip_prefix("hash-")
                .expect("Invalid contract format");
        let chain_name = std::env::var("ODRA_CASPER_LIVENET_CHAIN_NAME")
            .expect("Missing ODRA_CASPER_LIVENET_CHAIN_NAME");
        let mut x402_token_address = [0u8; 32];
        let bytes = hex::decode(x402_token_address_str).expect("Invalid address format");
        x402_token_address.copy_from_slice(&bytes);
        x402_eip712::x402_domain(&chain_name, x402_token_address)
    }).await
}

/// Validate a `CasperAuthorization` against payment requirements — off-chain only.
pub async fn verify_authorization(
    auth: &CasperAuthorization,
    requirements: &PaymentRequirements,
) -> Result<String, String> {
    let transfer = &auth.transfer;

    // 1. Check destination matches requirements
    let expected_to = x402_eip712::casper_address_to_bytes(&requirements.pay_to)
        .map_err(|e| format!("invalid pay_to: {}", e))?;
    let actual_to = x402_eip712::casper_address_to_bytes(&transfer.to)
        .map_err(|e| format!("invalid transfer.to address: {}", e))?;
    if actual_to != expected_to {
        return Err(format!(
            "payment destination mismatch: got {}, want {}",
            x402_eip712::format_casper_address(&transfer.to),
            x402_eip712::format_casper_address(&requirements.pay_to),
        ));
    }
    // 2. Check amount matches requirements
    let required_amount: u64 = requirements
        .amount
        .parse()
        .map_err(|e| format!("invalid required amount: {}", e))?;
    let mut required_value = [0u8; 32];
    required_value[32 - 8..].copy_from_slice(&required_amount.to_be_bytes());
    if transfer.value != required_value {
        return Err(format!(
            "amount mismatch: got {}, want {}",
            hex::encode(transfer.value),
            requirements.amount
        ));
    }

    // 3. Time window check
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    if now <= transfer.valid_after {
        return Err("authorization not yet valid".to_string());
    }
    if now >= transfer.valid_before {
        return Err("authorization expired".to_string());
    }

    // 4. Decode public key
    let pk_bytes =
        hex::decode(&auth.public_key).map_err(|e| format!("invalid public_key hex: {}", e))?;
    let (public_key, _) = PublicKey::from_bytes(&pk_bytes)
        .map_err(|e| format!("cannot parse public key: {:?}", e))?;

    // 5. Verify public_key → from address
    let derived_hash = AccountHash::from(&public_key);
    let from_bytes = x402_eip712::casper_address_to_bytes(&transfer.from)
        .map_err(|e| format!("invalid from address: {}", e))?;
    if derived_hash != AccountHash(from_bytes) {
        return Err("public key does not match from address".to_string());
    }

    // 6. Build EIP-712 message and verify signature
    let message = casper_eip_712::hash_typed_data(x402_domain().await, transfer);

    let sig_bytes =
        hex::decode(&auth.signature).map_err(|e| format!("invalid signature hex: {}", e))?;
    let (signature, _) = Signature::from_bytes(&sig_bytes)
        .map_err(|e| format!("cannot parse signature: {:?}", e))?;

    verify(&message, &signature, &public_key)
        .map_err(|e| format!("signature verification failed: {:?}", e))?;

    Ok(x402_eip712::format_casper_address(&transfer.from))
}

pub async fn handle_verify(
    State(_state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    let auth = match CasperAuthorization::from_payload_value(&req.payment_payload.payload) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(VerifyResponse {
                    is_valid: false,
                    invalid_reason: Some(format!("Cannot parse authorization from payload: {}", e)),
                    payer: None,
                }),
            );
        }
    };

    match verify_authorization(&auth, &req.payment_requirements).await {
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
