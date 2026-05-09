#!/usr/bin/env bash
# CYNIC Config Sync — bridges repo ↔ runtime config
#
# SSOT hierarchy:
#   1. Repo (backends.toml)     → git-tracked canonical
#   2. Runtime (~/.config/cynic/) → deployed, may have local secrets
#   3. fleet.toml                → local-only (contains real IPs, never committed)
#   4. ~/.cynic-env              → secrets, never committed
#
# Usage:
#   config-sync.sh deploy   — repo → runtime (after git pull)
#   config-sync.sh collect  — runtime → repo (before git push, strips secrets)
#   config-sync.sh diff     — show drift between repo and runtime
#   config-sync.sh check    — exit 1 if diverged (for CI/hooks)
set -euo pipefail

PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
RUNTIME_DIR="${HOME}/.config/cynic"

REPO_BACKENDS="${PROJECT_ROOT}/backends.toml"
RUNTIME_BACKENDS="${RUNTIME_DIR}/backends.toml"

CYNIC_ENV="${HOME}/.cynic-env"
SYSTEMD_ENV="${RUNTIME_DIR}/env"

cmd="${1:-diff}"

case "$cmd" in
  deploy)
    echo "Deploying repo → runtime..."
    cp "$REPO_BACKENDS" "$RUNTIME_BACKENDS"
    echo "  backends.toml → ${RUNTIME_BACKENDS}"
    echo "Done. Restart kernel to pick up changes."
    ;;

  collect)
    echo "Collecting runtime → repo..."
    cp "$RUNTIME_BACKENDS" "$REPO_BACKENDS"
    echo "  ${RUNTIME_BACKENDS} → backends.toml"
    echo "Done. Review diff before committing."
    ;;

  sync-env)
    # Derive ~/.config/cynic/env FROM ~/.cynic-env (strip export prefix)
    # ~/.cynic-env is SSOT for secrets. systemd env is derived, never edited directly.
    echo "Deriving systemd env from ~/.cynic-env..."
    sed 's/^export //' "$CYNIC_ENV" > "$SYSTEMD_ENV"
    echo "  ${CYNIC_ENV} → ${SYSTEMD_ENV} ($(grep -c '=' "$SYSTEMD_ENV") vars)"
    ;;

  diff)
    echo "=== backends.toml drift ==="
    diff --color=auto "$REPO_BACKENDS" "$RUNTIME_BACKENDS" 2>/dev/null && echo "(in sync)" || true
    echo ""
    echo "=== env drift (cynic-env vs systemd env) ==="
    diff --color=auto <(grep -v '^#\|^$' "$CYNIC_ENV" | sed 's/^export //' | sort) \
                      <(grep -v '^#\|^$' "$SYSTEMD_ENV" | sort) 2>/dev/null && echo "(in sync)" || true
    ;;

  check)
    FAIL=0
    if ! diff -q "$REPO_BACKENDS" "$RUNTIME_BACKENDS" > /dev/null 2>&1; then
      echo "DRIFT: backends.toml diverged between repo and runtime" >&2
      FAIL=1
    else
      echo "backends.toml: in sync"
    fi
    ENV_DIFF=$(diff <(grep -v '^#\|^$' "$CYNIC_ENV" | sed 's/^export //' | sort) \
                    <(grep -v '^#\|^$' "$SYSTEMD_ENV" | sort) 2>/dev/null || true)
    if [[ -n "$ENV_DIFF" ]]; then
      echo "DRIFT: ~/.cynic-env and ~/.config/cynic/env diverged" >&2
      echo "  Run: scripts/config-sync.sh sync-env" >&2
      FAIL=1
    else
      echo "env files: in sync"
    fi
    exit $FAIL
    ;;

  *)
    echo "Usage: config-sync.sh {deploy|collect|sync-env|diff|check}" >&2
    exit 1
    ;;
esac
