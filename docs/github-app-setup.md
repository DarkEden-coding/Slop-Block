# GitHub App setup

Create one GitHub App for each deployment environment. The App receives repository events, posts verification guidance, and uses OAuth to confirm the human user's GitHub identity.

## 1. Create the App

In GitHub, open **Settings → Developer settings → GitHub Apps → New GitHub App**.

Recommended values:

- **GitHub App name:** a deployment-specific name, for example `github-human-auth-prod`.
- **Homepage URL:** `WEB_BASE_URL`, for example `https://auth.example.com`.
- **Callback URL:** `API_BASE_URL/api/github/oauth/callback`, for example `https://auth-api.example.com/api/github/oauth/callback`.
- **Webhook URL:** `API_BASE_URL/api/github/webhook`, for example `https://auth-api.example.com/api/github/webhook`.
- **Webhook secret:** generate a long random value and set it as `GITHUB_WEBHOOK_SECRET`.
- **Expire user authorization tokens:** enabled when available.

For local testing with a tunnel, use the tunnel HTTPS URL for the webhook and OAuth callback, and set `API_BASE_URL`/`WEB_BASE_URL` to matching reachable URLs.

## 2. Permissions

Use least privilege. Grant only the repositories where verification is needed.

Minimum permissions for the current MVP:

| Permission | Access | Why |
| --- | --- | --- |
| Metadata | Read-only | Required by GitHub Apps. |
| Issues | Read and write | Read issue author/context and comment or update verification state. |
| Pull requests | Read and write | Read PR author/context and comment or update verification state. |
| Contents | Read-only | Allows repository metadata checks without write access to code. |

Do not grant organization administration, member administration, secrets, actions, deployments, or code write permissions unless a future feature explicitly requires them.

## 3. Subscribe to events

Subscribe to repository events that drive verification:

- `issues`
- `issue_comment`
- `pull_request`
- `pull_request_review_comment`

The API verifies `X-Hub-Signature-256` with `GITHUB_WEBHOOK_SECRET` before processing webhook payloads.

## 4. Generate credentials

After creating the App:

1. Copy **App ID** to `GITHUB_APP_ID`.
2. Generate a **private key** and store the PEM in `GITHUB_PRIVATE_KEY` or your secret manager.
3. Copy **Client ID** to `GITHUB_OAUTH_CLIENT_ID`.
4. Generate a **Client secret** and set `GITHUB_OAUTH_CLIENT_SECRET`.
5. Install the App on selected repositories only.

## 5. OAuth callback URLs

The OAuth callback registered in GitHub must exactly match the deployed API callback:

```text
https://<api-host>/api/github/oauth/callback
```

The web app initiates verification flows and the API completes the callback. If the callback URL, cookie domain, or `COOKIE_SECURE` setting does not match the public deployment, users will be unable to finish verification.

## 6. Validate

- `GET /healthz` returns success when the API process is up.
- `GET /readyz` returns success when the API can reach PostgreSQL.
- GitHub App webhook deliveries receive a non-2xx response if the signature is invalid, and a 2xx/accepted response when signed correctly.
