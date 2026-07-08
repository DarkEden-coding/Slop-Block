use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::GitHubError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebhookDelivery {
    pub id: String,
    pub event: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookHeaders {
    pub delivery: String,
    pub event: String,
    pub signature_sha256: String,
}

impl WebhookHeaders {
    pub fn new(
        delivery: impl Into<String>,
        event: impl Into<String>,
        signature_sha256: impl Into<String>,
    ) -> Self {
        Self {
            delivery: delivery.into(),
            event: event.into(),
            signature_sha256: signature_sha256.into(),
        }
    }
}

pub fn verify_webhook_signature(
    secret: &[u8],
    body: &[u8],
    signature_header: &str,
) -> Result<(), GitHubError> {
    let signature = signature_header
        .strip_prefix("sha256=")
        .ok_or(GitHubError::InvalidSignature)?;
    let provided = hex::decode(signature).map_err(|_| GitHubError::InvalidSignature)?;
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| GitHubError::InvalidSignature)?;
    mac.update(body);
    mac.verify_slice(&provided)
        .map_err(|_| GitHubError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    #[test]
    fn verifies_webhook_signature() {
        let secret = b"topsecret";
        let body = br#"{"action":"opened"}"#;
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_webhook_signature(secret, body, &sig).is_ok());
        assert!(verify_webhook_signature(secret, b"{}", &sig).is_err());
    }
}
