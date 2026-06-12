#!/usr/bin/env bash
set -euo pipefail

# Source local coordination env
source ~/.cynic-env >/dev/null 2>&1 || true

# Exec the real MCP binary
exec "$HOME/bin/cynic-kernel-mcp"
