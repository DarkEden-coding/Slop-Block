# Self-hosting

GitHub Human Auth runs as three services: PostgreSQL, the Rust API, and the Next.js web app. Docker Compose is the fastest supported deployment path for the current MVP.

## Requirements

- Docker and Docker Compose
- A GitHub App configured with webhook and OAuth credentials
- A Cloudflare Turnstile site/secret for production CAPTCHA
- Public HTTPS endpoints for the web app and API in production

## Docker Compose quickstart

```sh
cp .env.example .env
# edit .env and fill GitHub, Turnstile, URL, and database secrets
docker compose up --build -d
```

Check services:

```sh
curl http://localhost:8080/healthz
curl http://localhost:8080/readyz
open http://localhost:3000
```

Compose starts:

- `postgres` with persistent volume `postgres-data`
- `api` on `${API_PORT:-8080}`
- `web` on `${WEB_PORT:-3000}`

The API runs migrations on startup. Readiness fails until PostgreSQL is reachable.

## Production deployment notes

- Put a reverse proxy or load balancer with HTTPS in front of `web` and `api`.
- Set `WEB_BASE_URL`, `API_BASE_URL`, and `NEXT_PUBLIC_API_BASE_URL` to public HTTPS URLs.
- Set `COOKIE_SECURE=true` and a strict `CORS_ALLOWED_ORIGINS`.
- Set `ADMIN_GITHUB_LOGINS` to the GitHub logins allowed to configure this installation.
- Keep PostgreSQL on a private network.
- Use strong database credentials and platform secret injection.
- Configure the GitHub App webhook URL as `https://<api-host>/api/github/webhook`.
- Configure the OAuth callback as `https://<api-host>/api/github/oauth/callback`.

## Turnstile CAPTCHA

Create a Cloudflare Turnstile widget for the public web domain, then set `TURNSTILE_SECRET` for the API. Local development may use `TURNSTILE_DEV_BYPASS=true`; never enable this on a public deployment.

## Backup

Back up PostgreSQL regularly. With Compose:

```sh
mkdir -p backups
docker compose exec -T postgres pg_dump \
  -U "${POSTGRES_USER:-github_human_auth}" \
  -d "${POSTGRES_DB:-github_human_auth}" \
  --format=custom > backups/github_human_auth-$(date +%Y%m%d-%H%M%S).dump
```

Encrypt backups and store them outside the host running Compose. A practical retention policy is daily backups for 7 days, weekly backups for 4 weeks, and monthly backups for 12 months, adjusted to your requirements.

## Restore

To restore into a fresh Compose database:

```sh
docker compose down
# remove only if you intentionally want a clean database
docker volume rm github-human-auth_postgres-data 2>/dev/null || true
docker compose up -d postgres
cat backups/<backup-file>.dump | docker compose exec -T postgres pg_restore \
  -U "${POSTGRES_USER:-github_human_auth}" \
  -d "${POSTGRES_DB:-github_human_auth}" \
  --clean --if-exists
docker compose up -d api web
```

Test restores before relying on backups. Restore commands overwrite existing database objects.

## Operations

Useful commands:

```sh
docker compose ps
docker compose logs -f api
docker compose logs -f web
docker compose pull
docker compose up --build -d
docker compose down
```

Monitor:

- API `/healthz` for process health
- API `/readyz` for database readiness
- GitHub App webhook delivery failures
- PostgreSQL disk usage and backup success
- Application logs for OAuth, webhook, and CAPTCHA errors

## No hosted paid dependency

The current MVP does not require a paid hosted service controlled by this project. You operate the API, web app, and database yourself. GitHub is required for GitHub App/OAuth operation, and Turnstile is the CAPTCHA provider; both are configured directly in your own accounts.
