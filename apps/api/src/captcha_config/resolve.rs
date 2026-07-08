use std::sync::Arc;

use captcha::{
    CaptchaProvider, CloudflareTurnstile, DevBypass, GoogleRecaptchaV2, HCaptcha,
    PROVIDER_CLOUDFLARE_TURNSTILE, PROVIDER_DEV_BYPASS, PROVIDER_GOOGLE_RECAPTCHA_V2,
    PROVIDER_HCAPTCHA,
};
use serde_json::Value;

use super::types::{CaptchaProviderInfo, CredentialsSource, PROVIDER_CATALOG};
use crate::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderCredentials {
    pub site_key: String,
    pub secret: String,
    pub source: CredentialsSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedProvider {
    pub id: String,
    pub label: String,
    pub credentials: Option<ProviderCredentials>,
}

fn stored_field(stored: Option<&Value>, provider_id: &str, field: &str) -> Option<String> {
    stored?
        .get("providers")?
        .get(provider_id)?
        .get(field)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn env_field(config: &Config, provider_id: &str, field: &str) -> Option<String> {
    let value = match (provider_id, field) {
        (PROVIDER_CLOUDFLARE_TURNSTILE, "site_key") => config.turnstile_site_key.clone(),
        (PROVIDER_CLOUDFLARE_TURNSTILE, "secret") => config.turnstile_secret.clone(),
        (PROVIDER_HCAPTCHA, "site_key") => config.hcaptcha_site_key.clone(),
        (PROVIDER_HCAPTCHA, "secret") => config.hcaptcha_secret.clone(),
        (PROVIDER_GOOGLE_RECAPTCHA_V2, "site_key") => config.recaptcha_site_key.clone(),
        (PROVIDER_GOOGLE_RECAPTCHA_V2, "secret") => config.recaptcha_secret.clone(),
        _ => return None,
    }?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub(crate) fn resolve_site_key(
    config: &Config,
    stored: Option<&Value>,
    provider_id: &str,
) -> Option<String> {
    stored_field(stored, provider_id, "site_key")
        .or_else(|| env_field(config, provider_id, "site_key"))
}

fn resolve_secret(config: &Config, stored: Option<&Value>, provider_id: &str) -> Option<String> {
    stored_field(stored, provider_id, "secret")
        .and_then(|value| crate::secret_box::decrypt_field(config, &value))
        .or_else(|| env_field(config, provider_id, "secret"))
}

pub(crate) fn resolve_credentials(
    config: &Config,
    stored: Option<&Value>,
    provider_id: &str,
) -> Option<ProviderCredentials> {
    let site_key = resolve_site_key(config, stored, provider_id)?;
    let secret = resolve_secret(config, stored, provider_id)?;
    let source = if stored_field(stored, provider_id, "site_key").is_some()
        || stored_field(stored, provider_id, "secret").is_some()
    {
        CredentialsSource::Dashboard
    } else {
        CredentialsSource::Environment
    };
    Some(ProviderCredentials {
        site_key,
        secret,
        source,
    })
}

pub(crate) fn resolve_providers(config: &Config, stored: Option<&Value>) -> Vec<ResolvedProvider> {
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

pub(crate) fn configured_provider_ids(config: &Config, stored: Option<&Value>) -> Vec<String> {
    resolve_providers(config, stored)
        .into_iter()
        .filter(|provider| provider.credentials.is_some())
        .map(|provider| provider.id)
        .collect()
}

pub(crate) fn provider_infos(config: &Config, stored: Option<&Value>) -> Vec<CaptchaProviderInfo> {
    resolve_providers(config, stored)
        .into_iter()
        .map(|provider| {
            let credentials = provider.credentials.as_ref();
            CaptchaProviderInfo {
                id: provider.id,
                label: provider.label,
                site_key: credentials
                    .and_then(|value| (!value.site_key.is_empty()).then(|| value.site_key.clone())),
                configured: credentials.is_some(),
                secret_set: credentials.is_some(),
                source: credentials.map(|value| value.source.clone()),
            }
        })
        .collect()
}

pub fn hostname_allowed(config: &Config, reported: &str) -> bool {
    match url_host(&config.web_base_url) {
        Some(expected) => expected.eq_ignore_ascii_case(reported.trim()),
        None => true,
    }
}

fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
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
