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
#   config-sync.sh deploy   — resolve placeholders, repo → runtime (after git pull)
#   config-sync.sh collect  — runtime → repo (before git push, strips secrets)
#   config-sync.sh collect --no-secrets — collect with real IP masking
#   config-sync.sh substitute — resolve placeholders in repo backends.toml
#   config-sync.sh hermes-config — resolve placeholders in Hermes ouroboros config
#   config-sync.sh diff     — show drift between repo and runtime
#   config-sync.sh check    — exit 1 if diverged (for CI/hooks)
set -euo pipefail

PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
RUNTIME_DIR="${HOME}/.config/cynic"
SCRIPTS_DIR="${PROJECT_ROOT}/scripts"

REPO_BACKENDS="${PROJECT_ROOT}/backends.toml"
RUNTIME_BACKENDS="${RUNTIME_DIR}/backends.toml"
FLEET_GEN="${SCRIPTS_DIR}/fleet-gen.py"

CYNIC_ENV="${HOME}/.cynic-env"
SYSTEMD_ENV="${RUNTIME_DIR}/env"

cmd="${1:-diff}"

case "$cmd" in
  substitute)
    echo "Resolving placeholders in repo backends.toml..."
    if [[ ! -f "$FLEET_GEN" ]]; then
      echo "Error: fleet-gen.py not found at ${FLEET_GEN}" >&2
      exit 1
    fi
    python3 "$FLEET_GEN" "$REPO_BACKENDS"
    ;;

  hermes-config)
    # Substitute <TAILSCALE_*> placeholders in Hermes ouroboros config template
    # Reads: .hermes_ouroboros/config.yaml.tpl (repo template)
    # Outputs: ~/.config/cynic/hermes-ouroboros-config.yaml (runtime)
    echo "Resolving placeholders in Hermes ouroboros config..."
    HERMES_TEMPLATE="${PROJECT_ROOT}/.hermes_ouroboros/config.yaml.tpl"
    HERMES_RUNTIME="${RUNTIME_DIR}/hermes-ouroboros-config.yaml"

    if [[ ! -f "$HERMES_TEMPLATE" ]]; then
      echo "Error: Hermes config template not found at ${HERMES_TEMPLATE}" >&2
      exit 1
    fi
    if [[ ! -f "$FLEET_GEN" ]]; then
      echo "Error: fleet-gen.py not found at ${FLEET_GEN}" >&2
      exit 1
    fi

    # Use fleet-gen.py to substitute all <TAILSCALE_*> placeholders
    python3 "$FLEET_GEN" "$HERMES_TEMPLATE" > "$HERMES_RUNTIME"

    # Verify no unresolved placeholders remain
    if grep -q '<TAILSCALE_' "$HERMES_RUNTIME"; then
      echo "ERROR: Unresolved <TAILSCALE_*> placeholders remain in ${HERMES_RUNTIME}:" >&2
      grep '<TAILSCALE_' "$HERMES_RUNTIME" >&2
      exit 1
    fi

    echo "  .hermes_ouroboros/config.yaml.tpl (resolved) → ${HERMES_RUNTIME}"
    echo "Done. Note: API keys are injected from ~/.cynic-env by systemd EnvironmentFile."
    ;;

  deploy)
    echo "Deploying repo → runtime (with placeholder resolution)..."
    if [[ ! -f "$FLEET_GEN" ]]; then
      echo "Error: fleet-gen.py not found at ${FLEET_GEN}" >&2
      exit 1
    fi
    # Resolve placeholders and copy to runtime
    python3 "$FLEET_GEN" "$REPO_BACKENDS" > "$RUNTIME_BACKENDS"
    echo "  backends.toml (resolved) → ${RUNTIME_BACKENDS}"
    echo "Done. Restart kernel to pick up changes."
    ;;

  collect)
    if [[ "${2:-}" == "--no-secrets" ]]; then
      echo "Collecting runtime → repo with IP masking..."
      # Mask real Tailscale IPs (100.x.y.z) before committing
      sed 's/100\.[0-9]\{1,3\}\.[0-9]\{1,3\}\.[0-9]\{1,3\}/<TAILSCALE_MASKED>/g' "$RUNTIME_BACKENDS" > "${REPO_BACKENDS}.masked"
      echo "  ${RUNTIME_BACKENDS} → backends.toml.masked (IPs masked)"
      echo "Review, then: cp ${REPO_BACKENDS}.masked ${REPO_BACKENDS}"
    else
      echo "⚠️  WARNING: collect mode exposes real IPs. Use --no-secrets flag instead:"
      echo "    config-sync.sh collect --no-secrets"
      echo ""
      echo "Collecting runtime → repo..."
      cp "$RUNTIME_BACKENDS" "$REPO_BACKENDS"
      echo "  ${RUNTIME_BACKENDS} → backends.toml"
      echo "Done. Review diff before committing."
    fi
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
    echo "Usage: config-sync.sh {deploy|collect|substitute|hermes-config|sync-env|diff|check}" >&2
    echo "       config-sync.sh collect [--no-secrets]" >&2
    exit 1
    ;;
esac
