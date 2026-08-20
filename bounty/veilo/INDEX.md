# Bounty Veilo — Consolidation des Findings (audit parallèle 6 agents)

**Program**: GYy4kM6GHhpgLCUscuABbzkD2ZbJ2fneYryaZ6Ch7fFU (privacy_pool, mainnet)
**Deadline**: 2026-08-20T22:59:59 UTC (~19h restantes)
**Agent access**: HUMAN_ONLY → soumission humaine requise

## Couverture audit (100% des fichiers du program)
| Fichier | Auditeur | Résultat | Finding éligible |
|---------|----------|----------|------------------|
| perps.rs + predictions.rs | Agent 1 | SÛR (over-mint impossible, signer strict, checked arith, PDA uniques) | Aucun critique. Finding A (MEDIUM, sous-crédit SOL `close_account` sweep) + B (LOW/TBD circuit) |
| zk.rs + groth16.rs | Agent 4 | Soundness pairing OK | BUG 1 HAUTE (VK TRANSACTION/SWAP partagent α/β/γ — à confirmer hors-ligne, non soumis tel quel) + BUG 2/3 FAIBLE (mitigés) |
| lib.rs | Agent 6 | Cœur sain (double-spend bloqué, root vérifiée, PDA uniques) | F-1 (MEDIUM, initialize_global_config sans accès) + F-2 (FAIBLE, TVL divergence) |
| phoenix.rs | Agent 0 (relancé) | SÛR hors V-01 connu | V-01 (overcredit ember_unwrap, DÉJÀ PUBLIC → backup) + 2 MEDIUM non-extractibles (dstTrader non validé, max_slot_amount inflation) |
| swap.rs | Agent 2 (relancé) | **§1 HIGH extractable** | **Swap §1 (relayer skim full spread — NOUVEAU, reproductible)** + §2 MEDIUM (claimant binding circuit-only) |
| positions.rs + merkle_tree.rs | Agent 3 (relancé) | SÛR (double-spend bloqué, amounts proof-bound, vault borné) | Aucun critique |

## CIBLE DE SOUMISSION RECOMMANDÉE
**Swap §1** — `transact_swap` relayer skims the entire swap spread.
- Fichier: `swap.rs` (L54-57, L827-828, L851-868, L860) + `lib.rs` `SwapParams::hash()`
- Mécanisme: `swap_data_hash` NON proof-bound → relayer contrôle le route DEX, `relayer_fee` only floored (L860), jamais plafonné → extraction répétable du spread entier.
- PoC: `swap_s1_skim.rs` (test Rust logic-level, exécutable) + draft conceptuel dans `submission_swap_s1.md`.
- Éligible scope: "critical logic / access control bug that allows funds to be extracted".

## BACKUP (si Swap §1 rejeté comme "design choice")
- V-01 (phoenix_ember_unwrap overcredit) — `backup_v01.md` — MAIS DÉJÀ PUBLIC (risque doublon).

## STATUS DES ARTEFACTS
- [x] `submission_swap_s1.md` — draft soumission Swap §1 (affected/explanation/fund-loss/PoC/fix)
- [x] `swap_s1_skim.rs` — PoC Rust logic-level (exécutable, prouve le gap)
- [x] `backup_v01.md` — V-01 backup
- [x] `../veilo_perps_predictions_audit.md` — rapport agent 1
- [x] `../veilo_phoenix_audit.md` — rapport agent 0
- [x] `../veilo_swap_audit.md` — rapport agent 2 (détaillé §1)
- [x] `../veilo_lib_audit.md` — rapport agent 6
- [x] `../veilo_positions_audit.md` — rapport agent 3
- [ ] `anchor build` en cours (proc_abcc9afc5ba5) — si OK, IDL dispo pour test TS plus riche

## PROCHAINES ÉTAPES (humain)
1. Vérifier compte Superteam + wallet Solana (réception 2000 USDC).
2. Finaliser soumission Swap §1 sur superteam.fun/earn/listing/veilo-bounty avant 22:59:59 UTC.
3. Joindre `swap_s1_skim.rs` + citations de lignes + explication.
4. Optionnel: si `anchor build` réussit, enrichir avec test TS sur localnet.
