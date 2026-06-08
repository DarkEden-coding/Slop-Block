# Open Source Build Plan

## Goal

Build the open source core first: a self-hostable GitHub App that verifies unknown issue/PR authors with GitHub OAuth + CAPTCHA, then updates labels, comments, and optional PR checks.

The OSS product should be useful by itself, easy to self-host, and structured so paid/closed modules can later add hosted operations, global trust, analytics, LLM detection, and third-party integrations.

This plan targets **Rust as the backend from the beginning** to avoid a difficult rewrite later and to optimize for safety, speed, low memory usage, and a strong self-hosting story.

## Recommended Initial Architecture

### Stack

Use Rust for backend/API/webhook processing and TypeScript for the dashboard frontend.

- **Backend/API**: Rust
- **Web framework**: Axum
- **Async runtime**: Tokio
- **GitHub API integration**: Octocrab plus small custom GitHub App auth helpers where needed
- **Webhook verification**: Rust HMAC/SHA-256 implementation using GitHub webhook signature headers
- **Dashboard/frontend**: Next.js + Tailwind CSS
- **Database**: PostgreSQL
- **Database access**: SQLx with compile-time checked queries, or SeaORM if a higher-level ORM is preferred
- **Migrations**: SQLx migrations or refinery
- **Queue/background jobs**: Start with database-backed jobs; add Redis only if needed
- **Self-hosting**: Docker Compose
- **CAPTCHA providers**: Rust provider abstraction with Turnstile/reCAPTCHA/hCaptcha implementations

### Why Rust backend first

Rust is a good fit because the OSS core should be:

- Efficient on small VPS/self-hosted installs
- Safe and reliable when handling webhooks, tokens, and secrets
- Easy to package as a small backend container
- Suitable for long-running background jobs and high-volume webhook processing
- A stable foundation that avoids a future TypeScript-to-Rust rewrite

The tradeoff is that GitHub App ecosystem support is less complete than Node/Octokit, so early phases should deliberately de-risk GitHub App authentication, webhook handling, and API permissions.

## Repository Layout

```txt
.
├── apps/
│   ├── api/                    # Rust Axum backend: API, webhooks, verification, jobs
│   └── web/                    # Next.js dashboard + contributor-facing pages
├── crates/
│   ├── github/                 # GitHub App auth, webhook parsing, API helpers
│   ├── captcha/                # CAPTCHA provider abstraction and implementations
│   ├── policy/                 # Verification policy engine
│   ├── db/                     # SQLx models, queries, migrations helpers
│   ├── jobs/                   # Background job types and runner
│   └── shared/                 # Shared Rust types/utilities
├── packages/
│   └── shared-ui/              # Optional shared frontend utilities/components later
├── migrations/                 # SQL migrations
├── docker/
│   ├── Dockerfile.api
│   ├── Dockerfile.web
│   └── entrypoint scripts
├── docs/
│   ├── self-hosting.md
│   ├── github-app-setup.md
│   ├── configuration.md
│   └── security.md
├── docker-compose.yml
├── README.md
├── PRODUCT_DISCOVERY.md
└── BUILD_PLAN.md
```

For the earliest MVP, `apps/api` should handle API routes, GitHub webhooks, OAuth callbacks, verification completion, and background job polling. Split workers into a separate process only when needed.

The frontend can either:

1. Serve dashboard and verification UI through Next.js, calling the Rust API, or
2. Be built into static assets and served by the Rust backend later.

Recommended MVP choice: keep Next.js separate for faster dashboard iteration, while keeping all sensitive logic in Rust.

## Core Rust Components

### `crates/github`

Responsibilities:

- Verify GitHub webhook signatures
- Parse webhook event types
- Generate GitHub App JWTs
- Exchange App JWTs for installation access tokens
- Wrap GitHub REST API calls needed by the product
- Add labels/remove labels
- Create/update issue comments
- Create/update PR checks or commit statuses
- Fetch collaborators/permissions for exemption checks

Important: validate GitHub App auth and installation-token behavior early, because this is the main ecosystem risk versus Node/Octokit.

### `crates/captcha`

Responsibilities:

- Define provider trait/interface
- Implement Cloudflare Turnstile first
- Add hCaptcha and Google reCAPTCHA later
- Normalize provider result into common success/failure/error type

Example trait shape:

```rust
#[async_trait::async_trait]
pub trait CaptchaProvider {
    async fn verify(&self, token: &str, remote_ip: Option<&str>) -> anyhow::Result<CaptchaVerification>;
}
```

### `crates/policy`

Responsibilities:

- Decide whether a GitHub actor needs verification
- Apply repo/org policy settings
- Handle exemptions:
  - collaborators
  - verified bots/apps
  - manual allowlist
  - previously verified users
  - optionally previous accepted contributors
- Return deterministic actions for the GitHub layer to execute

Policy should be mostly pure Rust logic and easy to unit test.

### `crates/db`

Responsibilities:

- SQLx query wrappers
- Data models
- Repository methods
- Migration helpers

Use explicit SQL and strongly typed models. This keeps behavior transparent and production-friendly.

### `crates/jobs`

Responsibilities:

- Database-backed job queue
- Retry/backoff
- Idempotency
- Auto-close jobs if enabled
- GitHub artifact update jobs

Start with a simple `jobs` table in Postgres rather than adding Redis on day one.

## Core Concepts

### Installation

Represents a GitHub App installation on an org/user account.

Stores:

- GitHub installation ID
- Account type: user/org
- Account login and ID
- Installation status
- Created/updated timestamps

### Repository

Represents a repository covered by an installation.

Stores:

- GitHub repository ID
- Owner/name
- Visibility
- Installation ID
- Enabled/disabled state

### Policy

Configurable verification behavior per repository or organization.

Settings include:

- Verify issues: yes/no
- Verify PRs: yes/no
- PR check mode: disabled, non-blocking, blocking-capable
- Required label name
- Verified label name
- Auto-close timeout: disabled by default
- Re-verification interval
- Exempt collaborators: default yes
- Exempt verified bots/apps: default yes
- Auto-trust previous contributors: optional
- CAPTCHA provider
- Comment cleanup behavior

### Verification Session

A short-lived verification flow created when the bot comments on an issue/PR.

Stores:

- Session ID
- Signed token hash
- GitHub user expected to verify
- Repo/issue/PR target
- Expiration
- Completion status
- CAPTCHA provider result
- OAuth GitHub user identity

### Trusted Subject

Represents a user trusted for a repository/org.

Stores:

- GitHub user ID/login
- Scope: repo/org
- Verification source: CAPTCHA, manual allowlist, collaborator exemption, previous contributor import
- Verified timestamp
- Expiration timestamp if configured
- Revoked timestamp if revoked

### Bot Artifact

Tracks comments, labels, and PR checks created by the app so they can be updated/removed later.

Stores:

- Issue/PR target
- Comment ID
- Labels applied
- Check run/status ID if applicable
- Last known state

## MVP User Flow

### Maintainer setup

1. Maintainer installs the GitHub App.
2. App receives installation webhook.
3. Maintainer opens dashboard.
4. Maintainer signs in with GitHub OAuth/admin auth.
5. Maintainer selects repositories.
6. Maintainer configures default policy or accepts defaults.
7. App confirms labels/check names that will be used.

### Issue/PR opened by unknown user

1. GitHub sends webhook: `issues.opened`, `pull_request.opened`, or `reopened`.
2. Rust backend verifies webhook signature and stores delivery ID for idempotency.
3. Backend loads repository policy.
4. Backend checks whether author is exempt:
   - collaborator/member
   - verified bot/app
   - allowlisted
   - already verified and not expired
5. If trusted, backend optionally applies `verified` label and exits.
6. If untrusted:
   - create verification session
   - add required label
   - post bot comment with verification link
   - for PRs, create pending/non-success check if enabled

### Contributor verification

1. Contributor clicks verification link.
2. Verification page explains what is happening and which GitHub account must verify.
3. Contributor signs in with GitHub OAuth.
4. Rust backend confirms OAuth user matches the issue/PR author.
5. Contributor completes CAPTCHA.
6. Rust backend verifies CAPTCHA server-side.
7. Backend stores trusted subject record.
8. Backend updates GitHub:
   - remove required label
   - add verified label
   - edit/hide/update bot comment where possible
   - mark PR check/status successful if enabled
9. Contributor sees success page with link back to issue/PR.

## Build Phases

## Phase 0: Rust Project Foundation

Deliverables:

- Cargo workspace
- Axum API app
- Next.js/Tailwind dashboard app
- Shared local development setup
- Docker Compose with Postgres
- SQLx migrations
- Environment variable validation in Rust
- Structured logging/tracing
- Basic test setup
- Initial README

Acceptance criteria:

- `docker compose up` starts Postgres, Rust API, and dashboard
- API health endpoint works
- Dashboard home page loads locally
- Database migrations run
- CI can format/lint/test/build Rust and frontend code

Recommended Rust crates:

- `axum`
- `tokio`
- `tower` / `tower-http`
- `serde` / `serde_json`
- `sqlx`
- `tracing` / `tracing-subscriber`
- `config` or `figment`
- `thiserror` / `anyhow`
- `jsonwebtoken`
- `hmac` / `sha2`
- `reqwest`
- `octocrab`
- `uuid`
- `time` or `chrono`

## Phase 1: GitHub App Skeleton

Deliverables:

- GitHub App registration documentation
- Rust webhook endpoint
- GitHub webhook signature verification
- Installation created/deleted/suspended webhook handling
- GitHub App JWT generation
- Installation access token exchange
- Minimal GitHub API client wrapper
- Basic dashboard view of installations/repositories

Acceptance criteria:

- A local/dev GitHub App can be installed on a test repo
- Webhook events are received and verified
- Installation/repository records are stored
- Backend can create an installation access token
- Backend can make at least one authenticated GitHub API call as the installation

This phase is the largest technical risk because Rust GitHub App tooling is less turnkey than Node. Complete it before building too much dashboard functionality.

## Phase 2: Policy Configuration

Deliverables:

- Default policy model in Rust
- Repository policy database schema
- Dashboard policy editor
- Repository enable/disable toggle
- Label/check-name configuration
- Policy engine crate with unit tests

Acceptance criteria:

- Maintainer can enable verification for a repo
- Maintainer can choose whether issues and/or PRs are verified
- Maintainer can save policy changes through the dashboard
- Policy engine can be tested without GitHub or database access

## Phase 3: Unverified Issue/PR Handling

Deliverables:

- Rust webhook handlers for issue opened/reopened events
- Rust webhook handlers for PR opened/reopened/synchronize events
- Exemption checks for collaborators and verified bots/apps
- Add/remove labels through GitHub API
- Create bot comment with verification link
- Optional PR check/status creation
- Bot artifact tracking
- Webhook idempotency by GitHub delivery ID

Acceptance criteria:

- Unknown issue author gets required label and comment
- Unknown PR author gets required label, comment, and optional pending check
- Collaborators and known bots are skipped by default
- Duplicate comments are avoided on repeated webhook delivery
- Webhook handlers are covered by unit/integration tests using mocked GitHub API responses

## Phase 4: Verification Flow

Deliverables:

- Verification session model
- Signed/unguessable verification links
- GitHub OAuth login flow handled by Rust backend
- CAPTCHA provider abstraction
- Cloudflare Turnstile implementation first
- Verification success/failure pages
- Trusted subject storage
- GitHub artifact cleanup after success

Acceptance criteria:

- Correct GitHub user can verify successfully
- Wrong GitHub user cannot verify someone else’s issue/PR
- Expired/invalid links fail safely
- CAPTCHA result is verified server-side
- Labels/comments/checks update after successful verification

## Phase 5: Self-Hosting Polish

Deliverables:

- Production Rust API Docker image
- Production web Docker image
- Docker Compose example
- `.env.example`
- Setup docs
- GitHub App creation guide
- CAPTCHA provider setup guide
- Backup/restore notes
- Healthcheck and readiness endpoints

Acceptance criteria:

- A maintainer can self-host on a VPS using Docker Compose
- Setup docs are complete enough for an external beta user
- No paid-hosted dependency is required for OSS core
- Rust backend image is reasonably small and production-ready

## Phase 6: Beta Hardening

Deliverables:

- Retry/backoff for GitHub API failures
- Database-backed background jobs
- Audit log for verification decisions
- Admin/manual allowlist UI
- Configurable auto-close job, disabled by default
- Data retention settings
- Basic security review
- Rate-limit handling
- Observability docs

Acceptance criteria:

- Webhook retries do not create duplicate comments/checks
- Maintainers can manually trust or revoke users
- Failed GitHub API calls are retried safely
- Sensitive secrets are not logged
- Project is ready for real public-repo beta testing

## First CAPTCHA Provider Recommendation

Use an abstraction from the start, but implement one provider first.

Recommended first provider: **Cloudflare Turnstile**.

Reasons:

- Often less annoying than traditional CAPTCHA
- Free tier is attractive for OSS maintainers
- Good privacy positioning
- Easy hosted and self-host configuration
- Simple server-side verification API

Then add:

- hCaptcha
- Google reCAPTCHA

## Suggested Data Model Sketch

Tables:

- `github_installations`
- `github_repositories`
- `repository_policies`
- `github_users`
- `trusted_subjects`
- `verification_sessions`
- `bot_artifacts`
- `webhook_events`
- `jobs`
- `audit_log`

Important indexes:

- GitHub installation ID
- GitHub repository ID
- GitHub user ID
- Verification session token hash
- Trusted subject by scope/user
- Webhook delivery ID for idempotency
- Job status/run-at time

## API / Route Sketch

Rust API routes:

- `GET /healthz`
- `GET /readyz`
- `POST /api/github/webhook`
- `GET /api/github/oauth/start`
- `GET /api/github/oauth/callback`
- `GET /api/installations`
- `GET /api/repos/:repo_id`
- `POST /api/repos/:repo_id/policy`
- `POST /api/repos/:repo_id/allowlist`
- `DELETE /api/repos/:repo_id/allowlist/:user_id`
- `GET /api/verify/:session_id`
- `POST /api/verify/:session_id/captcha`

Frontend routes:

- `/`
- `/dashboard`
- `/dashboard/installations`
- `/dashboard/repos/[repoId]`
- `/verify/[sessionId]`
- `/verify/[sessionId]/success`
- `/verify/[sessionId]/error`

Frontend pages should call the Rust API. Sensitive operations, OAuth token exchange, CAPTCHA verification, and GitHub API calls must happen only in Rust.

## Security Requirements

Must-have from the beginning:

- Verify GitHub webhook signatures
- Store hashed verification tokens, not raw tokens
- Expire verification sessions
- Bind verification to GitHub user ID via OAuth by default
- Validate CAPTCHA server-side
- Avoid logging OAuth tokens, CAPTCHA secrets, GitHub private keys, GitHub installation tokens, or verification tokens
- Use least-privilege GitHub App permissions
- Make private repo content storage explicit in docs/config
- Use secure cookie/session handling for dashboard and OAuth flows
- Add CSRF protection where browser-authenticated state-changing actions exist

## GitHub App Permissions to Investigate

Likely needed:

- Metadata: read
- Issues: read/write
- Pull requests: read/write
- Checks: read/write, if using check runs
- Commit statuses: read/write, if using commit statuses instead of checks
- Members/Collaborators: read, depending on exemption strategy

This should be validated during Phase 1 because GitHub permission boundaries can affect implementation details.

## Rust-Specific Risks

Risks to address early:

- GitHub App JWT and installation-token flow may require custom code around `octocrab`
- Webhook payload typing may be less complete than Node libraries
- GitHub Checks API support may need custom request structs
- More boilerplate for OAuth/session handling
- Type sharing with the Next.js frontend is less automatic

Mitigations:

- Build `crates/github` as a focused internal abstraction early
- Store raw webhook payloads for debugging while also parsing typed event structs
- Use OpenAPI or generated TypeScript types later if API type drift becomes painful
- Keep policy logic in pure Rust and heavily unit-tested
- De-risk Checks API in Phase 1/3 before relying on it for merge-blocking workflows

## Non-Goals for OSS MVP

Do not build these first:

- LLM detection
- GPTZero integration
- Global shared trust
- Paid billing
- Multi-tenant hosted operations
- Enterprise SSO/SAML
- Advanced reputation scoring
- Browser extension or pre-submit gating

## Open Questions

These should be resolved before or during Phase 0/1:

1. Should the frontend remain a separate Next.js service, or eventually be served as static assets by Rust?
2. Should the database layer use SQLx or SeaORM?
3. Should PR verification use GitHub Checks API, commit statuses, or support both?
4. Which CAPTCHA providers should be included in the initial OSS release after Turnstile?
5. Should the first version use one combined Rust API/job process, or split API and worker immediately?
6. Which license should the OSS core use?
7. How strict should default private-repo data storage be?
8. Should API types be shared with the frontend through OpenAPI generation, manual TypeScript types, or a tool like ts-rs?

## Recommended Next Step

Start with Phase 0 and Phase 1 together, with emphasis on de-risking GitHub App support in Rust:

1. Create the Cargo workspace and Next.js app.
2. Add Docker Compose with Postgres.
3. Add a minimal Axum API with health endpoints.
4. Add SQLx migrations.
5. Add GitHub webhook endpoint with signature verification.
6. Add GitHub App JWT generation and installation-token exchange.
7. Document how to create and install a development GitHub App.
8. Confirm the Rust backend can receive a real GitHub webhook and make an authenticated GitHub API call.

Once a test GitHub App can send real webhooks into the Rust backend and the backend can call GitHub as an installation, the rest of the product can be built incrementally around real GitHub behavior.
