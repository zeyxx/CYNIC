#!/usr/bin/env bash
set -euo pipefail

if [[ -f "$HOME/.cynic-env" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$HOME/.cynic-env"
  set +a
fi

export RUST_LOG="${RUST_LOG:-off}"
exec /home/user/bin/cynic-kernel-mcp
