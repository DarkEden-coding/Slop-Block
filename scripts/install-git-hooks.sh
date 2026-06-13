#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push

echo "Git hooks installed via core.hooksPath=.githooks"
echo "Use SKIP_HOOKS=1 git <command> only for exceptional emergency bypasses."
