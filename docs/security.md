# Security

GitHub Human Auth is designed for self-hosting. There is no hosted paid dependency required to operate the current MVP: PostgreSQL, the Rust API, the Next.js web app, GitHub App integration, and Cloudflare Turnstile can all be run or configured directly by the operator.

## Private repository data posture

The service should be treated as sensitive infrastructure because it receives GitHub webhook payloads and stores verification state. For private repositories:

- Store only the minimum repository, issue/PR, user, policy, session, and allowlist data required for verification and auditability.
- Do not mirror repository contents.
- Do not log private issue/PR bodies, secrets, OAuth tokens, private keys, or webhook payloads at debug level in production.
- Restrict dashboard/API access to trusted maintainers and private networks where possible.
- Keep PostgreSQL encrypted at rest using the host/cloud provider storage controls.
- Limit GitHub App installation to repositories that actually need verification.

## Least privilege

Configure the GitHub App with the minimal permissions listed in [GitHub App setup](github-app-setup.md). Install it on selected repositories rather than an entire organization unless organization-wide coverage is intentional.

Operational least privilege:

- Run containers as non-root where your platform supports it.
- Do not expose PostgreSQL to the public internet.
- Restrict inbound traffic to the web and API ports only.
- Use separate development, staging, and production GitHub Apps and databases.
- Rotate credentials when team members leave or logs/secrets may have been exposed.

## Secrets

Protect these as production secrets:

- `GITHUB_PRIVATE_KEY`
- `GITHUB_WEBHOOK_SECRET`
- `GITHUB_OAUTH_CLIENT_SECRET`
- `TURNSTILE_SECRET`
- `DATABASE_URL` / database password

Do not commit `.env`. Prefer Docker secrets, Kubernetes secrets, SOPS, 1Password/Secrets Automation, Vault, or your cloud secret manager.

## CAPTCHA and Turnstile

Cloudflare Turnstile is used to reduce bot verification. Configure a Turnstile site for your public web domain and set `TURNSTILE_SECRET` in the API.

`TURNSTILE_DEV_BYPASS=true` disables real CAPTCHA verification. It is only for local development and CI-style tests. Enabling it in production defeats a core control and lets automated clients pass the CAPTCHA step.

## Transport and cookies

- Serve production traffic over HTTPS.
- Set `COOKIE_SECURE=true` in production.
- Set `CORS_ALLOWED_ORIGINS` to exact trusted origins; do not use broad wildcards.
- Keep OAuth callback and webhook URLs on HTTPS public endpoints.

## Webhook validation

The API validates GitHub webhook signatures using `GITHUB_WEBHOOK_SECRET`. Use a high-entropy secret and rotate it if delivery logs, environment files, or deployment systems are exposed.

## Backups and restore security

Backups contain private repository metadata and verification history. Encrypt backups, restrict access, and test restore procedures regularly. Delete old backups according to your retention policy and legal requirements.
