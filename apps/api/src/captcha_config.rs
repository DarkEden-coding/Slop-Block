use std::collections::HashMap;
use std::sync::Arc;

use captcha::{
    CaptchaProvider, CloudflareTurnstile, DevBypass, GoogleRecaptchaV2, HCaptcha,
    PROVIDER_CLOUDFLARE_TURNSTILE, PROVIDER_DEV_BYPASS, PROVIDER_GOOGLE_RECAPTCHA_V2,
    PROVIDER_HCAPTCHA,
};
use policy::VerificationPolicy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{AppState, Config};

pub const SETTINGS_KEY: &str = "captcha";

const PROVIDER_CATALOG: [(&str, &str); 3] = [
    (PROVIDER_CLOUDFLARE_TURNSTILE, "Cloudflare Turnstile"),
    (PROVIDER_HCAPTCHA, "hCaptcha"),
    (PROVIDER_GOOGLE_RECAPTCHA_V2, "Google reCAPTCHA v2"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialsSource {
    Dashboard,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaProviderInfo {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_key: Option<String>,
    pub configured: bool,
    pub secret_set: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<CredentialsSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaSettings {
    pub available_providers: Vec<CaptchaProviderInfo>,
    pub enabled_providers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    pub dev_bypass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaPublicConfig {
    pub providers: Vec<CaptchaPublicProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    pub dev_bypass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaPublicProvider {
    pub id: String,
    pub label: String,
    pub site_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CaptchaProviderCredentialsUpdate {
    #[serde(default)]
    pub site_key: Option<String>,
    #[serde(default)]
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CaptchaSettingsUpdate {
    #[serde(default)]
    pub enabled_providers: Vec<String>,
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub providers: HashMap<String, CaptchaProviderCredentialsUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCaptchaConfig {
    pub provider: String,
    pub site_key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternate_providers: Vec<CaptchaPublicProvider>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderCredentials {
    site_key: String,
    secret: String,
    source: CredentialsSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedProvider {
    id: String,
    label: String,
    credentials: Option<ProviderCredentials>,
}

fn env_credentials(config: &Config, provider_id: &str) -> Option<ProviderCredentials> {
    let (site_key, secret) = match provider_id {
        PROVIDER_CLOUDFLARE_TURNSTILE => (
            config.turnstile_site_key.as_deref(),
            config.turnstile_secret.as_deref(),
        ),
        PROVIDER_HCAPTCHA => (
            config.hcaptcha_site_key.as_deref(),
            config.hcaptcha_secret.as_deref(),
        ),
        PROVIDER_GOOGLE_RECAPTCHA_V2 => (
            config.recaptcha_site_key.as_deref(),
            config.recaptcha_secret.as_deref(),
        ),
        _ => return None,
    };
    let site_key = site_key.filter(|value| !value.is_empty())?;
    let secret = secret.filter(|value| !value.is_empty())?;
    Some(ProviderCredentials {
        site_key: site_key.to_string(),
        secret: secret.to_string(),
        source: CredentialsSource::Environment,
    })
}

fn stored_provider_credentials(stored: Option<&Value>, provider_id: &str) -> Option<ProviderCredentials> {
    let provider = stored?
        .get("providers")?
        .get(provider_id)?;
    let site_key = provider
        .get("site_key")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())?;
    let secret = provider
        .get("secret")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())?;
    Some(ProviderCredentials {
        site_key: site_key.to_string(),
        secret: secret.to_string(),
        source: CredentialsSource::Dashboard,
    })
}

fn resolve_credentials(config: &Config, stored: Option<&Value>, provider_id: &str) -> Option<ProviderCredentials> {
    stored_provider_credentials(stored, provider_id).or_else(|| env_credentials(config, provider_id))
}

fn resolve_providers(config: &Config, stored: Option<&Value>) -> Vec<ResolvedProvider> {
    let mut providers = PROVIDER_CATALOG
        .iter()
        .map(|(id, label)| ResolvedProvider {
            id: (*id).to_string(),
            label: (*label).to_string(),
            credentials: resolve_credentials(config, stored, id),
        })
        .collect::<Vec<_>>();
    if config.turnstile_dev_bypass {
        providers.push(ResolvedProvider {
            id: PROVIDER_DEV_BYPASS.into(),
            label: "Development bypass".into(),
            credentials: Some(ProviderCredentials {
                site_key: String::new(),
                secret: String::new(),
                source: CredentialsSource::Environment,
            }),
        });
    }
    providers
}

fn configured_provider_ids(config: &Config, stored: Option<&Value>) -> Vec<String> {
    resolve_providers(config, stored)
        .into_iter()
        .filter(|provider| provider.credentials.is_some())
        .map(|provider| provider.id)
        .collect()
}

fn provider_infos(config: &Config, stored: Option<&Value>) -> Vec<CaptchaProviderInfo> {
    resolve_providers(config, stored)
        .into_iter()
        .map(|provider| {
            let credentials = provider.credentials.as_ref();
            CaptchaProviderInfo {
                id: provider.id,
                label: provider.label,
                site_key: credentials.map(|value| value.site_key.clone()).filter(|value| !value.is_empty()),
                configured: credentials.is_some(),
                secret_set: credentials.is_some(),
                source: credentials.map(|value| value.source.clone()),
            }
        })
        .collect()
}

pub fn parse_stored_preferences(
    stored: Option<&Value>,
    configured_ids: &[String],
) -> (Vec<String>, Option<String>) {
    let Some(value) = stored else {
        return (
            configured_ids.to_vec(),
            configured_ids.first().cloned(),
        );
    };
    let enabled = value
        .get("enabled_providers")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .filter(|id| configured_ids.contains(id))
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| configured_ids.to_vec());
    let default_provider = value
        .get("default_provider")
        .and_then(|v| v.as_str())
        .filter(|id| enabled.contains(&id.to_string()))
        .map(str::to_string)
        .or_else(|| enabled.first().cloned());
    (enabled, default_provider)
}

pub async fn load_settings(state: &AppState) -> CaptchaSettings {
    let stored = match state.db.as_ref() {
        Some(pool) => db::get_app_setting(pool, SETTINGS_KEY).await.ok().flatten(),
        None => None,
    };
    let configured_ids = configured_provider_ids(&state.config, stored.as_ref());
    let (enabled_providers, default_provider) =
        parse_stored_preferences(stored.as_ref(), &configured_ids);
    CaptchaSettings {
        available_providers: provider_infos(&state.config, stored.as_ref()),
        enabled_providers,
        default_provider,
        dev_bypass: state.config.turnstile_dev_bypass,
    }
}

pub fn public_config(settings: &CaptchaSettings) -> CaptchaPublicConfig {
    let providers: Vec<CaptchaPublicProvider> = settings
        .available_providers
        .iter()
        .filter(|provider| {
            settings.enabled_providers.contains(&provider.id)
                && provider.site_key.as_ref().is_some_and(|key| !key.is_empty())
        })
        .map(|provider| CaptchaPublicProvider {
            id: provider.id.clone(),
            label: provider.label.clone(),
            site_key: provider.site_key.clone().unwrap_or_default(),
        })
        .collect();
    let default_provider = settings
        .default_provider
        .clone()
        .filter(|id| providers.iter().any(|provider| &provider.id == id));
    CaptchaPublicConfig {
        providers,
        default_provider,
        dev_bypass: settings.dev_bypass,
    }
}

pub fn resolve_provider_id(
    settings: &CaptchaSettings,
    policy: &VerificationPolicy,
) -> Option<String> {
    if let Some(provider) = policy.captcha_provider.as_ref() {
        if settings.enabled_providers.contains(provider) {
            return Some(provider.clone());
        }
    }
    settings.default_provider.clone()
}

pub fn session_captcha_config(
    settings: &CaptchaSettings,
    provider_id: &str,
) -> Option<SessionCaptchaConfig> {
    if settings.dev_bypass && provider_id == PROVIDER_DEV_BYPASS {
        return Some(SessionCaptchaConfig {
            provider: PROVIDER_DEV_BYPASS.into(),
            site_key: String::new(),
            label: "Development bypass".into(),
            alternate_providers: Vec::new(),
        });
    }
    let provider = settings.available_providers.iter().find(|p| p.id == provider_id)?;
    let site_key = provider.site_key.clone().filter(|key| !key.is_empty())?;
    let alternate_providers = public_config(settings)
        .providers
        .into_iter()
        .filter(|p| p.id != provider_id)
        .collect();
    Some(SessionCaptchaConfig {
        provider: provider.id.clone(),
        site_key,
        label: provider.label.clone(),
        alternate_providers,
    })
}

pub async fn verify_token(
    config: &Arc<Config>,
    stored: Option<&Value>,
    provider_id: &str,
    token: &str,
) -> Result<captcha::CaptchaVerification, captcha::CaptchaError> {
    if config.turnstile_dev_bypass && provider_id == PROVIDER_DEV_BYPASS {
        return DevBypass::new(true).verify(token, None).await;
    }
    let credentials = resolve_credentials(config, stored, provider_id)
        .ok_or(captcha::CaptchaError::NotConfigured)?;
    match provider_id {
        PROVIDER_CLOUDFLARE_TURNSTILE => {
            CloudflareTurnstile::new(credentials.secret)
                .verify(token, None)
                .await
        }
        PROVIDER_HCAPTCHA => HCaptcha::new(credentials.secret).verify(token, None).await,
        PROVIDER_GOOGLE_RECAPTCHA_V2 => {
            GoogleRecaptchaV2::new(credentials.secret)
                .verify(token, None)
                .await
        }
        _ => Err(captcha::CaptchaError::NotConfigured),
    }
}

pub fn merge_settings_update(
    existing: Option<Value>,
    update: &CaptchaSettingsUpdate,
    config: &Config,
) -> Result<Value, String> {
    let mut value = existing.unwrap_or_else(|| json!({}));
    let providers = value
        .as_object_mut()
        .ok_or_else(|| "stored captcha settings are invalid".to_string())?
        .entry("providers")
        .or_insert_with(|| json!({}));
    let provider_map = providers
        .as_object_mut()
        .ok_or_else(|| "stored captcha provider settings are invalid".to_string())?;

    for (provider_id, credentials) in &update.providers {
        if !PROVIDER_CATALOG.iter().any(|(id, _)| *id == provider_id.as_str()) {
            continue;
        }
        let entry = provider_map
            .entry(provider_id.clone())
            .or_insert_with(|| json!({}));
        let entry_obj = entry
            .as_object_mut()
            .ok_or_else(|| format!("stored credentials for {provider_id} are invalid"))?;
        if let Some(site_key) = credentials
            .site_key
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            entry_obj.insert("site_key".into(), json!(site_key));
        }
        if let Some(secret) = credentials
            .secret
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            entry_obj.insert("secret".into(), json!(secret));
        }
    }

    let configured_ids = configured_provider_ids(config, Some(&value));
    let (enabled_providers, default_provider) = if update.enabled_providers.is_empty()
        && update.default_provider.is_none()
    {
        parse_stored_preferences(Some(&value), &configured_ids)
    } else {
        validate_preferences_update(update, &configured_ids)?
    };

    let root = value.as_object_mut().unwrap();
    root.insert("enabled_providers".into(), json!(enabled_providers));
    root.insert(
        "default_provider".into(),
        default_provider.map(Value::String).unwrap_or(Value::Null),
    );
    Ok(value)
}

fn validate_preferences_update(
    update: &CaptchaSettingsUpdate,
    configured_ids: &[String],
) -> Result<(Vec<String>, Option<String>), String> {
    let enabled: Vec<String> = update
        .enabled_providers
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .filter(|id| configured_ids.contains(id))
        .collect();
    if enabled.is_empty() {
        return Err("Select at least one configured CAPTCHA provider".into());
    }
    let default_provider = update
        .default_provider
        .as_ref()
        .map(|id| id.trim().to_string())
        .filter(|id| enabled.contains(id))
        .or_else(|| enabled.first().cloned());
    Ok((enabled, default_provider))
}

pub fn validate_settings_update(
    update: &CaptchaSettingsUpdate,
    config: &Config,
    stored: Option<&Value>,
) -> Result<(), String> {
    let merged = merge_settings_update(stored.map(Clone::clone), update, config)?;
    let configured_ids = configured_provider_ids(config, Some(&merged));
    let (enabled, _) = parse_stored_preferences(Some(&merged), &configured_ids);
    if enabled.is_empty() {
        return Err("Configure at least one CAPTCHA provider with a site key and secret".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> Config {
        Config {
            host: "127.0.0.1".into(),
            port: 8080,
            database_url: "postgres://user:pass@localhost/db".into(),
            cors_allowed_origins: vec!["http://localhost:3000".into()],
            cookie_secure: false,
            session_cookie_name: "gho_session".into(),
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
        }
    }

    #[test]
    fn merges_dashboard_credentials_without_overwriting_existing_secret() {
        let config = test_config();
        let existing = json!({
            "providers": {
                "cloudflare-turnstile": {
                    "site_key": "site-old",
                    "secret": "secret-old"
                }
            }
        });
        let update = CaptchaSettingsUpdate {
            enabled_providers: vec![PROVIDER_CLOUDFLARE_TURNSTILE.into()],
            default_provider: Some(PROVIDER_CLOUDFLARE_TURNSTILE.into()),
            providers: HashMap::from([(
                PROVIDER_CLOUDFLARE_TURNSTILE.into(),
                CaptchaProviderCredentialsUpdate {
                    site_key: Some("site-new".into()),
                    secret: None,
                },
            )]),
        };
        let merged = merge_settings_update(Some(existing), &update, &config).unwrap();
        let provider = &merged["providers"][PROVIDER_CLOUDFLARE_TURNSTILE];
        assert_eq!(provider["site_key"], "site-new");
        assert_eq!(provider["secret"], "secret-old");
    }
}
