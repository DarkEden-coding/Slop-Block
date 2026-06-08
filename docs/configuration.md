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
| `ADMIN_API_TOKEN` | Optional legacy bearer token for machine access to policy and allowlist administration endpoints. Use at least 32 random characters in production. |
| `ADMIN_GITHUB_LOGINS` | Optional comma-separated GitHub logins allowed to use the browser dashboard. If empty, any GitHub-authenticated user can sign in. Set this in production. |
| `ADMIN_SESSION_SECRET` | Optional secret used to sign dashboard browser sessions. Use at least 32 random characters. If unset, the OAuth client secret is used. |

## Service and URL variables

| Variable | Default | Notes |
| --- | --- | --- |
| `API_HOST` | `127.0.0.1` locally, `0.0.0.0` in Compose | Bind address for the Rust API. |
| `API_PORT` | `8080` | API port. |
| `WEB_PORT` | `3000` | Compose host port for the web app. |
| `WEB_BASE_URL` | `http://localhost:3000` | Public dashboard URL. Use HTTPS in production. |
| `API_BASE_URL` | `http://localhost:8080` | Public API URL for browser redirects/callbacks; inside Compose the web container uses `http://api:8080` when calling server-side. |
| `NEXT_PUBLIC_API_BASE_URL` | `http://localhost:8080` | Browser-visible API URL baked into the Next.js build. |
| `ADMIN_SESSION_COOKIE_NAME` | `gho_admin_session` | Cookie name for the login-protected dashboard session. |
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

## Dashboard login

The main page and dashboard use GitHub OAuth. Click **Login with GitHub**; the API stores a signed, HttpOnly dashboard session cookie and the browser dashboard can then call settings routes with `credentials: include`.

The dashboard OAuth start route is `/api/auth/github/start`, but it deliberately reuses the existing GitHub App callback URL `/api/github/oauth/callback` so contributor verification and dashboard login work with the same GitHub App OAuth configuration.

For production, set `ADMIN_GITHUB_LOGINS` to the GitHub accounts that may manage this installation. If it is empty, any successfully authenticated GitHub user can use the dashboard.

## Production checklist

- Use HTTPS public URLs for `WEB_BASE_URL`, `API_BASE_URL`, and GitHub callback/webhook settings.
- Set `COOKIE_SECURE=true`.
- Restrict `CORS_ALLOWED_ORIGINS` to your dashboard origin.
- Set `ADMIN_GITHUB_LOGINS` for browser administrators and optionally `ADMIN_API_TOKEN` for machine/API administration.
- Inject secrets through your platform secret store rather than committing `.env`.
- Rotate GitHub App keys, webhook secrets, OAuth secrets, and database passwords after exposure.
