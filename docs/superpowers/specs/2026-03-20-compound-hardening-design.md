# CYNIC Compound Hardening — Design Spec

**Date:** 2026-03-20
**Authors:** T. (backend), Claude (AI eng)
**Status:** Approved (reviewed, issues fixed)

## Problem Statement

189 commits in 10 days produced 41 audit findings (7 HIGH, 12 MEDIUM). Root cause analysis reveals 5 systemic patterns that recur because enforcement lives in text (CLAUDE.md, memories) rather than in mechanisms (hooks, tests, gates). Individual bug fixes don't compound — the same class of bug returns in the next session.

**Measurable gap:** 1 partial storage integration test exists (`store_and_retrieve_verdict` in `surreal.rs`, using namespace `test_cynic/ci`) but covers only 1 of 16 `StoragePort`+`CoordPort` methods. No `make check-storage` target. No mechanical enforcement of rules #8-12. Layer 3-4 rules depend on LLM compliance, violating the no-hope-engineering principle.

## Success Criteria

1. Every `StoragePort` + `CoordPort` method has at least one integration test against a real SurrealDB instance
2. `make check-storage` exists and catches SQL incompatibilities before deploy
3. Rules #8, #10, #11 have mechanical enforcement (compiler lint, tests, hooks)
4. Remaining MEDIUM debt items are fixed via TDD (test first, then fix)
5. Re-audit via deterministic checklist shows zero HIGH findings and <5 MEDIUM

## Non-Goals

- CI/CD (GitHub billing debt, no Actions available)
- Continuous monitoring / alerting (separate initiative)
- Frontend changes
- New features

## Design

### Architecture: Test Infrastructure

```
cynic-kernel/
  tests/
    integration_storage.rs    # Storage contract tests (real SurrealDB)
    common/
      mod.rs                  # Test helpers: setup_db, cleanup_db, test_config
```

**Test DB isolation:** Tests use namespace `cynic_test`, database `test_<unix_millis>`. Each test function gets a fresh DB via helper. Cleanup drops the database after. Consolidates existing `test_cynic/ci` namespace into the new scheme.

**Activation:** `#[ignore]` by default. `CYNIC_TEST_DB=1 cargo test --release -- --ignored` or `make check-storage`.

**Prerequisite:** SurrealDB running on localhost:8000 (already in place).

### Sprint 1: Storage Integration Tests

**Target:** Every public method on `StoragePort` + `CoordPort` gets at least one integration test.

| Method | Test Cases | Bug Class Prevented |
|--------|-----------|-------------------|
| `ping` | Connectivity, bootstrap validation | Boot hang on unresponsive DB |
| `store_verdict` + `get_verdict` | Round-trip, field integrity | Schema drift |
| `list_verdicts` | Multi-insert, limit enforcement | Query syntax changes |
| `observe_crystal` | New crystal, update existing, state transitions | SurrealDB 3.x SQL syntax |
| `get_crystal` + `list_crystals` | Read after write, sorting | Read path divergence |
| `store_observation` + `query_observations` | Insert, frequency query | Silent error swallowing |
| `query_session_targets` | Session co-occurrence query | CCM aggregation input |
| `register_agent` + `heartbeat` + `deactivate_agent` | Full agent lifecycle | Coord foundation |
| `claim` + `release` + `who` | Single agent, conflict detection, expiry | Scalar/object mismatch, TOCTOU |
| `claim_batch` | Batch claim, partial conflict | Atomicity of batch override |
| `store_audit` + `query_audit` | Insert, time-bounded query, round-trip | SQL syntax changes |
| `expire_stale` | Session expiry, claim cascade (requires heartbeat setup) | Multi-statement ordering |
| `flush_usage` (via build_flush_sql) | Upsert accumulation, idempotency | SurrealQL IF...THEN...END syntax |

**Deliverable:** `make check-storage` in Makefile. Pre-push hook runs it if `src/storage/` changed.

### Sprint 2: TDD Bug Fixes

Each remaining MEDIUM fixed with test-first discipline. Order matters for compound:

1. **build_flush_sql → StoragePort migration** — Add `flush_usage(&self, dogs: &[(String, DogUsage)]) -> Result<(), StorageError>` to StoragePort. Write integration test. Move SQL to adapter. Domain keeps `snapshot()` only. Test green throughout. (Must come BEFORE N+1 fix because Sprint 1 tests reference this interface.)
2. **N+1 CCM observe_crystal (D3)** — Write perf test measuring query count. Then batch. Test goes green.
3. **Dead code sweep** — `embed_batch()` and other zero-caller public methods. Mark intentional trait defaults with `#[allow(dead_code)]` + comment.
4. **Rule #12 one-time sweep** — grep all known bug class patterns across codebase NOW (not deferred to a commit template). Patterns: `let _ =` on I/O, bare `.await` in spawns, `Display` without `Error`.

### Sprint 3: Mechanical Enforcement

Promote CLAUDE.md rules from text to compiler/hooks:

| Rule | Enforcement | Mechanism |
|------|------------|-----------|
| #8 No `let _ =` on I/O | pre-commit grep | `grep -rn 'let _ =' src/ \| grep -v '// ok:' \| grep -v 'fmt::Write'` — fail if new matches. Existing intentional suppressions (`heartbeat` fire-and-forget) get `// ok: fire-and-forget` escape comment BEFORE this hook goes live. |
| #9 Wire or delete | compiler | `#![deny(dead_code)]` in lib.rs + targeted `#[allow(dead_code)]` on documented exceptions (trait defaults like `embed_batch`, `claim_batch`). Rust compiler catches this with zero false positives on concrete types. |
| #10 Timeout background await | pre-commit grep + escape | `grep -B2 'tokio::spawn'` without `timeout` in the spawned block — warn. Intentional fire-and-forget spawns (audit writes) get `// ok: fire-and-forget` escape. |
| #11 Display implies Error | unit test | `compile_fail` or runtime check: for each error enum implementing Display, assert it also implements Error. |
| #12 Fix the class | Sprint 2 sweep | One-time sweep in Sprint 2 (item 4). No commit template — that's LLM-compliance-dependent. |

### Sprint 4: Compound Audit

**Deterministic re-audit procedure** (not free-form agent analysis):

Checklist of 41 original findings from 2026-03-19 audit. For each:
- Still present? → count as finding
- Fixed but could regress? → verify test/hook exists
- Fixed with enforcement? → confirmed closed

Measure:
- Findings count: 41 → target <15
- HIGH count: 7 → target 0
- MEDIUM count: 12 → target <5
- Enforcement coverage: % of findings with mechanical prevention

## Execution Order

```
Sprint 1 → Sprint 2 → Sprint 3 → Sprint 4
  (foundation)  (TDD fixes)  (enforcement)  (measure)
```

Sprint 1 today. Each subsequent sprint builds on the previous.

## Risks

1. **SurrealDB test isolation** — Concurrent test runs could collide. Mitigation: use `test_<unix_millis>` DB names (millisecond granularity).
2. **Integration tests slow** — Real DB queries add latency. Mitigation: `#[ignore]` keeps `cargo test` fast, `make check-storage` is explicit.
3. **Pre-commit hooks slow down dev** — Mitigation: grep-based checks are <1s. Storage tests only run conditionally on storage file changes.
4. **build_flush_sql migration + production** — Moving SQL generation from domain to adapter while kernel is running a 60s flush loop. Mitigation: coordinated restart after deploy (standard `make deploy` flow).
5. **Rule #8 existing suppressions** — `let _ = self.heartbeat(...)` at surreal.rs:417,561 are intentional fire-and-forget. Mitigation: add `// ok: fire-and-forget` escape comments before Sprint 3 enforcement goes live.
6. **Re-audit non-determinism** — Agent-based audit produces variable results. Mitigation: Sprint 4 uses deterministic checklist against the 41 original findings, not free-form agent re-analysis.
