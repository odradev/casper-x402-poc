use serde::Serialize;

pub use x402_types::{
    CasperAuthorization, PaymentPayload, PaymentRequired, SettleRequest, SettleResponse,
};

#[derive(Debug, Serialize)]
pub struct FlowStep {
    pub step: u8,
    pub title: String,
    pub status: String,
    pub details: serde_json::Value,
}
