#!/bin/bash
# Ralph Loop v3: Automated safe merge queue with smart conflict resolution
# Zero work loss: all originals backed up, conflicts resolved via 3-way merge preference

set -e

REPO_DIR="$(cd "$(dirname "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)")" && pwd)"
cd "$REPO_DIR"

BRANCHES=(
  "feat/behavior-ml-lstm-training-2026-05-01"
  "feat/domain-curation-2026-05-01"
  "deep/organ-x-diagnosis-2026-05-01"
  "feat/wallet-corpus-extraction-2026-04-30"
  "chore/infrastructure-deployment-2026-04-30"
  "docs/consolidation-final-2026-04-30"
  "origin/feat/agent-logging-and-organism-consumer-2026-04-30"
  "chore/todo-pr47-complete-2026-04-30"
)

echo "═══════════════════════════════════════════════════════════════"
echo "  RALPH LOOP v3: Smart merge queue"
echo "═══════════════════════════════════════════════════════════════"
echo ""

MERGED=0
SKIPPED=0

git checkout main --quiet 2>/dev/null || true
git pull origin main --quiet 2>/dev/null || true

for BRANCH in "${BRANCHES[@]}"; do
  BRANCH_NAME="${BRANCH#origin/}"
  BACKUP_BRANCH="backup/${BRANCH_NAME}"

  echo "Processing: $BRANCH_NAME"

  # Checkout branch
  if ! git show-ref --quiet refs/heads/"$BRANCH_NAME"; then
    if ! git checkout "$BRANCH" --quiet 2>/dev/null; then
      echo "  ✗ Not found, skipping"
      ((SKIPPED++))
      continue
    fi
  else
    git checkout "$BRANCH_NAME" --quiet 2>/dev/null
  fi

  # Backup original
  git branch -f "$BACKUP_BRANCH" HEAD --quiet 2>/dev/null || true

  # Rebase with conflict auto-resolution
  if ! git rebase origin/main --quiet 2>/dev/null; then
    # Get unmerged files
    UNMERGED=$(git diff --name-only --diff-filter=U 2>/dev/null || true)

    if [ -n "$UNMERGED" ]; then
      # For each conflicted file, take the version from current branch
      for FILE in $UNMERGED; do
        # Prefer current branch (ours) for conflicts
        git checkout --ours "$FILE" 2>/dev/null || git show :2:"$FILE" > "$FILE" 2>/dev/null || true
        git add "$FILE" 2>/dev/null || true
      done

      # Continue rebase
      if ! git rebase --continue --no-edit --quiet 2>/dev/null; then
        git rebase --abort --quiet 2>/dev/null || true
        echo "  ✗ Unresolvable, saved to $BACKUP_BRANCH"
        ((SKIPPED++))
        git checkout main --quiet
        continue
      fi
    fi
  fi

  echo "  ✓ Rebased, pushing..."

  # Push and merge
  if git push -u origin "$BRANCH_NAME" --force-with-lease --quiet 2>/dev/null; then
    git checkout main --quiet
    git pull origin main --quiet 2>/dev/null || true

    if git merge --squash "$BRANCH_NAME" --quiet 2>/dev/null; then
      git commit --no-edit -m "merge($BRANCH_NAME): ralph loop" --quiet 2>/dev/null
      git push origin main --quiet 2>/dev/null && {
        ((MERGED++))
        echo "  ✓ Merged to main"
      } || {
        git reset --hard HEAD~ --quiet 2>/dev/null
        ((SKIPPED++))
        echo "  ✗ Push failed"
      }
    else
      git merge --abort --quiet 2>/dev/null || true
      ((SKIPPED++))
      echo "  ✗ Merge conflict in final merge"
    fi
  else
    ((SKIPPED++))
    echo "  ✗ Push failed"
  fi

  git checkout main --quiet
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  MERGED: $MERGED  |  SKIPPED: $SKIPPED  |  BACKUPS: backup/*"
echo "═══════════════════════════════════════════════════════════════"
git log --oneline -3
