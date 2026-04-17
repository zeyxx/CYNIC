#!/usr/bin/env bash
set -e

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
LOCK_DIR="${HOME}/.cynic-exercise"
LOCK_FILE="${LOCK_DIR}/$(date +%Y-%m-%d).lock"
WISDOM_FILE="${LOCK_DIR}/wisdom-needed.txt"

mkdir -p "$LOCK_DIR"

# Vérifier si déjà créé aujourd'hui
if [ -f "$LOCK_FILE" ]; then
  echo "[EXERCISE] Event déjà créé aujourd'hui."
  exit 0
fi

# Observer chiffres BRUTS depuis 00:00 today
echo "[OBSERVE] Analyzing git activity since 00:00..."
TODAY_START="$(date +%Y-%m-%d) 00:00:00"

cd "$PROJECT_DIR"
COMMIT_COUNT=$(git log --since="$TODAY_START" --oneline 2>/dev/null | wc -l || echo 0)

if [ "$COMMIT_COUNT" -eq 0 ]; then
  echo "[EXERCISE] Zero commits today → no event."
  exit 0
fi

# Récupérer stats brutes
FILES_CHANGED=$(git diff --name-only HEAD~${COMMIT_COUNT}..HEAD 2>/dev/null | wc -l || echo 0)
STAT_OUTPUT=$(git diff --stat HEAD~${COMMIT_COUNT}..HEAD 2>/dev/null | tail -1 || echo "")
LINES_ADDED=$(echo "$STAT_OUTPUT" | grep -oP '\d+(?= insertion)' || echo 0)
LINES_DELETED=$(echo "$STAT_OUTPUT" | grep -oP '\d+(?= deletion)' || echo 0)

echo "[OBSERVED]"
echo "  Commits: $COMMIT_COUNT"
echo "  Files changed: $FILES_CHANGED"
echo "  Lines +/−: +$LINES_ADDED / −$LINES_DELETED"

# Calculer intensité
WORK_INTENSITY=$((COMMIT_COUNT + FILES_CHANGED / 5))

# Durée conservatrice
DURATION=15
if [ "$WORK_INTENSITY" -ge 2 ] && [ "$WORK_INTENSITY" -lt 5 ]; then
  DURATION=25
elif [ "$WORK_INTENSITY" -ge 5 ] && [ "$WORK_INTENSITY" -lt 8 ]; then
  DURATION=35
elif [ "$WORK_INTENSITY" -ge 8 ]; then
  DURATION=40
fi

echo "[DURATION] $DURATION min (conservative baseline)"
echo "[REALITY] Intensity score: $WORK_INTENSITY"

# Écrire marker pour cynic-wisdom
cat > "$WISDOM_FILE" << EOF
COMMITS=$COMMIT_COUNT
FILES=$FILES_CHANGED
LINES_ADDED=$LINES_ADDED
LINES_DELETED=$LINES_DELETED
DURATION=$DURATION
EOF

echo "[WISDOM] Marker written. Next Gemini session should invoke /cynic-skills:cynic-wisdom"
echo "[FRICTION] Force cognitive: reality check before calendar event creation"

# Lock aujourd'hui
touch "$LOCK_FILE"
echo "✓ Exercise anchor locked for 19:00"
