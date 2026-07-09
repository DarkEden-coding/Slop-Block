use base64::Engine as _;
use std::{env, net::SocketAddr};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub cookie_secure: bool,
    pub github_webhook_secret: Option<String>,
    pub github_app_id: Option<String>,
    pub github_private_key: Option<String>,
    pub github_web_url: String,
    pub github_api_base: String,
    pub github_oauth_client_id: Option<String>,
    pub github_oauth_client_secret: Option<String>,
    pub api_base_url: String,
    pub web_base_url: String,
    pub turnstile_secret: Option<String>,
    pub turnstile_site_key: Option<String>,
    pub hcaptcha_secret: Option<String>,
    pub hcaptcha_site_key: Option<String>,
    pub recaptcha_secret: Option<String>,
    pub recaptcha_site_key: Option<String>,
    pub turnstile_dev_bypass: bool,
    pub admin_api_token: Option<String>,
    pub admin_github_logins: Vec<String>,
    pub admin_session_cookie_name: String,
    pub admin_session_secret: Option<String>,
    pub secrets_encryption_key: Option<Vec<u8>>,
    pub trust_proxy_headers: bool,
    pub hosted_mode: bool,
    pub database_max_connections: u32,
    pub database_acquire_timeout_secs: u64,
    pub job_workers: usize,
    pub job_poll_interval_ms: u64,
    pub backfill_subject_delay_seconds: u64,
    pub github_http_timeout_secs: u64,
    pub github_http_connect_timeout_secs: u64,
    pub max_installation_concurrency: usize,
    pub github_content_max_per_minute: u32,
    pub github_content_max_per_hour: u32,
    pub retention_days: i64,
    pub dashboard_list_page_size: i64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_getter(|key| env::var(key).ok())
    }

    pub fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let host = get("API_HOST").unwrap_or_else(|| "127.0.0.1".to_string());
        if host.trim().is_empty() {
            return Err(ConfigError::Invalid("API_HOST must not be empty"));
        }

        let port = match get("API_PORT") {
            Some(value) => value
                .parse::<u16>()
                .map_err(|_| ConfigError::Invalid("API_PORT must be a valid u16"))?,
            None => 8080,
        };

        let database_url = get("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;
        if !(database_url.starts_with("postgres://") || database_url.starts_with("postgresql://")) {
            return Err(ConfigError::Invalid(
                "DATABASE_URL must be a postgres:// or postgresql:// URL",
            ));
        }

        let cors_allowed_origins = get("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|| "http://localhost:3000".to_string())
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if cors_allowed_origins.is_empty() {
            return Err(ConfigError::Invalid(
                "CORS_ALLOWED_ORIGINS must include at least one origin",
            ));
        }

        let cookie_secure = get("COOKIE_SECURE")
            .map(|value| parse_bool("COOKIE_SECURE", &value))
            .transpose()?
            .unwrap_or(true);
        let github_webhook_secret = get("GITHUB_WEBHOOK_SECRET").filter(|value| !value.is_empty());
        let github_app_id = get("GITHUB_APP_ID").filter(|value| !value.is_empty());
        let github_private_key = get("GITHUB_PRIVATE_KEY").filter(|value| !value.is_empty());
        let github_web_url = get("GITHUB_WEB_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "http://localhost:3000".into());
        let github_api_base = get("GITHUB_API_BASE")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://api.github.com".into());
        let github_oauth_client_id =
            get("GITHUB_OAUTH_CLIENT_ID").filter(|value| !value.is_empty());
        let github_oauth_client_secret =
            get("GITHUB_OAUTH_CLIENT_SECRET").filter(|value| !value.is_empty());
        let api_base_url = get("API_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("http://{host}:{port}"));
        let web_base_url = get("WEB_BASE_URL")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| github_web_url.clone());
        let turnstile_secret = get("TURNSTILE_SECRET").filter(|value| !value.is_empty());
        let turnstile_site_key = get("TURNSTILE_SITE_KEY")
            .or_else(|| get("NEXT_PUBLIC_TURNSTILE_SITE_KEY"))
            .filter(|value| !value.is_empty());
        let hcaptcha_secret = get("HCAPTCHA_SECRET").filter(|value| !value.is_empty());
        let hcaptcha_site_key = get("HCAPTCHA_SITE_KEY")
            .or_else(|| get("NEXT_PUBLIC_HCAPTCHA_SITE_KEY"))
            .filter(|value| !value.is_empty());
        let recaptcha_secret = get("RECAPTCHA_SECRET").filter(|value| !value.is_empty());
        let recaptcha_site_key = get("RECAPTCHA_SITE_KEY")
            .or_else(|| get("NEXT_PUBLIC_RECAPTCHA_SITE_KEY"))
            .filter(|value| !value.is_empty());
        let turnstile_dev_bypass = get("TURNSTILE_DEV_BYPASS")
            .map(|value| parse_bool("TURNSTILE_DEV_BYPASS", &value))
            .transpose()?
            .unwrap_or(false);
        if turnstile_dev_bypass && cookie_secure {
            return Err(ConfigError::Invalid(
                "TURNSTILE_DEV_BYPASS may only be enabled when COOKIE_SECURE=false",
            ));
        }
        let admin_api_token = get("ADMIN_API_TOKEN").filter(|value| !value.is_empty());
        if admin_api_token
            .as_ref()
            .is_some_and(|value| value.len() < 32)
        {
            return Err(ConfigError::Invalid(
                "ADMIN_API_TOKEN must be at least 32 characters when set",
            ));
        }
        let admin_github_logins = get("ADMIN_GITHUB_LOGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|login| !login.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let admin_session_cookie_name =
            get("ADMIN_SESSION_COOKIE_NAME").unwrap_or_else(|| "gho_admin_session".into());
        if admin_session_cookie_name.trim().is_empty() || admin_session_cookie_name.contains(';') {
            return Err(ConfigError::Invalid("ADMIN_SESSION_COOKIE_NAME is invalid"));
        }
        let admin_session_secret = get("ADMIN_SESSION_SECRET").filter(|value| !value.is_empty());
        if admin_session_secret
            .as_ref()
            .is_some_and(|value| value.len() < 32)
        {
            return Err(ConfigError::Invalid(
                "ADMIN_SESSION_SECRET must be at least 32 characters when set",
            ));
        }

        let trust_proxy_headers = get("TRUST_PROXY_HEADERS")
            .map(|value| parse_bool("TRUST_PROXY_HEADERS", &value))
            .transpose()?
            .unwrap_or(false);
        let hosted_mode = get("HOSTED_MODE")
            .map(|value| parse_bool("HOSTED_MODE", &value))
            .transpose()?
            .unwrap_or(false);
        let database_max_connections = parse_u32(
            "DATABASE_MAX_CONNECTIONS",
            get("DATABASE_MAX_CONNECTIONS").as_deref(),
            20,
        )?;
        let database_acquire_timeout_secs = parse_u64(
            "DATABASE_ACQUIRE_TIMEOUT_SECS",
            get("DATABASE_ACQUIRE_TIMEOUT_SECS").as_deref(),
            30,
        )?;
        let job_workers = parse_usize("JOB_WORKERS", get("JOB_WORKERS").as_deref(), 4)?.max(1);
        let job_poll_interval_ms = parse_u64(
            "JOB_POLL_INTERVAL_MS",
            get("JOB_POLL_INTERVAL_MS").as_deref(),
            1000,
        )?;
        let backfill_subject_delay_seconds = parse_u64(
            "BACKFILL_SUBJECT_DELAY_SECONDS",
            get("BACKFILL_SUBJECT_DELAY_SECONDS").as_deref(),
            22,
        )?
        .max(1);
        let github_http_timeout_secs = parse_u64(
            "GITHUB_HTTP_TIMEOUT_SECS",
            get("GITHUB_HTTP_TIMEOUT_SECS").as_deref(),
            30,
        )?
        .max(1);
        let github_http_connect_timeout_secs = parse_u64(
            "GITHUB_HTTP_CONNECT_TIMEOUT_SECS",
            get("GITHUB_HTTP_CONNECT_TIMEOUT_SECS").as_deref(),
            10,
        )?
        .max(1);
        let max_installation_concurrency = parse_usize(
            "MAX_INSTALLATION_CONCURRENCY",
            get("MAX_INSTALLATION_CONCURRENCY").as_deref(),
            1,
        )?
        .max(1);
        let github_content_max_per_minute = parse_u32(
            "GITHUB_CONTENT_MAX_PER_MINUTE",
            get("GITHUB_CONTENT_MAX_PER_MINUTE").as_deref(),
            72,
        )?
        .max(1);
        let github_content_max_per_hour = parse_u32(
            "GITHUB_CONTENT_MAX_PER_HOUR",
            get("GITHUB_CONTENT_MAX_PER_HOUR").as_deref(),
            450,
        )?
        .max(1);
        let retention_days =
            parse_i64("RETENTION_DAYS", get("RETENTION_DAYS").as_deref(), 14)?.max(1);
        let dashboard_list_page_size = parse_i64(
            "DASHBOARD_LIST_PAGE_SIZE",
            get("DASHBOARD_LIST_PAGE_SIZE").as_deref(),
            100,
        )?
        .clamp(1, 500);
        let secrets_encryption_key = match get("SECRETS_ENCRYPTION_KEY").filter(|v| !v.is_empty()) {
            Some(raw) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(raw.trim())
                    .map_err(|_| {
                        ConfigError::Invalid("SECRETS_ENCRYPTION_KEY must be base64-encoded")
                    })?;
                if bytes.len() != 32 {
                    return Err(ConfigError::Invalid(
                        "SECRETS_ENCRYPTION_KEY must decode to exactly 32 bytes",
                    ));
                }
                Some(bytes)
            }
            None => None,
        };

        if hosted_mode && secrets_encryption_key.is_none() {
            return Err(ConfigError::Invalid(
                "SECRETS_ENCRYPTION_KEY is required when HOSTED_MODE=true to encrypt OAuth tokens",
            ));
        }

        let oauth_configured =
            github_oauth_client_id.is_some() && github_oauth_client_secret.is_some();
        if oauth_configured {
            if admin_session_secret.is_none() {
                return Err(ConfigError::Invalid(
                    "ADMIN_SESSION_SECRET is required when GITHUB_OAUTH_CLIENT_ID/SECRET are set",
                ));
            }
            if !hosted_mode && admin_github_logins.is_empty() {
                return Err(ConfigError::Invalid(
                    "ADMIN_GITHUB_LOGINS must list at least one login when GitHub OAuth is set unless HOSTED_MODE=true",
                ));
            }
        }

        Ok(Self {
            host,
            port,
            database_url,
            cors_allowed_origins,
            cookie_secure,
            github_webhook_secret,
            github_app_id,
            github_private_key,
            github_web_url,
            github_api_base,
            github_oauth_client_id,
            github_oauth_client_secret,
            api_base_url,
            web_base_url,
            turnstile_secret,
            turnstile_site_key,
            hcaptcha_secret,
            hcaptcha_site_key,
            recaptcha_secret,
            recaptcha_site_key,
            turnstile_dev_bypass,
            admin_api_token,
            admin_github_logins,
            admin_session_cookie_name,
            admin_session_secret,
            secrets_encryption_key,
            trust_proxy_headers,
            hosted_mode,
            database_max_connections,
            database_acquire_timeout_secs,
            job_workers,
            job_poll_interval_ms,
            backfill_subject_delay_seconds,
            github_http_timeout_secs,
            github_http_connect_timeout_secs,
            max_installation_concurrency,
            github_content_max_per_minute,
            github_content_max_per_hour,
            retention_days,
            dashboard_list_page_size,
        })
    }

    #[cfg(test)]
    pub fn test_fixture() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            database_url: "postgres://user:pass@localhost/db".into(),
            cors_allowed_origins: vec!["http://localhost:3000".into()],
            cookie_secure: false,
            github_webhook_secret: None,
            github_app_id: None,
            github_private_key: None,
            github_web_url: "http://localhost:3000".into(),
            github_api_base: "https://api.github.com".into(),
            github_oauth_client_id: None,
            github_oauth_client_secret: None,
            api_base_url: "http://127.0.0.1:8080".into(),
            web_base_url: "http://localhost:3000".into(),
            turnstile_secret: None,
            turnstile_site_key: None,
            hcaptcha_secret: None,
            hcaptcha_site_key: None,
            recaptcha_secret: None,
            recaptcha_site_key: None,
            turnstile_dev_bypass: false,
            admin_api_token: None,
            admin_github_logins: vec![],
            admin_session_cookie_name: "gho_admin_session".into(),
            admin_session_secret: None,
            secrets_encryption_key: None,
            trust_proxy_headers: false,
            hosted_mode: false,
            database_max_connections: 20,
            database_acquire_timeout_secs: 30,
            job_workers: 4,
            job_poll_interval_ms: 1000,
            backfill_subject_delay_seconds: 22,
            github_http_timeout_secs: 30,
            github_http_connect_timeout_secs: 10,
            max_installation_concurrency: 1,
            github_content_max_per_minute: 72,
            github_content_max_per_hour: 450,
            retention_days: 14,
            dashboard_list_page_size: 100,
        }
    }

    pub fn addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|_| ConfigError::Invalid("API_HOST/API_PORT do not form a socket address"))
    }
}

fn parse_bool(name: &'static str, value: &str) -> Result<bool, ConfigError> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::InvalidBool(name)),
    }
}

fn parse_u32(name: &'static str, value: Option<&str>, default: u32) -> Result<u32, ConfigError> {
    match value {
        Some(raw) => raw.parse::<u32>().map_err(|_| ConfigError::Invalid(name)),
        None => Ok(default),
    }
}

fn parse_u64(name: &'static str, value: Option<&str>, default: u64) -> Result<u64, ConfigError> {
    match value {
        Some(raw) => raw.parse::<u64>().map_err(|_| ConfigError::Invalid(name)),
        None => Ok(default),
    }
}

fn parse_usize(
    name: &'static str,
    value: Option<&str>,
    default: usize,
) -> Result<usize, ConfigError> {
    match value {
        Some(raw) => raw.parse::<usize>().map_err(|_| ConfigError::Invalid(name)),
        None => Ok(default),
    }
}

fn parse_i64(name: &'static str, value: Option<&str>, default: i64) -> Result<i64, ConfigError> {
    match value {
        Some(raw) => raw.parse::<i64>().map_err(|_| ConfigError::Invalid(name)),
        None => Ok(default),
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(&'static str),
    #[error("{0} must be a boolean")]
    InvalidBool(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_with(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map = vars.iter().copied().collect::<HashMap<_, _>>();
        Config::from_getter(|key| map.get(key).map(|value| value.to_string()))
    }

    #[test]
    fn parses_config_defaults() {
        let cfg = config_with(&[("DATABASE_URL", "postgres://user:pass@localhost/db")]).unwrap();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8080);
        assert!(cfg.cookie_secure);
    }

    #[test]
    fn rejects_missing_database_url() {
        assert!(matches!(
            config_with(&[]),
            Err(ConfigError::Missing("DATABASE_URL"))
        ));
    }

    #[test]
    fn rejects_turnstile_dev_bypass_with_secure_cookies() {
        assert!(matches!(
            config_with(&[
                ("DATABASE_URL", "postgres://user:pass@localhost/db"),
                ("TURNSTILE_DEV_BYPASS", "true")
            ]),
            Err(ConfigError::Invalid(_))
        ));
    }
}
