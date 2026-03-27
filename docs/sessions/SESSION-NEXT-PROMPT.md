# Next Session — v0.8 "Guérir l'Organisme"

*Written 2026-03-27. Start here.*

## Identity (settled)

CYNIC is the perennial epistemology made computational.
Read: `docs/architecture/CYNIC-PERENNIAL-EPISTEMOLOGY.md`

## Architectural Truths (researched, not yet falsified)

Typestate rejected (evidence-based). Proposed foundation: enum + MatureCrystal newtype + StoragePort contract + CacheKey newtype + voter_count.
Read: `docs/architecture/CYNIC-ARCHITECTURAL-TRUTHS-V08.md`

**STATUS: HYPOTHESES, NOT DECISIONS.** Apply scientific method (Rule 34) before implementation:

1. **Hypothesis:** MatureCrystal newtype at format_crystal_context prevents the highest-risk bypass
   - **Falsification test:** Can we find a code path where Forming crystals reach Dog prompts WITHOUT going through format_crystal_context?
   - **If falsified:** The newtype protects the wrong boundary

2. **Hypothesis:** StoragePort min_dogs/voter_count contract prevents epistemic gate bypass
   - **Falsification test:** Can the SurrealDB adapter ignore the parameter? Can a new adapter bypass it?
   - **If falsified:** Contract is too weak; need sealed trait or integration-test-enforced gate

3. **Hypothesis:** CacheKey newtype prevents cross-domain contamination
   - **Falsification test:** Can cache be hit without constructing a proper CacheKey? Any other lookup path?
   - **If falsified:** Cache has other access paths that bypass the key

4. **Hypothesis:** voter_count in Verdict makes degradation visible
   - **Falsification test:** Do consumers (REST, MCP, UI) actually USE voter_count? Or is it a field nobody reads?
   - **If falsified:** Structural visibility without consumers = dead architecture (Rule 21)

## Protocol for This Session

Apply scientific method to each hypothesis:
```
1. State hypothesis clearly
2. Design falsification test (what would disprove it?)
3. Run the test against the actual codebase
4. If survives → approved for implementation
5. If falsified → reformulate or discard, find alternative
```

Only implement what survives falsification.

## Scope: v0.8 = ALL 51 Open Findings

See: `docs/audit/CYNIC-FINDINGS-TRACKER.md`

**3 root causes (close ~15 findings via 3 boundary enforcements):**
- RC-DEEP-A: epistemic gate bypass → StoragePort contract + MatureCrystal
- RC-DEEP-B: invisible degradation → voter_count in Verdict
- RC-DEEP-D: cache cross-domain → CacheKey newtype

**Straightforward fixes (~6 findings, no research needed):**
- F2: X-Forwarded-For → ConnectInfo
- F13: content.chars().count()
- F22: /ready caching
- F23: /events connection limit
- F6: gemma parse failure
- RC1-6: event injection input validation

**Need research first (~8 findings):**
- F9/F10/F11: DeterministicDog heuristics (what patterns cause false positives?)
- F5: Sovereign concurrency (fundamental limitation?)
- F7: SurrealDB 401 (root cause investigation)
- F14: Prompt injection implementation
- RC7: Observability design (request_id propagation)

**Infrastructure/ops (~10 items):**
- RC2-3/4/5, RC3-2, RC6-1/2/3/4/5, RC8-4

**Benign/accepted (~12 items):**
- A1-A7 concurrency warns, F1/F3/F4/F8/F12 accepted

## VERSION.md

v0.8 section needs rewriting from the new identity + architectural truths. Derive gates AFTER falsification — don't write gates for approaches that might be disproved.

## Rules to Follow

- Rule 1: Diagnose before fixing
- Rule 25: Fix → Test → Gate → Verify
- Rule 31: Measure before AND after
- Rule 34: **Falsify before adopting** (NEW — this session's protocol)
- Workflow: `/build` after any kernel code change
- Workflow: `/cynic-kernel` before touching kernel source

## References

- Identity: `docs/architecture/CYNIC-PERENNIAL-EPISTEMOLOGY.md`
- Architectural truths: `docs/architecture/CYNIC-ARCHITECTURAL-TRUTHS-V08.md`
- Findings tracker: `docs/audit/CYNIC-FINDINGS-TRACKER.md`
- Deep audit: `docs/audit/CYNIC-DEEP-AUDIT-2026-03-25.md`
- VERSION.md: root
- Session memory: `project_session_2026_03_26_identity.md`
