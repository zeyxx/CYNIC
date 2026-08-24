#!/usr/bin/env nix-shell
#!nix-shell -i bash -p python3
# Isolated build runner for cynic crates — avoids rustc ICE segfaults.
# Uses codegen-units=1 + no-embed-bitcode + high stack.
# Ref: AGENTS.md "Industrial Bar" validation policy.
set -euo pipefail
ulimit -s unlimited
export RUST_MIN_STACK="${RUST_MIN_STACK:-33554432}"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-1.96.0}"

cd "$(dirname "$0")"

cargo test "$@" \
  -Z build-std=std,panic_abort \
  -C link-arg=-Wl,--no-check-sections \
  --config 'build.jobs=1' \
  "$@"
