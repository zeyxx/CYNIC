#!/usr/bin/env bash
# CYNIC — Bootstrap Git Hygiene Sense (Tier 1 EXPERIMENTAL)
#
# One-time script: reconstructs historical git hygiene metrics from git history
# and POSTs them as backdated observations to /observe domain=git-hygiene.
# Gives CCM a baseline from birth so thresholds emerge from data, not assumptions.
#
# Run once, verify, delete. If not promoted to Tier 2 by 2026-06-08, delete.
#
# Usage: ./scripts/bootstrap-git-hygiene.sh [--dry-run]
set -euo pipefail

DRY_RUN=false
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=true

source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

PROJECT_DIR=$(git rev-parse --show-toplevel 2>/dev/null || pwd)

# ── Verify kernel is up ──
if ! $DRY_RUN; then
    HEALTH=$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/health" 2>/dev/null || echo "000")
    if [[ "$HEALTH" != "200" && "$HEALTH" != "503" ]]; then
        echo "ERROR: kernel not reachable (HTTP ${HEALTH}). Start kernel first." >&2
        exit 1
    fi
fi

# ── Precompute all data into temp files for efficient per-day lookup ──
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Collecting data..."

# Branch creation timestamps (unix epoch)
git -C "$PROJECT_DIR" for-each-ref \
    --format='%(creatordate:unix)' refs/heads/ 2>/dev/null \
    | sort -n > "$TMPDIR/branch_ts.txt"

# Commits per day
git -C "$PROJECT_DIR" log --format='%ai' --all 2>/dev/null \
    | cut -d' ' -f1 | sort | uniq -c | awk '{print $2, $1}' > "$TMPDIR/commits_per_day.txt"

# Merged PR dates
PR_DATES_FILE="$TMPDIR/pr_dates.txt"
touch "$PR_DATES_FILE"
if command -v gh &>/dev/null; then
    gh pr list --state merged --limit 200 --json mergedAt 2>/dev/null \
        | jq -r '.[].mergedAt // empty' 2>/dev/null \
        | cut -dT -f1 | sort | uniq -c | awk '{print $2, $1}' > "$PR_DATES_FILE" || true
fi

# ── Date range ──
# pipefail + head = SIGPIPE; isolate in subshell with pipefail off
FIRST_COMMIT_DATE=$(set +o pipefail; git -C "$PROJECT_DIR" rev-list --all --reverse 2>/dev/null | head -1 | xargs -I{} git -C "$PROJECT_DIR" log -1 --format='%ad' --date=short {} 2>/dev/null || true)
TODAY=$(date +%Y-%m-%d)

if [[ -z "$FIRST_COMMIT_DATE" ]]; then
    echo "ERROR: no commits found" >&2
    exit 1
fi

echo "Date range: ${FIRST_COMMIT_DATE} → ${TODAY}"

# ── Walk each day ──
POSTED=0
CURRENT="$FIRST_COMMIT_DATE"

while [[ "$CURRENT" < "$TODAY" || "$CURRENT" == "$TODAY" ]]; do
    NEXT_TS=$(date -d "$CURRENT 23:59:59" +%s 2>/dev/null || echo 0)
    [[ "$NEXT_TS" -eq 0 ]] && { CURRENT=$(date -d "$CURRENT + 1 day" +%Y-%m-%d); continue; }

    # Branches created on or before this date (monotonically increasing — count lines <= threshold)
    BRANCHES_ON_DATE=$(awk -v ts="$NEXT_TS" '$1 <= ts {c++} END {print c+0}' "$TMPDIR/branch_ts.txt")

    # Commits this day
    COMMITS_TODAY=$(awk -v d="$CURRENT" '$1 == d {print $2; exit}' "$TMPDIR/commits_per_day.txt")
    COMMITS_TODAY=${COMMITS_TODAY:-0}

    # PRs merged this day
    PRS_MERGED=$(awk -v d="$CURRENT" '$1 == d {print $2; exit}' "$PR_DATES_FILE")
    PRS_MERGED=${PRS_MERGED:-0}

    CTX="branches_local=${BRANCHES_ON_DATE} commits_today=${COMMITS_TODAY} prs_merged=${PRS_MERGED} stashes_total=unknown stash_max_age_days=unknown"

    if $DRY_RUN; then
        echo "[DRY] ${CURRENT}: ${CTX}"
    else
        curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
            -H "Content-Type: application/json" \
            ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
            -d "{\"agent_id\":\"bootstrap\",\"tool\":\"git_hygiene_sense\",\"target\":\"bootstrap\",\"domain\":\"git-hygiene\",\"context\":\"${CTX}\",\"tags\":[\"git-hygiene\",\"sense\",\"bootstrap\"],\"consumer\":\"ccm\",\"action\":\"crystallize git hygiene patterns\"}" \
            > /dev/null 2>&1 || true
        POSTED=$((POSTED + 1))
    fi

    CURRENT=$(date -d "$CURRENT + 1 day" +%Y-%m-%d)
done

if $DRY_RUN; then
    echo "Dry run complete. Would have posted ${POSTED:-~60} observations."
else
    echo "Bootstrap complete: ${POSTED} observations posted to domain=git-hygiene"
    echo "Verify: curl -s \${CYNIC_REST_ADDR}/observations?domain=git-hygiene&limit=5 -H \"Authorization: Bearer \${CYNIC_API_KEY}\""
fi
