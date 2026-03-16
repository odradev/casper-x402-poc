use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;

use crate::{
    X_PAYMENT_REQUIRED,
    types::{PaymentPayload, PaymentRequired, SettleRequest, SettleResponse},
};

enum ProcessingError {
    BadRequest(String),
    BadGateway(String),
    PaymentFailed(String),
}

impl IntoResponse for ProcessingError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::BadGateway(msg) => (StatusCode::BAD_GATEWAY, msg),
            Self::PaymentFailed(msg) => (StatusCode::PAYMENT_REQUIRED, msg),
        };
        (status, HeaderMap::new(), message).into_response()
    }
}

#[derive(Clone)]
pub struct ResourceServerState {
    pub payment_requirements: PaymentRequired,
    pub facilitator_url: String,
    pub http_client: reqwest::Client,
}

impl ResourceServerState {
    fn decode_payment(header: &axum::http::HeaderValue) -> Result<PaymentPayload, ProcessingError> {
        let bytes = STANDARD
            .decode(header.as_bytes())
            .map_err(|_| ProcessingError::BadRequest("Cannot base64-decode X-PAYMENT".into()))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessingError::BadRequest(format!("Cannot parse payment payload: {e}")))
    }

    async fn settle_payment(
        &self,
        payload: PaymentPayload,
    ) -> Result<SettleResponse, ProcessingError> {
        let url = format!("{}/settle", self.facilitator_url);

        let req = SettleRequest {
            payment_payload: payload,
            payment_requirements: self.payment_requirements.clone(),
        };

        let resp = self
            .http_client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| ProcessingError::BadGateway(format!("Facilitator unreachable: {e}")))?;

        println!(
            "[server] Received response from facilitator with status {}",
            resp.status()
        );

        resp.json()
            .await
            .map_err(|e| ProcessingError::BadGateway(format!("Invalid facilitator response: {e}")))
    }
}

async fn process_payment(state: &ResourceServerState, payload: PaymentPayload) -> Response {
    let settle_resp = match state.settle_payment(payload).await {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };

    if !settle_resp.success {
        return ProcessingError::PaymentFailed(format!(
            "Payment failed: {}",
            settle_resp.error_reason.unwrap_or_default()
        ))
        .into_response();
    }

    let tx_hash = settle_resp.transaction.unwrap_or_default();
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-payment-response",
        tx_hash
            .parse()
            .unwrap_or_else(|_| "unknown".parse().unwrap()),
    );
    (
        StatusCode::OK,
        headers,
        json!({"data": "secret-resource-content"}).to_string(),
    )
        .into_response()
}

async fn handle_data(
    State(state): State<ResourceServerState>,
    headers: HeaderMap,
) -> Response {
    let Some(payment_header) = headers.get("x-payment") else {
        println!("[server] No payment provided. Responding with 402 and requirements.");
        return respond_with_payment_required(&state.payment_requirements).into_response();
    };

    println!("[server] Forwarding payment to facilitator for settlement...");

    match ResourceServerState::decode_payment(payment_header) {
        Ok(payload) => process_payment(&state, payload).await,
        Err(e) => e.into_response(),
    }
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

    (
        StatusCode::PAYMENT_REQUIRED,
        headers,
        "Payment required".to_string(),
    )
}
