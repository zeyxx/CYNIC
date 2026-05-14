#!/usr/bin/env bash
# CYNIC date-based versioning — bump version and create release tag
set -euo pipefail

TODAY=$(date +%y.%m.%d)
GIT_ROOT=$(cd "$(dirname "$0")/.." && pwd)
KERNEL_TOML="${GIT_ROOT}/cynic-kernel/Cargo.toml"

[[ -f "$KERNEL_TOML" ]] || { echo "Error: $KERNEL_TOML not found"; exit 1; }

CURRENT_VERSION=$(grep "^version = " "$KERNEL_TOML" | grep -oE "[0-9]{2}\.[0-9]\.[0-9]{2}" | head -1 || echo "0.0.0")

if [[ "$CURRENT_VERSION" != "$TODAY" ]]; then
    echo "Bumping version: $CURRENT_VERSION → $TODAY"
    sed -i "s/^version = \"[^\"]*\"/version = \"$TODAY\"/" "$KERNEL_TOML"
    cd "$GIT_ROOT"
    cargo update -p cynic-kernel
    git add cynic-kernel/Cargo.toml Cargo.lock
    git commit -m "chore(release): Bump version to $TODAY"
else
    echo "Version already at $TODAY"
fi

cd "$GIT_ROOT"
TAG_NAME="prod-${TODAY//./-}"
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
    echo "Tag $TAG_NAME already exists"
else
    git tag -a "$TAG_NAME" -m "Production release: CYNIC kernel $TODAY"
    echo "Tag created: $TAG_NAME"
fi
