# Design Protocol — the scientific engine for agents

## Status

Foundation rule. Every design session (human or agent) obeys it, or its
output is not merged. Short on purpose: opposable beats exhaustive.

## The engine

CYNIC's scientific engine is not one tool but five organs working as a loop:

| Organ | Role | Lives in |
|---|---|---|
| Protocol (this file) | How a session is run | `docs/foundation/DESIGN_PROTOCOL.md` |
| Judge | Falsification gate before merge | `cynic-judge` skill (7 axioms, geometric Q, 61.8% ceiling) |
| Ledger | Append-only proof + emergence log | `docs/specs/cynic-traces-carnet.md` |
| Traces | The sessions themselves, preserved lossless | `cynic-traces` crate + frozen captures |
| Retrieval | Future agents grounded on proven past | `docs/specs/geodesic-retrieval-delta.md` |

Sessions run under the Protocol → judged → logged → preserved → retrieved.
Each loop turns agent sessions into proven knowledge. That loop IS the engine.

## The five gates

1. **Hypothesis + refutation first.** No design without a written claim and
   the observation that would refute it. (VERIFY)
2. **Pre-register.** Spec or plan lands before execution, never after.
   A post-hoc spec is a rationalization, not a protocol.
3. **Machine proof over narrative.** Counts, hashes, builder verdicts, parity
   checks. A claim without read-back is a draft. (FIDELITY)
4. **Adversarial audit.** A `cynic-judge` pass seeks refutation before merge.
   Geometric Q: one axiom at zero rejects the whole. Confidence never
   exceeds 61.8%; residual unknowns are declared, never hidden.
   (COHERENCE, CULTURE, BURN, SOVEREIGNTY, KENOSIS)
5. **Append-only log.** Proofs and emergences go to the ledger. Never rewrite
   an entry — supersede it with a new one.

## Non-negotiables

- No silent best-effort: disagreement fails closed to the operator.
- No stubs, no placeholders, no post-hoc specs.
- Restraint counts as output: an empty diff can be the right verdict. (KENOSIS)
