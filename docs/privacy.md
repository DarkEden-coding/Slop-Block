# Privacy Policy

_Last updated: 2026-07-04_

Agent Block is a GitHub App that helps repository maintainers require human verification for GitHub issues and pull requests. This policy describes what data the app processes and stores.

## Data we process

Agent Block receives GitHub webhook events and GitHub OAuth responses needed to operate verification flows. Depending on repository configuration, the app may process or store:

- GitHub App installation IDs and account metadata, such as account login, account type, and account ID
- Repository metadata, such as repository ID, owner/name, full name, visibility, and default branch
- GitHub user IDs, logins, and optional avatar URLs for dashboard users, verified contributors, and allowlisted users
- Verification session state, policy settings, trusted-user allowlists, labels, comment/check-run artifact IDs, and audit events
- Encrypted dashboard OAuth access tokens used to verify hosted-mode installation access
- Encrypted CAPTCHA provider secrets if configured through the dashboard
- Trimmed webhook routing summaries, such as event action, installation ID, repository ID, sender login, and issue or pull request number

## Data we do not intentionally store

Agent Block is designed to avoid storing sensitive repository content. It does not intentionally persist:

- Repository source code
- Issue or pull request bodies
- Issue or pull request titles
- Diffs, patches, or file contents
- Full webhook payloads
- GitHub installation access tokens
- Plaintext OAuth tokens or CAPTCHA secrets

Installation access tokens are minted on demand and are not stored. Stored OAuth tokens and dashboard-managed CAPTCHA secrets are encrypted at rest using the deployment's configured encryption key.

## Third-party services

Agent Block depends on:

- GitHub API, GitHub Apps, GitHub OAuth, and GitHub webhooks
- A configured CAPTCHA provider, such as Cloudflare Turnstile, hCaptcha, or Google reCAPTCHA
- PostgreSQL for application state
- Infrastructure services used by the hosted deployment, such as Cloudflare Tunnel/DNS for HTTPS routing

Data submitted to third-party services is subject to those services' own privacy policies.

## Cookies and sessions

Agent Block uses cookies for dashboard login sessions and contributor verification sessions. In production, cookies should be configured as secure and sent only over HTTPS.

## Data retention

Agent Block stores verification and policy state as long as needed to operate the installed repository configuration. Maintainers may remove trusted users, disable policies, or uninstall the GitHub App. Uninstall events mark installations and repositories inactive where supported.

Backups, if used by the operator, may retain data for longer according to the operator's backup retention policy.

## Security

Agent Block validates GitHub webhook signatures, uses GitHub OAuth for identity verification, supports CAPTCHA verification, encrypts sensitive stored secrets, and limits dashboard access to authorized installation administrators. Operators are responsible for securing their deployment environment, database, backups, and secrets.

## Contact

For privacy or security questions, contact the repository owner through the GitHub project:

https://github.com/DarkEden-coding/Slop-Block
