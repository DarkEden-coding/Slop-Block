use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::web_util::{constant_time_eq, sign_hmac_url_safe};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateCookie {
    pub session_id: Uuid,
    pub token_hash: String,
    pub state: String,
    pub session_token: String,
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn encode_state_cookie(
    secret: &str,
    session_id: Uuid,
    token_hash: &str,
    state: &str,
    session_token: &str,
) -> Option<String> {
    let payload = format!("{session_id}:{token_hash}:{state}:{session_token}");
    let sig = sign_hmac_url_safe(secret, payload.as_bytes())?;
    Some(format!("{payload}:{sig}"))
}

pub fn parse_state_cookie(value: &str, expected_state: &str, secret: &str) -> Option<StateCookie> {
    let (payload, sig) = value.rsplit_once(':')?;
    if !constant_time_eq(
        sign_hmac_url_safe(secret, payload.as_bytes())?.as_bytes(),
        sig.as_bytes(),
    ) {
        return None;
    }
    let mut parts = payload.splitn(4, ':');
    let session_id = parts.next()?.parse().ok()?;
    let token_hash = parts.next()?.to_string();
    let state = parts.next()?.to_string();
    let session_token = parts.next()?.to_string();
    if state != expected_state || token_hash.len() != 64 || session_token.is_empty() {
        return None;
    }
    Some(StateCookie {
        session_id,
        token_hash,
        state,
        session_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_cookie_round_trip_and_rejects_bad_state() {
        let id = Uuid::new_v4();
        let hash = token_hash("secret-token");
        let value =
            encode_state_cookie("signing-secret", id, &hash, "state-1", "session-token").unwrap();
        let parsed = parse_state_cookie(&value, "state-1", "signing-secret").unwrap();
        assert_eq!(parsed.session_id, id);
        assert_eq!(parsed.session_token, "session-token");
        assert!(parse_state_cookie(&value, "other", "signing-secret").is_none());
        assert!(parse_state_cookie(&value, "state-1", "wrong-secret").is_none());
    }

    #[test]
    fn token_hash_is_stable() {
        assert_eq!(token_hash("abc"), token_hash("abc"));
        assert_ne!(token_hash("abc"), token_hash("abcd"));
    }
}
