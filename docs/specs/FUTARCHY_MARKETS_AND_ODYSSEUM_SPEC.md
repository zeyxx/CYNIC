# Spécification d'Ingénierie Système : Moteur de Marchés de Futarchie & Odysseum

**Status :** Normatif & Contractuel (Layer 3 Futarchy Coordination — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. TYPES & SCHÉMAS (crates/cynic-intel/src/futarchy/types.rs)               │
│    • ConditionalMarket, MarketResolution, PredictionBet, MarketDecision     │
│    • Schéma : cynic.futarchy_market.v1                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. MOTEUR DE MATCHING CPMM (crates/cynic-intel/src/futarchy/cpmm.rs)        │
│    • Invariant de produit constant k = x * y en arithmétique u128/u64.      │
│    • calculate_prices_bps, execute_bet, resolve_market.                     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. COFFRES DE LIQUIDITÉ ODYSSEUM (crates/cynic-intel/src/futarchy/vault.rs) │
│    • LiquidityVault : Isolation du risque (Zero-Contagion).                 │
│    • Schéma : cynic.liquidity_vault.v1                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Formules)

### 1. Invariant de Marché CPMM en Arithmétique Entière
* Pour éviter toute instabilité de flottants, les prix sont calculés en points de base ($10\,000 \text{ bps} = 1.00$).
* **Prix du OUI** :
  $$\text{Price}_{\text{YES}} = \frac{\text{Pool}_{\text{NO}} \times 10\,000}{\text{Pool}_{\text{YES}} + \text{Pool}_{\text{NO}}}$$
* **Parts achetées lors d'un pari $dx$** :
  $$dy = \frac{y \cdot dx}{x + dx}$$

### 2. Invariant d'Isolation Odysseum (Zero-Contagion)
* Chaque marché conditionnel est financé par une allocation bornée d'un `LiquidityVault`.
* La perte ou la résolution d'un marché n'affecte jamais les réserves allouées aux autres marchés actifs.

---

## III. HOW (Comment s'exécute une décision par Futarchie ?)

```text
 1. Dépôt d'une Proposition de Gouvernance (`FutarchyProposal`)
            │
            ▼
 2. Création du Marché Conditionnel & Seeding (`create_market`)
    ├── Pool YES initial = N unités
    └── Pool NO initial  = N unités (Prix initial = 50.00%)
            │
            ▼
 3. Phase de Pari & Découverte de Prix (`execute_bet`)
    ├── Les participants parient en $CYNIC / Units
    └── Les prix oscillent selon la conviction du marché
            │
            ▼
 4. Clôture & Résolution Automatique (`resolve_market`)
    ├── Si Price(YES) >= Threshold (ex: 60%) -> Decision::Pass
    └── Si Price(YES) < Threshold            -> Decision::Fail
            │
            ▼
 5. Exécution Bornée par le Kernel (si Pass)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi la Futarchie plutôt que le vote démocratique 1-token-1-vote ?**
   * *Risque prévenu* : La ploutocratie et l'apathie des électeurs. En futarchie, les participants doivent engager du capital réel sur leurs prédictions ; les menteurs et incompétents sont financièrement pénalisés.
2. **Pourquoi l'arithmétique entière fixe pour le CPMM ?**
   * *Risque prévenu* : Divergence de calcul entre processeurs lors de la résolution de liquidité.
