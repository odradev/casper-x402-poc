use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use base64::{engine::general_purpose::STANDARD, Engine};

use x402_types::{VerifyRequest, VerifyResponse};

use crate::{X_PAYMENT_REQUIRED, types::{FlowStep, PaymentPayload, PaymentRequired}};

pub struct UiState {
    pub secret_key: casper_types::crypto::SecretKey,
    pub public_key: casper_types::crypto::PublicKey,
    pub resource_url: String,
    pub facilitator_url: String,
}

pub async fn handle_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn handle_run_flow(
    State(state): State<Arc<UiState>>,
) -> impl IntoResponse {
    let http = reqwest::Client::new();
    let data_url = format!("{}/api/data", state.resource_url);
    let mut steps: Vec<FlowStep> = Vec::new();

    // Step 1: Unauthenticated request → expect 402
    let resp = match http.get(&data_url).send().await {
        Ok(r) => r,
        Err(e) => {
            steps.push(FlowStep {
                step: 1,
                title: "Request Resource".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": format!("Failed to reach resource server: {}", e) }),
            });
            return (StatusCode::OK, Json(steps));
        }
    };

    let status_code = resp.status().as_u16();
    let payment_required_b64 = resp
        .headers()
        .get(X_PAYMENT_REQUIRED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if status_code != 402 {
        steps.push(FlowStep {
            step: 1,
            title: "Request Resource".into(),
            status: "error".into(),
            details: serde_json::json!({ "error": format!("Expected 402, got {}", status_code) }),
        });
        return (StatusCode::OK, Json(steps));
    }

    let payment_required_b64 = match payment_required_b64 {
        Some(v) => v,
        None => {
            steps.push(FlowStep {
                step: 1,
                title: "Request Resource".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": "Missing X-PAYMENT-REQUIRED header" }),
            });
            return (StatusCode::OK, Json(steps));
        }
    };

    let payment_required: PaymentRequired = match STANDARD
        .decode(&payment_required_b64)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(pr) => pr,
        None => {
            steps.push(FlowStep {
                step: 1,
                title: "Request Resource".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": "Failed to decode payment requirements" }),
            });
            return (StatusCode::OK, Json(steps));
        }
    };

    steps.push(FlowStep {
        step: 1,
        title: "Request Resource".into(),
        status: "success".into(),
        details: serde_json::json!({
            "response": "402 Payment Required",
            "amount": payment_required.amount,
            "pay_to": payment_required.pay_to,
            "scheme": payment_required.scheme,
            "network": payment_required.network,
        }),
    });

    // Step 2: Sign authorization
    let authorization = match crate::client::sign_authorization(
        &state.secret_key,
        &state.public_key,
        &payment_required,
    ) {
        Ok(auth) => auth,
        Err(e) => {
            steps.push(FlowStep {
                step: 2,
                title: "Sign Authorization".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": format!("{}", e) }),
            });
            return (StatusCode::OK, Json(steps));
        }
    };

    steps.push(FlowStep {
        step: 2,
        title: "Sign Authorization".into(),
        status: "success".into(),
        details: serde_json::json!({
            "from": authorization.from,
            "to": authorization.to,
            "amount": authorization.amount,
            "valid_after": authorization.valid_after,
            "valid_before": authorization.valid_before,
            "nonce": authorization.nonce,
        }),
    });

    // Build payment payload (used for both verify and pay steps)
    let payload = PaymentPayload {
        x402_version: payment_required.x402_version,
        scheme: payment_required.scheme.clone(),
        network: payment_required.network.clone(),
        asset: payment_required.asset.clone(),
        authorization,
    };

    // Step 3: Verify payment with facilitator
    let verify_url = format!("{}/verify", state.facilitator_url);
    let verify_req = VerifyRequest {
        payment_payload: payload.clone(),
        payment_requirements: payment_required.clone(),
    };

    let verify_result = http.post(&verify_url).json(&verify_req).send().await;

    match verify_result {
        Ok(resp) => match resp.json::<VerifyResponse>().await {
            Ok(verify_resp) => {
                if verify_resp.is_valid {
                    steps.push(FlowStep {
                        step: 3,
                        title: "Verify Payment".into(),
                        status: "success".into(),
                        details: serde_json::json!({
                            "is_valid": true,
                            "payer": verify_resp.payer,
                        }),
                    });
                } else {
                    steps.push(FlowStep {
                        step: 3,
                        title: "Verify Payment".into(),
                        status: "error".into(),
                        details: serde_json::json!({
                            "is_valid": false,
                            "reason": verify_resp.invalid_reason,
                        }),
                    });
                    return (StatusCode::OK, Json(steps));
                }
            }
            Err(e) => {
                steps.push(FlowStep {
                    step: 3,
                    title: "Verify Payment".into(),
                    status: "error".into(),
                    details: serde_json::json!({ "error": format!("Invalid verify response: {}", e) }),
                });
                return (StatusCode::OK, Json(steps));
            }
        },
        Err(e) => {
            steps.push(FlowStep {
                step: 3,
                title: "Verify Payment".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": format!("Facilitator unreachable: {}", e) }),
            });
            return (StatusCode::OK, Json(steps));
        }
    }

    // Step 4: Pay & Access Resource
    let payload_json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => {
            steps.push(FlowStep {
                step: 4,
                title: "Pay & Access Resource".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": format!("Serialization failed: {}", e) }),
            });
            return (StatusCode::OK, Json(steps));
        }
    };
    let payment_header = STANDARD.encode(payload_json.as_bytes());

    let resp = match http
        .get(&data_url)
        .header("x-payment", &payment_header)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            steps.push(FlowStep {
                step: 4,
                title: "Pay & Access Resource".into(),
                status: "error".into(),
                details: serde_json::json!({ "error": format!("Request failed: {}", e) }),
            });
            return (StatusCode::OK, Json(steps));
        }
    };

    let final_status = resp.status().as_u16();
    let tx_hash = resp
        .headers()
        .get("x-payment-response")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("(none)")
        .to_string();
    let body = resp.text().await.unwrap_or_default();

    if final_status == 200 {
        steps.push(FlowStep {
            step: 4,
            title: "Pay & Access Resource".into(),
            status: "success".into(),
            details: serde_json::json!({
                "status": final_status,
                "transaction": tx_hash,
                "body": body,
            }),
        });
    } else {
        steps.push(FlowStep {
            step: 4,
            title: "Pay & Access Resource".into(),
            status: "error".into(),
            details: serde_json::json!({
                "status": final_status,
                "body": body,
            }),
        });
    }

    (StatusCode::OK, Json(steps))
}

const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>x402 Demo — Casper Payment Flow</title>
<style>
  *, *::before, *::after { box-sizing: border-box; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background: #0f1117;
    color: #e0e0e0;
    margin: 0;
    padding: 2rem;
    min-height: 100vh;
  }
  .container { max-width: 720px; margin: 0 auto; }
  h1 { color: #fff; font-size: 1.6rem; margin-bottom: 0.25rem; }
  .subtitle { color: #888; font-size: 0.95rem; margin-bottom: 2rem; }
  button#run-btn {
    background: #6366f1;
    color: #fff;
    border: none;
    padding: 0.75rem 2rem;
    font-size: 1rem;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.2s;
  }
  button#run-btn:hover { background: #4f46e5; }
  button#run-btn:disabled { background: #444; cursor: not-allowed; }
  #timeline { margin-top: 2rem; }
  .step {
    border-left: 3px solid #333;
    padding: 0.75rem 1rem 0.75rem 1.25rem;
    margin-bottom: 0.5rem;
    border-radius: 0 8px 8px 0;
    background: #1a1d27;
    animation: fadeIn 0.3s ease;
  }
  .step.success { border-left-color: #22c55e; }
  .step.error   { border-left-color: #ef4444; }
  .step-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.35rem;
  }
  .step-number {
    background: #333;
    color: #ccc;
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    font-weight: 600;
  }
  .step.success .step-number { background: #166534; color: #bbf7d0; }
  .step.error   .step-number { background: #7f1d1d; color: #fca5a5; }
  .step-title { font-weight: 600; font-size: 0.95rem; }
  .step-details {
    font-size: 0.82rem;
    color: #aaa;
    background: #12141c;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .spinner {
    display: inline-block;
    width: 18px; height: 18px;
    border: 2px solid #555;
    border-top-color: #6366f1;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    vertical-align: middle;
    margin-right: 0.5rem;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
</style>
</head>
<body>
<div class="container">
  <h1>x402 Payment Protocol Demo</h1>
  <p class="subtitle">Casper blockchain &middot; HTTP 402 micropayments</p>
  <button id="run-btn">Pay &amp; Access Resource</button>
  <div id="timeline"></div>
</div>
<script>
const btn = document.getElementById('run-btn');
const timeline = document.getElementById('timeline');

btn.addEventListener('click', async () => {
  btn.disabled = true;
  timeline.innerHTML = '<div><span class="spinner"></span> Running payment flow&hellip;</div>';

  try {
    const res = await fetch('/api/run-flow', { method: 'POST' });
    const steps = await res.json();
    timeline.innerHTML = '';
    for (const s of steps) {
      const div = document.createElement('div');
      div.className = 'step ' + s.status;
      div.innerHTML =
        '<div class="step-header">' +
          '<span class="step-number">Step ' + s.step + '</span>' +
          '<span class="step-title">' + escapeHtml(s.title) + '</span>' +
        '</div>' +
        '<div class="step-details">' + escapeHtml(JSON.stringify(s.details, null, 2)) + '</div>';
      timeline.appendChild(div);
    }
  } catch (err) {
    timeline.innerHTML = '<div class="step error"><div class="step-header"><span class="step-number">!</span><span class="step-title">Network Error</span></div><div class="step-details">' + escapeHtml(err.message) + '</div></div>';
  } finally {
    btn.disabled = false;
  }
});

function escapeHtml(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}
</script>
</body>
</html>
"##;
