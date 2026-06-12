#!/usr/bin/env bash
set -euo pipefail

# Source local coordination env and export its assignments for the MCP process.
case "$-" in
    *a*) had_allexport=1 ;;
    *) had_allexport=0 ;;
esac
set -a
source ~/.cynic-env >/dev/null 2>&1 || true
if [[ "$had_allexport" -eq 0 ]]; then
    set +a
fi

# Exec the real MCP binary
exec "$HOME/bin/cynic-kernel-mcp"
