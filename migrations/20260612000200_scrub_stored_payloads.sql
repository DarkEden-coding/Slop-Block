-- Historical webhook payloads and raw GitHub objects may contain private repository
-- content (issue/PR titles and bodies, repo descriptions). The application now stores
-- trimmed summaries only; scrub everything persisted before this change.
UPDATE webhook_events SET payload = '{}'::jsonb WHERE payload <> '{}'::jsonb;
UPDATE github_installations SET raw = '{}'::jsonb WHERE raw <> '{}'::jsonb;
UPDATE github_repositories SET raw = '{}'::jsonb WHERE raw <> '{}'::jsonb;
UPDATE github_users SET raw = jsonb_build_object('login', login) WHERE raw ?| array['email', 'name', 'company', 'location', 'bio'];
