#!/usr/bin/env bash
# CYNIC — Hard verification that systemd user units resolve to repo-managed sources.
# Scans infra/systemd/ dynamically — no hardcoded list.
set -euo pipefail

fail() {
    echo "FAIL Unit Drift: $*" >&2
    FAILURES=$((FAILURES + 1))
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
SOURCE_DIR="$PROJECT_DIR/infra/systemd"

FAILURES=0
CHECKED=0

for src in "$SOURCE_DIR"/*.service "$SOURCE_DIR"/*.timer; do
    [ -f "$src" ] || continue
    name="$(basename "$src")"

    # Skip template units (foo@.service) and scripts
    case "$name" in *@.*|*.sh) continue ;; esac

    dst="$SYSTEMD_USER_DIR/$name"
    CHECKED=$((CHECKED + 1))

    if [ ! -e "$dst" ]; then
        fail "$name: not deployed (missing from $SYSTEMD_USER_DIR)"
        continue
    fi

    if [ ! -L "$dst" ]; then
        # Regular file, not symlink — check content drift
        if ! diff -q "$dst" "$src" >/dev/null 2>&1; then
            fail "$name: regular file with content drift (not a symlink, content differs from repo)"
        else
            echo "WARN $name: regular file (not a symlink) but content matches repo"
        fi
        continue
    fi

    resolved_actual="$(resolve_link_target "$dst")" || { fail "$name: cannot resolve symlink target"; continue; }
    resolved_expected="$(resolve_path "$src")"
    if [ "$resolved_actual" != "$resolved_expected" ]; then
        fail "$name: resolves to $resolved_actual, expected $resolved_expected"
    fi
done

if [ "$FAILURES" -gt 0 ]; then
    echo "✗ $FAILURES unit(s) drifted out of $CHECKED checked"
    exit 1
fi

echo "✓ $CHECKED systemd units verified (0 drift)"
