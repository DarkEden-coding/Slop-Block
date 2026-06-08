-- MVP database schema for GitHub Human Auth

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE github_installations (
    id BIGSERIAL PRIMARY KEY,
    installation_id BIGINT NOT NULL UNIQUE,
    account_login TEXT NOT NULL,
    account_id BIGINT,
    account_type TEXT,
    access_token TEXT,
    access_token_expires_at TIMESTAMPTZ,
    raw JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_github_installations_account_login ON github_installations(account_login);

CREATE TABLE github_repositories (
    id BIGSERIAL PRIMARY KEY,
    repository_id BIGINT NOT NULL UNIQUE,
    installation_id BIGINT NOT NULL REFERENCES github_installations(installation_id) ON DELETE CASCADE,
    owner TEXT NOT NULL,
    name TEXT NOT NULL,
    full_name TEXT NOT NULL UNIQUE,
    private BOOLEAN NOT NULL DEFAULT false,
    default_branch TEXT,
    raw JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(owner, name)
);
CREATE INDEX idx_github_repositories_installation_id ON github_repositories(installation_id);
CREATE INDEX idx_github_repositories_owner ON github_repositories(owner);

CREATE TABLE repository_policies (
    id BIGSERIAL PRIMARY KEY,
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repository_id)
);

CREATE TABLE github_users (
    id BIGSERIAL PRIMARY KEY,
    github_user_id BIGINT NOT NULL UNIQUE,
    login TEXT NOT NULL UNIQUE,
    avatar_url TEXT,
    raw JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE trusted_subjects (
    id BIGSERIAL PRIMARY KEY,
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    github_user_id BIGINT REFERENCES github_users(github_user_id) ON DELETE SET NULL,
    trusted BOOLEAN NOT NULL DEFAULT true,
    trusted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repository_id, subject_type, subject_id)
);
CREATE INDEX idx_trusted_subjects_repo_trusted ON trusted_subjects(repository_id, trusted);
CREATE INDEX idx_trusted_subjects_subject ON trusted_subjects(subject_type, subject_id);

CREATE TABLE verification_sessions (
    id BIGSERIAL PRIMARY KEY,
    public_id UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    github_user_id BIGINT REFERENCES github_users(github_user_id) ON DELETE SET NULL,
    token_hash TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending',
    challenge_provider TEXT,
    completed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('pending','completed','expired','failed'))
);
CREATE INDEX idx_verification_sessions_public_id_token ON verification_sessions(public_id, token_hash);
CREATE INDEX idx_verification_sessions_repo_subject ON verification_sessions(repository_id, subject_type, subject_id);

CREATE TABLE bot_artifacts (
    id BIGSERIAL PRIMARY KEY,
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    external_id TEXT,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repository_id, subject_type, subject_id, artifact_type)
);
CREATE INDEX idx_bot_artifacts_external_id ON bot_artifacts(external_id);

CREATE TABLE webhook_events (
    id BIGSERIAL PRIMARY KEY,
    delivery_id TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL,
    installation_id BIGINT,
    repository_id BIGINT,
    payload JSONB NOT NULL,
    processed_at TIMESTAMPTZ,
    processing_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_webhook_events_unprocessed ON webhook_events(created_at) WHERE processed_at IS NULL;
CREATE INDEX idx_webhook_events_repo ON webhook_events(repository_id);

CREATE TABLE jobs (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'queued',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 5,
    run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_by TEXT,
    locked_at TIMESTAMPTZ,
    last_error TEXT,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('queued','running','completed','failed'))
);
CREATE INDEX idx_jobs_claim ON jobs(status, run_at, id);
CREATE INDEX idx_jobs_kind ON jobs(kind);

CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    actor TEXT,
    action TEXT NOT NULL,
    repository_id BIGINT,
    subject_type TEXT,
    subject_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_log_repo_created ON audit_log(repository_id, created_at DESC);
CREATE INDEX idx_audit_log_action ON audit_log(action);
