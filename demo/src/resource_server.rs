use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use x402_types::{VerifyRequest, VerifyResponse};

use crate::{X_PAYMENT_REQUIRED, types::{PaymentPayload, PaymentRequired, SettleRequest, SettleResponse}};


#[derive(Clone)]
pub struct ResourceServerState {
    pub payment_requirements: PaymentRequired,
    pub facilitator_url: String,
    pub http_client: reqwest::Client,
}

async fn handle_data(
    State(state): State<ResourceServerState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(payment_header) = headers.get("x-payment") {
        // Client has provided payment — forward to facilitator for settlement
        println!("[server] Forwarding payment to facilitator for settlement...");
        let payload_json = match STANDARD.decode(payment_header.as_bytes()) {
            Ok(b) => b,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    HeaderMap::new(),
                    "Cannot base64-decode X-PAYMENT".to_string(),
                )
                    .into_response()
            }
        };

        let payment_payload: PaymentPayload =
            match serde_json::from_slice(&payload_json) {
                Ok(p) => p,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        HeaderMap::new(),
                        format!("Cannot parse payment payload: {}", e),
                    )
                        .into_response()
                }
            };

        let verify_req = VerifyRequest {
            payment_payload: payment_payload.clone(),
            payment_requirements: state.payment_requirements.clone(),
        };

        let verify_url = format!("{}/verify", state.facilitator_url);
        println!("[server] Verifying payment with facilitator at {}", verify_url);
        let resp = match state
            .http_client
            .post(&verify_url)
            .json(&verify_req)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    HeaderMap::new(),
                    format!("Facilitator unreachable: {}", e),
                )
                    .into_response()
            }
        };

        let verify_resp: VerifyResponse = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    HeaderMap::new(),
                    format!("Invalid facilitator response: {}", e),
                )
                    .into_response()
            }
        };
        if !verify_resp.is_valid {
            return respond_with_payment_required(&state.payment_requirements).into_response();
        }

        let settle_req = SettleRequest {
            payment_payload: payment_payload,
            payment_requirements: state.payment_requirements.clone(),
        };
        let settle_url = format!("{}/settle", state.facilitator_url);
        let resp = match state
            .http_client
            .post(&settle_url)
            .json(&settle_req)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    HeaderMap::new(),
                    format!("Facilitator unreachable: {}", e),
                )
                    .into_response()
            }
        };

        println!("[server] Received response from facilitator with status {}", resp.status());
        println!("[server] Facilitator response headers: {:#?}", resp.headers());

        let settle_resp: SettleResponse = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    HeaderMap::new(),
                    format!("Invalid facilitator response: {}", e),
                )
                    .into_response()
            }
        };

        if settle_resp.success {
            let tx_hash = settle_resp.transaction.unwrap_or_default();
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "x-payment-response",
                tx_hash
                    .parse()
                    .unwrap_or_else(|_| "unknown".parse().unwrap()),
            );
            return (
                StatusCode::OK,
                resp_headers,
                json!({"data": "secret-resource-content"}).to_string(),
            )
                .into_response();
        } else {
            return (
                StatusCode::PAYMENT_REQUIRED,
                HeaderMap::new(),
                format!(
                    "Payment failed: {}",
                    settle_resp.error_reason.unwrap_or_default()
                ),
            )
                .into_response();
        }
    }

    // No payment provided — respond with 402 and payment requirements
    println!("[server] No payment provided. Responding with 402 and requirements.");
    respond_with_payment_required(&state.payment_requirements).into_response()
}

pub fn build_router(state: ResourceServerState) -> Router {
    Router::new()
        .route("/api/data", get(handle_data))
        .with_state(state)
}

fn respond_with_payment_required(requirements: &PaymentRequired) -> impl IntoResponse {
    let requirements_json =
        serde_json::to_string(requirements).unwrap_or_else(|_| "{}".to_string());
    let encoded = STANDARD.encode(requirements_json.as_bytes());

    let mut headers = HeaderMap::new();
    headers.insert(X_PAYMENT_REQUIRED, encoded.parse().expect("header value"));

    (StatusCode::PAYMENT_REQUIRED, headers, "Payment required".to_string())
}
