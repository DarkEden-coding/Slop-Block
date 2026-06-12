-- Installation access tokens are minted on demand and never persisted. Drop the unused
-- columns so a stored GitHub credential can never be extracted from the database.
ALTER TABLE github_installations DROP COLUMN IF EXISTS access_token;
ALTER TABLE github_installations DROP COLUMN IF EXISTS access_token_expires_at;
