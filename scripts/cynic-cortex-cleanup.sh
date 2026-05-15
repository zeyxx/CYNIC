#!/usr/bin/env bash
# CYNIC — Prune stale worktrees (cron: daily)
# Deletes worktrees older than 24h that have no unpushed commits.
set -euo pipefail

CYNIC_ROOT="${CYNIC_ROOT:-$HOME/Bureau/CYNIC}"
WORKTREE_BASE="/tmp/cynic-worktrees"

[ -d "$WORKTREE_BASE" ] || exit 0

NOW=$(date +%s)
CLEANED=0

for wt in "$WORKTREE_BASE"/*/; do
    [ -d "$wt" ] || continue
    CREATED=$(stat -c '%Y' "$wt" 2>/dev/null || echo "$NOW")
    AGE=$(( (NOW - CREATED) / 3600 ))
    [ "$AGE" -lt 24 ] && continue

    BRANCH=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
    if [ -n "$BRANCH" ]; then
        AHEAD=$(git -C "$wt" log --oneline "origin/main..$BRANCH" 2>/dev/null | wc -l)
        if [ "$AHEAD" -gt 0 ]; then
            PUSHED=$(git -C "$wt" log --oneline "origin/$BRANCH..$BRANCH" 2>/dev/null | wc -l)
            if [ "$PUSHED" -gt 0 ]; then
                echo "SKIP: $wt has $PUSHED unpushed commits (branch: $BRANCH)"
                continue
            fi
        fi
    fi

    git -C "$CYNIC_ROOT" worktree remove "$wt" --force 2>/dev/null && CLEANED=$((CLEANED+1))
done

echo "Cleaned $CLEANED stale worktrees"
