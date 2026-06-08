# Configuration

GitHub Human Auth is configured with environment variables. Copy `.env.example` to `.env`, fill secrets, then run with Docker Compose or your own process manager.

## Required for runtime

| Variable | Purpose |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection string used by the API. Must be `postgres://` or `postgresql://`. |
| `GITHUB_APP_ID` | Numeric GitHub App ID. |
| `GITHUB_PRIVATE_KEY` | GitHub App private key PEM. In Compose, paste the PEM contents or inject via your secret manager. |
| `GITHUB_WEBHOOK_SECRET` | Shared secret used to verify GitHub webhook signatures. |
| `GITHUB_OAUTH_CLIENT_ID` | OAuth client ID from the same GitHub App. |
| `GITHUB_OAUTH_CLIENT_SECRET` | OAuth client secret from the same GitHub App. |
| `TURNSTILE_SECRET` | Cloudflare Turnstile secret key for CAPTCHA verification. |
| `ADMIN_API_TOKEN` | Optional bearer token protecting policy and allowlist administration endpoints. Use at least 32 random characters in production. |

## Service and URL variables

| Variable | Default | Notes |
| --- | --- | --- |
| `API_HOST` | `127.0.0.1` locally, `0.0.0.0` in Compose | Bind address for the Rust API. |
| `API_PORT` | `8080` | API port. |
| `WEB_PORT` | `3000` | Compose host port for the web app. |
| `WEB_BASE_URL` | `http://localhost:3000` | Public dashboard URL. Use HTTPS in production. |
| `API_BASE_URL` | `http://localhost:8080` | Public API URL for browser redirects/callbacks; inside Compose the web container uses `http://api:8080` when calling server-side. |
| `NEXT_PUBLIC_API_BASE_URL` | `http://localhost:8080` | Browser-visible API URL baked into the Next.js build. |
| `NEXT_PUBLIC_ADMIN_API_TOKEN` | unset | Optional browser-visible token used by the dashboard for protected admin API calls. Only use for trusted/private dashboard deployments; prefer putting the dashboard behind your own access control. |
| `CORS_ALLOWED_ORIGINS` | `http://localhost:3000` | Comma-separated allowed web origins. Set to the exact production web origin. |
| `COOKIE_SECURE` | `true` in API defaults, `false` in local example | Set `true` behind HTTPS. |
| `SESSION_COOKIE_NAME` | `gho_session` | Session cookie name. |
| `GITHUB_WEB_URL` | `http://localhost:3000` | Public web URL used in GitHub-facing links. |
| `GITHUB_API_BASE` | `https://api.github.com` | Override only for compatible GitHub API testing. |
| `RUST_LOG` | `info` | Rust tracing filter. |

## PostgreSQL variables for Docker Compose

| Variable | Default |
| --- | --- |
| `POSTGRES_DB` | `github_human_auth` |
| `POSTGRES_USER` | `github_human_auth` |
| `POSTGRES_PASSWORD` | `github_human_auth` |
| `POSTGRES_PORT` | `5432` |

Use a strong `POSTGRES_PASSWORD` in production and do not expose PostgreSQL publicly.

## Turnstile development bypass

`TURNSTILE_DEV_BYPASS=true` skips real CAPTCHA verification for local development and automated tests. The API now rejects this setting unless `COOKIE_SECURE=false`; never enable it in production or any internet-accessible environment.

## Production checklist

- Use HTTPS public URLs for `WEB_BASE_URL`, `API_BASE_URL`, and GitHub callback/webhook settings.
- Set `COOKIE_SECURE=true`.
- Restrict `CORS_ALLOWED_ORIGINS` to your dashboard origin.
- Set `ADMIN_API_TOKEN` and send it as `Authorization: Bearer <token>` for policy/allowlist administration routes.
- Inject secrets through your platform secret store rather than committing `.env`.
- Rotate GitHub App keys, webhook secrets, OAuth secrets, and database passwords after exposure.
