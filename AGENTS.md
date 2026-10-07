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

47 workspace crates in 16 families. Canonical taxonomy and LOC census: `infra/CANONICAL_CRATE_MAP.md` v3 (its per-crate LOC table is the source; regenerate it rather than editing these numbers by hand). God-object ceiling: 25k LOC (`contracts/foundation.json` hard boundary). A crate exceeding it, or implementing another domain's ports, must be split.

- L0 pure: `cynic-protocol`, `cynic-types`, `cynic-identity`.
- L1 host substrate: `cynic-host`, `cynic-host-probes`, `cynic-network`, `cynic-lxc`, `cynic-sandbox`, `cynic-pkg`, `cynic-forensics`, `cynic-lifecycle`, `cynic-release`, `cynic-discovery-host`.
- L2 domains: `cynic-storage`, `cynic-inference`, `cynic-inference-runtime`, `cynic-build`, `cynic-vcs` (Forgejo absorbed as `cynic-vcs/src/adapters/forgejo.rs`), `cynic-eval`, `cynic-askesis`, `cynic-domain-ops`, `cynic-domain-cloudflare`, `cynic-domain-porkbun`, `cynic-domain-askesis`, `cynic-intel`, `cynic-intel-escore`, `cynic-intel-futarchy`, `cynic-intel-soc`, `cynic-solana-pay`, `cynic-zeroclaw-solana`, `cynic-consensus`, `cynic-discovery`, `cynic-node-protocol`, `cynic-agent-bus`, `cynic-agent-bus-server`, `cynic-agent-bus-coordinator`, `cynic-channel`, `cynic-pool`, `cynic-market`.
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
5. Commit identity: every agent commit carries the agent's own identity (`-c user.name=<agent-id> -c user.email=<agent>@agent.local`), never the repo default. The default identity belongs to the human operator alone; agent work signed with it is indistinguishable from operator work and untriagable (observed 2026-10-07).

## Git isolation

Shared worktree. Never `git add -A` or `git add .`. Stage explicit paths, verify the staged list equals your file list (`git diff --cached --stat`), one isolated commit per slice with proof counts in the message, read back with `git show --stat HEAD`. Other agents' `M`/`D`/`??` entries are their debt: never repair them silently, never delete their files. Work on `agent/<name>/<scope>`; never push to main directly.

## Boundaries

Always: typed argv execution, explicit `--yes` / `--apply` for mutation, machine-readable JSON proof, dynamic registry over hardcoded hosts or paths. Library-first: domain types, ports, and use cases in `src/lib.rs`, thin `src/main.rs` facade.
Architecture & Crate Invariants (Mandatory):
- Strict DAG downward: Higher layers depend on lower layers, never the reverse. Pure domain crates (`cynic-protocol`, `cynic-vcs`, etc.) must NEVER depend on orchestrator or host crates (`cynic-host`, `cynic-kernel`, `cynic-node`).
- Zero-I/O Domain Core: Domain modules (`entry.rs`, `parse.rs`, state machines) are 100% pure (no `std::process`, no `reqwest`, no `tokio::spawn`). Side-effects stay behind explicit Ports in `runner.rs` or `adapters/`.
- Facade & Ratchet: Monolith crates (`cynic-host`, `cynic-node`) are under strict ratchet. No net growth allowed. Autonomous sub-domains must be clean-room extracted to micro-crates, with dead branches pruned destructively rather than kept as legacy baggage.
- Automated Architectural Tests: Every crate boundary must be guarded by executable tests in `tests/architecture_boundary.rs`.
- FCIS structure (supersedes strict hexagonal/DDD): every crate splits into Functional Core (pure types, parsers, planners, drift evaluators — zero `std::process`/`reqwest`/`tokio::spawn`) and Imperative Shell (`adapters/` implementing crate-owned ports). An adapter lives in the crate that declares its port, never by default in `cynic-host`. No struct-level secret fields — `SecretProviderPort` resolved at call time.
- Implicit domains under consolidation: `cynic-host` is the monolith under ratchet (its current LOC and file count come from the `infra/CANONICAL_CRATE_MAP.md` census — read them there, not from this line). It legitimately owns ONE thing: host substrate observation + local actuation per family (`domain/<family>` pure probe + port, `adapters/local_*`, `adapters/remote_*` via SSH, `fleet_*_observer` rollup). Everything else inside it is a guest awaiting clean-room extraction to its owning micro-crate: network actuation/fleet observers → `cynic-network`; enrollment/activation/membership decisions → `cynic-node-protocol` (pure) with runtimes behind ports; build scheduling → `cynic-build`; storage → `cynic-storage`; VCS → `cynic-vcs`; secrets → `cynic-secrets`; node orchestration/usecases → `cynic-node`. Interconnection rule: readers consume `infra/registry.json` (Tier-1 desired state) through typed ports; Tier-2 observed state (discovery reports, telemetry, leases) never flows back into Git; fleet rollup picks Local vs Remote adapter by `observed_hostname()`, never by hardcoded role.
- `cynic-kernel` is NOT a god object and must not be described as one. Genesis `1ee8185b` (2026-06-09) held 202 files / ~70k LOC as the whole runtime (MCP + REST API, dispatch, judge tools, and full business domains such as elections and chess). Those domains were clean-room extracted into L0–L2 micro-crates; the crate is now ~7k LOC and is a legitimate L3 orchestrator. It owns runtime boot (`main.rs`), bootstrap verify, domain type composition, and the MCP JSON-RPC proxy — nothing else. A future proposal to move domain logic back into it is a regression.
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
