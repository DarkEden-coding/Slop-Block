# GitHub Human Auth

A self-hostable GitHub App that verifies unfamiliar issue and pull request authors proving that they are human before maintainers pour coffee into the triage machine.

GitHub Human Auth combines GitHub OAuth, CAPTCHA, repository policies, and trusted contributor allowlists so maintainers can spend less time fighting drive-by slop, spammy PRs, and bot-generated issue confetti — and more time reviewing work from real people.

## Why this exists

Open source maintainers are generous, not infinite. When every repo becomes a magnet for low-effort AI sludge, fake reports, and suspicious PRs, real contributors get buried and maintainers lose momentum.

This project adds a lightweight verification gate for unknown authors:

1. GitHub sends an issue or PR event to the app.
2. The app checks the repository policy and allowlist.
3. Unknown contributors are routed through GitHub OAuth plus CAPTCHA.
4. Verified humans can proceed; trusted users can be allowlisted for smoother future contributions.

## Features

- **Self-hostable stack** — Rust/Axum API, Next.js dashboard, and PostgreSQL via Docker Compose.
- **GitHub App webhook receiver** — validates `X-Hub-Signature-256` before processing GitHub events.
- **GitHub OAuth verification** — confirms the person completing verification controls the relevant GitHub account.
- **CAPTCHA support** — Cloudflare Turnstile, hCaptcha, and reCAPTCHA configuration support, with an explicit local-dev bypass.
- **Maintainer dashboard** — GitHub-login-protected UI for installed projects, policies, CAPTCHA/OAuth settings, labels, comments, and allowlists.
- **Repository-level policies** — enable/disable verification and tune requirements per repository.
- **Trusted contributor allowlists** — let known-good humans skip the hoop-jumping.
- **Admin controls** — browser sessions for maintainers plus optional legacy bearer-token access for automation.
- **PostgreSQL-backed state** — migrations run on API startup.
- **Health checks** — `/healthz` for process health and `/readyz` for database readiness.
- **No project-owned hosted dependency** — you run the app and database yourself; GitHub and your CAPTCHA provider stay in your accounts.

## Architecture

```text
GitHub webhooks ──> Rust API ──> PostgreSQL
GitHub OAuth  ─────┘    ▲
CAPTCHA provider ───────┘
Next.js dashboard ──> Rust API
```

- `apps/api` receives GitHub webhooks, handles OAuth callbacks, validates CAPTCHA tokens, stores verification state, and exposes policy/admin APIs.
- `apps/web` provides contributor verification pages and a maintainer dashboard.
- `crates/*` contains shared Rust libraries for GitHub, CAPTCHA, policy, database, jobs, and common types.
- `migrations` contains SQL schema migrations.
- `docs` contains operator guides for setup, configuration, security, and self-hosting.

## Requirements

For the Docker Compose path:

- Docker and Docker Compose
- A GitHub App
- PostgreSQL, supplied by Compose by default
- A CAPTCHA provider account/key for production
- Public HTTPS URLs for production webhook and OAuth callback traffic

For local development checks:

- Rust toolchain from `rust-toolchain.toml`
- Node.js/pnpm

## Quick start with Docker Compose

```sh
cp .env.example .env
```

Edit `.env` and fill in at least:

- `POSTGRES_PASSWORD` / `DATABASE_URL`
- `GITHUB_APP_ID`
- `GITHUB_PRIVATE_KEY`
- `GITHUB_WEBHOOK_SECRET`
- `GITHUB_OAUTH_CLIENT_ID`
- `GITHUB_OAUTH_CLIENT_SECRET`
- `ADMIN_GITHUB_LOGINS`
- `ADMIN_SESSION_SECRET`
- `SECRETS_ENCRYPTION_KEY`
- CAPTCHA secrets/site keys for the provider you use

Generate useful local secrets:

```sh
openssl rand -base64 32   # good for SECRETS_ENCRYPTION_KEY
openssl rand -hex 32      # good for ADMIN_SESSION_SECRET or webhook-style secrets
```

Start everything:

```sh
docker compose up --build -d
```

Check the API and dashboard:

```sh
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
open http://localhost:3000
```

Compose starts:

- `postgres` with a persistent volume
- `api` on `${API_PORT:-8080}`
- `web` on `${WEB_PORT:-3000}`

## GitHub App setup

Create a GitHub App in **GitHub → Settings → Developer settings → GitHub Apps**.

Recommended values:

- **Homepage URL:** `WEB_BASE_URL`, for example `https://auth.example.com`
- **Callback URL:** `API_BASE_URL/api/github/oauth/callback`
- **Webhook URL:** `API_BASE_URL/api/github/webhook`
- **Webhook secret:** a long random value matching `GITHUB_WEBHOOK_SECRET`

Minimum permissions for the current app:

| Permission | Access | Why |
| --- | --- | --- |
| Metadata | Read-only | Required by GitHub Apps |
| Issues | Read and write | Read issue context and post/update verification guidance |
| Pull requests | Read and write | Read PR context and post/update verification guidance |
| Contents | Read-only | Read repository metadata without code write access |

Subscribe to these events:

- `issues`
- `issue_comment`
- `pull_request`
- `pull_request_review_comment`

After creating the app, copy its App ID, private key, OAuth client ID, and OAuth client secret into `.env`. Install the app only on repositories where you want verification.

See [`docs/github-app-setup.md`](docs/github-app-setup.md) for the detailed walkthrough.

## Production notes

Before pointing real repositories at it:

- Use HTTPS for `WEB_BASE_URL`, `API_BASE_URL`, and `NEXT_PUBLIC_API_BASE_URL`.
- Set `COOKIE_SECURE=true`.
- Set `CORS_ALLOWED_ORIGINS` to your exact dashboard origin.
- Set `ADMIN_GITHUB_LOGINS` to the maintainer GitHub accounts allowed into the dashboard.
- Keep PostgreSQL private.
- Inject secrets through your deployment platform rather than committing `.env`.
- Never enable `TURNSTILE_DEV_BYPASS=true` on a public deployment.
- Back up PostgreSQL and test restores.

More detail lives in:

- [`docs/self-hosting.md`](docs/self-hosting.md)
- [`docs/configuration.md`](docs/configuration.md)
- [`docs/security.md`](docs/security.md)

## Local development

Install JavaScript dependencies:

```sh
pnpm install
```

Run checks:

```sh
cargo fmt --all --check
cargo test --workspace
pnpm --filter web lint
pnpm --filter web build
```

Run the API locally:

```sh
cargo run -p api
```

Run the web app locally:

```sh
pnpm --filter web dev
```

For local CAPTCHA-free testing, set:

```env
COOKIE_SECURE=false
TURNSTILE_DEV_BYPASS=true
```

The API intentionally rejects that bypass when secure-cookie production posture is enabled.

## Useful Docker commands

```sh
docker compose ps
docker compose logs -f api
docker compose logs -f web
docker compose up --build -d
docker compose down
```

## Repository layout

```text
apps/api      Rust/Axum API service
apps/web      Next.js dashboard and verification UI
crates/       Shared Rust crates
migrations/   SQL migrations
docker/       Container build assets
docs/         Operator documentation
```

## Maintainer-friendly promise

GitHub Human Auth is built to reduce the slop tax: fewer mystery accounts, fewer bot-shaped chores, fewer “please review my 4,000-line vibe PR” moments. Real contributors still get a path in. Maintainers get a calmer queue. Everyone gets to keep a little more sanity.
