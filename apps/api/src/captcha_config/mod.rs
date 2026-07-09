mod resolve;
mod types;

pub use resolve::{hostname_allowed, verify_token};
pub use types::*;

use policy::VerificationPolicy;
use serde_json::{json, Value};

use crate::{AppState, Config};
use resolve::{configured_provider_ids, provider_infos};

pub fn parse_stored_preferences(
    stored: Option<&Value>,
    configured_ids: &[String],
) -> (Vec<String>, Option<String>) {
    let Some(value) = stored else {
        return (configured_ids.to_vec(), configured_ids.first().cloned());
    };
    let enabled = value
        .get("enabled_providers")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .filter(|id| configured_ids.contains(&id.to_string()))
                .map(str::to_string)
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
                && provider
                    .site_key
                    .as_ref()
                    .is_some_and(|key| !key.is_empty())
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
    use captcha::PROVIDER_DEV_BYPASS;

    if settings.dev_bypass && provider_id == PROVIDER_DEV_BYPASS {
        return Some(SessionCaptchaConfig {
            provider: PROVIDER_DEV_BYPASS.into(),
            site_key: String::new(),
            label: "Development bypass".into(),
            alternate_providers: Vec::new(),
        });
    }
    let provider = settings
        .available_providers
        .iter()
        .find(|p| p.id == provider_id)?;
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
        if !types::PROVIDER_CATALOG
            .iter()
            .any(|(id, _)| *id == provider_id.as_str())
        {
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
            let encrypted = crate::secret_box::encrypt_field(config, &secret)?;
            entry_obj.insert("secret".into(), json!(encrypted));
        }
    }

    let configured_ids = configured_provider_ids(config, Some(&value));
    let (enabled_providers, default_provider) =
        if update.enabled_providers.is_empty() && update.default_provider.is_none() {
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
    let merged = merge_settings_update(stored.cloned(), update, config)?;
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
    use captcha::PROVIDER_CLOUDFLARE_TURNSTILE;
    use resolve::resolve_credentials;
    use serde_json::json;
    use std::collections::HashMap;

    fn test_config() -> Config {
        let mut config = Config::test_fixture();
        config.secrets_encryption_key = Some(vec![9_u8; 32]);
        config
    }

    #[test]
    fn validates_captcha_hostname_against_web_base_url() {
        let mut config = test_config();
        config.web_base_url = "https://app.example.com:8443/some/path".into();
        assert!(hostname_allowed(&config, "App.Example.Com"));
        assert!(!hostname_allowed(&config, "evil.example.net"));
    }

    #[test]
    fn merges_env_secret_with_dashboard_site_key() {
        let mut config = test_config();
        config.turnstile_secret = Some("env-secret".into());
        let merged = merge_settings_update(
            None,
            &CaptchaSettingsUpdate {
                enabled_providers: vec![PROVIDER_CLOUDFLARE_TURNSTILE.into()],
                default_provider: Some(PROVIDER_CLOUDFLARE_TURNSTILE.into()),
                providers: HashMap::from([(
                    PROVIDER_CLOUDFLARE_TURNSTILE.into(),
                    CaptchaProviderCredentialsUpdate {
                        site_key: Some("site-from-dashboard".into()),
                        secret: None,
                    },
                )]),
            },
            &config,
        )
        .unwrap();
        let credentials =
            resolve_credentials(&config, Some(&merged), PROVIDER_CLOUDFLARE_TURNSTILE).unwrap();
        assert_eq!(credentials.site_key, "site-from-dashboard");
        assert_eq!(credentials.secret, "env-secret");
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
