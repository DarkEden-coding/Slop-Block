# Terms of Service

_Last updated: 2026-07-04_

These terms describe the intended use and limitations of Agent Block, a GitHub App for human verification on GitHub issues and pull requests.

## Intended use

Agent Block is intended to help repository maintainers reduce spam, bot activity, and low-quality drive-by contributions by asking unknown issue and pull request authors to complete GitHub OAuth and CAPTCHA verification.

Maintainers can configure repository policies, labels, comments, trusted users, and verification behavior from the dashboard.

## Acceptable use

You may use Agent Block to:

- Request human verification from issue and pull request authors
- Label or annotate unverified issues and pull requests
- Maintain repository-specific allowlists of trusted contributors
- Configure verification policies for repositories where you have appropriate authority

You may not use Agent Block to:

- Harass, discriminate against, or unfairly target contributors
- Misrepresent verification status or impersonate another user
- Attempt to bypass GitHub, CAPTCHA, or Agent Block security controls
- Use the service for unlawful, abusive, or malicious activity
- Install or configure the app on repositories you are not authorized to administer

## Content moderation and maintainer responsibility

Agent Block does not replace maintainer judgment. It provides workflow automation and verification signals, but maintainers remain responsible for repository moderation decisions, contribution policies, and enforcement outcomes.

Agent Block does not determine whether a contribution is good, correct, lawful, safe, or acceptable. It only helps verify GitHub account control and CAPTCHA completion according to configured repository policy.

## Service limitations

Agent Block is provided without guarantees that it will block all spam, bots, abuse, AI-generated content, or low-quality contributions. Verification can reduce unwanted activity, but it is not a complete security or moderation solution.

The hosted deployment may experience downtime, GitHub API limitations, CAPTCHA provider outages, network interruptions, or other operational issues. Maintainers should not rely on Agent Block as their only repository security control.

## Data and privacy

Agent Block processes GitHub and verification data as described in the Privacy Policy:

https://github.com/DarkEden-coding/Slop-Block/blob/main/docs/privacy.md

By installing or using the app, you acknowledge that GitHub API/OAuth, GitHub webhooks, CAPTCHA providers, PostgreSQL, and deployment infrastructure may be involved in providing the service.

## Repository access and permissions

Agent Block should be installed only on repositories where verification is desired. The GitHub App should be granted only the permissions required for operation. Users configuring the app must have appropriate authorization to manage the relevant repositories or installations.

## Changes

These terms may be updated as the app evolves. Continued use of Agent Block after changes means you accept the updated terms.

## Contact

For questions, issues, or security concerns, use the GitHub repository:

https://github.com/DarkEden-coding/Slop-Block
