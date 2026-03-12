use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{
    routes::verify::verify_authorization,
    types::{SettleRequest, SettleResponse},
    AppState,
};

pub async fn handle_settle(
    State(state): State<AppState>,
    Json(req): Json<SettleRequest>,
) -> impl IntoResponse {
    let auth = &req.payment_payload.authorization;

    // Verify off-chain first
    let payer = match verify_authorization(auth, &req.payment_requirements) {
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

    // Real settlement via Casper node
    match state
        .settler
        .call_transfer_with_authorization(
            &auth.from,
            &auth.to,
            auth.amount,
            auth.valid_after,
            auth.valid_before,
            &auth.nonce,
            &auth.public_key,
            &auth.signature,
        ).await
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
