#!/usr/bin/env bash
set -euo pipefail

# Shared fragmentation guard for CYNIC git hooks.
# Prevent accidental markdown snapshots in runtime directories.

POLICY_FILE="$(dirname "$0")/markdown-fragmentation-policy.json"

POLICY_SPECIALS=()
POLICY_CANONICAL=()
POLICY_CANONICAL_PREFIXES=()
POLICY_CANONICAL_ROOTS=()
POLICY_RUNTIME_PREFIXES=()

load_list_from_policy() {
    local field="$1"
    local -n out_list="$2"

    if ! command -v jq >/dev/null 2>&1; then
        return 1
    fi
    if [ ! -f "$POLICY_FILE" ]; then
        return 1
    fi

    mapfile -t out_list < <(
        jq -r ".${field}[]? | select(type==\"string\")" "$POLICY_FILE" 2>/dev/null \
            | sed '/^$/d'
    )

    if [ "${#out_list[@]}" -eq 0 ]; then
        return 1
    fi
    return 0
}

if ! load_list_from_policy "canonical_prefixes" POLICY_CANONICAL_PREFIXES; then
    POLICY_CANONICAL_PREFIXES=(
        "docs/"
        "packages/"
        "crates/"
        "organs/"
        "cynic/"
        ".cortex/"
        ".cynic/"
        "data/"
        ".github/"
    )
fi

if ! load_list_from_policy "special_allowed" POLICY_SPECIALS; then
    POLICY_SPECIALS=(
        "scripts/README-token-screening.md"
        "scripts/hermes-x/HERMES_LOGIN_SETUP.md"
        "organs/*/state/README.md"
        "organs/*/state/SKILL.md"
        "apps/kyon-mobile/README.md"
    )
fi

if ! load_list_from_policy "canonical_patterns" POLICY_CANONICAL; then
    POLICY_CANONICAL=(
        "organs/*/README.md"
        "organs/*/SKILL.md"
        "organs/twitter/*.md"
        "organs/twitter/src/*.md"
        "organs/organ-cloud/*.md"
        "packages/*/README.md"
        "packages/*/CLAUDE.md"
        "crates/*/README.md"
        "crates/*/BENCHMARK_ROUTING.md"
        "crates/*/domains/*.md"
        "crates/percolator-cli/*.md"
        "crates/percolator-cli/DEPLOY_CHECKLIST.md"
    )
fi

if ! load_list_from_policy "canonical_roots" POLICY_CANONICAL_ROOTS; then
    POLICY_CANONICAL_ROOTS=(
        "CLAUDE.md"
        "AGENTS.md"
        "CODEX.md"
        "ANTIGRAVITY.md"
        "README.md"
        "CHANGELOG.md"
        "API.md"
        "VERSION.md"
        "TOPOLOGY.md"
        ".codex/CODEX-PROTOCOL.md"
        "packages/cynic-ui/CLAUDE.md"
        "packages/talaria-demo/README.md"
        "packages/talaria-landing/README.md"
        "crates/cynic-askesis/README.md"
        "apps/kyon-mobile/README.md"
    )
fi

if ! load_list_from_policy "runtime_prefixes" POLICY_RUNTIME_PREFIXES; then
    POLICY_RUNTIME_PREFIXES=(
        "services/"
        "infra/"
        "scripts/"
        "cynic-python/"
        "apps/"
        "organs/*/state/"
    )
fi

if [ "${ALLOW_MARKDOWN_FRAGMENTATION_GUARD:-}" = "1" ]; then
    exit 0
fi

MODE=${1:-commit}
FILE_LIST=${2:-}
if [ -z "$FILE_LIST" ]; then
    exit 0
fi

is_allowed_special_md() {
    local path="$1"
    if [[ "$path" != *.md ]]; then
        return 1
    fi

    local pattern
    for pattern in "${POLICY_SPECIALS[@]}"; do
        case "$path" in
            $pattern)
                return 0
                ;;
        esac
    done
    return 1
}

FORBIDDEN_MD=()
while IFS= read -r path; do
  case "$path" in
    *.md)
      if is_allowed_special_md "$path"; then
        continue
      fi
      allowed=0

      # Runtime/runtime-like trees: no markdown drift unless explicitly centralized.
      for pattern in "${POLICY_RUNTIME_PREFIXES[@]}"; do
          case "$path" in
              $pattern|$pattern*)
                  FORBIDDEN_MD+=( "$path" )
                  continue 2
                  ;;
          esac
      done

      # Canonical documentation planes.
      for pattern in "${POLICY_CANONICAL_PREFIXES[@]}"; do
          case "$path" in
              $pattern*)
                  allowed=1
                  ;;
          esac
      done
      if [ $allowed -eq 1 ]; then
          continue
      fi

      # Canonical documentation exact files.
      for pattern in "${POLICY_CANONICAL[@]}"; do
          case "$path" in
              $pattern)
                  allowed=1
                  ;;
          esac
      done
      if [ $allowed -eq 1 ]; then
          continue
      fi

      # Root protocol/manifest markdown files.
      for pattern in "${POLICY_CANONICAL_ROOTS[@]}"; do
          if [ "$path" = "$pattern" ]; then
              allowed=1
          fi
      done
      if [ $allowed -eq 1 ]; then
          continue
      fi

      # Unknown paths are treated as forbidden to avoid policy gray zones.
      FORBIDDEN_MD+=( "$path" )
      ;;
    *) : ;;
  esac
done <<EOF
$(printf '%s\n' "$FILE_LIST")
EOF

if [ ${#FORBIDDEN_MD[@]} -eq 0 ]; then
    exit 0
fi

echo "FRAGMENTATION GATE ($MODE): markdown updates in runtime directories detected:"
printf '%s\n' "${FORBIDDEN_MD[@]}"
echo ""
echo "Move canonical docs under docs/ and keep runtime artifacts in explicit state paths or kernel stream."
echo "If this is intentional, set ALLOW_MARKDOWN_FRAGMENTATION_GUARD=1 for this ${MODE}."
exit 1
