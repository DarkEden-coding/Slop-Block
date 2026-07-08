#!/usr/bin/env python3
"""Create a bounded number of GitHub test issues.

This script is intended for development/testing only. It deliberately includes
safety limits so it is not useful for spam or ToS-abusive automation:

- dry-run by default; requires --execute to create issues
- hard cap of 500 issues per run, matching GitHub's documented general
  secondary content-creation guidance of no more than 500 content-generating
  requests per hour
- default cap of 100 issues per run
- minimum delay of 0.8 seconds between issue creations, staying just under
  GitHub's documented general guidance of no more than 80 content-generating
  requests per minute
- prompts for a GitHub token with input() when executing, unless
  GITHUB_TOKEN/GH_TOKEN is already set
- requires the token user to have admin permission on the target repository
- requires an explicit --i-own-this-repo acknowledgment

Token permissions: fine-grained token with Issues: Read and write for the target
repository, or a classic token with the public_repo/repo scope as appropriate.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from typing import Any

API_BASE = "https://api.github.com"
DEFAULT_COUNT = 100
HARD_MAX_COUNT = 500
MIN_DELAY_SECONDS = 0.8


def positive_int(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be an integer") from exc
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return parsed


def delay_seconds(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be a number") from exc
    if parsed < MIN_DELAY_SECONDS:
        raise argparse.ArgumentTypeError(
            f"must be at least {MIN_DELAY_SECONDS:g} seconds to stay below GitHub's documented general content-creation limits"
        )
    return parsed


def github_request(token: str, method: str, path: str, body: dict[str, Any] | None = None) -> tuple[int, dict[str, str], Any]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    req = urllib.request.Request(
        f"{API_BASE}{path}",
        data=data,
        method=method,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
            "User-Agent": "github-human-auth-test-issue-seeder",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8")
            payload = json.loads(raw) if raw else None
            return resp.status, dict(resp.headers), payload
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            payload = json.loads(raw) if raw else {"message": exc.reason}
        except json.JSONDecodeError:
            payload = {"message": raw or exc.reason}
        return exc.code, dict(exc.headers), payload


def remaining_rate_limit(headers: dict[str, str]) -> str:
    remaining = headers.get("x-ratelimit-remaining")
    reset = headers.get("x-ratelimit-reset")
    if not remaining:
        return "unknown"
    if reset and reset.isdigit():
        reset_at = datetime.fromtimestamp(int(reset), tz=timezone.utc).isoformat()
        return f"{remaining}, resets at {reset_at}"
    return remaining


def main() -> int:
    parser = argparse.ArgumentParser(description="Safely create a limited number of GitHub test issues.")
    parser.add_argument("repo", help="Target repository in owner/name form, e.g. octocat/hello-world")
    parser.add_argument("--count", type=positive_int, default=DEFAULT_COUNT, help=f"Number of issues to create. Default: {DEFAULT_COUNT}. Hard max: {HARD_MAX_COUNT}.")
    parser.add_argument("--delay", type=delay_seconds, default=MIN_DELAY_SECONDS, help=f"Seconds to wait between creations. Minimum: {MIN_DELAY_SECONDS:g}.")
    parser.add_argument("--title-prefix", default="GHA test issue", help="Title prefix for generated issues.")
    parser.add_argument("--label", action="append", default=[], help="Optional label to apply. Can be repeated.")
    parser.add_argument("--execute", action="store_true", help="Actually create issues. Without this flag, the script only prints what it would do.")
    parser.add_argument(
        "--i-own-this-repo",
        action="store_true",
        help="Required with --execute. Confirms this is your repository and test issue creation is authorized.",
    )
    args = parser.parse_args()

    if "/" not in args.repo or args.repo.count("/") != 1:
        print("error: repo must be in owner/name format", file=sys.stderr)
        return 2

    if args.count > HARD_MAX_COUNT:
        print(f"error: --count cannot exceed {HARD_MAX_COUNT} per run", file=sys.stderr)
        return 2

    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if args.execute and not args.i_own_this_repo:
        print("error: --execute requires --i-own-this-repo", file=sys.stderr)
        return 2

    if not token and args.execute:
        token = input("GitHub token (input will be visible; prefer GITHUB_TOKEN env var for privacy): ").strip()
        if not token:
            print("error: token is required before using --execute", file=sys.stderr)
            return 2

    owner, repo = args.repo.split("/", 1)
    print(f"Repository: {owner}/{repo}")
    print(f"Issues:     {args.count}")
    print(f"Delay:      {args.delay:g}s")
    print(f"Mode:       {'EXECUTE' if args.execute else 'DRY RUN'}")
    print()

    if args.execute:
        assert token is not None
        status, headers, repo_info = github_request(token, "GET", f"/repos/{owner}/{repo}")
        if status != 200 or not isinstance(repo_info, dict):
            message = repo_info.get("message", repo_info) if isinstance(repo_info, dict) else repo_info
            print(f"error: could not inspect repository ownership/admin permission: HTTP {status}: {message}", file=sys.stderr)
            return 1
        permissions = repo_info.get("permissions") or {}
        if not permissions.get("admin"):
            print(
                "error: refusing to create issues because this token does not have admin permission on the target repository",
                file=sys.stderr,
            )
            return 2
        status, _, user_info = github_request(token, "GET", "/user")
        login = user_info.get("login", "unknown") if status == 200 and isinstance(user_info, dict) else "unknown"
        print(f"Authenticated as: {login}; repository admin permission confirmed.")
        print(f"rate limit remaining: {remaining_rate_limit(headers)}")
        print()

    if not args.execute:
        for i in range(1, args.count + 1):
            print(f"DRY RUN: would create issue {i}/{args.count}: {args.title_prefix} #{i}")
        print("\nNo issues created. Re-run with --execute to create them.")
        return 0

    assert token is not None
    created = 0
    for i in range(1, args.count + 1):
        title = f"{args.title_prefix} #{i}"
        body = (
            "This is an automated test issue created by a local development script.\n\n"
            "If this was created in the wrong repository, close/delete it and check the script arguments."
        )
        payload: dict[str, Any] = {"title": title, "body": body}
        if args.label:
            payload["labels"] = args.label

        status, headers, response = github_request(token, "POST", f"/repos/{owner}/{repo}/issues", payload)
        if status not in (200, 201):
            message = response.get("message", response) if isinstance(response, dict) else response
            print(f"error: GitHub returned HTTP {status}: {message}", file=sys.stderr)
            print(f"rate limit remaining: {remaining_rate_limit(headers)}", file=sys.stderr)
            return 1

        created += 1
        html_url = response.get("html_url", "<unknown>") if isinstance(response, dict) else "<unknown>"
        print(f"created {created}/{args.count}: {html_url}")
        print(f"rate limit remaining: {remaining_rate_limit(headers)}")

        if i < args.count:
            time.sleep(args.delay)

    print(f"\nDone. Created {created} test issues.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
