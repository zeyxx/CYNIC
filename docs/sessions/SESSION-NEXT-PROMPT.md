# Next Session — v0.8 "Guérir l'Organisme"

*Written 2026-03-27. Falsified 2026-03-27 (all 6 hypotheses survived). Ready for implementation.*

## Identity (settled)

CYNIC is the perennial epistemology made computational.
Read: `docs/architecture/CYNIC-PERENNIAL-EPISTEMOLOGY.md`

## Architectural Truths (FALSIFIED — approved for implementation)

Typestate rejected (evidence-based). Foundation: enum + 6 boundary enforcements across 3 axes.
All 6 hypotheses tested against actual codebase. All survived. One critical discovery relocated T7.
Read: `docs/architecture/CYNIC-ARCHITECTURAL-TRUTHS-V08.md`

**The 3 axes (each has a read-side gate + write/feedback-side gate):**

| Axis | Read gate | Write/feedback gate |
|---|---|---|
| Crystal lifecycle | MatureCrystal newtype (T4) | Content sanitization in StoragePort (T7) |
| Consensus | voter_count in Verdict (T5) | Quorum gate — pipeline AND REST handler (T8+T9) |
| Cache isolation | CacheKey newtype (T6) | — (single-sided) |

**STATUS: APPROVED.** All hypotheses falsification-tested against codebase:

| H# | Result | Key evidence |
|---|---|---|
| H1 | SURVIVES | `format_crystal_context` (pipeline.rs:159) = only crystal→prompt path |
| H2 | SURVIVES | `observe_crystal` = required method (no default). Need adapter-agnostic test. |
| H3 | SURVIVES | `lookup(&Embedding)` = only cache access. dogs_hash > domain in impact. |
| H4 | SURVIVES | **CONFIRMED:** single-Dog → max_disagreement=0 → full crystal weight |
| H5 | SURVIVES + RELOCATED | **`data.rs:211` bypasses pipeline.** T7 must be in StoragePort. |
| H6 | SURVIVES | Crystal state doesn't decay without obs. min_quorum=2, no starvation. |

**Critical discovery:** `POST /crystal/{id}/observe` bypasses pipeline entirely (no epistemic gate, no quorum). T7+T8 enforcement must be at StoragePort level, not pipeline-only.

## Implementation Order

Based on falsification results, implement in dependency order:

### Wave 0 — Foundation types (no behavior change, enables everything else)
1. `domain/dog.rs` — add `voter_count: usize` to Verdict struct
2. `domain/sanitize.rs` — new module: content sanitization (directive strip + length cap)
3. `domain/ccm.rs` — MatureCrystal newtype with private fields + TryFrom<Crystal>

### Wave 1 — Write-side enforcement (closes the bypass)
4. `domain/storage.rs` — observe_crystal signature: add `voter_count: usize`
5. `storage/surreal.rs` — enforce min_quorum in SQL + call sanitize on content + voter_count column
6. `pipeline.rs` — quorum gate in observe_crystal_for_verdict + pass voter_count
7. `api/rest/data.rs` — observe_crystal_handler: require voter_count or restrict endpoint

### Wave 2 — Read-side enforcement (compile-time safety)
8. `domain/ccm.rs` — format_crystal_context accepts &[MatureCrystal] + delimiter wrap
9. `pipeline.rs` — wire MatureCrystal conversion from storage results

### Wave 3 — Cache isolation
10. `domain/verdict_cache.rs` — CacheKey newtype with domain + dogs_hash

### Each wave: Fix → Test → Gate → Verify (Rule 25)

## Scope: v0.8 = ALL 51 Open Findings

See: `docs/audit/CYNIC-FINDINGS-TRACKER.md`

**4 root causes (close ~17 findings via 6 boundary enforcements):**
- RC-DEEP-A: epistemic gate bypass → StoragePort contract + MatureCrystal + content sanitization
- RC-DEEP-B: invisible degradation → voter_count in Verdict + quorum gate on crystallization
- RC-DEEP-C: prompt injection → sanitization at StoragePort level (covers pipeline + REST)
- RC-DEEP-D: cache cross-domain → CacheKey newtype

**Straightforward fixes (~6 findings, no research needed):**
- F2: X-Forwarded-For → ConnectInfo
- F13: content.chars().count()
- F22: /ready caching
- F23: /events connection limit
- F6: gemma parse failure
- RC1-6: event injection input validation

**Need research first (~7 findings):**
- F9/F10/F11: DeterministicDog heuristics (what patterns cause false positives?)
- F5: Sovereign concurrency (fundamental limitation?)
- F7: SurrealDB 401 (root cause investigation)
- RC7: Observability design (request_id propagation)

**Infrastructure/ops (~10 items):**
- RC2-3/4/5, RC3-2, RC6-1/2/3/4/5, RC8-4

**Benign/accepted (~12 items):**
- A1-A7 concurrency warns, F1/F3/F4/F8/F12 accepted

## Rules to Follow

- Rule 1: Diagnose before fixing
- Rule 25: Fix → Test → Gate → Verify
- Rule 31: Measure before AND after
- Rule 34: Falsify before adopting (**DONE** — all hypotheses passed)
- Workflow: `/build` after any kernel code change
- Workflow: `/cynic-kernel` before touching kernel source

## References

- Identity: `docs/architecture/CYNIC-PERENNIAL-EPISTEMOLOGY.md`
- Architectural truths: `docs/architecture/CYNIC-ARCHITECTURAL-TRUTHS-V08.md`
- Findings tracker: `docs/audit/CYNIC-FINDINGS-TRACKER.md`
- Deep audit: `docs/audit/CYNIC-DEEP-AUDIT-2026-03-25.md`
- VERSION.md: root
