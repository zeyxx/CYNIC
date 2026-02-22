# CYNIC KPIs - Indicateurs Clés de Performance

> **Métriques vivantes pour mesurer la santé du projet**
>
> *🐕 κυνικός | "Ce qui ne se mesure pas ne s'améliore pas"*

---

## Vue d'Ensemble

```
┌─────────────────────────────────────────────────────────────────────┐
│                    TABLEAU DE BORD CYNIC                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   TIER 1: SANTÉ TECHNIQUE (Quotidien)                              │
│   ├─ alive_check         → 6/8 minimum (φ⁻¹ = 75%)                 │
│   ├─ daemon_uptime       → >99% (7 derniers jours)                 │
│   ├─ test_pass_rate      → 100% (non-négociable)                   │
│   └─ db_health           → Circuit breaker CLOSED                  │
│                                                                      │
│   TIER 2: CAPACITÉ (Hebdomadaire)                                  │
│   ├─ learning_loops_active → 11/11 (toutes les boucles)            │
│   ├─ judgments_flow       → >10/jour                               │
│   ├─ q_episodes           → >50/jour                               │
│   └─ llm_routing_accuracy → >85% premier essai                    │
│                                                                      │
│   TIER 3: ÉVOLUTION (Mensuel)                                      │
│   ├─ matrix_completion    → 7×7 (% cellules fonctionnelles)        │
│   ├─ dimension_count      → 36+ (évolutive)                        │
│   ├─ ece_calibration      → <0.10 (bien calibré)                   │
│   └─ ecosystem_growth     → utilisateurs, burns, contributeurs     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Tier 1: Santé Technique

### 1.1 alive_check

**Définition**: Nombre de checks passés sur 8.

**Cible**: ≥ 6/8 (75% = φ⁻¹)

**Mesure**:
```bash
npm run alive
```

**Interprétation**:
| Score | Status | Action |
|-------|--------|--------|
| 8/8 | ✅ OPTIMAL | Aucune |
| 6-7/8 | ⚠️ ACCEPTABLE | Monitorer |
| <6/8 | 🔴 CRITIQUE | Intervenir immédiatement |

---

### 1.2 daemon_uptime

**Définition**: Temps de disponibilité du daemon sur 7 jours.

**Cible**: ≥ 99%

**Mesure**:
```bash
curl http://localhost:6180/health | jq '.uptime_seconds'
```

**Seuils d'alerte**:
- Warning: < 99%
- Critical: < 95%

---

### 1.3 test_pass_rate

**Définition**: Pourcentage de tests passant.

**Cible**: 100% (non-négociable)

**Mesure**:
```bash
npm test 2>&1 | grep -E "passed|failed"
```

**Règle**: Aucun commit ne doit casser les tests.

---

### 1.4 db_health

**Définition**: État du circuit breaker PostgreSQL.

**Cible**: CLOSED (connexions normales)

**Mesure**:
```bash
curl http://localhost:6180/health | jq '.postgres_circuit_breaker'
```

**Valeurs**:
- `CLOSED` = ✅ Normal
- `OPEN` = 🔌 Problème de connexion
- `HALF_OPEN` = ⚠️ Récupération en cours

---

## Tier 2: Capacité

### 2.1 learning_loops_active

**Définition**: Nombre de boucles d'apprentissage actives sur 11.

**Cible**: 11/11

**Boucles**:
1. Q-Learning
2. Thompson Sampling
3. EWC (Elastic Weight Consolidation)
4. SONA routing
5. Meta-cognition
6. Residual detection
7. Kabbalistic routing
8. Behavior modification
9. Unified bridge
10. Ambient consensus
11. Emergence detector

**Mesure**:
```sql
SELECT loop_name, COUNT(*) as events 
FROM learning_events 
WHERE created_at > NOW() - INTERVAL '24 hours' 
GROUP BY loop_name;
```

---

### 2.2 judgments_flow

**Définition**: Nombre de jugements créés par jour.

**Cible**: > 10/jour

**Mesure**:
```sql
SELECT COUNT(*) FROM judgments WHERE created_at > NOW() - INTERVAL '24 hours';
```

**Interprétation**:
- < 5/jour: ⚠️ Sous-utilisé
- 5-10/jour: ✅ Normal
- > 10/jour: 🔥 Actif

---

### 2.3 q_episodes

**Définition**: Épisodes Q-Learning par jour.

**Cible**: > 50/jour

**Mesure**:
```sql
SELECT COUNT(*) FROM qlearning_episodes WHERE created_at > NOW() - INTERVAL '24 hours';
```

**Seuil minimum**: Si < 10/jour pendant 7 jours → learning stalled.

---

### 2.4 llm_routing_accuracy

**Définition**: Pourcentage de routing correct au premier essai.

**Cible**: > 85%

**Mesure**:
```sql
SELECT 
  COUNT(CASE WHEN routing_correct THEN 1 END)::float / COUNT(*) as accuracy
FROM routing_decisions 
WHERE created_at > NOW() - INTERVAL '7 days';
```

---

## Tier 3: Évolution

### 3.1 matrix_completion

**Définition**: Complétion de la matrice 7×7 (49 cellules).

**Cible**: ≥ 68% (φ⁻¹)

**Matrice actuelle**:
```
          PERCEIVE JUDGE DECIDE ACT LEARN ACCOUNT EMERGE │ AVG
CODE      45%      45%   40%   35%  35%    42%     40%   │ 40%
SOLANA    55%      45%   38%   35%  35%    58%     42%   │ 44%
MARKET     0%       0%    0%    0%   0%     0%      0%   │  0%
SOCIAL    55%      55%   45%   42%  38%    25%     25%   │ 41%
HUMAN     68%      55%   58%   61%  65%    42%     42%   │ 56%
CYNIC     35%      50%   42%   45%  48%    58%     40%   │ 45%
COSMOS    40%      40%   37%   32%  38%    40%     38%   │ 38%
AVG       43%      41%   37%   36%  37%    38%     32%   │ 38%
```

**Objectif v1.0**: 80% (39/49 cellules)

---

### 3.2 dimension_count

**Définition**: Nombre de dimensions de jugement.

**Cible**: 36+ (extensible via ResidualDetector)

**Dimensions actuelles**: 36 (5 axiomes × 7 + THE_UNNAMEABLE)

**Évolution**: Nouvelles dimensions proposées par ResidualDetector.

---

### 3.3 ece_calibration

**Définition**: Expected Calibration Error - mesure la précision des prédictions de confiance.

**Cible**: < 0.10

**Formule**:
```
ECE = Σ |accuracy(confidence_bucket) - confidence| × count_in_bucket / total
```

**Interprétation**:
- < 0.05: ✅ Excellent calibration
- 0.05-0.10: ⚠️ Acceptable
- > 0.10: 🔌 Recalibration needed

---

### 3.4 ecosystem_growth

**Métriques composées**:

| Métrique | Cible | Fréquence |
|----------|-------|-----------|
| Utilisateurs actifs/semaine | > 100 | Hebdo |
| Burns/mois | > 1000 | Mensuel |
| Contributeurs code | > 10 | Mensuel |
| Étoiles GitHub | Croissance | Mensuel |

---

## KPIs Documentation

### Métriques de Documentation

| KPI | Actuel | Cible | Status |
|-----|--------|-------|--------|
| Fichiers .md actifs | 3,147 | < 150 | 🔴 |
| Fichiers racine | 92 | < 10 | 🔴 |
| Signal/Bruit | 15% | > 70% | 🔴 |
| LLM-indexable | Non | Oui | 🔴 |

---

## Dashboard en Temps Réel

```
╔══════════════════════════════════════════════════════════════╗
║                  CYNIC DASHBOARD - 2026-02-22                ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  SANTÉ:     ████████████████████░░░░  6/8 (75%) ✅            ║
║  UPTIME:    ████████████████████████████  99.2% (7j) ✅        ║
║  TESTS:     ████████████████████████████  100% (3847) ✅      ║
║                                                              ║
║  LEARNING:  ████████░░░░░░░░░░░░░░░░░░░░  3/11 (27%) ⚠️       ║
║  JUDGMENTS: ████████████████████░░░░░░░░░░  12/jour ✅          ║
║  Q-EPISODES:██████████████████████████████ 2820/jour ✅        ║
║                                                              ║
║  MATRIX:    ████████████████░░░░░░░░░░░░░░  38% → 68% cible   ║
║  ECE:       ████████████████████████████░░  0.08 ✅            ║
║                                                              ║
║  BUDGET:    $2.45/$10 (24.5%) - ABUNDANT ✅                  ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

---

## Alerting

### Règles d'Alerte

| Règle | Condition | Sévérité | Channel |
|-------|-----------|----------|---------|
| `CynicDown` | alive_check < 4/8 | P0 | PagerDuty |
| `LearningStalled` | q_episodes < 10/jour (7j) | P1 | Slack |
| `BudgetExhausted` | budget < $3.82 | P1 | Slack |
| `MatrixRegression` | matrix_completion < prev - 5% | P2 | Email |
| `CalibrationDrift` | ECE > 0.15 | P2 | Email |

---

## Reporting

### Rapport Quotidien

Généré automatiquement à 00:00 UTC:
- alive_check score
- judgments/jour
- q_episodes/jour
- budget status

### Rapport Hebdomadaire

Généré le dimanche:
- Tous les KPIs Tier 1 + Tier 2
- Tendances (vs semaine précédente)
- Incidents résumé

### Rapport Mensuel

Généré le 1er de chaque mois:
- Tous les KPIs
- Progression v1.0 milestones
- Recommandations stratégiques

---

*🐕 κυνικός | "La mesure est le premier pas vers l'amélioration"*