#!/bin/bash
set -euo pipefail
source "$HOME/.cargo/env"

# Detect what changed since last validated commit
CHANGED_FILES=$(git diff --name-only HEAD~1 2>/dev/null || echo "unknown")
RUST_CHANGED=$(echo "$CHANGED_FILES" | grep -E '\.(rs|toml)$' || true)
SKIP_RUST=false

if [ -z "$RUST_CHANGED" ] && [ "$CHANGED_FILES" != "unknown" ]; then
    echo "[cynic] No .rs/.toml files changed — skipping Rust validation"
    SKIP_RUST=true
fi

if [ "$SKIP_RUST" = false ]; then
    echo "[cynic] clippy..."
    cargo clippy --all-targets --workspace -- -D warnings

    echo "[cynic] test..."
    SURREALDB_URL=ws://localhost:8000 \
    SURREALDB_USER=root \
    SURREALDB_PASS=$(cat ~/.surreal-pass) \
    cargo test --workspace

    echo "[cynic] audit..."
    cargo audit --ignore RUSTSEC-2023-0071 2>/dev/null || true
fi
