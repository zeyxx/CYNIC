# CYNIC — Minimal Agent Protocol

CYNIC is a low-layer sovereign Rust runtime for multi-agent systems. It bounds, connects, executes, observes, and proves. No mutation without read-back proof.

## Commands

Canonical validation gate (admitted build runner only):

make check
make audit
make release.deploy

Same gate through other projections: MCP `cynic_validate`, `cynic audit all --json`.

Targeted fast checks (stripped debuginfo, deterministic):

RUSTFLAGS="-C debuginfo=0" cargo check -p <crate> --lib
cynic audit crate --json
cynic audit secret --json
cynic audit all --json
cynic source worktree --json
cynic source deploy --json

Compiler and build policy: `infra/RUST_COMPILER_ALPHAS.md`. `make test` exists but is not a canonical gate while the pinned rustc ICE paths persist. `make build` and `make build-release` **do not exist** — they were named here for longer than they were implemented, and a target depended on one of them.

## Structure

47 workspace crates in 16 families. Canonical taxonomy, LOC census, and extraction verdicts: `infra/CANONICAL_CRATE_MAP.md` v3. God-object ceiling: 25k LOC (`contracts/foundation.json` hard boundary).

- L0 pure: `cynic-protocol`, `cynic-types`, `cynic-identity`.
- L1 host substrate: `cynic-host`, `cynic-host-probes`, `cynic-network`, `cynic-lxc`, `cynic-sandbox`, `cynic-pkg`, `cynic-forensics`, `cynic-lifecycle`, `cynic-release`, `cynic-discovery-host`.
- L2 domains: `cynic-storage`, `cynic-inference`, `cynic-inference-runtime`, `cynic-build`, `cynic-vcs`, `cynic-vcs-forgejo`, `cynic-eval`, `cynic-askesis`, `cynic-domain-ops`, `cynic-domain-cloudflare`, `cynic-domain-porkbun`, `cynic-domain-askesis`, `cynic-intel`, `cynic-intel-escore`, `cynic-intel-futarchy`, `cynic-intel-soc`, `cynic-solana-pay`, `cynic-zeroclaw-solana`, `cynic-observer`, `cynic-consensus`, `cynic-discovery`, `cynic-node-protocol`, `cynic-agent-bus`, `cynic-agent-bus-server`, `cynic-agent-bus-coordinator`, `cynic-channel`, `cynic-pool`, `cynic-market`.
- L3 orchestration: `cynic-kernel`, `cynic-node`.
- L4 projections: `cynic-cli`, `cynic-mcp` (replaceable agent-facing projection, never a bootstrap prerequisite).
- Cross-cutting: `cynic-secrets`, `cynic-workload`.
- `contracts/foundation.json`: constitutional and evolutive policy contract.
- `infra/runtime-authority.json`: operational authority map and surface owners.
- `infra/directory-tree.json`: canonical filesystem arborescence (path SSOT).
- `infra/registry.json`: placement registry. Read it; never hardcode hosts, IPs, or paths.

## Authority

When files disagree:

1. observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json`
5. `Cargo.toml` workspace
6. canonical Rust crates under `crates/`
7. `infra/agent-exposure.json`

Deleted authorities: `axioms/`, `organs/`, `docs/` as runtime law, Python lifecycle gates. Stale prose never outranks executable contracts or observed state.

## Coherence pattern

desired state -> observe -> reconcile -> bounded action -> read-back proof. Retain only explicit, machine-readable evidence.

Truth flows upward; intent flows downward. A surface projects, it does not own. Unknown is data: report `unknown` or `blocked`, never fabricate convergence. Repair flows downward to the violated contract. Every bug is evidence of an unmodelled contract; fix the owning layer and add a regression proof there.

Design gate (answer bottom-up before adding code, commands, crates, or contracts): desired state and its authority; read-only observation and port; pure selection rule; exact argv, privilege, consent, rollback, and stop condition; post-action read-back and producing layer; owning boundary for each answer.

## Session protocol

1. `cynic agent claim` the affected scope before mutation (file, module, or crate; never a whole crate when a file will do).
2. Mutate only inside the claimed scope and budget.
3. `cynic agent release` with lease id and proof reference when done.
4. MCP or coordination unreachable is an explicit unknown. Without human-admitted bounded fallback, report `blocked_disconnected`. A handshake or queue acknowledgement is never execution authority.

## Git isolation

Shared worktree. Never `git add -A` or `git add .`. Stage explicit paths, verify the staged list equals your file list (`git diff --cached --stat`), one isolated commit per slice with proof counts in the message, read back with `git show --stat HEAD`. Other agents' `M`/`D`/`??` entries are their debt: never repair them silently, never delete their files. Work on `agent/<name>/<scope>`; never push to main directly.

## Boundaries

Always: typed argv execution, explicit `--yes` / `--apply` for mutation, machine-readable JSON proof, dynamic registry over hardcoded hosts or paths. Library-first: domain types, ports, and use cases in `src/lib.rs`, thin `src/main.rs` facade.
Architecture & Crate Invariants (Mandatory):
- Strict DAG downward: Higher layers depend on lower layers, never the reverse. Pure domain crates (`cynic-protocol`, `cynic-vcs`, etc.) must NEVER depend on orchestrator or host crates (`cynic-host`, `cynic-kernel`, `cynic-node`).
- Zero-I/O Domain Core: Domain modules (`entry.rs`, `parse.rs`, state machines) are 100% pure (no `std::process`, no `reqwest`, no `tokio::spawn`). Side-effects stay behind explicit Ports in `runner.rs` or `adapters/`.
- Facade & Ratchet: Monolith crates (`cynic-host`, `cynic-node`) are under strict ratchet. No net growth allowed. Autonomous sub-domains must be clean-room extracted to micro-crates, with dead branches pruned destructively rather than kept as legacy baggage.
- Automated Architectural Tests: Every crate boundary must be guarded by executable tests in `tests/architecture_boundary.rs`.
Ask first: new crate, command, contract, or dependency; anything touching credentials, network, fleet, release, or another agent's scope.
Never: commit secrets; mask unverified code with `allow(dead_code)`; recreate contract forests or Python organs as authority; turn a missing observer, adapter, or proof into a success report; amplify your own authority, budget, or scope.

## Canonical pointers

- Crate ownership and extraction: `infra/ARCHITECTURE_CONSTELLATIONS.md`, enforced by the dev audit surfaces above. A crate over ~40k LOC or implementing another domain's ports must be split.
- Host substrate families: types in `cynic-protocol`, pure parsers in `cynic-host` domain probes, `Local`/`Remote` adapters, fleet rollup in `cynic-host`, thin CLI projection. An absent lower layer stays an explicit unknown.
- Operator priority follows fresh organic opportunities (read-only proposals, never tasks or mutation authority). They never override sovereignty, admission, claims, or proof.
- Runtime secrets are injected (env, then principal-owned file, then OS store behind a port). Never git, docs, chat logs, or proof JSON. Reports show presence, never the secret.
- Runtime authority stays local (principal `cynic`). Forgejo is the sovereign candidate SSOT; GitHub `origin` is an optional public relay.
- Context file placement (SOUL vs AGENTS vs MEMORY vs USER): `infra/CONTEXT_PLACEMENT.md`.

## Language invariant

Technical substrate is 100% English: code, tests, error codes, commits, docstrings. No French anywhere in code or schemas. Operator dialogue with the human is French; that belongs in chat, never in the repo.
