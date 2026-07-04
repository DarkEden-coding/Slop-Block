# Hosted multi-tenant service

This mode lets you run one public deployment and let maintainers install your GitHub App into their repositories without self-hosting.

## Required environment

```env
HOSTED_MODE=true
GITHUB_APP_SLUG=<your-public-github-app-slug>
NEXT_PUBLIC_GITHUB_APP_SLUG=<your-public-github-app-slug>
GITHUB_APP_ID=<app-id>
GITHUB_PRIVATE_KEY=<app-private-key-pem>
GITHUB_WEBHOOK_SECRET=<webhook-secret>
GITHUB_OAUTH_CLIENT_ID=<oauth-client-id>
GITHUB_OAUTH_CLIENT_SECRET=<oauth-client-secret>
ADMIN_SESSION_SECRET=<random-32+-chars>
```

In hosted mode, GitHub OAuth dashboard login is allowed for any GitHub user, but API data is scoped to installations GitHub reports as accessible to that user's OAuth token. The service stores the OAuth token encrypted at rest with `SECRETS_ENCRYPTION_KEY` so setup callbacks can be verified before granting installation admin access. Bearer-token access remains global and should be kept private.

## GitHub App settings

Set the app as installable by any account, then configure:

- Homepage URL: `WEB_BASE_URL`
- Webhook URL: `API_BASE_URL/api/github/webhook`
- OAuth callback URL: `API_BASE_URL/api/github/oauth/callback`
- Setup URL: `WEB_BASE_URL/setup/github/install-complete`

Subscribe to:

- `installation`
- `installation_repositories`
- `issues`
- `issue_comment`
- `pull_request`
- `pull_request_review_comment`

## User flow

1. User opens `/install`.
2. User installs your GitHub App on selected repositories.
3. GitHub redirects to `/setup/github/install-complete?installation_id=...`.
4. User signs in with GitHub.
5. The app verifies the signed-in user can access that GitHub App installation, records them as an installation admin, and syncs repositories.
6. User configures repo policies in `/dashboard`.

## Resource usage notes

The hosted implementation keeps runtime usage low by:

- minting GitHub installation tokens only on demand, never storing them;
- storing only trimmed GitHub object metadata, not full webhook payloads;
- syncing repositories only when installation setup/sync is requested, on dashboard OAuth login, or webhooks arrive;
- filtering dashboard queries by indexed `installation_admins.github_user_id` and active repositories;
- defaulting new repositories to disabled until explicitly configured.

For a large public service, add external rate limiting at the reverse proxy and monitor webhook volume, database connections, and GitHub API rate limits.
