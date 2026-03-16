use serde::{Deserialize, Serialize};

/// Sent by resource server in the `X-PAYMENT-REQUIRED` header (base64-encoded JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequired {
    #[serde(rename = "x402Version")]
    pub x402_version: u8,
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub amount: u64,
    #[serde(rename = "payTo")]
    pub pay_to: String,
    #[serde(rename = "maxTimeoutSecs")]
    pub max_timeout_secs: u64,
    pub resource: String,
}

/// The signed authorization (all fields hex-encoded bytes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasperAuthorization {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub valid_after: u64,
    pub valid_before: u64,
    pub nonce: String,
    pub public_key: String,
    pub signature: String,
}

/// Payload the client attaches in the `X-PAYMENT` header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPayload {
    pub x402_version: u8,
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub authorization: CasperAuthorization,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub payment_payload: PaymentPayload,
    pub payment_requirements: PaymentRequired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub is_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettleRequest {
    pub payment_payload: PaymentPayload,
    pub payment_requirements: PaymentRequired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettleResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<String>,
}
