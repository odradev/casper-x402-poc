use serde::Serialize;

use serde_json::json;
pub use x402_types::{
    CasperAuthorization, PaymentPayload, PaymentRequired, PaymentRequirements, ResourceInfo,
    SettleRequest, SettleResponse,
};

#[derive(Debug, Serialize)]
pub struct FlowStep {
    pub step: u8,
    pub title: String,
    pub status: String,
    pub details: serde_json::Value,
}

impl FlowStep {
    pub fn step_1_error<T: ToString>(reason: T) -> Self {
        Self {
            step: 1,
            title: "Request Resource".to_string(),
            status: "error".to_string(),
            details: json!({ "reason": reason.to_string() }),
        }
    }

    pub fn step_1_success(req: &PaymentRequirements) -> Self {
        Self {
            step: 1,
            title: "Request Resource".to_string(),
            status: "success".to_string(),
            details: serde_json::json!({
                "response": "402 Payment Required",
                "amount": req.amount,
                "pay_to": req.pay_to,
                "scheme": req.scheme,
                "network": req.network,
            }),
        }
    }

    pub fn step_2_error<T: ToString>(reason: T) -> Self {
        Self {
            step: 2,
            title: "Sign Authorization".to_string(),
            status: "error".to_string(),
            details: json!({ "reason": reason.to_string() }),
        }
    }

    pub fn step_2_success(authorization: &CasperAuthorization) -> Self {
        Self {
            step: 2,
            title: "Sign Authorization".to_string(),
            status: "success".to_string(),
            details: serde_json::json!({
                "from": authorization.from,
                "to": authorization.to,
                "amount": authorization.amount,
                "valid_after": authorization.valid_after,
                "valid_before": authorization.valid_before,
                "nonce": authorization.nonce,
            }),
        }
    }

    pub fn step_3_error(reason: Option<String>) -> Self {
        Self {
            step: 3,
            title: "Verify Payment".to_string(),
            status: "error".to_string(),
            details: serde_json::json!({
                "is_valid": false,
                "reason": reason,
            }),
        }
    }

    pub fn step_3_success(payer: Option<String>) -> Self {
        Self {
            step: 3,
            title: "Verify Payment".to_string(),
            status: "success".to_string(),
            details: serde_json::json!({
                "is_valid": true,
                "payer": payer,
            }),
        }
    }

    pub fn step_4_error<T: ToString>(reason: T) -> Self {
        Self {
            step: 4,
            title: "Pay & Access Resource".into(),
            status: "error".into(),
            details: serde_json::json!({
                "reason": reason.to_string()
            }),
        }
    }

    pub fn step_4_payment_error(status: u16, body: String) -> Self {
        Self {
            step: 4,
            title: "Pay & Access Resource".into(),
            status: "error".into(),
            details: serde_json::json!({
                "status": status,
                "body": body,
            }),
        }
    }

    pub fn step_4_success(status: u16, body: String, payment_response: Option<SettleResponse>) -> Self {
        Self {
            step: 4,
            title: "Pay & Access Resource".into(),
            status: "success".into(),
            details: serde_json::json!({
                "status": status,
                "payment_response": payment_response,
                "body": body,
            }),
        }
    }
}
