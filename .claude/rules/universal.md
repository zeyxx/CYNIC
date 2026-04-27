---
description: Core development principles — always active
globs: ["**"]
---

## Meta-Principle

**Fix → Test → Gate → Verify.** Every fix: code change + regression test + mechanical gate + gate verification. Enforcement must be mechanical, not LLM compliance.

## Enforced Rules (verified by make check / hooks)

1. **Zero hardcoded paths.** Use `$(git rev-parse --show-toplevel)` or `${CYNIC_REST_ADDR}`. Never absolute paths in skills, hooks, or configs. — `make lint-rules`
2. **Handle all fallible I/O.** Propagate with `?` or log + retry/skip. No silent `.ok()`. — `make lint-rules` (full `src/` scope; exceptions: probes, `to_str()`, `from_utf8`, `filter_map`, `lock()`/`read()` per K14, `create_dir_all`/`remove_file`)
3. **Every producer needs a consumer.** `store_*` without a read path = invisible waste. — `make lint-drift`
4. **Commit before completing.** `git status --short` at session end must show 0 modified files. — `session-stop.sh` (warns)
5. **No dead architecture.** Commented modules need `DORMANT:` tag. Skills in CLAUDE.md must exist on disk. Hooks on disk must be wired in settings. Crate deps in Cargo.toml must have ≥1 `use` in `src/` — dead deps rot silently until audit flags them (bincode 2026-04-27). — `make lint-drift`

## Design Principles (judgment guidance — no mechanical gate)

6. **Diagnose before fixing.** Read errors, trace data, one hypothesis, test minimally. 2 fix attempts max — obvious → alternative → escalate. Never brute-force. Memory about runtime state is stale by default — probe live (`curl`, DB query) before acting on memory claims about what's running.
7. **Measure before AND after.** Every "improved X" claim needs before/after numbers. Part of the Scientific Protocol (workflow.md).
8. **Port contracts first.** New dependency → trait → adapter → test.
9. **Bounded everything.** Channels, retries, confidence. Unbounded = debt.
10. **Wire or delete.** Every public symbol must have a caller AND a test. Zero-caller = delete.
11. **Fix the class, not the instance.** `grep` the entire codebase for the same pattern before closing a fix.
12. **One value, one source.** `backends.toml` = Dogs. `~/.cynic-env` = secrets. Never duplicate config. Applies to sensing too: if two systems measure the same signal, delete one — dual sensing diverges silently.
13. **Name things for what they ARE.** Code names match code behavior, not aspirations.
14. **Strong > no > weak foundation.** Prove E2E with real data before building on a subsystem.
15. **Falsify before adopting.** Architectural decisions require: hypothesis, falsification test, evidence from production systems. Apply the Scientific Protocol (workflow.md) — state what would make you reject the approach before starting.
16. **Gate at the lowest common caller.** Security gates live at the function ALL paths call, not at one convenient caller.
17. **Bugs before abstractions.** Data-corrupting SQL bug outranks a compile-time newtype. Fix what breaks data first.
18. **Deploy from main only.** Feature branches never touch production binaries.
19. **Scripts are thin.** Bash = `curl + status code`. Logic lives in the kernel.
20. **Every feature must DO what it CLAIMS.** No relabeling — if it says "enforced," there must be a gate.
21. **Verify gates catch violations.** Every new gate (lint target, hook, check) must be tested against a known violation before trusting. A gate that has never failed is not a gate. — `make lint-rules`, `make lint-security`
22. **USE before architecture.** Trust comes from measured results on real data, not from infrastructure. Test on a second domain before adding plumbing. 20 days of infra without a user experiment = yak shaving.
23. **Subprocess env is explicit, not inherited.** Hooks, scripts, and `tokio::process::Command` do NOT inherit `.cargo/config.toml`, project `edition`, or `RUSTFLAGS` the way interactive `cargo` does. Every `rustfmt`/`cargo`/external-tool invocation from subprocess context must set the project's env/flags explicitly (`--edition 2024`, `RUST_MIN_STACK=67108864`, `RUSTFLAGS="-C debuginfo=1"`). Default inheritance = silent drift. Seen 2026-04-16 × 2: rustfmt-rs.sh fighting `cargo fmt` on import ordering (edition), and MCP `run_validate` missing rmcp linker env. File-level exemption: `# R23-exempt: <reason>` (shell) / `// R23-exempt: <reason>` (Rust). — `make lint-subprocess-env` (R21 falsified in `make test-gates`: R23a cargo, R23b rustfmt, R23c Command::new)
