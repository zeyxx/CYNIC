# CYNIC Market — Open Mathematical Questions

## Status

Questions ouvertes avec propositions de résolution. Subordonné à
`contracts/foundation.json`, `AGENTS.md`, `MARKET_ARCHITECTURE.md`.
Chaque question est documentée avec : contexte, proposition, méthode
de validation.

---

## Q1. Hausdorff Dimension de la variété des prédictions

**Contexte** : Le subagent audit a proposé que le Proof-of-Coverage (PoCoverage)
est un ε-net mathématique, mesuré via Hausdorff dimension.

**Question** : Quelle métrique définit la "distance" entre deux prédictions ?

**Proposition** : Utiliser la **distance de Wasserstein** (Earth Mover's Distance).
- Pour chaque prédiction, on a une distribution P(outcome)
- W(P, Q) = coût minimal pour transformer P en Q
- L'espace est borné (probabilités ∈ [0,1])
- La dimension de Hausdorff est calculable empiriquement par box-counting

**Méthode** : Calculer W pour 1000 paires de prédictions. Estimer la dimension.

**Priorité** : MEDIUM

**Statut** : résolu (proposition)

---

## Q2. Dimensionalité de la formule PoW

**Contexte** : La formule proposée mélange des unités incohérentes.

**Question** : Comment normaliser les unités ?

**Proposition** : Séparer en sous-scores normalisés + agrégation multiplicative.
```
PoW_base  = normalize(tasks) × normalize(success_rate) × normalize(complexity)
PoW_cost  = normalize(E_compute) × normalize(latency)
PoW_judge = normalize(judge_quality)

PoW = (PoW_base / PoW_cost) × PoW_judge
```
Chaque dimension est normalisée individuellement (z-score ou min-max).

**Méthode** : Data-centric. Collecte de données réelles → distributions → validation croisée.

**Priorité** : HIGH

**Statut** : résolu (proposition)

---

## Q3. KL Divergence vs Jensen-Shannon

**Contexte** : Le subagent a mentionné KL divergence. Gemini a proposé JSD.

**Question** : Quelle divergence utiliser pour mesurer le consensus entre jurés ?

**Proposition** : Utiliser JSD exclusivement. KL seulement pour des cas spécifiques (information mutuelle).

**Méthode** : JSD est symétrique, bornée [0,1], racine carrée est une métrique.

**Priorité** : LOW

**Statut** : résolu

---

## Q4. Convergence vers la vérité

**Contexte** : Le marché converge-t-il vers des prédictions calibrées ?

**Question** : Sous quelles conditions le système converge-t-il vers la vérité ?

**Proposition** : Bayesian Truth Serum (Prelec 2004) adapté.
- Pour chaque proposition, demander : "Que va-t-il se passer ?" ET "Que pensent les autres ?"
- Le BTS récompense les acteurs dont la prédiction est meilleure que la moyenne
  ET qui sous-estiment la moyenne (ils voient ce que les autres ne voient pas)
- Formule : `BTS_bonus = log(P_réel / P_moyenne) + log(P_réel / P_autre_réel)`

**Méthode** : Simulation avec acteurs calibrés/non-calibrés. Mesurer la convergence après N résolutions.

**Résultats de simulation** (100 acteurs, 200 propositions, zero-sum) :
- Authority-competence correlation : **0.9026** ✅ (forte corrélation)
- BTS gaming advantage : **-0.0469** ✅ (gamers légèrement désavantagés)
- Authority distribution : mean=0.52, std=0.42 (bonne dispersion)
- Convergence : pas de réduction d'erreur significative sur 200 propositions

**Garde-fous nécessaires** :
- Zero-sum authority updates (empêche la montée uniforme)
- Conformity penalty (pénalise les prédictions trop proches de la moyenne)
- BTS bonus asymétrique (pénalise plus les mauvaises prédictions que les bonnes récompensent)

**Limite identifiée** : La convergence n'est pas garantie sur 200 propositions. Il faut soit plus de propositions, soit un mécanisme de "truth serum" plus agressif (pénalité exponentielle pour la conformité).

**Priorité** : HIGH

**Statut** : résolu (proposition + simulation)

---

## Q5. Seuil de collusion tolérable

**Contexte** : Combien de colluders le système peut-il tolérer ?

**Question** : Quel est le seuil de collusion qui brise le marché ?

**Proposition** : Seuil dynamique basé sur la distribution d'autorité.
- Seuil classique BFT : f < n/3 (33% de colludeurs)
- Avec pondération d'autorité : `seuil_effectif = (1 - HHI_autorité) / 3`
  où HHI = Herfindahl-Hirschman Index (concentration)
- Si variance des prédictions < seuil → circuit breaker → escalade à l'opérateur

**Méthode** : Simulation de clusters de colludeurs avec différentes distributions d'autorité.

**Résultats de simulation** (100 acteurs, 200 propositions, zero-sum) :
| Fraction collusion | Avg HHI | Colluder advantage | Dangereux |
|--------------------|---------|-------------------|-----------|
| 10% (10 acteurs)   | 0.0134  | +0.0805           | NO ✅     |
| 20% (20 acteurs)   | 0.0127  | +0.0540           | NO ✅     |
| 30% (30 acteurs)   | 0.0125  | +0.0402           | NO ✅     |
| 40% (40 acteurs)   | 0.0121  | +0.0508           | NO ✅     |
| 50% (50 acteurs)   | 0.0117  | +0.0244           | NO ✅     |

- Aucun point dangereux détecté sur 5 graines
- Les colludeurs compétents gardent un léger avantage (+0.02 à +0.08)
- HHI bas (0.011-0.013) — pas de concentration dangereuse

**Découverte clé** : La collusion "intelligente" (compétents + coordination) est difficile à détecter. Les colludeurs compétents maintiennent un avantage sans déclencher le circuit breaker.

**Garde-fous nécessaires** :
- Zero-sum authority updates (empêche l'accumulation)
- Conformity penalty (pénalise les prédictions coordonnées)
- Détection de variance anormalement basse (circuit breaker)
- Seuils HHI dynamiques basés sur la distribution d'autorité

**Limite identifiée** : Les colludeurs compétents restent un risque. La détection doit être renforcée (analyse de motifs, détection de clusters).

**Priorité** : HIGH

**Statut** : résolu (proposition + simulation)

---

## Q6. Nombre minimal de jurés

**Contexte** : Le jury est 3→5→8 LLMs. Pourquoi ?

**Question** : Combien de LLMs pour que la variance soit statistiquement significative ?

**Proposition** : Power analysis adaptatif.
- N minimal : `n = (Z_{1-α/2} + Z_{1-β})² × 2σ² / d²`
- Avec α=0.05, β=0.2, σ=0.5, d=0.3 → n ≈ 5
- Si les LLMs sont corrélés (ρ > 0.3) : n × 1/(1-ρ)
- Règle : commencer à 3, mesurer la corrélation empirique, escalader si ρ > 0.3

**Méthode** : Mesurer la corrélation entre les prédictions des LLMs sur 100 propositions.

**Priorité** : MEDIUM

**Statut** : résolu (proposition)

---

## Q7. Justification de log(1+t)

**Contexte** : La pénalité temporelle est log(1+t)^0.3. Pourquoi ?

**Question** : Quelle est la justification théorique de log(1+t) ?

**Proposition** : Heuristic standard de "diminishing marginal utility". Justification informationnelle : l'information supplémentaire décroît avec le temps (taux d'information ∝ 1/t).

**Méthode** : Validation croisée avec d'autres fonctions (t^β, 1 - e^(-λt)).

**Priorité** : LOW

**Statut** : résolu (heuristic acceptable)

---

## Synthèse

| Question | Priorité | Méthode | Statut |
|----------|----------|---------|--------|
| Q1. Hausdorff dimension | MEDIUM | Distance de Wasserstein + box-counting | résolu (proposition) |
| Q2. Dimensionalité PoW | HIGH | Séparation sous-scores + normalisation | résolu (proposition) |
| Q3. KL vs JSD | LOW | JSD uniquement | résolu |
| Q4. Convergence vérité | HIGH | Bayesian Truth Serum | résolu (proposition + simulation) |
| Q5. Seuil collusion | HIGH | Seuil dynamique HHI + circuit breaker | résolu (proposition + simulation) |
| Q6. Jury minimal | MEDIUM | Power analysis adaptatif | résolu (proposition) |
| Q7. log(1+t) | LOW | Validation croisée | résolu (heuristic) |

## Prochaines étapes

1. **Simulation** : valider Q4 (BTS) et Q5 (collusion) empiriquement
2. **Peer-review** : soumettre à Ahmad (Graphs & Networks) et à la communauté
3. **Itération** : ajuster les poids et seuils basé sur les données
