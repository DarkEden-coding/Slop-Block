use jsonwebtoken::{Algorithm, EncodingKey, Header};
use time::{Duration, OffsetDateTime};

use crate::error::GitHubError;
use crate::types::JwtClaims;

pub fn create_app_jwt(app_id: impl ToString, private_key_pem: &str) -> Result<String, GitHubError> {
    create_app_jwt_at(app_id, private_key_pem, OffsetDateTime::now_utc())
}

pub fn create_app_jwt_at(
    app_id: impl ToString,
    private_key_pem: &str,
    now: OffsetDateTime,
) -> Result<String, GitHubError> {
    let claims = JwtClaims {
        iat: (now - Duration::seconds(60)).unix_timestamp(),
        exp: (now + Duration::minutes(9)).unix_timestamp(),
        iss: app_id.to_string(),
    };
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| GitHubError::Jwt(e.to_string()))?;
    jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|e| GitHubError::Jwt(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::JwtClaims;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    #[test]
    fn creates_decodable_app_jwt() {
        let pem = include_str!("../tests/fixtures/test_rsa.pem");
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let jwt = create_app_jwt_at("12345", pem, now).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header["alg"], "RS256");
        let claims: JwtClaims =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims.iss, "12345");
        assert_eq!(claims.iat, 1_699_999_940);
        assert_eq!(claims.exp, 1_700_000_540);
        assert!(!parts[2].is_empty());
    }
}
