#!/usr/bin/env bash
# CYNIC — R23 gate: subprocess env is explicit, not inherited
# Enforces universal rule #23: hooks, scripts, and tokio::process::Command
# do NOT inherit .cargo/config.toml the way interactive cargo does. Every
# `rustfmt`/`cargo` invocation from subprocess context must set env/flags
# explicitly.
#
# Scope:
#   Shell: .claude/hooks/*.sh + scripts/*.sh + scripts/git-hooks/*
#     - `cargo (build|test|clippy|run|check|audit)` → script must set RUST_MIN_STACK
#     - `rustfmt ...`                              → invocation line must set --edition
#     - `cargo fmt` is NOT flagged (no compilation, edition inherited from crate)
#   Rust:  cynic-kernel/src/**/*.rs + cynic-node/src/**/*.rs
#     - Command::new("cargo")   → file must contain .env("RUST_MIN_STACK"
#     - Command::new("rustfmt") → file must contain "--edition"
#
# Exemption: file-level `# R23-exempt: <reason>` (shell) or `// R23-exempt:` (Rust)
#
# Exit codes:
#   0 — no violations
#   1 — at least one violation
#
# Usage: scripts/lint-subprocess-env.sh
set -euo pipefail

PROJECT_DIR=${PROJECT_DIR_OVERRIDE:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}
cd "$PROJECT_DIR"

FAIL=0

# ── Shell scripts ────────────────────────────────────────────
shell_paths=()
for p in .claude/hooks scripts scripts/git-hooks; do
    [ -d "$p" ] || continue
    while IFS= read -r f; do shell_paths+=("$f"); done < <(find "$p" -maxdepth 1 -type f \( -name '*.sh' -o -name 'pre-commit' -o -name 'pre-push' -o -name 'commit-msg' \))
done

for f in "${shell_paths[@]}"; do
    [ -f "$f" ] || continue

    # File-level exemption
    if grep -qE '^#[[:space:]]*R23-exempt:' "$f"; then continue; fi

    # Cargo compile invocations: match on file first (so ^ anchors work), then
    # filter out comment lines and echo-prefixed lines (output text, not exec).
    cargo_compile=$(grep -nE '(^|[[:space:]\|&;])cargo[[:space:]]+(build|test|clippy|run|check|audit)([[:space:]]|$)' "$f" 2>/dev/null \
        | grep -vE '^[0-9]+:[[:space:]]*#' \
        | grep -vE '^[0-9]+:[[:space:]]*echo([[:space:]]|$)' \
        || true)

    if [ -n "$cargo_compile" ]; then
        if ! grep -qE '(^|[[:space:]])(export[[:space:]]+)?RUST_MIN_STACK=' "$f"; then
            echo "FAIL R23: $f — cargo compile invocation without RUST_MIN_STACK export:"
            printf '%s\n' "$cargo_compile" | sed 's/^/    /'
            FAIL=1
        fi
    fi

    # Standalone rustfmt invocations: match on file first, filter comments/echo,
    # then each remaining line must contain --edition.
    rustfmt_lines=$(grep -nE '(^|[[:space:]\|&;])rustfmt([[:space:]]|$)' "$f" 2>/dev/null \
        | grep -vE '^[0-9]+:[[:space:]]*#' \
        | grep -vE '^[0-9]+:[[:space:]]*echo([[:space:]]|$)' \
        || true)

    if [ -n "$rustfmt_lines" ]; then
        while IFS= read -r line; do
            [ -z "$line" ] && continue
            if ! printf '%s' "$line" | grep -qE -- '--edition[[:space:]=]'; then
                echo "FAIL R23: $f — rustfmt invocation without --edition:"
                echo "    $line"
                FAIL=1
            fi
        done <<< "$rustfmt_lines"
    fi
done

# ── Rust source files ────────────────────────────────────────
rust_files=$(grep -rln -E 'Command::new\("(cargo|rustfmt)"\)' \
    cynic-kernel/src cynic-node/src 2>/dev/null || true)

for f in $rust_files; do
    # File-level exemption
    if grep -qE '^//[[:space:]]*R23-exempt:' "$f"; then continue; fi

    if grep -q 'Command::new("cargo")' "$f"; then
        if ! grep -q '\.env("RUST_MIN_STACK"' "$f"; then
            echo "FAIL R23: $f — Command::new(\"cargo\") without .env(\"RUST_MIN_STACK\", ...)"
            FAIL=1
        fi
    fi

    if grep -q 'Command::new("rustfmt")' "$f"; then
        if ! grep -qE '"\-\-edition"' "$f"; then
            echo "FAIL R23: $f — Command::new(\"rustfmt\") without \"--edition\" arg"
            FAIL=1
        fi
    fi
done

if [ $FAIL -eq 0 ]; then
    echo "✓ R23: subprocess env is explicit"
fi
exit $FAIL
