#!/bin/bash
set -euo pipefail
source "$HOME/.cargo/env"

echo "[cynic] clippy..."
cargo clippy --all-targets --workspace -- -D warnings

echo "[cynic] test..."
SURREALDB_URL=ws://localhost:8000 \
SURREALDB_USER=root \
SURREALDB_PASS=$(cat ~/.surreal-pass) \
cargo test --workspace

echo "[cynic] audit..."
cargo audit --ignore RUSTSEC-2023-0071 2>/dev/null || true
