#!/bin/bash
# K15 Seam 2 Feedback Loop — Judge observations and learn patterns
#
# This script runs the complete feedback cycle:
#   1. judge_observations.py → sends observations to kernel /judge
#   2. gemini_learn_from_verdicts.py → extracts patterns from verdicts
#   3. Result: SKILL.md updated for Hermes 9B
#
# Usage:
#   ./run_feedback_loop.sh --organ-dir ~/.cynic/organs/hermes/x

set -e

ORGAN_DIR="${HOME}/.cynic/organs/hermes/x"

# Parse args
while [[ $# -gt 0 ]]; do
    case $1 in
        --organ-dir) ORGAN_DIR="$2"; shift 2 ;;
        *) echo "unknown flag: $1"; exit 1 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "K15 Seam 2: Running feedback loop"
echo "  Organ: $ORGAN_DIR"
echo

# Step 1: Judge observations
echo "[1/2] Judging observations..."
if python3 "$SCRIPT_DIR/judge_observations.py" --organ-dir "$ORGAN_DIR" --limit 10; then
    echo "✓ Observations judged"
else
    echo "⚠ Judgment step failed (kernel may be offline)"
    echo "  Continuing to learning step anyway..."
fi

echo

# Step 2: Learn from verdicts
echo "[2/2] Learning from verdicts..."
if python3 "$SCRIPT_DIR/gemini_learn_from_verdicts.py" --organ-dir "$ORGAN_DIR"; then
    echo "✓ SKILL.md updated"
else
    echo "✗ Learning step failed"
    exit 1
fi

echo
echo "✓ Feedback loop complete"
echo "  Next session: Hermes will read updated SKILL.md"
