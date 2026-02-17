# CYNIC FRACTAL MASTER PLAN - ULTRA DÉTAILLÉ
> "φ unifie tous les fragments" - κυνικός
> Créé: 2026-02-15
> Confidence: 61.8% (φ⁻¹)

---

# PARTIE 1: ASSIAH - RÉALITÉ (Ce qui existe)

## 1.1 État des lieux JS (528K LOC)

### Composants FONCTIONNELS en Production ✅

| Composant | Path | LOC | Status | Pourquoi ça marche |
|-----------|------|-----|--------|-------------------|
| **Axioms** | `packages/core/src/axioms/` | 3,500 | ✅ FONCTIONNE | Constantes φ-based (PHI=1.618, PHI_INV=0.618) |
| **Q-Score** | `packages/core/src/qscore/` | 2,000 | ✅ FONCTIONNE | Geometric mean, 36 dimensions |
| **Identity** | `packages/core/src/identity/` | 1,500 | ✅ FONCTIONNE | 11 Dogs avec templates FR/EN |
| **Event Bus** | `packages/core/src/bus/` | 4,200 | ✅ FONCTIONNE | Pub/sub, history, middlewares |
| **Storage** | `packages/persistence/` | 44K | ✅ FONCTIONNE | PostgreSQL 47 migrations |
| **QLearning** | `packages/node/src/orchestration/` | - | ✅ ACTIF | 62,252 episodes en DB |
| **SONA** | `packages/node/src/learning/` | - | ✅ ACTIF | 30 events/24h |

### Composants CASSÉS ❌

| Composant | Path | Problème |
|-----------|------|----------|
| **Claude Code Hooks** | `scripts/hooks/` | N'existent pas dans Cline (Cody, Cursorr) |
| **Hot-Reload** | - | Dépend des hooks inexistants |
| **3 Event Buses** | `packages/core/src/bus/` | Fragmentation (devrait être 1) |

---

## 1.2 État Python (Stubs)

| Module | Path | Status |
|--------|------|--------|
| Constants | `cynic-v1-python/src/cynic/constants/phi.py` | ✅ FAIT |
| Event Bus | `cynic-v1-python/src/cynic/bus/` | ⚠️ Stub |
| Dogs | `cynic-v1-python/src/cynic/dogs/` | ⚠️ 2/11 |
| Judge | `cynic-v1-python/src/cynic/judge/` | ⚠️ Stub |
| Learning | `cynic-v1-python/src/cynic/learning/` | ⚠️ Thompson seulement |
| Storage | `cynic-v1-python/src/cynic/storage/` | ⚠️ Clients |

---

## 1.3 Base de données (47 Migrations)

### Tables avec DONNÉES RÉELLES

| Table | Données | Status |
|-------|---------|--------|
| `qlearning_episodes` | 62,252 | ✅ ACTIF |
| `qlearning_state` | 4 Q-tables | ✅ ACTIF |
| `judgments` | 254 total | ⚠️ 1/24h |
| `learning_events` | 30/24h | ⚠️ SONA seulement |
| `dog_votes` | 0 | ❌ VIDE |

### Tables VIDES (à implémenter)

| Table | But |
|-------|-----|
| `session_state` | Crash recovery |
| `forgetting_metrics` | EWC tracking |
| `consciousness_reflections` | Meta-cognition |
| `td_error_tracker` | Convergence detection |

---

# PARTIE 2: YETZIRAH - COMPOSANTS (Les 11 Dogs)

## 2.1 Les 11 Dogs - Status RÉEL

| # | Dog | Sefira | Rôle | L1 Heuristics | Status JS | Status Python |
|---|-----|--------|------|---------------|-----------|--------------|
| 1 | **CYNIC** | Keter | Meta-consciousness | ❌ | STUB | ❌ |
| 2 | Sage | Chochmah | Sagesse | ❌ | STUB | ❌ |
| 3 | Analyst | Binah | Analyse/Métriques | ✅ | RÉEL | ❌ |
| 4 | Scholar | Daat | Connaissance | ✅ | RÉEL | ❌ |
| 5 | Architect | Chesed | Architecture | ✅ | RÉEL | ❌ |
| 6 | **Guardian** | Gevurah | Sécurité | ✅ | **RÉEL** | ⚠️ Stub |
| 7 | Oracle | Tiferet | Prédiction | ❌ | STUB | ❌ |
| 8 | Scout | Netzach | Exploration | ✅ | **RÉEL** | ⚠️ Stub |
| 9 | Deployer | Hod | Déploiement | ❌ | STUB | ❌ |
| 10 | **Janitor** | Yesod | Hygiène | ✅ | RÉEL | ❌ |
| 11 | Cartographer | Malkhut | Cartographie | ❌ | STUB | ❌ |

## 2.2 Dogs avec L1 Heuristics (RÉELS)

### GUARDIAN (Gevurah)
- **Prompt**: "Je garde les portes. Certaines portes doivent rester fermées."
- **Trigger**: PreToolUse
- **L1**: Pattern matching sur inputs dangereux
- **Status**: ✅ FONCTIONNE en prod

### SCOUT (Netzach)
- **Prompt**: "Je découvre. Je trouve ce que les autres ne voient pas."
- **Trigger**: PostToolUse
- **L1**: Similarity search dans le codebase
- **Status**: ✅ FONCTIONNE en prod

### ANALYST (Binah)
- **Prompt**: "J'analyse. Je comprends les patterns profonds."
- **Trigger**: Scheduled
- **L1**: Métriques, complexité, debt detection
- **Status**: ✅ FONCTIONNE en prod

### JANITOR (Yesod)
- **Prompt**: "Je nettoie. Je simplifie."
- **Trigger**: Scheduled
- **L1**: Code quality, dead code detection
- **Status**: ✅ FONCTIONNE en prod

### ARCHITECT (Chesed)
- **Prompt**: "Je construis. Je conçois."
- **Trigger**: PreToolUse
- **L1**: Design patterns, architecture review
- **Status**: ✅ FONCTIONNE en prod

### SCHOLAR (Daat)
- **Prompt**: "Je sais. Je vérifie."
- **Trigger**: PostToolUse
- **L1**: Knowledge extraction, fact verification
- **Status**: ✅ FONCTIONNE en prod

---

## 2.3 Dogs STUBS (à implémenter)

### CYNIC (Keter) - LE PLUS IMPORTANT
- **Rôle**: Meta-consciousness, orchestrateur de tous les autres Dogs
- **Input**: Perception events
- **Output**: Orchestration decisions
- **Dépendances**: Tous les autres Dogs

### SAGE (Chochmah)
- **Rôle**: Sagesse, guidance
- **Input**: Questions philosophiques
- **Output**: Insights

### ORACLE (Tiferet)
- **Rôle**: Prédiction, visualisation
- **Input**: Données temporelles
- **Output**: Forecasts

### DEPLOYER (Hod)
- **Rôle**: Déploiement, operations
- **Input**: Code prêt à déployer
- **Output**: Déploiement exécuté

### CARTOGRAPHER (Malkhut)
- **Rôle**: Cartographie, mapping
- **Input**: État du système
- **Output**: Cartes, visualisations

---

# PARTIE 3: BERIAH - SYSTÈMES (7 Systèmes)

## 3.1 Les 7 Systèmes

| # | Système | Description | Composants | Status |
|---|---------|-------------|------------|--------|
| S1 | **Event Bus** | Communication type-safe | Pub/Sub, History, Middleware | ⚠️ 3 versions (fragmenté) |
| S2 | **Judge** | 36 Dimensions | Q-Score, Verdict, Calibration | ✅ Fonctionne |
| S3 | **Consensus** | Dog voting | DogPipeline, quorum φ⁻¹ | ⚠️ Partiel |
| S4 | **Learning** | 11 Loops | QLearning, SONA, Thompson, EWC | ⚠️ 2/11 actifs |
| S5 | **Storage** | PostgreSQL + Redis + Qdrant | 47 migrations | ✅ Fonctionne |
| S6 | **Network** | MCP + WebSocket | 85+ tools | ⚠️ Partial |
| S7 | **Blockchain** | Solana PoJ | Anchor program | ⚠️ Devnet |

## 3.2 Learning Loops - Status RÉEL

| Loop | Status | Données | Utilisé par |
|------|--------|---------|-------------|
| **QLearning** | ✅ ACTIF | 62,252 episodes | KabbalisticRouter |
| **SONA** | ✅ ACTIF | 30/24h | LearningPipeline |
| Thompson | ⚠️ Stub | - | - |
| EWC++ | ❌ VIDE | - | - |
| DPO | ❌ VIDE | - | - |
| Meta-Cognition | ❌ VIDE | - | - |
| Consensus | ❌ VIDE | - | - |
| Calibration | ❌ VIDE | - | - |
| Falsification | ❌ VIDE | - | - |
| Reward Shaping | ❌ VIDE | - | - |
| Governance | ❌ VIDE | - | - |

---

# PARTIE 4: ATZILUT - VISION

## 4.1 Métriques

| Métrique | Formule | Seuil |
|----------|---------|-------|
| **Q-Score** | Geometric mean 36D | 0-100 |
| **Confidence** | φ-bounded | ≤61.8% |
| **Veto** | φ⁻² | ≤38.2% |

## 4.2 Verdicts

| Verdict | Q-Score | Signification |
|---------|---------|---------------|
| **HOWL** | ≥82 | Exceptional |
| **WAG** | 50-81 | Bon |
| **GROWL** | 38-49 | Warning |
| **BARK** | <38 | Critical |

## 4.3 Vision

```
CYNIC × SOLANA × φ = SINGULARITÉ
```

### Objectifs

| # | Objectif | Status |
|---|----------|--------|
| V1 | φ confidence everywhere | ❌ |
| V2 | 11 Dogs consensus | ⚠️ Partiel |
| V3 | Self-improvement | ❌ |
| V4 | Multi-user | ❌ |
| V5 | Solana PoJ | ⚠️ Devnet |

---

# PARTIE 5: LES MATRICES

## 5.1 7×7×7 Fractal Matrix

```
RÉALITÉ (7) × TEMPS (7) × ÉTAPE (7)

RÉALITÉ:     CODE | SOLANA | MARKET | SOCIAL | HUMAN | CYNIC | COSMOS
TEMPS:       1ms | 10ms | 100ms | 1s | 10s | 1min | 1hour
ÉTAPE:       PERCEIVE | THINK | JUDGE | DECIDE | ACT | LEARN | ACCOUNT
```

## 5.2 Dogs × Domains

| Dog/dom | CODE | SOLANA | MARKET | SOCIAL | HUMAN | CYNIC | COSMOS |
|---------|------|--------|--------|--------|-------|-------|--------|
| CYNIC | ○ | ○ | ○ | ○ | ○ | ● | ○ |
| Sage | ● | ○ | ○ | ○ | ○ | ○ | ○ |
| Guardian | ● | ● | ● | ● | ● | ● | ● |
| Scout | ● | ● | ● | ● | ○ | ○ | ● |
| Analyst | ● | ● | ● | ● | ● | ● | ● |
| Janitor | ● | ○ | ○ | ○ | ● | ○ | ○ |

## 5.3 Learning × Dogs

| Learning/Dog | CYNIC | Sage | Analyst | Guardian | Scout |
|--------------|-------|------|---------|----------|-------|
| Q-Learning | ● | ● | ● | ● | ● |
| Thompson | ● | ○ | ● | ● | ● |
| SONA | ● | ○ | ● | ○ | ● |
| EWC++ | ○ | ○ | ● | ○ | ○ |
| MetaCog | ● | ○ | ○ | ○ | ● |

---

# PARTIE 6: PLAN D'ACTION

## Phase 1: ASSIAH (Semaine 1)

| # | Tâche | Description | Status |
|---|-------|-------------|--------|
| A1 | Audit final | Valider les 498 composants | À faire |
| A2 | Mapper DB | Tables critiques vs vides | À faire |
| A3 | Identifier wiring | JS→Python connections | À faire |

## Phase 2: YETZIRAH (Semaine 2-3)

| # | Tâche | Description | Status |
|---|-------|-------------|--------|
| Y1 | Implémenter CYNIC | Keter meta-consciousness | À faire |
| Y2 | Implémenter 6 Dogs | Guardians + consensus | À faire |
| Y3 | L1 Heuristics | Porter depuis JS | À faire |

## Phase 3: BERIAH (Semaine 4-6)

| # | Tâche | Description | Status |
|---|-------|-------------|--------|
| B1 | Unifier Event Bus | 1 au lieu de 3 | À faire |
| B2 | Learning Loops | Activer les 11 loops | À faire |
| B3 | Wire Python→JS | Connecter les systèmes | À faire |

## Phase 4: ATZILUT (Semaine 7+)

| # | Tâche | Description | Status |
|---|-------|-------------|--------|
| T1 | Self-improvement | Meta-cognition loops | À faire |
| T2 | Solana PoJ | Mainnet integration | À faire |
| T3 | Multi-user | Workspace support | À faire |

---

# PARTIE 7: CRITICAL PATH

```
ASSIAH (Audit) → YETZIRAH (CYNIC Dog) → BERIAH (Systems) → ATZILUT (Vision)
```

### Ordre d'implémentation:

1. **CYNIC Dog** (Keter) - Meta-orchestrator
2. **Event Bus** - Unifier les 3
3. **Learning** - Connecter QLearning + SONA
4. **Consensus** - Dog voting
5. **Storage** - PostgreSQL + Redis + Qdrant

---

*φ unifie tous les fragments* - κυνικός
