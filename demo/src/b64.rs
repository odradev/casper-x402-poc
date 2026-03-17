use axum::http::HeaderValue;
use base64::{engine::general_purpose::STANDARD, Engine};

pub trait B64EncodeHeader {
    fn b64_encoded_header(&self) -> Result<HeaderValue, String>;
}

impl<T> B64EncodeHeader for T
where
    T: serde::Serialize,
{
    fn b64_encoded_header(&self) -> Result<HeaderValue, String> {
        let json =
            serde_json::to_string(self).map_err(|e| format!("Serialization failed: {}", e))?;
        STANDARD
            .encode(json.as_bytes())
            .parse()
            .map_err(|_| "Could not parse as header value".to_string())
    }
}

pub trait B64DecodeHeader: Sized {
    fn from_b64_header(header: &HeaderValue) -> Result<Self, String>;
}

impl<T> B64DecodeHeader for T
where
    T: serde::de::DeserializeOwned,
{
    fn from_b64_header(header: &HeaderValue) -> Result<Self, String> {
        let bytes = STANDARD
            .decode(header.as_bytes())
            .map_err(|_| "Cannot base64-decode header".to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("Cannot parse JSON from header: {e}"))
    }
}
