#!/usr/bin/env bash
# CYNIC date-based versioning — bump version and create release tag
# Usage: scripts/bump-version.sh [--tag] [--deploy]
#
# --tag: Create git annotated tag prod-YY-MM-DD
# --deploy: Also tag with deployment suffix (e.g., prod-26-05-14-phase1)

set -euo pipefail

# Parse arguments
TAG_ONLY=false
DEPLOY_SUFFIX=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag) TAG_ONLY=true; shift ;;
        --deploy) DEPLOY_SUFFIX="$2"; shift 2 ;;
        *) echo "Usage: $0 [--tag] [--deploy SUFFIX]"; exit 1 ;;
    esac
done

# Get today's date in YY.MM.DD format
TODAY=$(date +%y.%m.%d)
KERNEL_TOML="/home/user/Bureau/CYNIC/cynic-kernel/Cargo.toml"
GIT_ROOT=$(cd "$(dirname "$0")/.." && pwd)

# Verify we're in the right place
[[ -f "$KERNEL_TOML" ]] || { echo "Error: $KERNEL_TOML not found"; exit 1; }

# 1. Update Cargo.toml version (if different from today)
CURRENT_VERSION=$(grep "^version = " "$KERNEL_TOML" | grep -oE "[0-9]{2}\.[0-9]{2}\.[0-9]{2}" | head -1 || echo "0.0.0")

if [[ "$CURRENT_VERSION" != "$TODAY" ]]; then
    echo "Bumping version: $CURRENT_VERSION → $TODAY"
    sed -i "s/^version = \"[^\"]*\"/version = \"$TODAY\"/" "$KERNEL_TOML"
    cd "$GIT_ROOT"
    git add cynic-kernel/Cargo.toml
    git commit -m "chore(release): Bump version to $TODAY (date-based)"
else
    echo "Version already at $TODAY (no bump needed)"
fi

# 2. Create git tag (if --tag)
if [[ "$TAG_ONLY" == "true" ]]; then
    cd "$GIT_ROOT"
    TAG_NAME="prod-$TODAY"
    [[ -n "$DEPLOY_SUFFIX" ]] && TAG_NAME="prod-$TODAY-$DEPLOY_SUFFIX"

    if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
        echo "Tag $TAG_NAME already exists (skipping)"
    else
        echo "Creating tag: $TAG_NAME"
        git tag -a "$TAG_NAME" -m "Production release: CYNIC kernel $TODAY"
        echo "Tag created. Push with: git push origin $TAG_NAME"
    fi
fi

echo "✓ Version updated to $TODAY"
