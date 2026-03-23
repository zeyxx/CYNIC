#!/usr/bin/env bash
# Chess Benchmark — Progressive A/B stress test for CYNIC crystal loop.
# Usage: ./scripts/chess-bench.sh [wave]
#   wave 1 = 10 stimuli (default)
#   wave 2 = 20 stimuli
#   wave 3 = 30 stimuli
set -euo pipefail
source ~/.cynic-env
exec python3 "$(dirname "$0")/chess-bench.py" "${1:-1}"
