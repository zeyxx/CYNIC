# Spécification d'Ingénierie Système : Architecture On-Chain Solana Anchor 0.30+ / 2.0

**Status :** Normatif & Contractuel (Layer 4 On-Chain Settlement — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. PROGRAM ANCHOR 0.30+ (programs/cynic-anchor/)                            │
│    • Cargo.toml : anchor-lang = "0.30.1", anchor-spl (token_2022), bytemuck │
│    • src/state.rs : Comptes Zero-Copy `#[account(zero_copy)]` + `Pod`       │
│    • src/lib.rs   : Instructions on-chain (PoH, Futarchy, Ignis Burn)       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. PDAS CANONIQUES SUR LA SVM SOLANA                                        │
│    • cynic_poh        : [b"cynic_poh", node_authority, epoch.to_le_bytes()] │
│    • futarchy_market  : [b"futarchy_market", proposal_hash]                 │
│    • ignis_vault      : [b"ignis_vault", mint]                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Zero-Copy)

### 1. Invariant Zero-Copy (`bytemuck::Pod`)
Pour économiser les *Compute Units* (CU) sur la machine virtuelle Solana (SVM) et éliminer les coûts de désérialisation Borsh :
* Tous les comptes d'état utilisent `#[repr(C)]` avec `bytemuck::Pod`.
* Les données sont lues et écrites directement en mémoire par l'`AccountLoader`.

### 2. Invariant de Combustion Déflationniste Token-2022 (Ignis)
* L'instruction `execute_ignis_burn` invoque directement le CPI natif `anchor_spl::token_2022::burn`.
* Aucun compte de retrait n'existe : les tokens envoyés dans le coffre Ignis sont détruits de manière cryptographiquement irréversible.

---

## III. HOW (Comment s'exécute le cycle on-chain ?)

```text
 1. Ancrage d'Époque (`record_poh_checkpoint`)
    ├── Le nœud soumet sa racine Merkle BLAKE3 7D et son digest Nightshift
    └── Enregistrement dans le PDA `cynic_poh`
            │
            ▼
 2. Marché de Futarchie (`create_futarchy_market` / `place_futarchy_bet`)
    ├── Création du marché avec liquidité initiale Odysseum
    ├── Pari CPMM sans float : dy = (y * dx) / (x + dx)
    └── Clôture & Résolution contre `min_conviction_threshold_bps`
            │
            ▼
 3. Combustion Déflationniste (`execute_ignis_burn`)
    ├── Appel CPI vers SPL Token-2022
    └── Incrément du compteur `total_burned_units`
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi Anchor 0.30+ ?**
   * *Gains* : Support complet du nouvel IDL standardisé, compatibilité native avec les extensions SPL Token-2022 et performances de compilation accrues.
2. **Pourquoi l'arithmétique entière fixe sur Solana ?**
   * Les flottants ne sont pas supportés nativement dans le BPF Solana pour des raisons de consensus déterministe. Tout le calcul de prix et de parts est effectué en points de base (`u32`) et en arithmétique `u128`.
