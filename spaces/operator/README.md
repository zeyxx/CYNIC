# CYNIC Operator Space (Obsidian)

This space is the **Constituent Knowledge Base** of the sovereign operator (Titouan Corneloup / zeyxx).

It was named `vault/` until 2026-09-30. It is not called that any more, and the reason is
worth keeping: the runtime model store is `/var/lib/cynic/vault` — GGUF weights, declared
in `infra/registry.json` and described under `layers.runtime_state.tree.key_paths` in
`infra/directory-tree.json`. A doctrine tree and a weight store sharing one identifier is a
collision that `registry.json` exists to prevent, and it was live.

## Epistemic Authority Boundary

1. **Human Sovereign Space**: exploratory thinking, emergent hypotheses, mathematical
   intuitions, thermodynamic models, and research notes. Nothing here is a defect.
2. **Zero Runtime Execution Authority**: notes in this space are **pre-legislative**. No
   agent or machine may use them as execution authority, or as justification to mutate code
   or bypass runtime invariants.
3. **The Specification Gateway**: an idea here becomes active framework code only by being
   distilled into a specification under `specs/` with explicit invariants, bounds, and
   testable properties.
4. **Not indexed, on purpose**: nothing here appears in `contracts/claims.json`. Indexing it
   would grant runtime standing this space explicitly disclaims. It becomes a claim only on
   promotion through `specs/`.

## Naming

| Path | Meaning |
|---|---|
| `exploratory/` | Was `canon/`. Renamed because the word claims authority that the README above withdraws, and an authority a reader may invoke is the failure mode this repo has already paid for eleven times. |
| `exploratory/traditions/` | 30 files. Partly refuted — the 50-tradition claim was audited at 36–37 listed and 16 unassigned. Kept as operator space, marked non-normative. |
| `exploratory/whitepaper/` | 12 files. Cosmology, tokenomics, DEFCON, swarms. Refuted by red team as *philosophical marketing in the shape of a technical protocol*. Kept, non-normative. |
| `concepts/` | The material worth keeping. Two files. |

## Authority

Canonical architecture authority lives in `infra/CANONICAL_CRATE_MAP.md` v3 and
`infra/ARCHITECTURE_CONSTELLATIONS.md`. Mixed EN/FR content here is expected and is outside
the English-only technical-substrate invariant, which governs code, contracts, and schemas.
