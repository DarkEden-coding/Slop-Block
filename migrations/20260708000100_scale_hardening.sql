-- Scale hardening: retention helpers and claim-path indexes.

CREATE INDEX IF NOT EXISTS idx_jobs_completed_at
    ON jobs(completed_at)
    WHERE status = 'completed';

CREATE INDEX IF NOT EXISTS idx_webhook_events_processed_at
    ON webhook_events(processed_at)
    WHERE processed_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_audit_log_created_at
    ON audit_log(created_at);
