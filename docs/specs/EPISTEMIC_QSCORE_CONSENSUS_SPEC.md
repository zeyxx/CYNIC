# Spécification d'Ingénierie Système : Moteur Épistémique du Q-Score Borné en φ (61.8%)

**Status :** Normatif & Contractuel (Layer 3 Epistemic Judgment — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. TYPES & CONSTANTES D'OR (crates/cynic-intel/src/epistemic/types.rs)      │
│    • PHI_INV_BPS (6180), PHI_INV_SQ_BPS (3820), HOWL_THRESHOLD_BPS (5279)  │
│    • Schéma : cynic.epistemic_verdict.v1 & cynic.crystal.v1                 │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. CONSENSUS GÉOMÉTRIQUE (crates/cynic-intel/src/epistemic/consensus.rs)   │
│    • trimmed_mean_bps : Élimination des extrêmes (anti-outlier).            │
│    • geometric_mean_6_axioms : Moyenne géométrique des 6 axiomes.           │
│    • check_anomaly : Détection de divergence résiduelle (> φ⁻²).            │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. CYCLE FIBONACCI DES CRISTAUX (crates/cynic-intel/src/epistemic/crystal.rs│
│    • F8=21 observations -> Crystallized, F13=233 observations -> Canonical.│
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants Mathématiques)

### 1. Invariant de Bornage d'Or ($\varphi^{-1} = 61.8034\%$)
* **Loi de Kénose Épistémique** : Aucun modèle, aucun nœud ni aucun consensus ne peut prétendre à une certitude supérieure à l'inverse du Nombre d'Or :
  $$\text{QScore} = \min\left(\sqrt[6]{\prod_{i=1}^6 \text{Axiom}_i},\; \varphi^{-1}\right) \le 6180 \text{ bps}$$
* **Conséquence** : L'espace résiduel ($1 - \varphi^{-1} = 38.1966\% = \varphi^{-2}$) représente le **doute calibré universel**.

### 2. Les 6 Axiomes Fondamentaux
1. **Fidelity** : Fidélité au réel, exactitude factuelle.
2. **Phi** : Harmonie structurelle, proportion et minimalisme.
3. **Verify** : Réfutabilité, testabilité, présence de preuves d'exécution.
4. **Culture** : Respect des patterns et de la continuité historique.
5. **Burn** : Efficience thermodynamique, absence de gaspillage computationnel.
6. **Sovereignty** : Préservation de l'autonomie et de l'indépendance de l'opérateur.

---

## III. HOW (Comment s'exécute le consensus ?)

```text
 1. Évaluation Parallèle par N Validateurs (Dogs: Qwen, Llama, Heuristique)
            │
            ▼
 2. Moyenne Tronquée par Axiome (`trimmed_mean_bps`)
    └── Si >= 4 Dogs : suppression du score minimum et maximum
            │
            ▼
 3. Moyenne Géométrique 6D (`geometric_mean_6_axioms`)
    └── Un seul axiome effondré fait chuter la note globale
            │
            ▼
 4. Biais de Bornage d'Or (`min(raw, 6180)`)
            │
            ▼
 5. Détection d'Anomalie Résiduelle (`check_anomaly`)
    └── Si Max(Dog) - Min(Dog) > 3820 bps -> Flag Anomaly
            │
            ▼
 6. Classification du Verdict (`VerdictRating`)
    ├── Howl  (>= 5279 bps) : Exceptionnel
    ├── Wag   (>= 3820 bps) : Favorable
    ├── Growl (>= 2361 bps) : Douteux
    └── Bark  (< 2361 bps)  : Rejeté
            │
            ▼
 7. Mise à jour de la Connaissance Cristallisée (`CrystalKnowledge`)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi la moyenne géométrique plutôt qu'arithmétique ?**
   * *Principe de Non-Compensation* : En moyenne arithmétique, un 100/100 en "Culture" peut masquer un 0/100 en "Verify". La moyenne géométrique exige l'équilibre : un seul zéro détruit la note globale.
2. **Pourquoi les seuils de Fibonacci pour la cristallisation (21, 233) ?**
   * Empêche qu'une hallucination temporaire d'un modèle devienne une règle établie. Un motif doit survivre à 21 validations indépendantes pour se cristalliser, et 233 pour devenir loi canonique du système.
