#!/usr/bin/env bash
# CYNIC — Hard verification that Git hooks resolve to the repo-owned sources.
set -euo pipefail

fail() {
    echo "FAIL Hook Drift: $*" >&2
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
GIT_DIR_RAW="${GIT_DIR_OVERRIDE:-$(git -C "$PROJECT_DIR" rev-parse --git-dir 2>/dev/null || echo "$PROJECT_DIR/.git")}"

case "$GIT_DIR_RAW" in
    /*) GIT_DIR="$GIT_DIR_RAW" ;;
    *) GIT_DIR="$PROJECT_DIR/$GIT_DIR_RAW" ;;
esac

verify_hook() {
    local name="$1"
    local actual="$GIT_DIR/hooks/$name"
    local expected="$PROJECT_DIR/scripts/git-hooks/$name"
    local resolved_actual resolved_expected

    [ -e "$expected" ] || fail "expected source missing: $expected"
    [ -L "$actual" ] || fail "$actual is not a symlink to $expected"
    [ -x "$expected" ] || fail "$expected is not executable"

    resolved_actual="$(resolve_link_target "$actual")" || fail "cannot resolve symlink target: $actual"
    resolved_expected="$(resolve_path "$expected")"

    [ "$resolved_actual" = "$resolved_expected" ] || fail "$actual resolves to $resolved_actual, expected $resolved_expected"
}

verify_hook pre-commit
verify_hook pre-push

echo "✓ Git hooks verified: pre-commit, pre-push"
