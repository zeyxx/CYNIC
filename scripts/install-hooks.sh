#!/bin/sh
# CYNIC — Install git hooks from scripts/git-hooks/ into .git/hooks/
# Run once after clone or when hooks are updated.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_SRC="$SCRIPT_DIR/git-hooks"
HOOKS_GIT_DIR="$(git -C "$PROJECT_DIR" rev-parse --git-dir)"

case "$HOOKS_GIT_DIR" in
    /*) ;;
    *) HOOKS_GIT_DIR="$PROJECT_DIR/$HOOKS_GIT_DIR" ;;
esac

HOOKS_DST="$HOOKS_GIT_DIR/hooks"

if [ ! -d "$HOOKS_DST" ]; then
    echo "ERROR: .git/hooks/ not found — are you in a git repo?"
    exit 1
fi

for hook in "$HOOKS_SRC"/*; do
    name=$(basename "$hook")
    chmod +x "$hook"
    ln -sfn "$hook" "$HOOKS_DST/$name"
    echo "Linked: $HOOKS_DST/$name -> $hook"
done

bash "$SCRIPT_DIR/verify-hooks.sh"
echo "Done. Git hooks are active."
