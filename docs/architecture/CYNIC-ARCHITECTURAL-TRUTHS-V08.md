# CYNIC v0.8 — Architectural Truths for Invariant Enforcement

*Crystallized 2026-03-26. Amended 2026-03-27. Falsified 2026-03-27 (all 6 hypotheses tested against codebase).*
*Two research agents + crystallize-truth protocol × 2 + codebase falsification protocol (Rule 34).*

---

## The Question

How should CYNIC enforce epistemic invariants (crystal lifecycle, quorum, cache isolation) for the next 7 years?

## Research Conducted

1. **Production typestate analysis** — SquirrelFS (OSDI 2024), Statum, Serde, statig, enum_dispatch, session types, formal verification (Verus/Prusti). Sources from crates.io, Rust forum, academic papers.
2. **CYNIC codebase impact analysis** — every Crystal construction site, DB deserialization path, state check, function accepting Crystal, Verdict construction site, CacheEntry usage.
3. **Crystallize-truth protocol** — 10 modes, recursive descent (4 levels), metathinking.
4. **Write-side gap analysis (2026-03-27)** — code audit of store_crystal write paths, prompt injection chain (observe → content → Dog prompts), single-Dog crystallization feedback loop, StoragePort default method traps.
5. **Codebase falsification (2026-03-27)** — traced every hypothesis against actual code paths. H1-H6 all survived. Critical discovery: `POST /crystal/{id}/observe` (data.rs:211) bypasses pipeline entirely — T7 must live in StoragePort, not pipeline.

---

## Truth Statements

| T# | Truth | Confidence | Design impact |
|----|-------|------------|---------------|
| T1 | Full typestate is wrong for CYNIC — crystal transitions depend on runtime values (float confidence, int observations) that cannot be lifted into the type system | 58% | Keep CrystalState enum. No PhantomData approach. |
| T2 | The current enum + row_to_crystal + SQL-filtered queries is correct and matches every production Rust system with persistence | 55% | No restructuring of storage or domain layers |
| T3 | The disease is lack of enforcement at 3 specific boundaries, not wrong type representation | 58% | Focus on 3 boundaries, not everywhere |
| T4 | MatureCrystal newtype at format_crystal_context prevents highest-risk bypass (Forming in Dog prompts) at compile time | 55% ✓ | ONE typestate-adjacent change, highest ROI. **Falsified:** format_crystal_context is the ONLY crystal→prompt path. Dual gate (SQL+memory) already solid. MatureCrystal adds compile-time layer. |
| T5 | Contract enforcement (min_dogs/voter_count in StoragePort signature) + integration test is strictly stronger than application-layer check | 52% ✓ | Change observe_crystal signature, add **adapter-agnostic** contract test (not just SurrealDB). |
| T6 | CacheKey newtype requiring (embedding, domain, dogs_hash) makes cross-domain contamination impossible at construction time | 55% ✓ | Newtype replaces raw struct. **Falsified:** dogs_hash more impactful than domain (0.95 cosine threshold makes cross-domain via embedding unlikely). |
| **T7** | **Crystal content injected verbatim into Dog prompts — `POST /crystal/{id}/observe` (data.rs:211) bypasses pipeline entirely, calling StoragePort directly** | **58% ✓** | **Sanitization MUST be in StoragePort.observe_crystal (adapter layer), NOT pipeline. Defense-in-depth: also in format_crystal_context output. REST handler must gate or restrict.** |
| **T8** | **Single-Dog verdicts must NOT accumulate crystal observations — CONFIRMED: max_disagreement=0 → epistemic_gate returns weight=1.0 → full crystallization** | **58% ✓** | **Quorum gate (min_quorum=2) in BOTH pipeline observe_crystal_for_verdict AND REST observe_crystal_handler. Pipeline alone is insufficient.** |
| **T9** | **voter_count separates availability from integrity — serve single-Dog verdicts (transparency) but only consensus crystallizes (enforcement)** | **55% ✓** | **voter_count on ALL verdicts (RC-DEEP-B). Quorum gate on ALL crystal observation paths (pipeline + REST).** |

---

## Why Full Typestate Fails for CYNIC

1. **Runtime-computed transitions** — `classify(confidence, observations) -> CrystalState` operates on floats and ints from SurrealDB. Typestate can't enforce float comparisons at compile time.
2. **DB boundary erases types** — `row_to_crystal` reads `"forming"` string from JSON. Must have runtime match. SquirrelFS avoids this (persistent memory, no query layer). Statum explicitly handles it with enum at boundary.
3. **Mixed-state collections** — `list_crystals()` returns `Vec<Crystal>` with mixed states. Can't have typed heterogeneous collection without enum wrapper (rebuilds what exists).
4. **5 states, potential additional dimensions** — type explosion risk if orthogonal axes added later.
5. **Community consensus** — "Use typestate for API surface enforcement; use enums when state is determined by external input" (corrode.dev, Yosh Wuyts, hoverbear, Rust forum).

## What the Evidence Supports

### Architecture: Enum + 6 Boundary Enforcements (falsified against codebase)

Two crystal observation paths exist. Both must be gated:
```
Path A (pipeline): judge → side_effects → observe_crystal_for_verdict → StoragePort
Path B (REST):     POST /crystal/{id}/observe → observe_crystal_handler → StoragePort
                   ↑ BYPASSES pipeline entirely (data.rs:211)
```

Enforcement layers:
```
READ SIDE (crystal → Dog prompts):

  DB Layer (SurrealDB):
    SQL filters crystallized|canonical in KNN + domain queries  ← KEEP AS-IS
    row_to_crystal matches string → CrystalState enum           ← KEEP AS-IS

  Domain Layer:
    MatureCrystal newtype (Crystallized|Canonical only)          ← ADD (T4)
    format_crystal_context accepts &[MatureCrystal]              ← CHANGE (T4)
    format_crystal_context wraps content in delimiters           ← ADD (T7, defense-in-depth)

  Cache Layer:
    CacheKey newtype(embedding, domain, dogs_hash)               ← ADD (T6)
    Lookup requires exact CacheKey match                         ← CHANGE (T6)

WRITE SIDE (observation → crystal confidence):

  StoragePort (covers ALL callers — pipeline AND REST):
    observe_crystal sanitizes content BEFORE storage             ← ADD (T7)
    observe_crystal requires voter_count parameter               ← CHANGE (T5)
    Integration test: adapter REJECTS voter_count < min_quorum   ← ADD (T5)
    Adapter-agnostic contract test on trait (not just SurrealDB) ← ADD (H2 caveat)

  Pipeline (Path A):
    observe_crystal_for_verdict: quorum gate                     ← ADD (T8)
    dog_scores.len() < min_quorum → skip crystal obs             ← ADD (T8)

  REST Handler (Path B — data.rs:211):
    observe_crystal_handler: require voter_count in request      ← CHANGE (T8)
    OR restrict endpoint to admin-only                           ← ALTERNATIVE
    Content sanitization already enforced at StoragePort level   ← T7

  Verdict:
    voter_count: usize field in Verdict struct                   ← ADD (RC-DEEP-B)
    All verdicts carry voter_count (transparency)                ← T9
    Only quorum verdicts crystallize (integrity)                 ← T8+T9
```

### Migration Impact

**8 files, ~400 lines:**
- `domain/ccm.rs` — MatureCrystal newtype + format_crystal_context signature + delimiter wrap on output (T4, T7)
- `domain/storage.rs` — observe_crystal signature: add `voter_count: usize` param (T5)
- `domain/verdict_cache.rs` — CacheKey newtype, CacheEntry includes domain/dogs_hash (T6)
- `domain/dog.rs` — voter_count field in Verdict struct (RC-DEEP-B)
- `storage/surreal.rs` — observe_crystal adapter: enforce min_quorum + content sanitization on write + row_to_verdict voter_count (T5, T7)
- `pipeline.rs` — wire MatureCrystal conversion, quorum gate on observe_crystal_for_verdict, pass voter_count (T4, T8, T9)
- `api/rest/data.rs` — observe_crystal_handler: pass voter_count (from request or deny if absent), content sanitization already at StoragePort (T8)
- `domain/sanitize.rs` — content sanitization module: directive stripping + length cap (T7, new file)

**API surface:** `POST /crystal/{id}/observe` gains required `voter_count` field (breaking for direct callers — intentional, forces quorum declaration). Verdict responses gain `voter_count` (additive). `state` field unchanged.

### What This Closes

The 6 boundary enforcements address 4 root causes:

| Root Cause | Read-side enforcement | Write/feedback enforcement | Findings Closed |
|---|---|---|---|
| RC-DEEP-A (epistemic bypass) | MatureCrystal newtype (T4) | Content sanitization (T7) | F15, F16, F18, **F14**, Chain 1, Chain 3, Cancer |
| RC-DEEP-B (invisible degradation) | Verdict voter_count (T5) | Quorum gate on crystallization (T8+T9) | F20, 9 partials, Chain 2 visibility |
| RC-DEEP-C (prompt injection) | — | Delimiter sandwich + directive strip (T7) | **F14**, Chain 2 injection |
| RC-DEEP-D (cache cross-domain) | CacheKey newtype (T6) | — | F17, F19, Chain 4 |

~17 findings closed from 6 enforcements + tests. F14 promoted from "needs research" to "addressed by T7."
Remaining ~18 findings need individual fixes (DeterministicDog, observability, infra).

---

## Recursive Descent — Key Arguments Survived

**"Current enum is correct"** — survived counterargument ("if correct, why 51 findings?"). Answer: the enum represents state correctly; the BOUNDARY enforcement is what's missing.

**"MatureCrystal is justified"** — survived counterargument ("SQL already filters"). Answer: REST POST /crystal creates Forming crystals; list_crystals returns all states; non-semantic-search code path can bypass SQL filter.

**"Contract enforcement is the right level"** — survived counterargument ("parameter can be passed as 0"). Answer: trait signature + integration test is strictly stronger than application-layer check and mechanically testable.

### Amendment 2026-03-27 — New arguments survived

**"Prompt sanitization is v0.8, not deferred"** — survived counterargument ("attack requires API key, so it's not an escalation"). Answer: the key gives temporary access; the injection gives PERMANENT influence on all future verdicts in the domain. Also: legitimate content discussing AI safety ("ignore previous instructions") poisons accidentally. This is a correctness issue, not just security.

**"Quorum gate on crystallization, not on verdicts"** — survived counterargument ("hard enforcement makes the system unavailable when Dogs are down"). Answer: SEPARATE the concerns. Serve single-Dog verdicts for availability (with visible voter_count). Block single-Dog verdicts from crystallizing for integrity. The gate is in `observe_crystal_for_verdict`, not in `judge`.

**"store_crystal write bypass is v0.8.1, not v0.8"** — survived counterargument ("it's a concrete gap, fix it now"). Answer: in production, `store_crystal` creates Forming crystals (correct behavior). The MatureCrystal newtype at read time prevents Forming from reaching Dog prompts. The write-side risk is real but theoretical — defense-in-depth for a future pass, not a v0.8 blocker.

### Falsification 2026-03-27 — Codebase Verification (Rule 34)

All 6 hypotheses tested against actual code paths. Results:

| H# | Hypothesis | Result | Critical finding |
|---|---|---|---|
| H1 | MatureCrystal read gate | **SURVIVES** | `format_crystal_context` is the ONLY crystal→prompt path (pipeline.rs:159). Dual gate (SQL+memory) confirmed. |
| H2 | StoragePort contract | **SURVIVES** | `observe_crystal` is required (no default). Caveat: need adapter-agnostic contract test. |
| H3 | CacheKey newtype | **SURVIVES** | `lookup(&Embedding)` is the ONLY cache access path. dogs_hash > domain in impact. |
| H4 | voter_count + quorum | **SURVIVES** | **CONFIRMED:** single-Dog → max_disagreement=0 → epistemic_gate("agreed", 1.0) → full crystal weight (pipeline.rs:316). |
| H5 | Content sanitization | **SURVIVES + RELOCATED** | **`POST /crystal/{id}/observe` (data.rs:211) bypasses pipeline entirely.** T7 must live in StoragePort, not pipeline. |
| H6 | Quorum + availability | **SURVIVES** | Crystal state doesn't decay without observations. min_quorum=2 causes no starvation. |

**Critical discovery (H5):** The REST handler `observe_crystal_handler` calls `storage.observe_crystal` directly — no pipeline, no epistemic gate, no quorum check. An authenticated client can crystallize arbitrary content in ~42 seconds via 21 direct API calls. This means:
- T7 (sanitization) must be in the `StoragePort` adapter, not the pipeline
- T8 (quorum gate) must also be enforced at the StoragePort level OR the REST handler must require `voter_count`
- Pipeline-only enforcement creates a false sense of security

This discovery **strengthened** the plan: the layered defense now covers both observation paths (pipeline + REST) instead of assuming all observations flow through the pipeline.

---

## What Needs Research Before Implementation

| Item | Research needed | Why |
|---|---|---|
| F9/F10/F11 (DeterministicDog) | What patterns cause false positives? Real examples needed | Heuristic changes without data = worse heuristics |
| F5 (Sovereign concurrency) | Fundamental serial queue limitation. Solvable? | May need inference engine change, not kernel fix |
| F7 (SurrealDB 401) | Root cause investigation | Intermittent, needs reproduction |
| RC7 (Observability) | request_id propagation design | Architectural: how does ID flow through PipelineDeps? |
| F14 (Prompt injection) | ~~Delimiter sandwich implementation~~ **PROMOTED to T7 — addressed in boundary enforcements** | Design is: sanitize on write (observe_crystal) + delimiter on read (format_crystal_context) |
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
