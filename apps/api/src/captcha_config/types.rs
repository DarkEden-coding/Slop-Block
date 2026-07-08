use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const SETTINGS_KEY: &str = "captcha";

pub(crate) const PROVIDER_CATALOG: [(&str, &str); 3] = [
    (
        captcha::PROVIDER_CLOUDFLARE_TURNSTILE,
        "Cloudflare Turnstile",
    ),
    (captcha::PROVIDER_HCAPTCHA, "hCaptcha"),
    (captcha::PROVIDER_GOOGLE_RECAPTCHA_V2, "Google reCAPTCHA v2"),
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
