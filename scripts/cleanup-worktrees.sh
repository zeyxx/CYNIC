#!/bin/bash
# Cleanup git worktrees

echo "Listing worktrees:"
git worktree list

echo ""
echo "Removing worktrees (except current and main)..."

# List all worktrees except current
git worktree list --porcelain | grep '^worktree ' | grep -v '^\*' | while read -r line; do
    WORKTREE_PATH=$(echo "$line" | sed 's/^worktree //')
    if [[ "$WORKTREE_PATH" != *"cynic-clean"* ]]; then
        echo "Removing: $WORKTREE_PATH"
        git worktree remove "$WORKTREE_PATH" --force 2>/dev/null || true
    fi
done

echo ""
echo "Remaining worktrees:"
git worktree list
