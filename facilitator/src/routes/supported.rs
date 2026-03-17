use axum::Json;
use serde_json::{json, Value};

pub async fn handle_settle() -> Json<Value> {
    let asset = std::env::var("X402_TOKEN_ADDRESS").unwrap();
    Json(json!({
        "kinds" : [
            {
                "x402Version": 2,
                "scheme": "exact",
                "network": "casper:nctl",
                "asset": asset,
                "extra": {
                    "feePayer": "...",
                    "decimals": 9,
                    "symbol": "CSPR",
                    "name": "Casper",
                    "version": "2"
                }
            }
        ],
        "extensions" : [],
        "signers": {
            "casper:*": []
        }
    }))
}
