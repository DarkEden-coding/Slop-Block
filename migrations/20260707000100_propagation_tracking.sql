-- Track verification propagation across open issues/PRs for maintainer visibility.

CREATE TABLE IF NOT EXISTS propagation_runs (
    id BIGSERIAL PRIMARY KEY,
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    github_user_id BIGINT,
    login TEXT,
    session_public_id UUID,
    status TEXT NOT NULL DEFAULT 'running',
    total_subjects INTEGER NOT NULL DEFAULT 0,
    processed_subjects INTEGER NOT NULL DEFAULT 0,
    current_subject_type TEXT,
    current_subject_id TEXT,
    last_error TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('running', 'completed', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_propagation_runs_repo_status
    ON propagation_runs(repository_id, status, started_at DESC);
