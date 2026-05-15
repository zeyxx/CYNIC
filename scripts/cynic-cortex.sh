#!/usr/bin/env bash
# CYNIC Agent Protocol v1 — cortex launcher
# Creates an isolated git worktree per session, then launches Claude Code inside it.
# This is the ONLY way to start a coordinated cortex session.
set -euo pipefail

CYNIC_ROOT="${CYNIC_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null)}"
if [ -z "$CYNIC_ROOT" ]; then
    echo "ERROR: not in a git repository. Run from CYNIC project root." >&2
    exit 1
fi

SESSION_ID=$(head -c4 /dev/urandom | xxd -p)
WORKTREE_DIR="/tmp/cynic-worktrees/$SESSION_ID"
BRANCH="cortex/${SESSION_ID}-$(date +%Y-%m-%d)"

# Gate: shared checkout must be clean (tracked files only — untracked are ok)
DIRTY=$(git -C "$CYNIC_ROOT" diff --name-only HEAD 2>/dev/null)
if [ -n "$DIRTY" ]; then
    echo "BLOCKED: shared checkout has uncommitted tracked changes:" >&2
    echo "$DIRTY" >&2
    echo "" >&2
    echo "Another cortex left dirty files. Resolve before starting:" >&2
    echo "  cd $CYNIC_ROOT && git stash  # or git checkout -- <files>" >&2
    exit 1
fi

# Fetch latest main
git -C "$CYNIC_ROOT" fetch origin main --quiet 2>/dev/null || true

# Create worktree
mkdir -p /tmp/cynic-worktrees
git -C "$CYNIC_ROOT" worktree add "$WORKTREE_DIR" -b "$BRANCH" origin/main --quiet
echo "Worktree created: $WORKTREE_DIR (branch: $BRANCH)"

# Launch Claude Code inside worktree
cd "$WORKTREE_DIR"
exec claude "$@"
