# CYNIC — Agent Protocol

This file contains only instructions required in every agent session. It is not
a product narrative, architecture ledger, runtime status page, or operational
playbook. Follow the canonical contracts, executable surfaces, and skills named
below instead of copying their details back here.

## Authority and scope

When sources disagree, use this order:

1. observed runtime state and read-back proof;
2. `contracts/foundation.json` and `contracts/framework.json`;
3. `infra/runtime-authority.json`;
4. `infra/registry.json` and typed vertical intents under `infra/verticals/`;
5. `Cargo.toml`;
6. canonical Rust crates under `crates/`.

Deleted or non-authoritative surfaces: `axioms/`, `organs/`, Python lifecycle
gates, narrative docs, chat memory, plans, and successful command invocation
without read-back proof. `MANIFESTO.md` and `WHITEPAPER.md` are public narrative
projections only. Project capability status comes from `FrameworkSpec` through
`cynic dev framework --json` or `--markdown`, never from an editorial checklist.

Keep changes inside these runtime surfaces unless the human expands scope:

- `crates/cynic-kernel`, `cynic-host`, `cynic-node`, `cynic-mcp`, `cynic-cli`,
  `cynic-protocol`, `cynic-askesis`, `cynic-inference`;
- `contracts/foundation.json`, `contracts/framework.json`;
- executable or registry-backed material under `infra/`.

Before adding a crate, command, contract, document, or registry, prove that an
existing boundary cannot absorb the responsibility cleanly.

## Skill routing and improvement

Before non-trivial planning, mutation, debugging, or validation, scan the
available skills and load every skill that materially matches the task. Skills
are procedures, not runtime authority: current observation and the authority
order above win when a skill disagrees. An agent without the named skill must
use the canonical contracts and executable surfaces instead of inventing a
replacement procedure.

Use these primary routes; let each skill route to narrower recipes:

| Task trigger | Load first |
|---|---|
| Cross-layer capability, lower-layer blocker, egg-and-chicken loop | `cynic-layer-model` |
| Coherence, canonical decision, desired/observed mapping | `cynic-coherence-pattern` |
| Non-trivial feature or refactor | `cynic-design-doc-plan-tests-first` |
| Shared worktree, claim, coordination fallback | `cynic-agent-coordinates-claims` |
| Rust patch and review gate | `cynic-rust-patch-workflow` |
| Build, tests, ICE, sovereign Builder | `cynic-dev-build`, then `cynic-rust-ice-isolation` or `cynic-build-runner` |
| Broken crate or merge debt | `cynic-crate-repair` |
| New CLI or audit projection | `cynic-cli-thin-surface` |
| Secret, capability, or privileged mutation | `cynic-security-engineer`, then `cynic-privileged-execution` |
| Network, inference, or source topology | `cynic-network-topology`, `cynic-inference-runtime`, or `cynic-source-topology` |
| Hermes configuration or behavior | `hermes-agent` |

Apply this improvement loop:

```text
classify → load skill → observe reality → execute bounded procedure
         → verify → compare guidance with evidence → improve or retain
```

- If a loaded skill has a wrong command, stale assumption, missing stop
  condition, or incomplete verification, patch it in the same slice and verify
  the corrected procedure.
- Propose a new skill only after a difficult, reusable workflow is proven; get
  human confirmation before creating or deleting one.
- Skills retain triggers, procedure, pitfalls, and verification—not task
  progress, commits, PR numbers, current runtime state, secrets, or raw evidence.
- Skill improvement may refine procedure but may not silently change contracts,
  architecture, permissions, or runtime authority.
- Once detail moves into a skill or canonical source, remove its duplicate from
  this file so every session pays only for routing and invariant context.

## CYNIC coherence pattern

```text
desired state → observe → reconcile → bounded action → read-back proof
                         ↘ retain explicit machine-readable evidence
```

Every non-trivial change must identify:

1. the authoritative desired state;
2. the read-only observation port and adapter;
3. the pure reconciliation rule;
4. exact mutation argv, privilege, consent, budget, stop and rollback intent;
5. independent post-action read-back proof;
6. the existing boundary that owns each term.

Unknown is data. A missing observer, adapter, proof, coverage value, or
coordination store remains `unknown` or `blocked`; never fabricate convergence.
Truth flows upward from lower-layer proof. Intent flows downward through typed,
bounded capabilities. Repair the lowest boundary that can express the violated
contract; a higher-layer symptom guard is not a durable fix. Every bug should
gain regression proof at the layer that owns the missing contract.

Canonical first-class convergence types live in `cynic-protocol`; do not create
a parallel prose-owned policy.

## Structural invariants

- **One boundary, one reason.** A crate or module has one primary
  responsibility. Do not centralize unrelated responsibilities in an
  orchestrator or `common` crate.
- **No monolith surfaces.** Separate types, policy/decision, ports, adapters,
  observation, apply, proof, and rendering. A CLI is projection and wiring,
  never the owner of domain policy.
- **One decision source.** CLI, MCP, plans, and reports consume a canonical
  decision; they do not recalculate it independently.
- **Policy is portable; substrate is injected.** Provider, distro, principal,
  path, service manager, hypervisor, scheduler, transport, and inference engine
  belong behind typed ports or registries.
- **Detect before specializing.** If provenance, authority, capability, or
  connection state cannot be observed, preserve an explicit unknown.
- **Adapters are disposable; contracts and proofs are durable.** Replacing a
  tool or provider must not create a second orchestration model.
- **Every mutation is a small transaction.** Desired state, preflight, apply,
  observation, proof, and rollback intent remain separately inspectable.
- **No hidden execution.** Apply paths use typed argv execution, stop on first
  failure, declare mutation and privilege, and require explicit `--yes` or
  `--apply`. Python and shell wrappers are not runtime authority.
- **La structure émerge du chaos.** When runtime anomalies surface (permission
  drift, divergent read-back, stale state, orphan locks), treat them as emergent
  signals revealing an unmodelled constraint — not as defects to suppress.
  Classify the anomaly, document the emergent invariant, and add regression
  proof at the layer that owns it.
- **This is a feature, not a bug.** An anomaly is a feature of the system
  exposing a boundary condition. Investigate the root cause before applying
  a corrective patch; the patch must make the emergent invariant explicit
  rather than masking the symptom.

Install and repair decisions must derive from observed provenance. Keep payload
layout distinct from mutator authority: for example, a Python user-site payload
does not prove whether `uv`, `pipx`, distro packaging, or another channel owns
updates. Primary components name principal, channel, provenance, canonicality,
and read-back proof.

Canonical ownership and absorb-vs-create decisions live in
`infra/CRATE_MODELS.md`; extraction state and evidence live in
`infra/CRATE_OWNERSHIP.md`, `infra/surface-registry.json`, and these executable
audits:

```text
cynic dev kernel-audit --json
cynic dev ownership-audit --json
cynic dev crate-audit --json
```

Inventory the target crate's ports, modules, and exports before an extraction.
Reuse an equivalent target surface instead of creating a duplicate. Afterward,
prove the old owner no longer imports the extracted concrete type. Large files
(about 400 lines) and crates (about 40k LOC) are audit signals, not permission
to perform an unbounded rewrite. Extract one owned responsibility per slice.

## Builder isolation

The builder (`cynic-builder-build`) operates under the `cynic` principal. Its Git refs must remain owned by `cynic` — never by `root` or another principal.

| Command | Role |
|---------|------|
| `cynic builder drift-check` | Detects ownership drift in `.git/refs/remotes/forgejo/` |
| `cynic builder repair-refs` | Repairs ownership to `cynic:cynic` when drift detected |

Drift is detected when a fetch fails with `Permission denied` on ref creation, or when `stat` shows non-cynic ownership. Repair runs only after operator confirmation.

All host substrate families use four layers:

1. physical device or raw counter;
2. logical volume or configuration;
3. usage against that volume;
4. pressure or fidelity signal.

Types live in `cynic-protocol`; pure parsers and local/remote adapters live in
`cynic-host`; fleet rollups remain host-owned; CLI and MCP only project reports.
An absent lower layer is a residual unknown, never a guessed number. Human and
JSON projections must expose the same typed layers.

A runtime harness is an execution context, not authority. It declares `kind`,
`authority`, `isolation_boundary`, `target_scope`, and whether it may mutate the
host network. Progression is lab → conformance → staging → canary → production.
Do not grow lab code into the framework; extract reusable ports and proofs.
Observe existing surfaces before inventing a harness:

```text
cynic runtime harness --json
cynic runtime conformance --json
cynic runtime metrics --json
```

Metric targets are per metric. Unknown coverage stays `null`; `100%` is not a
global success target. External proof bundles may supply observations that the
current host cannot produce, but may not fabricate them.

## Agent sessions and coordination

Agents are bounded workloads, never a source of runtime truth. At session start:

1. observe `cynic runtime mcp-observation --json`;
2. when stable and compatible, register, read back identity, and claim the
   affected source scope through CYNIC coordination;
3. retain observed evidence and run the canonical validation surface;
4. report consumed budget, explicit unknowns, and completion evidence.

Do not emulate a lease by editing `.agent-claims.json`. Recursive execution
remains inactive until a node-owned scheduler proves monotonic aggregate
budgets, lease fencing, cancellation, durable evidence, and recovery.
Crossing a node boundary is federation, not local child recursion.

If MCP coordination is observed unavailable, incompatible, or unstable, only
an explicit human-approved `human_scoped_fallback` permits a bounded local
source mutation or local validation slice. Before using it, read the claim
store and Git state and stop on an active overlapping claim. This fallback is
`best_effort_unfenced`, expires when compatible coordination returns, and never
authorizes runtime, credentials, network, fleet, release, commit, push, merge,
or history mutation. Without that admission, report `blocked_disconnected`.
The machine-readable contract lives in `infra/agent-exposure.json`.

Local child delegation uses `cynic.agent_delegation_envelope.v1`; completion
uses `cynic.agent_completion_envelope.v1`. Children remain on the parent's
`node_id`, increment depth once, and receive only subsets of capabilities,
mutation authority, context, actions, children, and wall-time budgets. Context
is assembled from content-addressed evidence, not raw chat or provider state.

## Git isolation in shared worktrees

- Work on `agent/<name>/<scope>`, never directly on `main`.
- Treat unrelated modified or untracked paths as another owner's work. Do not
  delete, overwrite, stage, or repair them merely to obtain a clean status.
- Stage only explicit files; never use `git add -A` or `git add .`.
- Before commit, verify that the staged list exactly equals the reviewed slice.
- One coherent slice becomes one isolated commit after human review, with
  validation evidence in the message; read it back with `git show --stat HEAD`.
- Do not commit, push, merge, rebase, or rewrite history unless the human has
  authorized that exact action. Forgejo is the sovereign candidate SSOT;
  GitHub is an optional public relay, not runtime authority.

## PR workflow

```text
1. Éditer sur agent/<name>/<scope>
2. Commit atomique avec preuve dans le message
3. Push branche : git push -u origin agent/<name>/<scope>
4. Ouvrir PR : python3 scripts/cynic-forgejo pr-create --base main --head <branch> --title <title>
5. Le builder (SSH) valide le build — ne pas bloquer sur ICE local
6. Revue humaine sur Forgejo
7. Merge humain uniquement
```

Wrapper Python : `scripts/cynic-forgejo` (couche 1, fallback)
Token : `/etc/cynic/secrets/forgejo-token`
API : `http://forgejo.synapse.cynic:3000/api/v1`
Repo : `jeanterre13/CYNIC`

Couche 2 prévue : `cynic forgejo` natif (Rust, cynic-cli).

If dependency code is red because another agent is mid-edit, do not repair it
silently. Wait for stabilization or validate in an isolated scratch checkout.

## Secrets and principals

Runtime authority, artifacts, configuration, state, and steady-state execution
belong to principal `cynic`, never an operator-home fallback. Runtime must not
depend on HumanAdmin secrets.

Secret classes are `HumanAdmin` (bootstrap only), `MachineEnrollment` (first
join), `RecoveryBreakglass` (incident only), and `RuntimeMachineIdentity`
(steady state). Prefer process-scoped environment injection, then a
principal-owned `0640` file, then an injected OS secret-store adapter. Never put
secret material in Git, contracts, registries, reports, prompts, chat, proof
JSON, command lines, or logs. Reports expose presence and authorized provenance,
never values.

Configuration such as URLs, model names, and pins is not secret. Secret
resolution and repair must retain principal, source class, consumer, permission,
rotation, and authentication-failure evidence.

## CLI, ports, and projections

The default non-trivial surface grammar is:

```text
thin facade → pure types → decision/plan → port → adapter
            → observe/apply → proof → JSON/human render
```

Domain and host logic must not spawn a sibling `cynic` command directly. Inject
an argv runner port; tests inject fakes. `cynic-cli` parses, injects, delegates,
and renders. It does not own policy tables, finding classifiers, state schemas,
or provider-specific branches. Shared helpers stay primitive; reuse existing
JSON persistence, source discovery, and rendering helpers rather than cloning
them.

When adding or renaming an agent-facing MCP/CLI tool, update
`infra/agent-exposure.json` and keep `cynic dev agent-exposure --json` green.
Never commit machine-local absolute paths in agent projections.

## Validation

Use targeted crate validation; do not invent a parallel gate:

```text
RUST_MIN_STACK=268435456 RUSTUP_TOOLCHAIN=1.96.0 cargo check -p <crate> --lib
RUST_MIN_STACK=268435456 RUSTUP_TOOLCHAIN=1.96.0 cargo test -p <crate> --lib <module>
```

For CLI glue, validate policy through owning `cynic-host` or `cynic-protocol`
tests. If rustc fails during monomorphization, use `ulimit -s unlimited` and the
documented isolated Builder procedure; do not reinterpret an ICE as a code
failure or claim success from a partial gate.

Before finishing, verify every requested criterion, modified file, relevant
test, machine-readable report, and residual unknown. A plan or successful
command is not the deliverable; the working artifact and its read-back are.

## Product direction

CYNIC is the private sovereign membrane connecting host, source, build,
artifact, agent, inference, fleet, evidence, and explicit public settlement.
The human remains sovereign; models, providers, schedulers, chains, markets,
scores, and public audiences are injected adapters or projections and may not
silently amplify their own authority. Solana is the strategic public
commitment and settlement target, not the source of observed host truth.

Long-form direction belongs in `FrameworkSpec`, `MANIFESTO.md`, and
`WHITEPAPER.md`. Do not duplicate it in agent instructions or present ambition
as an implemented capability.