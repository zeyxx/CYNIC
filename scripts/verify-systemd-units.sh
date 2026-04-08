#!/usr/bin/env bash
# CYNIC — Hard verification that systemd user units resolve to repo-managed sources.
set -euo pipefail

fail() {
    echo "FAIL Unit Drift: $*" >&2
    exit 1
}

resolve_path() {
    local path="$1"
    local dir base
    dir="$(cd "$(dirname "$path")" && pwd -P)"
    base="$(basename "$path")"
    printf '%s/%s\n' "$dir" "$base"
}

resolve_link_target() {
    local path="$1"
    local target
    target="$(readlink "$path")" || return 1
    case "$target" in
        /*) resolve_path "$target" ;;
        *) resolve_path "$(dirname "$path")/$target" ;;
    esac
}

PROJECT_DIR="${PROJECT_DIR_OVERRIDE:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SYSTEMD_USER_DIR="${SYSTEMD_USER_DIR_OVERRIDE:-$HOME/.config/systemd/user}"

managed_units=(
    "infra/systemd/cynic-kernel.service"
    "infra/systemd/surrealdb.service"
    "infra/systemd/cynic-healthcheck.service"
    "infra/systemd/cynic-healthcheck.timer"
    "infra/systemd/surrealdb-backup.service"
    "infra/systemd/surrealdb-backup.timer"
    "infra/systemd/llama-server.service"
)

for rel in "${managed_units[@]}"; do
    src="$PROJECT_DIR/$rel"
    dst="$SYSTEMD_USER_DIR/$(basename "$rel")"
    [ -e "$src" ] || fail "expected source missing: $src"
    [ -L "$dst" ] || fail "$dst is not a symlink to $src"

    resolved_actual="$(resolve_link_target "$dst")" || fail "cannot resolve symlink target: $dst"
    resolved_expected="$(resolve_path "$src")"
    [ "$resolved_actual" = "$resolved_expected" ] || fail "$dst resolves to $resolved_actual, expected $resolved_expected"
done

echo "✓ Systemd user units verified"
