#!/usr/bin/env bash
# KYON K15 Consumer — wrapper script
set -euo pipefail
cd "$(dirname "$0")/.."
HERMES_VENV="${HERMES_VENV:-$HOME/.cynic-venvs/hermes-x}"
exec "${HERMES_VENV}/bin/python3" scripts/hermes-x/consumers/k15_observation_consumer.py "$@"
