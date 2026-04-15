# Infra Build — A1 Debt

## Problem

`cynic-kernel` currently hits an upstream Rust/LLVM build debt around `rmcp` in debug builds.
The failure is reproducible on `cargo build -p cynic-kernel` without extra flags:

- `rust-lld: error: (.debug_str): offset is outside the section`
- `relocation R_X86_64_32 out of range`
- the failing object comes from `librmcp`

This is not a CYNIC logic bug. It is a toolchain stress case caused by very large debug sections.

## Root Cause

`rmcp` pulls together:

- heavy `serde` derives
- `schemars` schema generation
- MCP macro expansion
- large generic async service surfaces

That combination creates unusually large monomorphized object files. In debug profile, full DWARF metadata amplifies the size again. The linker then fails on debug section range limits before the binary is produced.

The stack issue is related but separate:

- deep Rust/LLVM passes around the same codepath need extra stack headroom
- too little `RUST_MIN_STACK` causes unstable compiler behavior on rmcp-heavy builds

## Decision

The repo standardizes on:

```bash
RUSTFLAGS="-C debuginfo=1"
RUST_MIN_STACK=67108864
```

Why these values:

- `debuginfo=1` keeps useful symbols while shrinking debug sections enough for `rust-lld`
- `67108864` (64 MiB) is the robustness default for current rmcp-heavy builds in this repo
- `16777216` (16 MiB) is only a lower bound that may pass on some paths and still fail intermittently
- larger values may still be used ad hoc for toolchain experiments, but they are not the project default

## Source Of Truth

Build policy lives in three layers, with different responsibilities:

1. `.cargo/config.toml`
   Sets repo-local defaults so plain `cargo build` works.
2. `cynic-kernel/build.rs`
   Cannot mutate the parent Cargo invocation, so it validates and warns when the environment drifts.
3. `.claude/rules/workflow.md`
   Documents the same requirement for humans and agents running commands outside normal repo-local Cargo config.

This split is intentional. `build.rs` is not a magic control plane for Cargo itself; it runs too late to inject parent `RUSTFLAGS`.

## Validation

Observed on 2026-04-15:

- `cargo build -p cynic-kernel` fails in debug without reduced debuginfo
- `RUSTFLAGS="-C debuginfo=1" cargo build -p cynic-kernel` succeeds
- the failure is centered on `rmcp` debug sections, not runtime code

## Exit Criteria

This A1 debt can be removed when all of the following are true:

- the pinned Rust toolchain no longer reproduces the rmcp debug-link failure
- `cargo build -p cynic-kernel` succeeds without custom `RUSTFLAGS`
- release and debug builds both pass without elevated `RUST_MIN_STACK`

Until then, treat this as mandatory infrastructure policy, not optional troubleshooting.
