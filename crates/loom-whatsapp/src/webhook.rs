// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Webhook signature verification for WhatsApp.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{Result, WhatsAppError};

type HmacSha256 = Hmac<Sha256>;

/// Verify the X-Hub-Signature-256 header from a WhatsApp webhook.
///
/// # Arguments
///
/// * `app_secret` - The app secret configured in Meta Business
/// * `signature_header` - The X-Hub-Signature-256 header value (e.g., "sha256=abc123...")
/// * `body` - The raw request body bytes
///
/// # Returns
///
/// * `Ok(())` if the signature is valid
/// * `Err(WhatsAppError::InvalidWebhookSignature)` if invalid
///
/// # Security
///
/// This function uses constant-time comparison to prevent timing attacks.
pub fn verify_webhook_signature(
    app_secret: &str,
    signature_header: &str,
    body: &[u8],
) -> Result<()> {
    const SIGNATURE_PREFIX: &str = "sha256=";

    // Verify header format
    if !signature_header.starts_with(SIGNATURE_PREFIX) {
        tracing::warn!("Invalid signature header format: missing sha256= prefix");
        return Err(WhatsAppError::InvalidWebhookSignature);
    }

    // Decode expected signature from hex
    let expected_sig = hex::decode(&signature_header[SIGNATURE_PREFIX.len()..])
        .map_err(|_| WhatsAppError::InvalidWebhookSignature)?;

    // Compute HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
        .map_err(|e| WhatsAppError::Hmac(e.to_string()))?;
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected_sig)
        .map_err(|_| WhatsAppError::InvalidWebhookSignature)?;

    tracing::debug!("Webhook signature verified successfully");
    Ok(())
}

/// Hash a verify token for storage using SHA-256.
pub fn hash_verify_token(token: &str) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify a token against its hash.
pub fn verify_token_hash(token: &str, hash: &str) -> bool {
    let computed = hash_verify_token(token);
    // Use constant-time comparison
    constant_time_eq(computed.as_bytes(), hash.as_bytes())
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_webhook_signature_valid() {
        let app_secret = "my_app_secret";
        let body = b"test payload";

        // Compute expected signature
        let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={}", signature);

        assert!(verify_webhook_signature(app_secret, &header, body).is_ok());
    }

    #[test]
    fn test_verify_webhook_signature_invalid() {
        let app_secret = "my_app_secret";
        let body = b"test payload";
        let header = "sha256=invalid_signature";

        assert!(matches!(
            verify_webhook_signature(app_secret, header, body),
            Err(WhatsAppError::InvalidWebhookSignature)
        ));
    }

    #[test]
    fn test_verify_webhook_signature_missing_prefix() {
        let app_secret = "my_app_secret";
        let body = b"test payload";
        let header = "abc123";

        assert!(matches!(
            verify_webhook_signature(app_secret, header, body),
            Err(WhatsAppError::InvalidWebhookSignature)
        ));
    }

    #[test]
    fn test_hash_verify_token() {
        let token = "my_verify_token";
        let hash = hash_verify_token(token);

        assert!(!hash.is_empty());
        assert!(verify_token_hash(token, &hash));
        assert!(!verify_token_hash("wrong_token", &hash));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }
}
