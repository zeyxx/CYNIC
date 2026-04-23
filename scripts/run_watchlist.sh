#!/bin/bash
# CYNIC Token Watchlist Runner — feeds real tokens to the kernel.
# Closes K15: verdicts must have a consumer (the UI reads /verdicts).
# Usage: ./scripts/run_watchlist.sh [watchlist_file]
# R23-exempt: standalone script, loads own env

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WATCHLIST="${1:-$SCRIPT_DIR/watchlist.txt}"

if [ ! -f "$WATCHLIST" ]; then
    echo "ERROR: watchlist not found: $WATCHLIST" >&2
    exit 1
fi

# Load CYNIC env for kernel submission
[ -f "$HOME/.cynic-env" ] && . "$HOME/.cynic-env"

SUCCESSES=0
FAILURES=0
TOTAL=0

while IFS= read -r mint; do
    # Skip empty lines and comments
    [ -z "$mint" ] && continue
    [[ "$mint" =~ ^# ]] && continue

    TOTAL=$((TOTAL + 1))
    RESULT=$(python3 "$SCRIPT_DIR/token_screener.py" "$mint" 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$RESULT" ]; then
        VERDICT=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('verdict','?'))" 2>/dev/null)
        QSCORE=$(echo "$RESULT" | python3 -c "import sys,json; print(f\"{json.load(sys.stdin).get('q_score',{}).get('total',0):.3f}\")" 2>/dev/null)
        echo "${mint:0:8}... → $VERDICT (Q=$QSCORE)"
        SUCCESSES=$((SUCCESSES + 1))
    else
        echo "${mint:0:8}... → FAIL" >&2
        FAILURES=$((FAILURES + 1))
    fi
done < "$WATCHLIST"

echo "---"
echo "Watchlist: $SUCCESSES/$TOTAL ok, $FAILURES failed"
