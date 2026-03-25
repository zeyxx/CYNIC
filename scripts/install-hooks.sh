#!/bin/sh
# CYNIC — Install git hooks from scripts/git-hooks/ into .git/hooks/
# Run once after clone or when hooks are updated.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_SRC="$SCRIPT_DIR/git-hooks"
HOOKS_DST="$PROJECT_DIR/.git/hooks"

if [ ! -d "$HOOKS_DST" ]; then
    echo "ERROR: .git/hooks/ not found — are you in a git repo?"
    exit 1
fi

for hook in "$HOOKS_SRC"/*; do
    name=$(basename "$hook")
    cp "$hook" "$HOOKS_DST/$name"
    chmod +x "$HOOKS_DST/$name"
    echo "Installed: .git/hooks/$name"
done

echo "Done. Git hooks are active."
