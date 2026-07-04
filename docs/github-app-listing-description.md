# GitHub App listing descriptions

## Listing description, max 125 characters

Human verification for GitHub issues and PRs using OAuth, CAPTCHA, labels, and maintainer-controlled policies.

## Introductory description, max 500 characters

**Agent Block** helps maintainers reduce issue and pull request spam by requiring unknown contributors to verify they are human before their work is prioritized. It labels unverified issues and PRs, posts a verification link, and uses GitHub OAuth plus CAPTCHA to confirm the contributor controls their GitHub account. Maintainers can configure repository policies, trusted users, and verification behavior from a hosted dashboard.

## Detailed description, 400-2,000 characters

**Agent Block** is a hosted GitHub App for maintainers who want a lightweight human-verification gate for issues and pull requests.

When an unknown contributor opens an issue or PR, Agent Block can label it as needing verification and post a clear verification link. The contributor completes GitHub OAuth to prove they control the GitHub account that opened the issue or PR, then completes CAPTCHA verification. Once verified, Agent Block can update labels, remove pending guidance, and mark the contributor as trusted according to your repository policy.

### What it helps with

- Reducing bot-created issue and pull request spam
- Filtering suspicious or low-quality drive-by contributions
- Adding CAPTCHA-backed verification to GitHub workflows
- Giving real contributors a clear path to proceed
- Letting maintainers prioritize verified humans with less manual triage

### Maintainer controls

From the hosted dashboard, maintainers can configure repository-level policies, trusted contributor allowlists, verification labels, comment behavior, and issue/PR enforcement settings. Agent Block is designed to be transparent: contributors are not silently blocked, and maintainers stay in control of how strict verification should be for each repository.

Agent Block does not require users to self-host anything. Install the GitHub App on selected repositories, complete setup, and manage verification from the dashboard.

## Transparency disclosures

Agent Block is designed as a security and safety tool for repository maintainers. It uses GitHub webhook signature verification, GitHub OAuth, CAPTCHA verification, HTTPS-only production cookies, scoped dashboard authorization, and least-privilege GitHub App permissions to reduce abuse while keeping maintainers in control.

### Data collected and stored

Agent Block stores only the data needed to operate verification and repository policy controls:

- GitHub App installation IDs, account login/type, repository IDs, repository full names, visibility, and default branch
- GitHub user IDs, logins, and optional avatar URLs for dashboard users, verified contributors, and allowlisted users
- Repository verification policies, labels, trusted-user allowlists, verification session status, audit events, and bot-created GitHub artifact IDs such as comment/check-run IDs
- Encrypted dashboard OAuth access tokens for hosted-mode installation-access checks
- Encrypted CAPTCHA provider secrets when configured through the dashboard
- Trimmed webhook routing summaries such as event action, installation ID, repository ID, sender login, and issue/PR number

Agent Block does **not** mirror repository contents and does **not** persist issue or pull request titles, bodies, comments, diffs, source code, secrets, or full webhook payloads. Historical stored payloads are scrubbed by migration, and installation access tokens are minted on demand rather than stored.

### Security and safety controls

- Webhooks are validated with `X-Hub-Signature-256` before processing.
- Hosted dashboard access is scoped to GitHub App installations the signed-in user can access through GitHub OAuth.
- New repositories default to disabled until a maintainer configures policy.
- PostgreSQL is intended to remain private; the provided Compose setup binds it to loopback only.
- Sensitive stored fields use application-level encryption with `SECRETS_ENCRYPTION_KEY`.
- CAPTCHA development bypass is rejected when secure-cookie production posture is enabled.
- OAuth/CAPTCHA endpoints include IP-based rate limiting, with reverse-proxy rate limits recommended for public deployments.

### Compliance notes

Agent Block does not provide AI decision-making, biometric identification, social scoring, employment/education evaluation, law-enforcement risk scoring, or other high-risk AI system functionality. It performs deterministic repository workflow automation based on maintainer-configured policy, GitHub account control, CAPTCHA completion, and allowlist state.

No third-party compliance reports, SOC 2 report, ISO 27001 certificate, or formal EU AI Act conformity assessment are currently offered. Operators should review their own deployment, data retention, backup encryption, access controls, and legal obligations before using Agent Block for regulated repositories.
