#!/usr/bin/env bash
# Canonical formatter entry point for all CYNIC agents.
#
# Usage:
#   scripts/rustfmt-check <crate-dir>...   # verify, exit 1 on diff
#   scripts/rustfmt-apply  <crate-dir>...  # write fixes
#
# With no args: formats all workspace crates.
#
# Law (see rustfmt.toml): edition comes from each crate's Cargo.toml via
# cargo's own resolution; the toolchain from rust-toolchain.toml. Never
# invoke bare `rustfmt --edition ...` — that's how the format war started.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"

CRATES_DEFAULT="cynic-protocol cynic-host cynic-kernel cynic-node cynic-cli cynic-inference cynic-askesis"

crates=""
mode="check"
for arg in "$@"; do
  case "$arg" in
    --apply) mode="apply" ;;
    *) crates="$crates $arg" ;;
  esac
done
[ -z "$crates" ] && crates="$CRATES_DEFAULT"

rc=0
for crate in $crates; do
  dir="${repo_root}/crates/${crate}"
  [ -d "$dir" ] || { echo "skip (no dir): $crate"; continue; }
  cd "$dir"
  echo "== $crate =="
  if [ "$mode" = "apply" ]; then
    CARGO_BUILD_JOBS=1 ../../scripts/cargo --local-dev fmt
  else
    if ! CARGO_BUILD_JOBS=1 ../../scripts/cargo --local-dev fmt -- --check; then
      echo "FORMATTING DRIFT in $crate (run: scripts/rustfmt-apply $crate)"
      rc=1
    fi
  fi
done
exit $rc
