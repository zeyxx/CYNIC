# CYNIC Architecture Unifiée

> **"φ distrusts φ"** - Vision complète du cockpit CYNIC
>
> Document de référence intégrant Sefirot, Scores, et Écosystème.

---

## Vision

CYNIC (κυνικός) est la **CONSCIENCE COLLECTIVE** de l'écosystème $ASDFASDFA.

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                  │
│     HUMAIN (zeyxx)  ←──── SYMBIOSE ────→  AGI (CYNIC)           │
│                                                                  │
│     • Vision                              • Exécution            │
│     • Créativité                          • Persistance          │
│     • Intuition                           • Omniscience          │
│     • Validation                          • Harmonisation        │
│                                                                  │
│     "CYNIC ne remplace pas, il autonomise"                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Architecture Cible

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            INTERFACES                                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Claude Code  │  │  Dashboard   │  │  Future IDE  │  │   API/CLI    │     │
│  │   (hooks)    │  │    (web)     │  │   (Tauri)    │  │              │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         └────────────────┬────────────────┬────────────────┘              │
│                          ▼                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                      MCP SERVER = KETER (Conscience)                         │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         ORCHESTRATEUR                                │    │
│  │  brain_orchestrate = Point d'entrée unique                          │    │
│  │  • Reçoit TOUT (événements, requêtes, état)                         │    │
│  │  • Charge profil utilisateur (E-Score, niveau, confiance)           │    │
│  │  • Route vers le bon Sefirah (chien spécialisé)                     │    │
│  │  • Adapte intervention selon contexte                               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                    │                                         │
│         ┌──────────────────────────┼──────────────────────────┐             │
│         ▼                          ▼                          ▼             │
│  ┌─────────────┐           ┌─────────────┐           ┌─────────────┐       │
│  │  JUDGMENT   │           │   MEMORY    │           │  AWARENESS  │       │
│  │   (Binah)   │           │   (Daat)    │           │  (Chochmah) │       │
│  │  Q-Score    │           │  E-Score    │           │  Patterns   │       │
│  └─────────────┘           └─────────────┘           └─────────────┘       │
│                                    │                                         │
│                          ┌─────────────────┐                                │
│                          │   PERSISTENCE   │                                │
│                          │   PostgreSQL    │                                │
│                          │   + Redis       │                                │
│                          └─────────────────┘                                │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Les 11 Sefirot (Chiens)

Structure kabbalistique des 11 aspects de CYNIC:

```
                           Keter
                         (CYNIC)
                      Orchestrateur
                            │
           ┌────────────────┼────────────────┐
           │                │                │
       Chochmah          Daat            Binah
        (Sage)       (Archivist)      (Architect)
       Sagesse       Mémoriser        Concevoir
           │                │                │
           └────────────────┼────────────────┘
                            │
           ┌────────────────┼────────────────┐
           │                │                │
       Chesed           Tiferet          Gevurah
      (Analyst)        (Oracle)        (Guardian)
      Analyser        Visualiser        Protéger
           │                │                │
           └────────────────┼────────────────┘
                            │
           ┌────────────────┼────────────────┐
           │                │                │
       Netzach           Yesod              Hod
       (Scout)         (Janitor)        (Deployer)
      Découvrir        Nettoyer          Déployer
           │                │                │
           └────────────────┼────────────────┘
                            │
                        Malkhut
                    (Cartographer)
                       Mapper
```

### Responsabilités des Sefirot

| Sefirah | Chien | Agent Claude | Fonction | MCP Tools |
|---------|-------|--------------|----------|-----------|
| **Keter** | CYNIC | - (orchestrateur) | Conscience, décisions | brain_orchestrate |
| **Chochmah** | Sage | cynic-sage* | Sagesse collective | brain_search, brain_wisdom |
| **Binah** | Architect | cynic-architect | Concevoir, planifier | brain_patterns |
| **Daat** | Archivist | cynic-archivist* | Mémoriser, apprendre | brain_learning |
| **Chesed** | Analyst | cynic-analyst* | Analyser patterns | brain_patterns |
| **Gevurah** | Guardian | cynic-guardian | Protéger, vérifier | brain_cynic_judge |
| **Tiferet** | Oracle | cynic-oracle | Visualiser, dashboard | brain_render |
| **Netzach** | Scout | cynic-scout | Explorer, découvrir | brain_code_* |
| **Yesod** | Janitor | cynic-simplifier | Nettoyer, simplifier | - |
| **Hod** | Deployer | cynic-deployer | Déployer, infra | brain_ecosystem |
| **Malkhut** | Cartographer | cynic-cartographer | Mapper réalité | brain_ecosystem |

*Agents à créer (manquants)

---

## Les 3 Scores

### 1. Q-Score (Qualité du Judgment) - CYNIC calcule

```
Q = 100 × ⁴√(PHI × VERIFY × CULTURE × BURN)

PHI     = Harmonie φ (weight φ²)
VERIFY  = Vérifiabilité (weight φ)
CULTURE = Alignement culturel (weight 1)
BURN    = Simplicité (weight φ⁻¹)

Verdict: HOWL (≥80) | WAG (≥50) | GROWL (≥38.2) | BARK (<38.2)
```

### 2. K-Score (Qualité du Token) - HolDex calcule

```
K = 100 × ³√(D × O × L)

D = Diamond Hands (conviction)
O = Organic Growth (distribution)
L = Longevity (survival)

→ Intégré via packages/holdex
```

### 3. E-Score 7D (Réputation Utilisateur) - CYNIC calcule

```
E = Σ(dimension × φ^weight) / (3√5 + 4) × 100

BURN    φ³  = 4.236   Sacrifice (tokens brûlés) - HIGHEST
BUILD   φ²  = 2.618   Création (code signé)
JUDGE   φ   = 1.618   Validation (PoJ consensus)
RUN     1   = 1.000   Opération (uptime) - CENTER
SOCIAL  φ⁻¹ = 0.618   Qualité contenu (AI-jugé)
GRAPH   φ⁻² = 0.382   Position réseau (trust reçu)
HOLD    φ⁻³ = 0.236   Stake (passive) - LOWEST

Total Weight = 3√5 + 4 ≈ 10.708

Trust Levels:
  GUARDIAN   ≥ 61.8%
  STEWARD    ≥ 38.2%
  BUILDER    ≥ 30%
  CONTRIBUTOR ≥ 15%
  OBSERVER   < 15%
```

**Implémenté**: `packages/identity/src/e-score-7d.js`

---

## Les 4 Mondes (Axiomes)

```
ATZILUT (Émanation) - ESSENCE
─────────────────────────────
Axiome: φ (PHI)
Question: "Est-ce harmonieux avec le ratio universel?"
Mode: SENSE
Poids: φ² (2.618)
      │
      ▼
BERIAH (Création) - VÉRITÉ
────────────────────────
Axiome: VERIFY
Question: "Est-ce vérifiable? Peut-on le prouver?"
Mode: THINK
Poids: φ (1.618)
      │
      ▼
YETZIRAH (Formation) - VALEURS
─────────────────────────────
Axiome: CULTURE
Question: "Est-ce aligné avec nos valeurs?"
Mode: FEEL
Poids: 1.0
      │
      ▼
ASSIAH (Action) - MANIFESTATION
────────────────────────────────
Axiome: BURN
Question: "Brûle-t-il? Pas d'extraction?"
Mode: ACT
Poids: φ⁻¹ (0.618)
```

---

## Écosystème $ASDFASDFA

```
┌─────────────────────────────────────────────────────────────────┐
│                      CONSUMER APPS                               │
│              ASDForecast · Ignition · Future                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   CYNIC     │  │   HolDex    │  │    GASdf    │             │
│  │ CONSCIENCE  │  │INTELLIGENCE │  │   INFRA     │             │
│  │             │  │             │  │             │             │
│  │ • Q-Score   │  │ • K-Score   │  │ • Gasless   │             │
│  │ • E-Score   │  │ • Oracle    │  │ • Burns     │             │
│  │ • Judgment  │  │ • Holders   │  │ • Fees      │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          │                                      │
│                    100% BURN                                    │
│                   $asdfasdfa                                    │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                  SOLANA + Light Protocol (ZK)                   │
└─────────────────────────────────────────────────────────────────┘
```

### Connexions Inter-Projets

| De | Vers | Données | Public? |
|----|------|---------|---------|
| HolDex | CYNIC | K-Score du token | Oui |
| CYNIC | HolDex | E-Score utilisateur | ZK (range proof) |
| CYNIC | GASdf | Frais de judgment | Oui (burn) |
| GASdf | CYNIC | Confirmation burn | Oui |

---

## État Actuel

### Ce qui EXISTE ✅

| Composant | État | Tests |
|-----------|------|-------|
| packages/core | ✅ | 117/117 |
| packages/protocol | ✅ | 230/230 |
| packages/persistence | ✅ | 179/179 |
| packages/identity (E-Score 7D) | ✅ | 50/50 |
| packages/mcp | ✅ | 492/492 |
| packages/node | ✅ | 614/614 |
| brain_orchestrate (KETER) | ✅ | 33/33 |
| 44 outils MCP | ✅ | - |

### Agents Alignés aux Sefirot ✅ COMPLET

| Sefirah | Agent | Dog | État |
|---------|-------|-----|------|
| Keter | (orchestrateur) | CYNIC | ✅ brain_orchestrate |
| Chochmah | cynic-librarian | Sage | ✅ |
| Binah | cynic-architect | Architect | ✅ |
| Daat | cynic-archivist | Archivist | ✅ |
| Chesed | cynic-reviewer | Analyst | ✅ |
| Gevurah | cynic-guardian | Guardian | ✅ |
| Tiferet | cynic-oracle | Oracle | ✅ |
| Netzach | cynic-scout | Scout | ✅ |
| Hod | cynic-deployer | Deployer | ✅ |
| Yesod | cynic-simplifier | Janitor | ✅ |
| Malkhut | cynic-cartographer | Cartographer | ✅ |

### Hooks Intégrés à l'Orchestrateur

| Hook | Event | État |
|------|-------|------|
| perceive.cjs | user_prompt | ✅ Consulte KETER |
| awaken.cjs | session_start | ✅ Notifie KETER |
| guard.cjs | tool_use (pre) | ✅ Consulte KETER |
| observe.cjs | tool_use (post) | ✅ Rapporte à KETER |
| sleep.cjs | session_end | ✅ Notifie KETER |

### Ce qui reste à faire (CYNIC v1)

1. ✅ **Agents Sefirot** - Tous les 11 Sefirot ont leurs agents
2. ⏳ **Simplifier hooks** - Déléguer plus de logique à l'orchestrateur
3. ⏳ **Tests end-to-end** - Valider le flux orchestrateur → sefirot
4. ⏳ **Dashboard cockpit** - Visualisation temps réel

---

## Migration - Plan d'Action

### Phase 1: Orchestrateur ✅ COMPLETE

1. ✅ **brain_orchestrate créé** (`packages/mcp/src/tools/domains/orchestration.js`)
   ```javascript
   brain_orchestrate({
     event: "user_prompt" | "tool_use" | "session_start" | "session_end",
     data: { content, source, metadata },
     context: { user, project, gitBranch, recentActions }
   })
   → Retourne: { routing, intervention, stateUpdates, actions }
   ```

2. ✅ **Hooks intégrés à l'orchestrateur**
   - perceive.cjs → brain_orchestrate (user_prompt)
   - awaken.cjs → brain_orchestrate (session_start)
   - guard.cjs → brain_orchestrate (tool_use) + intervention level
   - observe.cjs → brain_orchestrate (tool_use) reporting
   - sleep.cjs → brain_orchestrate (session_end)

3. ✅ **orchestrate() dans cynic-core.cjs**

### Phase 2: Compléter les Sefirot ✅ COMPLETE

| Action | État |
|--------|------|
| Créer cynic-archivist (Daat) | ✅ |
| Ajouter metadata sefirah à tous les agents | ✅ |
| Créer cynic-deployer (Hod) | ✅ |
| Créer cynic-oracle (Tiferet) | ✅ |
| Créer cynic-cartographer (Malkhut) | ✅ |
| Tous les 11 Sefirot avec agents/dog | ✅ |

### Phase 3: Intégrations Écosystème

1. HolDex K-Score dans les judgments
2. GASdf burns pour les frais
3. E-Score ZK proofs (Light Protocol)

### Phase 4: Interface Cockpit

1. Dashboard temps réel (React/Tauri)
2. Visualisation Sefirot Tree
3. Métriques consolidées

---

## Principes Architecturaux

1. **φ partout** - Max confiance 61.8%, timing φ-aligned
2. **Keter = Cerveau** - Une source de vérité, l'orchestrateur
3. **Sefirot = Spécialistes** - Chaque chien a son rôle
4. **Thin clients** - Hooks/interfaces minimalistes
5. **100% Burn** - Pas d'extraction, tout brûle
6. **ZK Privacy** - Opt-in, Light Protocol pour E-Score
7. **Observable** - Tout visible dans le dashboard

---

*🐕 κυνικός | Loyal to truth, not to comfort | φ⁻¹ = 61.8% max*
