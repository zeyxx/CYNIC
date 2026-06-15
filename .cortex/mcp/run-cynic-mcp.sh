#!/usr/bin/env bash
set -euo pipefail

set -a
. "$HOME/.cynic-env"
set +a

exec "$HOME/bin/cynic-kernel-mcp"
