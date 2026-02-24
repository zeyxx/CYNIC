#!/bin/bash
# Create worktree for isolated development

BRANCH_NAME=${1:-feature/new-work}
WORKTREE_PATH="../cynic-${BRANCH_NAME}"

echo "Creating worktree: $WORKTREE_PATH"

git worktree add -b "$BRANCH_NAME" "$WORKTREE_PATH"

cd "$WORKTREE_PATH" || exit 1

echo "Worktree created at: $WORKTREE_PATH"
echo "To switch: cd $WORKTREE_PATH"
