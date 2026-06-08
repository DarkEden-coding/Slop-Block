use std::sync::Arc;

use captcha::{
    CaptchaProvider, CloudflareTurnstile, DevBypass, GoogleRecaptchaV2, HCaptcha,
    PROVIDER_CLOUDFLARE_TURNSTILE, PROVIDER_DEV_BYPASS, PROVIDER_GOOGLE_RECAPTCHA_V2,
    PROVIDER_HCAPTCHA,
};
use policy::VerificationPolicy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppState, Config};

pub const SETTINGS_KEY: &str = "captcha";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaProviderInfo {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_key: Option<String>,
    pub configured: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaptchaSettingsUpdate {
    pub enabled_providers: Vec<String>,
    #[serde(default)]
    pub default_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCaptchaConfig {
    pub provider: String,
    pub site_key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternate_providers: Vec<CaptchaPublicProvider>,
}

pub fn available_providers(config: &Config) -> Vec<CaptchaProviderInfo> {
    let mut providers = Vec::new();
    if config.turnstile_dev_bypass {
        providers.push(CaptchaProviderInfo {
            id: PROVIDER_DEV_BYPASS.into(),
            label: "Development bypass".into(),
            site_key: None,
            configured: true,
        });
    }
    providers.push(CaptchaProviderInfo {
        id: PROVIDER_CLOUDFLARE_TURNSTILE.into(),
        label: "Cloudflare Turnstile".into(),
        site_key: config.turnstile_site_key.clone(),
        configured: config.turnstile_secret.is_some(),
    });
    providers.push(CaptchaProviderInfo {
        id: PROVIDER_HCAPTCHA.into(),
        label: "hCaptcha".into(),
        site_key: config.hcaptcha_site_key.clone(),
        configured: config.hcaptcha_secret.is_some(),
    });
    providers.push(CaptchaProviderInfo {
        id: PROVIDER_GOOGLE_RECAPTCHA_V2.into(),
        label: "Google reCAPTCHA v2".into(),
        site_key: config.recaptcha_site_key.clone(),
        configured: config.recaptcha_secret.is_some(),
    });
    providers
        .into_iter()
        .filter(|provider| provider.configured)
        .collect()
}

pub fn default_enabled_ids(config: &Config) -> Vec<String> {
    available_providers(config)
        .into_iter()
        .map(|provider| provider.id)
        .collect()
}

pub fn parse_stored_settings(value: Option<Value>, config: &Config) -> (Vec<String>, Option<String>) {
    let available_ids = default_enabled_ids(config);
    let Some(value) = value else {
        return (
            available_ids.clone(),
            available_ids.first().cloned(),
        );
    };
    let enabled = value
        .get("enabled_providers")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .filter(|id| available_ids.contains(id))
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| available_ids.clone());
    let default_provider = value
        .get("default_provider")
        .and_then(|v| v.as_str())
        .filter(|id| enabled.contains(&id.to_string()))
        .map(str::to_string)
        .or_else(|| enabled.first().cloned());
    (enabled, default_provider)
}

pub async fn load_settings(state: &AppState) -> CaptchaSettings {
    let available_providers = available_providers(&state.config);
    let stored = match state.db.as_ref() {
        Some(pool) => db::get_app_setting(pool, SETTINGS_KEY).await.ok().flatten(),
        None => None,
    };
    let (enabled_providers, default_provider) =
        parse_stored_settings(stored, &state.config);
    CaptchaSettings {
        available_providers,
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
    provider_id: &str,
    token: &str,
) -> Result<captcha::CaptchaVerification, captcha::CaptchaError> {
    if config.turnstile_dev_bypass && provider_id == PROVIDER_DEV_BYPASS {
        return DevBypass::new(true).verify(token, None).await;
    }
    match provider_id {
        PROVIDER_CLOUDFLARE_TURNSTILE => {
            let secret = config
                .turnstile_secret
                .as_ref()
                .ok_or(captcha::CaptchaError::NotConfigured)?;
            CloudflareTurnstile::new(secret).verify(token, None).await
        }
        PROVIDER_HCAPTCHA => {
            let secret = config
                .hcaptcha_secret
                .as_ref()
                .ok_or(captcha::CaptchaError::NotConfigured)?;
            HCaptcha::new(secret).verify(token, None).await
        }
        PROVIDER_GOOGLE_RECAPTCHA_V2 => {
            let secret = config
                .recaptcha_secret
                .as_ref()
                .ok_or(captcha::CaptchaError::NotConfigured)?;
            GoogleRecaptchaV2::new(secret).verify(token, None).await
        }
        _ => Err(captcha::CaptchaError::NotConfigured),
    }
}

pub fn validate_settings_update(
    update: &CaptchaSettingsUpdate,
    config: &Config,
) -> Result<(Vec<String>, Option<String>), String> {
    let available = available_providers(config);
    let available_ids = available
        .iter()
        .map(|provider| provider.id.as_str())
        .collect::<Vec<_>>();
    let enabled: Vec<String> = update
        .enabled_providers
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .filter(|id| available_ids.contains(&id.as_str()))
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
