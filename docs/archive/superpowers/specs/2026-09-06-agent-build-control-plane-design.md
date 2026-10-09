# CYNIC — Agent Build Control Plane

> Version: 0.1-draft
> Date: 2026-09-06
> Status: design for the primary dogfood vertical; no new mutation is admitted by this document
> Authority: subordinate to `contracts/foundation.json`, `infra/runtime-authority.json`, and observed runtime state

## 1. Purpose

Building CYNIC from CYNIC is the first primary agent use case.

An agent must be able to ask for a bounded build operation against an immutable source SHA; CYNIC must select a runner from observed fleet state, admit or refuse the request, execute through an explicit argv, and return a durable read-back proof. A CLI command, MCP tool, webhook, or human shell must not become an alternate authority.

The canonical loop is:

```text
desired build intent
  -> observe source + runner + cache + admission state
  -> reconcile/admit
  -> issue time-bounded workload grant
  -> deliver to fenced scheduler
  -> materialize immutable SHA workspace
  -> execute exact argv
  -> observe process/artifact/cache deltas
  -> retain signed RuntimeProof / EvidenceJournal entry
  -> judge outcome
```

## 2. Observed current state

Live code has valuable parts, but they form two incompatible paths.

| Existing surface | What is real | Gap |
|---|---|---|
| `cynic build trigger` | exact 40-hex SHA, per-SHA checkout, SSH argv, SHA read-back, persistent JSONL proof | bypasses workload admission, signed grant, durable delivery, scheduler and cancellation |
| `BuildRunnerPort` / `SshBuildRunner` | isolated checkout and bounded runner execution | currently conflates transport adapter, workspace lifecycle, compilation cache and proof construction |
| `BuildWorkflow` in `cynic-protocol` | useful pure workflow decision | decision/policy is wrongly in protocol; protocol must hold types and ports only |
| `orchestrate_build_submission` | admission → workload envelope → signed grant → kernel submit | not consumed by the CLI trigger path |
| build scheduler ports/use cases | workspace, bounded execution, cancellation, durable scheduler primitives | not yet the only execution path |
| `BuildProofBundle` | builder convergence read-back | too thin: no admission/grant identity, argv digest, workspace proof, artifact proof, cache evidence, or terminal workload record |
| sccache | installed at `/var/lib/cynic/bin/sccache`; caller can pass `--sccache-dir` | not yet reported in proof; zero stats on a warmed cargo target does not prove a compiler-cache hit |

The direct trigger remains a useful observation and compatibility adapter until the canonical path replaces it. It must not be treated as the final build authority.

## 3. Non-negotiable model

### 3.1 Truth and intent

- Git is the source DAG; a requested build binds an exact 40-hex commit, never a branch name.
- Runner identity and route are observed from the dynamic registry/fleet truth; never hardcoded in a CLI default.
- Intent flows down only through an admitted `BuildIntent` and its grant.
- Success flows up only through read-back proof after execution.
- A successful SSH exit code is not proof by itself.

### 3.2 One authoritative path

```text
CLI / MCP / Forgejo webhook / human request
  -> BuildSubmissionUseCase (node)
  -> admission + grant + durable workload submission (node)
  -> scheduler claim and fencing (node)
  -> BuildWorkspacePort + BuildExecutionPort (node adapters)
  -> proof collector (node)
  -> EvidenceJournal / workload terminal state
  -> read-only CLI and MCP projections
```

`cynic build trigger` becomes a thin projection over this path. It must not call SSH directly once the scheduler route is available.

### 3.3 Ownership

| Boundary | Owns | Does not own |
|---|---|---|
| `cynic-protocol` | versioned types, schemas, port traits, validation invariants | admission policy, thresholds, SSH, filesystem, CLI rendering |
| `cynic-node` | admission policy, grant issuance, routing decision, scheduler, workload transition, proof assembly | direct CLI parsing, host-specific command construction |
| `cynic-host` | source/runner/cache observation adapters, local host facts | build admission or state transitions |
| `cynic-cli` | clap → injected ports → JSON/text projection | direct `git`, `ssh`, `cargo`, policy reconstruction |

`BuildWorkflow::decide` must move out of `cynic-protocol` to a node-owned pure policy module before the vertical is expanded.

## 4. Canonical contracts

### 4.1 Build intent

```text
BuildIntent
  intent_id
  requested_by (AgentAccount or HumanAdmin)
  source_commit (exact SHA)
  source_proof_ref
  operation (check | test | build | release | activate)
  scope (workspace | crate | test target)
  recipe_ref + canonical argv digest
  resource_budget (wall time, memory, energy if observable)
  required_proofs
  idempotency_key
```

`operation` is not a free string. `check`, `test`, `build`, `release`, and `activate` are a closed protocol enum. A new operation requires a protocol change, a recipe, admission rule, rollback intent, and proof schema.

### 4.2 Build admission

```text
BuildAdmissionDecision
  admitted | planned | blocked
  route (runtime-fenced runner identity)
  grant_ref (only admitted)
  blockers
  residual_unknowns
  required_postconditions
```

Admission does not execute. Planning is always read-only and may not require mutation authority.

### 4.3 Build proof envelope

```text
BuildExecutionProof
  schema: cynic.build_execution_proof.v1
  workload_id + grant_id + fencing_token
  source_commit + source read-back digest
  runner observation + selected route
  workspace proof (canonical path, detached HEAD read-back)
  argv + argv digest + environment allowlist digest
  started_at / finished_at / timeout / cancellation generation
  execution read-back (exit code, bounded stdout/stderr digests, peak memory)
  artifact proof (only build/release/activate)
  cache evidence (configured / available / before / after / delta / residual unknowns)
  outcome (converged | failed | blocked | cancelled | degraded)
  evidence refs
```

A cache hit accelerates compilation only. It never substitutes for executing a test, checking an artifact digest, or observing activation.

## 5. Operation matrix

| Operation | Mutation class | Admission | Mandatory proof | Cache policy |
|---|---|---|---|---|
| `check` | source checkout only | L1 may auto-admit under bounded agent mandate | SHA, argv, compiler result, runner identity | sccache allowed; unavailable = explicit degraded performance, not fake success |
| `test` | source checkout only | L1/L2 by test scope | all `check` proof + test execution result; execution is never cached | sccache compiler reuse allowed |
| `build` | artifact workspace | explicit admitted grant | artifact digest + reproducible metadata | sccache allowed; cache evidence retained |
| `release` | publication candidate | HumanAdmin admission | signed artifact + provenance + promotion gate | sccache allowed, cold independent rebuild may be required by release policy |
| `activate` | runtime mutation | HumanAdmin admission + rollback intent | post-action service/runtime observation | compilation cache irrelevant after artifact is selected |

Sccache failure must not silently change a build result. It produces either:
- `degraded` accelerator evidence and an otherwise valid execution if policy allows uncached compilation; or
- `blocked` if the request explicitly requires cache availability / capacity.

The choice belongs to the admitted recipe, not a hidden adapter fallback.

## 6. Cache topology

```text
per-SHA detached workspace        = source isolation
shared Cargo registry/source cache = dependency acquisition reuse
shared target directory            = Cargo artifact reuse, but path-sensitive
shared sccache directory           = content-keyed compiler-object reuse
```

A raw shared target directory is never evidence of correctness. sccache is acceptable only because its compiler key includes compiler identity, source/preprocessor result, profile and flags. It is an accelerator, not an authority.

Required observations before enabling it in a recipe:
1. absolute binary path exists and reports expected version;
2. cache directory has owner/mode/capacity observation;
3. stats captured before and after compile;
4. proof records delta rather than claiming a hit from a warm Cargo target with zero compiler requests;
5. bounded eviction and maximum cache bytes are declared, never inferred.

## 7. Scheduler state machine

```text
Planned
  -> Admitted
  -> Granted
  -> Submitted
  -> Claimed (fencing token)
  -> WorkspacePrepared
  -> Executing
  -> Observed
  -> ProvenConverged | ProvenFailed | Cancelled | Blocked
```

No transition is implied by a process exit. `Executing -> Observed` needs execution read-back. `Observed -> ProvenConverged` needs all operation-specific postconditions. Terminal records are durable and append-only.

## 8. Implementation order and proof gates

### Phase 0 — Stop semantic drift
- Mark direct `SshBuildRunner` as a transitional adapter; no new policy in it.
- Move `BuildWorkflow` decision from protocol into a node pure policy module.
- Add contract tests ensuring CLI/MCP cannot bypass `BuildSubmissionUseCase`.

Exit proof: protocol contains types/ports only; all current paths compile.

### Phase 1 — Unified read-only build plan
- Add `cynic build plan --commit <sha> --operation ... --json`.
- Observe source SHA, runner route, scheduler availability, cache availability, and residual unknowns.
- No SSH build and no grant issuance.

Exit proof: same inputs produce deterministic plan; missing runner/cache is explicit unknown or blocked.

### Phase 2 — Admitted check and test
- Route CLI/MCP trigger through submission orchestrator and durable scheduler.
- Add `test` as a first-class closed operation.
- Produce `BuildExecutionProof` from scheduler completion.

Exit proof: request without grant is blocked; a test proof shows test execution even with cache hits.

### Phase 3 — Artifact and promotion
- Add artifact digest/provenance proof for build.
- Add HumanAdmin admission and rollback for release/activate.

Exit proof: no release/activation from `check` proof alone.

### Phase 4 — Cache as measured accelerator
- Add typed cache observation/proof types and bounded cache policy.
- Expose `cynic build cache status --json`; no cache mutation by default.
- Enable sccache only through recipe policy; retain before/after stats in proof.

Exit proof: two independent SHAs show compiler-cache counters; no claim is emitted when compilation did not invoke sccache.

## 9. Explicit non-goals

- No shell-script orchestration path.
- No direct branch-name build.
- No unbounded log retention, cache growth, retry, or scheduler loop.
- No implicit local fallback if the runtime-fenced runner is unavailable.
- No automatic merge, release, or activation from a green check.
- No new `cynic-git` crate in this vertical.

## 10. Immediate interface findings

1. `source pr list/get` is still missing; PR proof currently uses a temporary Forgejo adapter/curl outside CYNIC.
2. Agent CLI heartbeat is missing although protocol has heartbeat types.
3. Direct trigger proof lacks cache/admission/scheduler evidence.
4. The builder checkouts now fetch exact missing SHAs once, but checkout lifecycle/GC remains separate from workload ownership.

This document is the design authority for fixing those gaps in dependency order, not an assertion that they are already solved.
