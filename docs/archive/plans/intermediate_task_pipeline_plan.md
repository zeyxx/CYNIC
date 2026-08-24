# Plan Intermédiaire — Pipeline Task CYNIC

> **SUPERSEDED le 2026-08-24** par
> `.hermes/plans/2026-08-24_universal-time-energy-task-roi-plan.md`.
>
> Raisons de la supersession :
> 1. La progression de slots « lab 200 → staging 150 → stress 100 → canary 50 →
>    production 350 ms » était une supposition sans base observationnelle.
> 2. Le « mempool SQLite /var/lib/cynic/lifecycle/task-mempool.db » décrit ici
>    n'a jamais existé ; la file réelle est `DomainTaskQueue` (JSONL).
> 3. Le concept de slot/temps a été requalifié : universel et fractal, pas
>    propre à l'inférence ni au combat LLM.
> 4. Les matrices (`ContentionMatrix` λ ad hoc, `PoHCompressionMatrix`) et le
>    `HardwareBaseline` (cpu_cores/ram_gb non sondés) sont à refondre selon le
>    plan remplaçant (TickWindow/EnergyBudget/CapacityObservation).
>
> Contenu historique conservé ci-dessous pour traçabilité.

---

# Contenu original (historique)

## Contexte
Formalisation mathématique du système de queue/tâches CYNIC, inspiré de la pipeline Solana
(slot time reduction + PoH batching). Objectif: modèle testable, pas de supposition.

## Modèle mathématique — Théorème de convergence (CYNIC-POH)

Variables:
- `S` : slot time cible (ms)
- `E` : epoch size (tasks)
- `T(E)` : latence totale d'une époque
- `C` : compute budget par slot (tasks max/slot)
- `λ` : contention factor (λ = 0 = no contention)

Formule:
```
S_target = T(E) / (C * E) * (1 + λ)
```

Ajustement par epochs, convergence vers min théorique du matériel.

## Matrices à implémenter (Rust)
1. `SlotTimingMatrix` — slots/epoch, compute budget, convergence
2. `ContentionMatrix` — λ, blocking vs non-blocking
3. `PoHCompressionMatrix` — merkle root vs task count

## Status actuel
- Observation du substrate fait (SurrealDB SPOF, /var/lib/cynic = 105G utilisés)
- PoH batch existant dans `cynic-protocol/src/anchor_client.rs` (epoch, merkle_root, mais pas de slot_time)
- Phase: formalisation théorème + SlotTimingMatrix en cours

## Pending
- Plan de combat LLM Pi-harness (audit injection, benchmark, code audit)
- Implémentation TaskEpoch + SQLite mempool schema
- Execution combat + read-back proof

## Notes
- SurrealDB = cold storage (SPOF identifié, doit rester non-bloquant)
- SQLite mempool = hot path (source de vérité locale)
- Filesystem JSON = warm (read-back proofs versionnés)
