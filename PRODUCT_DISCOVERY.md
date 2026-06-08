# GitHub Human Verification / Contributor Trust Product Discovery

## Shared Understanding

The goal is to build a **Contributor Trust / Triage layer for GitHub**: a GitHub App/bot that helps maintainers reduce spam and bot-created issues/PRs by requiring unknown contributors to verify that they are human.

The repository is currently empty, so this document captures product and architecture discovery rather than implementation details.

## Core Product Idea

A GitHub App reacts **after** an issue or PR is created. If the author is not already trusted or verified, the bot:

- Adds a configurable label, such as `human-verification-required`
- Posts a comment with a verification link
- For PRs, optionally creates a GitHub check/status
- Does **not block by default**, but can be configured to create a merge-blocking required check
- For issues, uses labels and bot comments because GitHub issues do not have commit checks/statuses

The contributor clicks the link, authenticates with minimal GitHub OAuth, completes CAPTCHA, and then the bot updates the issue/PR:

- Removes the `human-verification-required` label
- Adds a `verified` label
- Updates or hides/edits the old bot comment where possible
- Updates the PR check/status if configured

## Initial Target

Primary target users are:

- Public OSS maintainers dealing with spam or bot-created issues
- Small projects that want an easy self-hosted option
- High-volume repositories and organizations that want a paid hosted version with easier setup and more integrations

Private repository support should exist in both self-hosted and paid modes, but private repos are not the initial marketing focus.

## MVP Scope

The MVP should include:

- GitHub App
- Webhook handling for issue/PR opened/reopened events
- CAPTCHA verification
- GitHub OAuth identity binding by default
- Labels/comments/checks/status behavior
- Hosted/self-host dashboard for configuration
- Single Docker Compose self-host setup with app, database, and dashboard
- Multiple CAPTCHA provider support through an abstraction

Explicitly post-MVP:

- Global trust network
- Analytics
- LLM-output detection
- GPTZero or similar integrations
- Broader spam/risk scoring

## Verification Model

Default verification should mean:

- Verified for the specific project/repo/org
- Repo owners can choose re-verification frequency
- Repo owners can optionally participate in a global/shared verification system
- Global shared verification must be opt-in for both repo owners and contributors
- Global shared status should only communicate “verified human,” not broader reputation by default

Default verification flow:

1. Contributor authenticates with GitHub OAuth using minimal identity scope.
2. Contributor solves CAPTCHA.
3. Verification is bound to the GitHub user ID/login.

Self-hosters may optionally allow CAPTCHA-only mode, but OAuth + CAPTCHA is the safer default.

## Trust and Exemptions

Exempt by default:

- Collaborators
- Verified GitHub bots/apps

Configurable options:

- Maintainers can manually allowlist trusted users per repo/org
- Setup can optionally auto-verify users with previous accepted contributions
- Auto-close can be enabled after a timeout, but should not be default

Repeated abuse by verified humans is not MVP’s job. Verification only proves “probably human”; moderation and spam scoring can come later.

## OSS / Paid Boundary

Preferred model:

- Open core
- OSS core includes GitHub App, dashboard, CAPTCHA verification, labels/comments/checks, and self-hosting
- Paid/closed modules include:
  - Hosted convenience
  - Global trust
  - Analytics
  - LLM/spam detection
  - Integrations like GPTZero
  - Enterprise features

The license/business model is not finalized, but the current preference is open core rather than purely permissive hosted SaaS.

## Configuration UX

Configuration should happen through a **web dashboard**, both for SaaS and self-hosted deployments.

Likely repo/org settings include:

- Verification required for issues, PRs, or both
- Default non-blocking/blocking behavior for PRs
- CAPTCHA provider
- Re-verification interval
- Auto-close timeout
- Allowlisted users
- Whether collaborators/bots are exempt
- Whether previous contributors are automatically trusted
- Cleanup/update behavior for bot comments
- Data retention settings

## Technical Preferences

Rust is preferred because of small binaries and safety, but TypeScript/Node is acceptable if there is a strong ecosystem reason.

Frontend preferences:

- Tailwind CSS
- Framework flexible: Next.js or similar is acceptable

Self-hosting target:

- Single Docker Compose setup
- Should include backend, dashboard, and database

Likely tradeoff to evaluate later:

- **Rust** gives safety, performance, and single-binary appeal.
- **TypeScript/Node/Probot/Octokit** may be faster for GitHub App development and ecosystem integration.
- A hybrid architecture is possible but probably too complex for the MVP.

## Data and Privacy Posture

The hosted paid service may store:

- GitHub user ID/login
- Repo/org IDs
- Verification timestamps
- Configuration
- Issue/PR metadata
- Issue/PR text, especially for future LLM/spam detection

Retention should be customer-configurable with privacy-conscious defaults.

This is important because storing private repository content or public contributor text for detection creates trust and legal concerns.

## Product Positioning

Best positioning:

> Contributor trust and triage layer for GitHub.

The product should not be framed only as “CAPTCHA for GitHub,” and it should not initially be framed primarily as an “AI detector.” Human verification is the entry point, but the long-term product can become a broader triage/trust layer.

## Beta Success Metric

The first beta succeeds if:

> Maintainers report fewer spam or low-effort issues reaching triage.

Secondary metrics:

- Contributor verification completion rate
- Low contributor frustration
- Install/self-host simplicity
- Hosted-service conversion interest

## Important Constraints and Risks

- GitHub issues cannot have commit statuses/checks; labels/comments are the issue-side mechanism.
- PR checks can become merge-blocking only if repo branch protection requires them.
- CAPTCHA alone does not prove GitHub identity unless tied to OAuth or a signed token.
- Contributor friction is a major risk; the verification page must feel safe and minimal.
- Global verification has privacy/trust implications and should be opt-in.
- LLM detection should not be in MVP unless the core anti-spam workflow works first.
- Open-core boundary needs care so the OSS version feels useful, not crippled.
