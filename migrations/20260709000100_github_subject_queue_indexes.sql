CREATE TABLE IF NOT EXISTS observed_github_subjects (
    repository_id BIGINT NOT NULL REFERENCES github_repositories(repository_id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('issue','pull_request')),
    subject_id TEXT NOT NULL, github_user_id BIGINT NOT NULL, login TEXT NOT NULL,
    html_url TEXT NOT NULL, head_sha TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (repository_id, subject_type, subject_id)
);
CREATE INDEX IF NOT EXISTS idx_observed_github_subjects_repo_user ON observed_github_subjects(repository_id, github_user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_backfill_subject_run_queued ON jobs (((payload->>'backfill_run_id')::bigint), run_at, id) WHERE kind='backfill_subject' AND status='queued';
CREATE INDEX IF NOT EXISTS idx_jobs_propagation_subject_run_active ON jobs (((payload->>'propagation_run_id')::bigint)) WHERE kind='propagation_subject' AND status IN ('queued','running');
CREATE INDEX IF NOT EXISTS idx_jobs_claim_queued_priority ON jobs (priority, run_at, id) WHERE status='queued';
