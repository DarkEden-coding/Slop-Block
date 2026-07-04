-- Store encrypted GitHub OAuth user tokens for hosted-mode permission checks.
CREATE TABLE IF NOT EXISTS dashboard_oauth_tokens (
    github_user_id BIGINT PRIMARY KEY,
    login TEXT NOT NULL,
    access_token_encrypted TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_dashboard_oauth_tokens_login ON dashboard_oauth_tokens(login);

CREATE INDEX IF NOT EXISTS idx_github_installations_not_deleted ON github_installations(account_login) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_github_repositories_installation_active ON github_repositories(installation_id, active, full_name);
