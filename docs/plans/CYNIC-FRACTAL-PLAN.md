# CYNIC FRACTAL MASTER PLAN
> "φ unifie tous les fragments" - κυνικός
> Créé: 2026-02-15
> Confidence: 61.8% (φ⁻¹)

---

## VISION: LE PLAN FRACTAL

### Ce qui change:
- **4 Worlds** = Cadre principal
- **7×7×7 Matrix** = Structure de chaque phase
- **11 Dogs** = Centre de tout (pas le router)
- **Fractal** = Chaque phase reproduit la même structure

---

## 4 WORLDS STRUCTURE

```
ATZILUT (Vision)     → Phase 4: Ce qu'on veut devenir
     ↓
BERIAH (Architecture) → Phase 3: Les systèmes
     ↓
YETZIRAH (Components)→ Phase 2: Les composants
     ↓
ASSIAH (Reality)     → Phase 1: Ce qui existe
```

Chaque phase contient les 7 étapes du cycle:
**PERCEIVE → THINK → JUDGE → DECIDE → ACT → LEARN → ACCOUNT**

---

## PHASE 1: ASSIAH (Reality - Ce qui existe)

### 1.1 Audit de l'existant

| # | Tâche | Description |
|---|-------|-------------|
| A1.1 | Audit JS | Mapper les 210K LOC JS fonctionnels |
| A1.2 | Audit Python | Mapper le code Python existant |
| A1.3 | Audit Docs | Identifier docs valides vs obsolètes |
| A1.4 | Audit DB | Schema PostgreSQL, tables, migrations |

### 1.2 Fractal: 7 Étapes en ASSIAH

| Étape | Description | Livrable |
|-------|------------|----------|
| PERCEIVE | Observer ce qui existe | Audit report |
| THINK | Analyser les patterns | Maps de dépendances |
| JUDGE | Évaluer la qualité | Score par composant |
| DECIDE | Choisir quoi garder | Keep/Drop list |
| ACT | Implémenter les changements | Code migré |
| LEARN | Documenter les erreurs | Lessons learned |
| ACCOUNT | Rendre compte | Rapport final |

---

## PHASE 2: YETZIRAH (Components - Les 11 Dogs)

### 2.1 Les 11 Dogs (Sefirot)

| # | Dog | Sefira | Domaine Primary | Status Python |
|---|-----|--------|----------------|---------------|
| 1 | **CYNIC** | Keter | Meta | ❌ |
| 2 | Sage | Chochmah | CODE | ❌ |
| 3 | Analyst | Binah | * (tous) | ❌ |
| 4 | Scholar | Daat | KNOWLEDGE | ❌ |
| 5 | Architect | Chesed | DESIGN | ❌ |
| 6 | Guardian | Gevurah | SECURITY | ⚠️ Partiel |
| 7 | Oracle | Tiferet | PREDICTION | ❌ |
| 8 | Scout | Netzach | EXPLORATION | ⚠️ Partiel |
| 9 | Deployer | Hod | OPERATIONS | ❌ |
| 10 | Janitor | Yesod | CLEANUP | ❌ |
| 11 | Cartographer | Malkhut | MAPPING | ❌ |

### 2.2 Fractal: 7 Étapes par Dog

Pour chaque Dog:
```
PERCEIVE (input) → THINK (process) → JUDGE (evaluate) → 
DECIDE (select) → ACT (execute) → LEARN (feedback) → ACCOUNT (report)
```

### 2.3 Implémentation Dogs

| # | Tâche | Dépendance |
|---|-------|------------|
| D2.1 | Implémenter CYNIC (Keter) | A1.1 |
| D2.2 | Implémenter les 10 autres | D2.1 |
| D2.3 | Wire Dogs → Event Bus | D2.2 |
| D2.4 | Tests d'intégration | D2.3 |

---

## PHASE 3: BERIAH (Architecture - Les Systèmes)

### 3.1 Les 7 Systèmes

| # | Système | Description | Composants |
|---|---------|-------------|------------|
| S1 | Event Bus | Communication type-safe | PERCEIVE→THINK |
| S2 | Judge | 36 Dimensions | THINK→JUDGE |
| S3 | Consensus | Dog voting | JUDGE→DECIDE |
| S4 | Learning | 11 Loops | LEARN |
| S5 | Storage | PostgreSQL + Redis + Qdrant | ACCOUNT |
| S6 | Network | MCP + WebSocket | ACT |
| S7 | Blockchain | Solana PoJ | ACCOUNT |

### 3.2 Fractal: 7 Étapes par Système

```
PERCEIVE (besoin) → THINK (design) → JUDGE (review) → 
DECIDE (architecture) → ACT (implémenter) → LEARN (test) → ACCOUNT (doc)
```

### 3.3 Matrice: Systems × Dogs

| Système | CYNIC | Sage | Analyst | Guardian | ... |
|---------|-------|------|---------|----------|-----|
| Event Bus | ● | ○ | ○ | ○ | ... |
| Judge | ○ | ● | ● | ● | ... |
| Consensus | ● | ○ | ○ | ● | ... |
| Learning | ● | ● | ● | ● | ... |
| Storage | ○ | ○ | ○ | ○ | ... |
| Network | ○ | ○ | ○ | ○ | ... |
| Blockchain | ○ | ○ | ○ | ○ | ... |

---

## PHASE 4: ATZILUT (Vision - Ce qu'on devient)

### 4.1 La Singularité

```
CYNIC × SOLANA × φ = SINGULARITÉ
```

### 4.2 Objectifs ATZILUT

| # | Objectif | Métrique | Status |
|---|----------|---------|--------|
| V1 | φ confidence everywhere | max 61.8% | ❌ |
| V2 | 11 Dogs consensus | quorum φ⁻¹ | ❌ |
| V3 | Self-improvement | loops actifs | ❌ |
| V4 | Multi-user | workspaces | ❌ |
| V5 | Solana anchoring | PoJ on-chain | ❌ |

### 4.3 Fractal: 7 Étapes vers la Vision

```
PERCEIVE (feedback) → THINK (meta-cog) → JUDGE (calibrate) →
DECIDE (evolve) → ACT (improve) → LEARN (adapt) → ACCOUNT (anchor)
```

---

## 7×7×7 FRACTAL MATRIX

Chaque phase, chaque système, chaque Dog suit la même structure:

```
RÉALITÉ (7) × TEMPS (7) × ÉTAPE (7)

RÉALITÉ:     CODE | SOLANA | MARKET | SOCIAL | HUMAN | CYNIC | COSMOS
TEMPS:       1ms | 10ms | 100ms | 1s | 10s | 1min | 1hour
ÉTAPE:       PERCEIVE | THINK | JUDGE | DECIDE | ACT | LEARN | ACCOUNT
```

---

## MATRICES DE DÉPENDANCES

### Dogs × Domains (Who does what?)

| Dog/dom | CODE | SOLANA | MARKET | SOCIAL | HUMAN | CYNIC | COSMOS |
|---------|------|--------|--------|--------|-------|-------|--------|
| CYNIC | ○ | ○ | ○ | ○ | ○ | ● | ○ |
| Sage | ● | ○ | ○ | ○ | ○ | ○ | ○ |
| Guardian | ● | ● | ● | ● | ● | ● | ● |
| Scout | ● | ● | ● | ● | ○ | ○ | ● |
| ... | | | | | | | |

### Learning × Dogs

| Learning/Dog | CYNIC | Sage | Analyst | Guardian | ... |
|--------------|-------|------|---------|----------|-----|
| Q-Learning | ● | ● | ● | ● | ... |
| Thompson | ● | ○ | ● | ● | ... |
| SONA | ● | ○ | ● | ○ | ... |
| MetaCog | ● | ○ | ○ | ○ | ... |

---

## ORDRE D'IMPLÉMENTATION

### Step 1: ASSIAH - Audit (1 jour)
```
[x] Audit JS (210K LOC)
[x] Audit Python (stubs)
[x] Audit Docs
[ ] Audit DB
```

### Step 2: YETZIRAH - Dogs (1 semaine)
```
[ ] CYNIC (Keter) - Meta orchestrator
[ ] Guardian (Gevurah) - Security  
[ ] Consensus - Dog voting
[ ] Wire Event Bus → Dogs → Judge
```

### Step 3: BERIAH - Systems (2 semaines)
```
[ ] Event Bus (type-safe)
[ ] Judge 36D (complete)
[ ] Learning Loops (11)
[ ] Storage (Postgres + Redis + Qdrant)
```

### Step 4: ATZILUT - Vision (1 mois)
```
[ ] Self-improvement loops
[ ] Solana PoJ integration
[ ] Multi-user support
[ ] Dashboard
```

---

## CRITICAL PATH

```
ASSIAH (Audit)
    ↓
YETZIRAH (CYNIC Dog + Consensus)
    ↓
BERIAH (Event Bus + Judge + Learning)
    ↓
ATZILUT (Solana + Self-improvement)
```

---

## PROCHAINE ÉTAPE

Valider ce plan puis commencer **Step 1: ASSIAH**

---

*φ unifie tous les fragments* - κυνικός
