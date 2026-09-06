# Plan — Agent Build Control Plane

> Design source: `docs/superpowers/specs/2026-09-06-agent-build-control-plane-design.md`
> Status: proposed; no remote/runtime mutation in this plan

## Acceptance contract

At completion, every agent-initiated CYNIC build uses one path:

```text
BuildIntent -> plan -> admission -> signed grant -> durable submission
-> fenced scheduler -> immutable workspace -> exact argv -> read-back proof
```

No CLI, MCP tool, webhook, or direct SSH adapter may independently admit, select a runner, or interpret a process exit as convergence.

## Slice 1 — Make policy ownership correct

Touches:
- `crates/cynic-protocol/src/build_workflow.rs`
- new pure node policy module under `crates/cynic-node/src/usecases/`
- current CLI projection only where imports change

Does not touch:
- scheduler persistence, remote runner, build cache, Forgejo, release/activate

Steps:
1. Add regression compile tests that protocol exports build shapes/ports but no workflow decision entrypoint.
2. Move `BuildWorkflow`, `BuildWorkflowOutcome`, and decision tests to a node-owned pure policy module.
3. Keep protocol types and validation behavior byte-compatible where possible.
4. Rewire every call site to the node policy.

Proof:
- protocol/ node/ cli targeted checks;
- policy regression tests;
- source scan shows no `BuildWorkflow::decide` in protocol.

Rollback: revert one cohesive commit; no state is mutated.

## Slice 2 — Introduce a read-only canonical build plan

Touches:
- typed `BuildIntent`, `BuildOperation`, `BuildPlan`, `CacheObservation` in protocol
- host observation adapter for exact source, runner health and cache availability
- node plan use case
- `cynic build plan --commit ... --operation ... --json`

Steps:
1. Define closed operation enum: check/test/build/release/activate.
2. Define bounded scope and resource budget types.
3. Add plan inputs and explicit residual unknowns.
4. Implement runner/cache/source observation without mutation.
5. Render a versioned JSON plan through a thin CLI facade.

Proof:
- deterministic pure plan tests;
- missing cache binary / missing runner produces `blocked` or explicit residual unknown;
- live `cynic build plan` returns exact source SHA and dynamic runner route.

Rollback: read-only surface; revert only.

## Slice 3 — Unify check through scheduler admission

Touches:
- build submission orchestrator
- scheduler use cases and workload proof types
- CLI/MCP build trigger projection

Steps:
1. Test first: a raw CLI trigger lacking an admitted delivery is rejected.
2. Route `check` through existing `orchestrate_build_submission` and scheduler claim/fence path.
3. Use `BuildWorkspacePort` / `BuildExecutionPort` for the runner, not a direct CLI SSH path.
4. Retain direct `SshBuildRunner` only behind the scheduler adapter boundary until retired.
5. Build a durable terminal record from execution read-back.

Proof:
- no grant -> blocked;
- mismatching source SHA -> blocked;
- stale fencing token -> blocked;
- one admitted check -> durable completion + exact SHA/workspace/argv proof;
- builder proof read back by workload ID.

Rollback: preserve the old adapter behind an explicitly named compatibility switch only while the scheduler migration is incomplete; no silent fallback.

## Slice 4 — Add test as a first-class operation

Touches:
- operation recipe validation
- scheduler execution request
- proof projection

Steps:
1. Add test target/scope type with max command arguments and max wall time.
2. Define test proof postcondition: test process genuinely executed.
3. Reject an attempt to satisfy a test proof from sccache/Cargo cache alone.
4. Expose `cynic build test` only as a projection over the canonical operation.

Proof:
- cache hit + failed test remains failed;
- source test command not on allowlist is blocked;
- result includes bounded output digest and test execution timestamp.

## Slice 5 — Cache observation and evidence

Touches:
- cache protocol types
- runner adapter / scheduler completion collector
- `cynic build cache status --json`

Steps:
1. Observe absolute sccache binary, version, cache path, ownership/mode, capacity, and counters before/after execution.
2. Put `RUSTC_WRAPPER` only in an admitted recipe environment allowlist.
3. Add bounded cache policy: max bytes, LRU/GC policy, max stats output.
4. Include cache delta in `BuildExecutionProof`.
5. Replace the current untyped `--sccache-dir` string with a named declared cache profile from registry/recipe data.

Proof:
- unavailable sccache yields explicit degraded or blocked state according to recipe policy;
- a first compile shows a miss delta; a second independent SHA with matching content shows a hit delta;
- zero compiler requests is reported as such, never called a hit.

## Slice 6 — Artifact, release and activation

Touches:
- artifact metadata/provenance types
- HumanAdmin admission surface
- activate rollback/read-back path

Proof:
- green check cannot be promoted to release;
- release requires artifact digest and HumanAdmin admission;
- activate requires rollback intent and post-action service observation.

## Cross-cutting gates for every slice

- `CYNIC_ALLOW_LOCAL_DEV_BUILD=1` targeted package checks/tests only.
- Builder check on the exact pushed 40-hex SHA for every affected crate.
- Read the external state back after any Forgejo/builder/cache mutation.
- Stage explicit paths only; one coherent commit per slice.
- If rustc ICE occurs, record it as substrate unknown and retry once; never mutate source to satisfy a changing compiler failure.

## Explicit decisions needed before Slice 3

1. Check/test auto-admission limits: action count, wall time, energy and allowed crates.
2. Scheduler persistence authority: current node store versus kernel journal.
3. Cache availability policy: degrade-to-uncached vs block-by-recipe.
4. First terminal proof store: local evidence journal only versus kernel-first.
