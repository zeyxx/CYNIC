# Moteur épistémique pour agents (protocole scientifique)

## Status

Draft — created as part of Robinho100 onboarding (task `robinho100-epistemic-engine`).

## Purpose

Define an epistemic engine for agents: a scientific protocol that turns
hypotheses into falsifiable claims, runs physical probes, and propagates
verified facts into the registry. Builds on CYNIC's Popperian falsification
loop (see `contracts/federation-mycelium-policy.json` → `epistemic_engine`).

## Scope

- Hypothesis generation and null-hypothesis formulation.
- Destructive physical probes with receipts.
- Popperian verdict and fact-registry update.
- Retroactive truth propagation.

## Next steps

- Formalize data structures and ports.
- Implement adapters behind FCIS boundaries.
- Add audit ratchets and build proofs on the builder.
EOF