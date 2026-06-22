#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT/organs/discord/state"
exec python3 -m http.server 8892 --bind 127.0.0.1
