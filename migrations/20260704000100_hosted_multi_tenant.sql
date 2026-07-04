-- Hosted multi-tenant support: lifecycle state and per-installation dashboard access.

ALTER TABLE github_installations
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMPTZ;

ALTER TABLE github_repositories
    ADD COLUMN IF NOT EXISTS active BOOLEAN NOT NULL DEFAULT true;

CREATE INDEX IF NOT EXISTS idx_github_repositories_active ON github_repositories(active);

CREATE TABLE IF NOT EXISTS installation_admins (
    id BIGSERIAL PRIMARY KEY,
    installation_id BIGINT NOT NULL REFERENCES github_installations(installation_id) ON DELETE CASCADE,
    github_user_id BIGINT NOT NULL,
    login TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'admin',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(installation_id, github_user_id)
);
CREATE INDEX IF NOT EXISTS idx_installation_admins_user ON installation_admins(github_user_id);
CREATE INDEX IF NOT EXISTS idx_installation_admins_installation ON installation_admins(installation_id);
