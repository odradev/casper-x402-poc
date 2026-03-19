use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{
    routes::verify::verify_authorization,
    types::{CasperAuthorization, SettleRequest, SettleResponse},
    AppState,
};

pub async fn handle_settle(
    State(state): State<AppState>,
    Json(req): Json<SettleRequest>,
) -> impl IntoResponse {
    let auth = match CasperAuthorization::from_payload_value(&req.payment_payload.payload) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(SettleResponse {
                    success: false,
                    transaction: None,
                    error_reason: Some(format!("Cannot parse authorization from payload: {}", e)),
                    payer: None,
                }),
            );
        }
    };

    // Verify off-chain first
    let payer = match verify_authorization(&auth, &req.payment_requirements).await {
        Ok(p) => p,
        Err(reason) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(SettleResponse {
                    success: false,
                    transaction: None,
                    error_reason: Some(reason),
                    payer: None,
                }),
            );
        }
    };

    let transfer = &auth.transfer;

    // Real settlement via Casper node
    match state
        .settler
        .call_transfer_with_authorization(
            transfer.from,
            transfer.to,
            transfer.value,
            transfer.valid_after,
            transfer.valid_before,
            transfer.nonce,
            auth.public_key.clone(),
            auth.signature.clone(),
        )
        .await
    {
        Ok(tx_hash) => (
            StatusCode::OK,
            Json(SettleResponse {
                success: true,
                transaction: Some(tx_hash),
                error_reason: None,
                payer: Some(payer),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SettleResponse {
                success: false,
                transaction: None,
                error_reason: Some(e.to_string()),
                payer: None,
            }),
        ),
    }
}
