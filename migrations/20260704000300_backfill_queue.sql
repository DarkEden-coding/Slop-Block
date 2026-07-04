-- Backfill and queue scalability support.

ALTER TABLE jobs
    ADD COLUMN IF NOT EXISTS dedupe_key TEXT,
    ADD COLUMN IF NOT EXISTS priority INTEGER NOT NULL DEFAULT 100,
    ADD COLUMN IF NOT EXISTS available_after_rate_limit BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS rate_limit_reset_at TIMESTAMPTZ;

CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_dedupe_active
    ON jobs(dedupe_key)
    WHERE dedupe_key IS NOT NULL AND status IN ('queued', 'running');
CREATE INDEX IF NOT EXISTS idx_jobs_claim_priority ON jobs(status, run_at, priority, id);
CREATE INDEX IF NOT EXISTS idx_jobs_rate_limit_reset ON jobs(rate_limit_reset_at) WHERE rate_limit_reset_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS backfill_runs (
    id BIGSERIAL PRIMARY KEY,
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    requested_by_github_user_id BIGINT,
    requested_by_login TEXT,
    include_issues BOOLEAN NOT NULL DEFAULT true,
    include_pull_requests BOOLEAN NOT NULL DEFAULT true,
    notify_authors BOOLEAN NOT NULL DEFAULT true,
    force_new_comments BOOLEAN NOT NULL DEFAULT false,
    status TEXT NOT NULL DEFAULT 'queued',
    total_discovered INTEGER NOT NULL DEFAULT 0,
    total_enqueued INTEGER NOT NULL DEFAULT 0,
    total_processed INTEGER NOT NULL DEFAULT 0,
    total_succeeded INTEGER NOT NULL DEFAULT 0,
    total_failed INTEGER NOT NULL DEFAULT 0,
    total_skipped INTEGER NOT NULL DEFAULT 0,
    current_phase TEXT,
    last_error TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('queued','scanning','running','completed','failed','cancelled')),
    CHECK (current_phase IS NULL OR current_phase IN ('queued','scanning_issues','scanning_pull_requests','enqueuing','processing','finalizing'))
);
CREATE INDEX IF NOT EXISTS idx_backfill_runs_repo_created ON backfill_runs(repository_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_backfill_runs_status ON backfill_runs(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_backfill_runs_one_active_per_repo
    ON backfill_runs(repository_id)
    WHERE status IN ('queued','scanning','running');

CREATE TABLE IF NOT EXISTS backfill_items (
    id BIGSERIAL PRIMARY KEY,
    backfill_run_id BIGINT NOT NULL REFERENCES backfill_runs(id) ON DELETE CASCADE,
    repository_id BIGINT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    github_user_id BIGINT,
    login TEXT,
    html_url TEXT,
    head_sha TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    decision_required BOOLEAN,
    decision_reason TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    started_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(backfill_run_id, repository_id, subject_type, subject_id),
    CHECK (subject_type IN ('issue','pull_request')),
    CHECK (status IN ('queued','running','succeeded','failed','skipped'))
);
CREATE INDEX IF NOT EXISTS idx_backfill_items_run_status ON backfill_items(backfill_run_id, status);
CREATE INDEX IF NOT EXISTS idx_backfill_items_subject ON backfill_items(repository_id, subject_type, subject_id);
CREATE INDEX IF NOT EXISTS idx_backfill_items_run_created ON backfill_items(backfill_run_id, created_at);

CREATE TABLE IF NOT EXISTS github_rate_limits (
    bucket TEXT PRIMARY KEY,
    remaining INTEGER,
    reset_at TIMESTAMPTZ,
    paused_until TIMESTAMPTZ,
    last_status INTEGER,
    last_error TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
