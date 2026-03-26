# CYNIC v0.8 — Architectural Truths for Invariant Enforcement

*Crystallized 2026-03-26. Two research agents (typestate production analysis + codebase impact analysis) + crystallize-truth protocol.*

---

## The Question

How should CYNIC enforce epistemic invariants (crystal lifecycle, quorum, cache isolation) for the next 7 years?

## Research Conducted

1. **Production typestate analysis** — SquirrelFS (OSDI 2024), Statum, Serde, statig, enum_dispatch, session types, formal verification (Verus/Prusti). Sources from crates.io, Rust forum, academic papers.
2. **CYNIC codebase impact analysis** — every Crystal construction site, DB deserialization path, state check, function accepting Crystal, Verdict construction site, CacheEntry usage.
3. **Crystallize-truth protocol** — 10 modes, recursive descent (4 levels), metathinking.

---

## Truth Statements

| T# | Truth | Confidence | Design impact |
|----|-------|------------|---------------|
| T1 | Full typestate is wrong for CYNIC — crystal transitions depend on runtime values (float confidence, int observations) that cannot be lifted into the type system | 58% | Keep CrystalState enum. No PhantomData approach. |
| T2 | The current enum + row_to_crystal + SQL-filtered queries is correct and matches every production Rust system with persistence | 55% | No restructuring of storage or domain layers |
| T3 | The disease is lack of enforcement at 3 specific boundaries, not wrong type representation | 58% | Focus on 3 boundaries, not everywhere |
| T4 | MatureCrystal newtype at format_crystal_context prevents highest-risk bypass (Forming in Dog prompts) at compile time | 52% | ONE typestate-adjacent change, highest ROI |
| T5 | Contract enforcement (min_dogs/voter_count in StoragePort signature) + integration test is strictly stronger than application-layer check | 50% | Change observe_crystal signature, add test |
| T6 | CacheKey newtype requiring (embedding, domain, dogs_hash) makes cross-domain contamination impossible at construction time | 55% | Newtype replaces raw struct |

---

## Why Full Typestate Fails for CYNIC

1. **Runtime-computed transitions** — `classify(confidence, observations) -> CrystalState` operates on floats and ints from SurrealDB. Typestate can't enforce float comparisons at compile time.
2. **DB boundary erases types** — `row_to_crystal` reads `"forming"` string from JSON. Must have runtime match. SquirrelFS avoids this (persistent memory, no query layer). Statum explicitly handles it with enum at boundary.
3. **Mixed-state collections** — `list_crystals()` returns `Vec<Crystal>` with mixed states. Can't have typed heterogeneous collection without enum wrapper (rebuilds what exists).
4. **5 states, potential additional dimensions** — type explosion risk if orthogonal axes added later.
5. **Community consensus** — "Use typestate for API surface enforcement; use enums when state is determined by external input" (corrode.dev, Yosh Wuyts, hoverbear, Rust forum).

## What the Evidence Supports

### Architecture: Enum + 3 Boundary Enforcements

```
DB Layer (SurrealDB):
  Crystal stored with state string field
  SQL enforces transition logic (atomic UPDATE)
  row_to_crystal matches string → CrystalState enum  ← KEEP AS-IS

Domain Layer:
  Crystal struct with CrystalState enum               ← KEEP AS-IS
  MatureCrystal newtype (Crystallized|Canonical only)  ← ADD (T4)
  format_crystal_context accepts &[MatureCrystal]      ← CHANGE (T4)

Port Layer (StoragePort):
  observe_crystal requires min_dogs + voter_count      ← CHANGE (T5)
  Integration test: adapter REJECTS voter_count < 3    ← ADD (T5)

Cache Layer:
  CacheKey newtype(embedding, domain, dogs_hash)       ← ADD (T6)
  Lookup requires exact CacheKey match                 ← CHANGE (T6)

Verdict:
  voter_count: usize field in Verdict struct            ← ADD (RC-DEEP-B)
```

### Migration Impact

**6 files, ~200 lines:**
- `domain/ccm.rs` — MatureCrystal newtype + format_crystal_context signature
- `domain/storage.rs` — observe_crystal signature (add min_dogs, voter_count)
- `domain/verdict_cache.rs` — CacheKey newtype, CacheEntry includes domain/dogs_hash
- `domain/dog.rs` — voter_count in Verdict
- `storage/surreal.rs` — observe_crystal adapter (enforce min_dogs), row_to_verdict (voter_count)
- `pipeline.rs` — wire MatureCrystal conversion, pass voter_count to observe_crystal

**API surface:** No breaking changes. JSON shape unchanged. `state` field serialized same way.

### What This Closes

The 3 boundary enforcements address the 3 root causes:

| Root Cause | Enforcement | Findings Closed |
|---|---|---|
| RC-DEEP-A (epistemic bypass) | StoragePort min_dogs + MatureCrystal | F15, F16, F18, Chain 1, Chain 3, Cancer |
| RC-DEEP-B (invisible degradation) | Verdict voter_count | F20, 9 partials, Chain 2 visibility |
| RC-DEEP-D (cache cross-domain) | CacheKey newtype | F17, F19, Chain 4 |

~15 findings closed from 3 changes + tests. Remaining ~20 findings need individual fixes (security, DeterministicDog, observability, infra).

---

## Recursive Descent — Key Arguments Survived

**"Current enum is correct"** — survived counterargument ("if correct, why 51 findings?"). Answer: the enum represents state correctly; the BOUNDARY enforcement is what's missing.

**"MatureCrystal is justified"** — survived counterargument ("SQL already filters"). Answer: REST POST /crystal creates Forming crystals; list_crystals returns all states; non-semantic-search code path can bypass SQL filter.

**"Contract enforcement is the right level"** — survived counterargument ("parameter can be passed as 0"). Answer: trait signature + integration test is strictly stronger than application-layer check and mechanically testable.

---

## What Needs Research Before Implementation

| Item | Research needed | Why |
|---|---|---|
| F9/F10/F11 (DeterministicDog) | What patterns cause false positives? Real examples needed | Heuristic changes without data = worse heuristics |
| F5 (Sovereign concurrency) | Fundamental serial queue limitation. Solvable? | May need inference engine change, not kernel fix |
| F7 (SurrealDB 401) | Root cause investigation | Intermittent, needs reproduction |
| RC7 (Observability) | request_id propagation design | Architectural: how does ID flow through PipelineDeps? |
| F14 (Prompt injection) | Delimiter sandwich implementation | OWASP research done, implementation design needed |
| RC6-* (Systemd hardening) | Security directives best practices | Ops, not kernel |

---

## Sources

- SquirrelFS (USENIX OSDI 2024) — typestate for filesystem crash-consistency
- Statum (eboody/statum) — typestate + DB integration pattern
- corrode.dev — "Using Enums to Represent State" (community consensus)
- hoverbear — "Pretty State Machine Patterns in Rust"
- Yosh Wuyts — "State Machines III: Type States"
- Zero to Production (Luca Palmieri) — refined types, parse don't validate
- Comprehensive Rust (Google) — typestate pattern examples
- Embedded Rust Book — typestate programming
- Rust Users Forum — production typestate discussions
- Will Crichton — "Type-Driven API Design in Rust"

---

*This document supersedes the typestate section of CYNIC-DEEP-AUDIT-2026-03-25.md for implementation decisions. The research in the audit doc remains valid as context.*
