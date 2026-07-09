use base64::{engine::general_purpose::STANDARD, Engine as _};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit, Nonce};
use rand::{rngs::OsRng, RngCore};

use crate::Config;

const VERSION_PREFIX: &str = "v1";

/// Encrypts a sensitive field for at-rest storage using the configured encryption key.
///
/// Returns an error string when no `SECRETS_ENCRYPTION_KEY` is configured so callers can
/// refuse to persist plaintext secrets.
pub fn encrypt_field(config: &Config, plaintext: &str) -> Result<String, String> {
    let key = config.secrets_encryption_key.as_deref().ok_or_else(|| {
        "SECRETS_ENCRYPTION_KEY must be set to store provider secrets".to_string()
    })?;
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| "SECRETS_ENCRYPTION_KEY is not a valid 32-byte key".to_string())?;
    let mut nonce_bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| "failed to encrypt provider secret".to_string())?;
    Ok(format!(
        "{VERSION_PREFIX}.{}.{}",
        STANDARD.encode(nonce_bytes),
        STANDARD.encode(ciphertext)
    ))
}

/// Decrypts a stored field. Values without the version prefix are treated as legacy plaintext
/// and returned unchanged. Returns `None` when an encrypted value cannot be decrypted.
pub fn decrypt_field(config: &Config, value: &str) -> Option<String> {
    let Some(rest) = value.strip_prefix(&format!("{VERSION_PREFIX}.")) else {
        tracing::warn!(
            "stored provider secret is legacy plaintext; re-save it via the dashboard so it is encrypted at rest"
        );
        return Some(value.to_string());
    };
    let (nonce_b64, ct_b64) = rest.split_once('.')?;
    let key = config.secrets_encryption_key.as_deref()?;
    let cipher = ChaCha20Poly1305::new_from_slice(key).ok()?;
    let nonce_bytes = STANDARD.decode(nonce_b64).ok()?;
    let ciphertext = STANDARD.decode(ct_b64).ok()?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .ok()?;
    String::from_utf8(plaintext).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_key(key: Option<Vec<u8>>) -> Config {
        let mut config = Config::test_fixture();
        config.secrets_encryption_key = key;
        config
    }

    #[test]
    fn round_trips_encrypted_secret() {
        let config = config_with_key(Some(vec![7_u8; 32]));
        let encrypted = encrypt_field(&config, "super-secret").unwrap();
        assert!(encrypted.starts_with("v1."));
        assert_ne!(encrypted, "super-secret");
        assert_eq!(
            decrypt_field(&config, &encrypted).as_deref(),
            Some("super-secret")
        );
    }

    #[test]
    fn treats_unprefixed_values_as_legacy_plaintext() {
        let config = config_with_key(Some(vec![7_u8; 32]));
        assert_eq!(decrypt_field(&config, "legacy").as_deref(), Some("legacy"));
    }

    #[test]
    fn refuses_to_encrypt_without_key() {
        let config = config_with_key(None);
        assert!(encrypt_field(&config, "secret").is_err());
    }

    #[test]
    fn fails_decryption_with_wrong_key() {
        let encrypted = encrypt_field(&config_with_key(Some(vec![1_u8; 32])), "secret").unwrap();
        assert!(decrypt_field(&config_with_key(Some(vec![2_u8; 32])), &encrypted).is_none());
    }
}
