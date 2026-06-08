# GitHub Human Auth

GitHub Human Auth is a self-hostable GitHub App for verifying unknown issue and pull request authors with GitHub OAuth and CAPTCHA before maintainers spend time triaging them.

## Current MVP

The current MVP provides:

- Rust/Axum API with health and readiness endpoints.
- PostgreSQL-backed state and startup migrations.
- GitHub App webhook receiver with signature validation.
- GitHub OAuth verification flow endpoints.
- Turnstile CAPTCHA integration with an explicit local-development bypass.
- Repository policy and allowlist API routes.
- Next.js dashboard scaffold.
- Docker Compose deployment for API, web, and PostgreSQL.

It is designed for self-hosting and does not require a hosted paid dependency from this project.

## Quickstart with Docker Compose

```sh
cp .env.example .env
# Fill .env with GitHub App credentials, Turnstile secret, URLs, and strong DB password.
docker compose up --build -d
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
open http://localhost:3000
```

For production, set HTTPS `WEB_BASE_URL`, `API_BASE_URL`, `NEXT_PUBLIC_API_BASE_URL`, `COOKIE_SECURE=true`, strict `CORS_ALLOWED_ORIGINS`, and production secrets.

## Architecture

```text
GitHub webhooks ──> Rust API ──> PostgreSQL
GitHub OAuth  ─────┘    ▲
Turnstile CAPTCHA ──────┘
Next.js dashboard ──> Rust API
```

- **GitHub App** sends issue/PR events to `/api/github/webhook` and grants least-privilege repository access.
- **OAuth flow** uses `/api/github/oauth/start` and `/api/github/oauth/callback` to confirm the user controls the GitHub account being verified.
- **CAPTCHA** uses Turnstile before verification is accepted.
- **API** stores verification state, policy, allowlists, sessions, and audit-relevant metadata in PostgreSQL.
- **Web app** provides the dashboard and user-facing verification surfaces.

## Documentation

- [Self-hosting](docs/self-hosting.md)
- [GitHub App setup](docs/github-app-setup.md)
- [Configuration](docs/configuration.md)
- [Security](docs/security.md)

## Repository layout

- `apps/api` — Axum API service.
- `apps/web` — Next.js and Tailwind dashboard scaffold.
- `crates/*` — Rust library crates for GitHub, CAPTCHA, policy, database, jobs, and shared types.
- `migrations` — SQL migrations.
- `docker` — container build assets.
- `docs` — operator documentation.

## Local development checks

```sh
cargo fmt --all --check
cargo test --workspace
pnpm install
pnpm --filter web lint
pnpm --filter web build
```

Run the API locally:

```sh
cargo run -p api
```

Default API endpoints include `GET /healthz` and `GET /readyz`. Configuration starts from `.env.example`.
