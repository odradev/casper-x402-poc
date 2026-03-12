use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequired {
    pub x402_version: u8,
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub amount: u64,
    pub pay_to: String,
    pub max_timeout_secs: u64,
    pub resource: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPayload {
    pub x402_version: u8,
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub authorization: CasperAuthorization,
}

#[derive(Debug, Deserialize)]
pub struct SettleResponse {
    pub success: bool,
    pub transaction: Option<String>,
    pub error_reason: Option<String>,
    pub payer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SettleRequest {
    pub payment_payload: PaymentPayload,
    pub payment_requirements: PaymentRequired,
}
