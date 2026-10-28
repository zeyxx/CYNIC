# Vertical Extraction Roadmap — CYNIC CLI / node / VCS

Date: 2026-10-28
Status: proposed (not yet applied)

## Goal

Make CYNIC evolvable without big-bang rewrites:
- Strict layered geometry (L0 protocol -> L1/L2 domain+adapters -> L3 composition -> L4 thin CLI).
- No god objects in `cynic-cli` and `cynic-node`.
- Canonical vocabulary and boundaries for VCS, build, release, SSH, coordination.

## Current geometry (observed)

- L0: `cynic-protocol` (42k), `cynic-types` (1k)
- L1: `cynic-host` (90k), network, storage, audit, lifecycle, ...
- L2: `cynic-vcs` (13k), `cynic-build` (15k), `cynic-release` (4k), inference, eval, ...
- L3: `cynic-node` (66k god), `cynic-kernel` (4.5k)
- L4: `cynic-cli` (46k god), `cynic-mcp`, `cynic-tui`, `cynic-fleet`

## Violations

- `cynic-cli` contains 224 subprocess calls and business logic in surfaces.
- `cynic-node` contains usecases, orchestration, adapters, config, network, dns, watchdog.
- VCS/build/release boundaries overlap.
- Coordination vocabulary overlaps (`coordination` vs `agent`).
- SSH is reimplemented across host/release/node/vcs/cli.
- `cynic git branch` currently panics (clap alias duplication).

## Canonical vocabulary

- `coordination`: agent-to-agent messaging, fire-and-forget, ledger, metrics.
- `agent`: governance, claims, scope ownership, heartbeat, session/intent.
- `queue`: local action plan (branch + discovery + GC), no cross-agent messaging.
- `source`: source-layer sync across Cortex/Forgejo/GitHub, PRs, worktrees, divergence, mirrors.
- `git`: branch topology, reconcile, hygiene (candidate to become `source branch` family).
- `build`: build execution, builder pool, cache, bootstrap, proof.
- `release`: gate, deploy, remote, ssh orchestration, contracts, evaluator.
- `vcs`: VCS primitives, source sync, publication plan, git hygiene.

## Target boundaries

### cynic-vcs
Owns:
- git topology
- worktrees
- reconcile
- PRs
- source sync
- publication plan
- git hygiene

Does not own:
- deploy
- release gate
- build execution

### cynic-build
Owns:
- builder pool
- cache
- bootstrap
- build queue
- build proof
- build reliability

Does not own:
- deploy
- release publication
- remote execution

### cynic-release
Owns:
- gate
- deploy plan
- deploy orchestrator
- remote deploy policy
- contracts
- evaluator

Does not own:
- build execution
- source topology
- ssh transport implementation

### cynic-ssh (new, candidate)
Owns:
- remote executor port
- ssh config
- ssh trust
- argv builders
- command/read-back helpers

Does not own:
- release policy
- host policy
- VCS policy

### cynic-node
Owns:
- composition root
- wiring
- usecase registration

Does not own:
- domain rules
- adapter implementations beyond wiring

### cynic-cli
Owns:
- argument parsing
- dispatch
- rendering
- thin operator surfaces

Does not own:
- business logic
- subprocess orchestration
- direct filesystem/network I/O except rendering

## Vertical extraction order (bottom-up causal)

0. Fix `cynic git branch` panic (blocking bug).
   - Remove duplicate clap alias `graph`.
   - Add CLI surface test.

1. L0 — types and pure predicates (`cynic-protocol`, `cynic-types`)
   - Stabilize contracts first.
   - Add unit tests for predicates.

2. L1 — ports and adapters (`cynic-host`, `cynic-network`)
   - Declare ports.
   - Implement adapters.
   - No business logic in adapters.

3. L2 — domain crates (`cynic-vcs`, `cynic-build`, `cynic-release`, `cynic-ssh`)
   - Move business logic here.
   - Keep build/release/vcs boundaries separate.
   - Keep SSH as shared transport crate.
   - Move `deploy*`, `runtime_release`, `release_publication`, `external_build` from `cynic-build` to `cynic-release`.
   - Move `source_release`, `source_repository_bootstrap`, `source_materialization` from `cynic-vcs` to owning crate.
   - Keep `cynic-build` build-only.

4. L3 — composition root (`cynic-node`)
   - Wiring only.
   - No domain rules.
   - No adapter implementations beyond wiring.
   - Extract `network_*`, `dns_*`, `runtime_supervision`, `watchdog` into dedicated crates or L2 domains.

5. L4 — thin CLI (`cynic-cli`)
   - Args -> usecase -> render.
   - No subprocess orchestration.
   - No business logic.
   - Absorb `git` and `queue` into `source` as deprecated aliases.
   - Remove business logic from surfaces into usecases.

6. Coordination/agent (separate vertical)
   - `coordination`: messaging, ledger, metrics.
   - `agent`: claims, scope, heartbeat, session, intent.
   - Keep boundary explicit; no overlap with VCS.

Note: each layer must be validated before moving to the next.

## Measurement

Track per phase:
- crate LOC
- number of CLI modules
- number of subprocess calls in CLI
- number of usecases moved out of CLI/node
- tests added/passing
- `cynic audit crate` and `cynic audit all`

## Non-goals

- No big-bang rewrite.
- No direct push to main.
- No mutation without worktree + claim + proof.
- No removal of working capabilities without replacement.

## First action

Fix `cynic git branch` panic, then create a worktree for vertical #1 (VCS worktrees/branches) and claim the scope.
