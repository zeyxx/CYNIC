# CYNIC — Minimal Agent Protocol

CYNIC is a low-layer sovereign Rust runtime for multi-agent systems. No mutation without read-back proof.

## Gates & Build Policy

- **Builder SSOT**: `10.10.10.119`. NEVER run cargo/rustc locally on cortex (faulty RAM).
- **Canonical gates**: `cynic audit crate` (`make audit-crate`; `make check` is a deprecated alias), `cynic audit all` (`make audit`), `make release.deploy`.
- **Fast checks**: `RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib`, `cynic audit all --json`.
- Note: `make build` and `make build-release` do not exist.

## Authority SSOT

1. Observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json` (hosts, IPs, paths — never hardcode)
5. `Cargo.toml` workspace & crates under `crates/` (census in `infra/crate-topology.json`)

## Policy Chain

- Applied entry: this file. Every agent reads it.
- Citizen law: `docs/specs/agent-constitution.md` (scopes, budgets, admission, proof, non-amplification).
- Role capabilities: `infra/agent-roles.json` (machine-read; enforced by Rust where loaded).
- One statement, one home. Pointers, never copies.

- **Facts dictate**: never ask the operator what data already decides; act, report the fact-to-action chain. Full routing: constitution §12.

## Invariants

- **Bottom-Up over Top-Down**: Always observe, measure, and validate physical substrate reality (hardware, raw bytes, exit codes, machine receipts, AST) before introducing speculative, probabilistic, or higher-layer abstractions across all domains. No model or policy without an empirically verified baseline.
- **Strict DAG downward**: Higher layers depend on lower layers, never the reverse.
- **FCIS / Zero-I/O Core**: Pure logic in domain modules; side effects behind ports in `adapters/`.
- **Boundaries**: Dynamic registries over hardcoded values; typed argv execution; explicit `--yes`/`--apply` for mutation.

## Git & Worktree Isolation

- **Forbidden**: `git add -A` and `git add .` are strictly prohibited. Stage explicit paths only.
- **Commit identity**: `-c user.name=<agent-id> -c user.email=<agent>@agent.local`.
- **Branches**: Work on `agent/<name>/<scope>`; never push to `main` directly.
- **Claims**: Scope claim before mutation; release with proof reference.

## Language

- **Technical substrate**: 100% English (code, tests, docstrings, commits, schemas).
- **Operator dialogue**: 100% French.

## Operator Readability

- **Replies stay airy**: short sentences, one idea per line, lists over dense blocks, plain words (operator reads with compensated dyslexia).
- **Shape follows task**: execution → changed / verified / left; explanation → one idea per line, then examples; sovereign choice → numbered options with (R); error → likely cause first, then numbered steps; plan → short steps, one question at a time.
