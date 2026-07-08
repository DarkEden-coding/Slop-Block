use github::GitHubApi;

use crate::AppState;

pub fn app_jwt(state: &AppState) -> anyhow::Result<String> {
    let app_id = state
        .config
        .github_app_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("github app id missing"))?;
    let private_key = state
        .config
        .github_private_key
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("github private key missing"))?;
    github::create_app_jwt(app_id, private_key).map_err(Into::into)
}

pub async fn installation_token(state: &AppState, installation_id: u64) -> anyhow::Result<String> {
    let jwt = app_jwt(state)?;
    let client = github::ReqwestGitHubClient::with_base_url(&state.config.github_api_base);
    Ok(client
        .exchange_installation_token(&jwt, installation_id)
        .await?
        .token)
}
