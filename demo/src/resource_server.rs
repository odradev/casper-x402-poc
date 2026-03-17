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
    b64::B64EncodeHeader,
    types::{PaymentPayload, PaymentRequired, SettleRequest, SettleResponse},
    HEADER_PAYMENT_REQUIRED, HEADER_PAYMENT_RESPONSE, HEADER_PAYMENT_SIGNATURE,
};

pub fn build_router(state: ResourceServerState) -> Router {
    Router::new()
        .route("/api/data", get(handle_data))
        .with_state(state)
}

async fn handle_data(State(state): State<ResourceServerState>, headers: HeaderMap) -> Response {
    let Some(payment_header) = headers.get(HEADER_PAYMENT_SIGNATURE) else {
        println!("[resource_server] No payment provided. Responding with 402 and requirements.");
        let mut headers = HeaderMap::new();
        headers.insert(
            HEADER_PAYMENT_REQUIRED,
            state.payment_requirements.b64_encoded_header().unwrap(),
        );
        return (
            StatusCode::PAYMENT_REQUIRED,
            headers,
            "Payment required".to_string(),
        )
            .into_response();
    };

    println!("[resource_server] Forwarding payment to facilitator for settlement...");
    match ResourceServerState::decode_payment(payment_header) {
        Ok(payload) => state.process_payment(payload).await,
        Err(e) => e.into_response(),
    }
}

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
        let bytes = STANDARD.decode(header.as_bytes()).map_err(|_| {
            ProcessingError::BadRequest("Cannot base64-decode PAYMENT-SIGNATURE".into())
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessingError::BadRequest(format!("Cannot parse payment payload: {e}")))
    }

    async fn settle_payment(
        &self,
        payload: PaymentPayload,
    ) -> Result<SettleResponse, ProcessingError> {
        let url = format!("{}/settle", self.facilitator_url);

        let req = SettleRequest {
            payment_requirements: payload.accepted.clone(),
            payment_payload: payload,
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

    async fn process_payment(&self, payload: PaymentPayload) -> Response {
        let settle_resp = match self.settle_payment(payload).await {
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

        let mut headers = HeaderMap::new();
        headers.insert(
            HEADER_PAYMENT_RESPONSE,
            settle_resp.b64_encoded_header().unwrap(),
        );
        (
            StatusCode::OK,
            headers,
            json!({"data": "secret-resource-content: https://odra.dev"}).to_string(),
        )
            .into_response()
    }
}
