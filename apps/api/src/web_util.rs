use axum::http::{header, HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn find_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| {
            cookie
                .strip_prefix(&format!("{name}="))
                .map(ToOwned::to_owned)
        })
}

pub fn random_state() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn sign_hmac_url_safe(secret: &str, message: &[u8]) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(message);
    Some(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

pub fn sign_source_payload(
    secret: &str,
    repository_id: i64,
    subject_type: &str,
    number: u64,
    github_user_id: i64,
    login: &str,
    subject_url: &str,
) -> Option<String> {
    let payload =
        format!("{repository_id}|{subject_type}|{number}|{github_user_id}|{login}|{subject_url}");
    sign_hmac_url_safe(secret, payload.as_bytes())
}
