# LLVM SIGSEGV with codegen-units=1 on Rust 1.95.0

**Date**: 2026-05-14
**Status**: Workaround applied (codegen-units=16)
**Upstream**: To be filed at https://github.com/rust-lang/rust

## Environment

- **rustc**: 1.95.0 (59807616e 2026-04-14)
- **LLVM**: 22.1.2
- **Host**: x86_64-unknown-linux-gnu
- **OS**: Linux 6.17.0-23-generic (Ubuntu 24.04)
- **RAM**: 28 GiB (8.4 GiB available at time of crash)

## Symptoms

Release builds with `codegen-units = 1` crash with SIGSEGV in LLVM's FastISel pass. The crashing crate is **non-deterministic** — different crate each run:

| Run | Crashing crate | Signal | LLVM function |
|-----|---------------|--------|---------------|
| 1 | `yoke-derive` | SIGSEGV | `FastISel::selectInstruction` |
| 2 | `zerovec` | SIGSEGV | `FastISel::selectInstruction` |
| 3 | `regex-syntax` | SIGSEGV | (backtrace not captured) |
| 4 | `serde_json` | SIGSEGV | (backtrace not captured) |

Additionally, a separate ICE (SIGABRT) occurs in clippy's `redundant_clone` lint when the incremental cache is warm:

| Run | Crashing crate | Signal | Rust function |
|-----|---------------|--------|---------------|
| 0 | `cynic_kernel` | SIGABRT | `ProjectionElem::decode` (expected 0..8, got 221) |

## Reproduction

```toml
# Cargo.toml
[profile.release]
opt-level = 1
codegen-units = 1
```

```bash
# Crashes non-deterministically
cargo test -p cynic-kernel --lib --release

# Works reliably
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo test -p cynic-kernel --lib --release
```

## What does NOT help

| Mitigation | Result |
|-----------|--------|
| `RUST_MIN_STACK=134217728` (128 MiB) | Still SIGSEGV (different crate) |
| `RUST_MIN_STACK=268435456` (256 MiB) | Still SIGSEGV (different crate) |
| Remove `-C debuginfo=1` | Still SIGSEGV (different crate) |
| `cargo clean` + fresh build | Still SIGSEGV |
| `rm -rf target/release/` between runs | Still SIGSEGV |

## What fixes it

| Mitigation | Result |
|-----------|--------|
| `codegen-units = 16` (default) | 685 tests pass reliably |
| `cargo check` (no codegen) | Passes (235 crates) |

## Compiler flags at crash time

From the failed rustc invocation:
```
-C opt-level=1 -C embed-bitcode=no -C codegen-units=1 -C strip=debuginfo -C debuginfo=1
```

## Backtrace (crash 1: yoke-derive)

```
FastISel::selectInstruction → SelectionDAGISel::SelectAllBasicBlocks →
SelectionDAGISel::runOnMachineFunction → FPPassManager::runOnFunction →
FPPassManager::runOnModule → legacy::PassManagerImpl::run →
LlvmCodegenBackend::codegen → WriteBackendMethods::codegen
```

## Backtrace (crash 0: clippy incremental cache)

```
ProjectionElem::decode (expected 0..8, actual 221) →
Place::decode → Body::decode → try_load_from_disk →
RedundantClone::check_fn → visit_fn
```

## History

- **2026-04-06**: `codegen-units=1` was added as a mitigation for an EARLIER LLVM bug where multi-unit codegen caused SIGSEGV during monomorphization fan-out.
- **2026-04-14**: Rust 1.95.0 released with LLVM 22.1.2.
- **2026-05-14**: `codegen-units=1` now itself causes SIGSEGV. Reverted to default (16).

## Analysis

The non-deterministic crate and the LLVM backtrace pointing to `FastISel::selectInstruction` suggest memory corruption within LLVM's code generation pass when constrained to a single codegen unit. With `codegen-units=1`, all monomorphized functions are emitted into a single LLVM module — this likely exposes a bug in LLVM 22.1.2's FastISel that doesn't manifest when the IR is split across multiple modules.

## Related issues

- rust-lang/rust#103767 (LLVM stack overflow on deep monomorphization)
- rust-lang/rust#122357 (SIGSEGV in codegen)
- rust-lang/rust#138561 (LLVM SROA bug)

## Workaround

```toml
[profile.release]
opt-level = 1
codegen-units = 16  # default; codegen-units=1 triggers LLVM SIGSEGV
```
