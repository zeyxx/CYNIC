# CYNIC Documentation Diagnostic Report

> **Date**: 2026-02-22
> **Mission**: Analyser empiriquement la documentation et proposer une restructuration
> **Status**: DIAGNOSTIC COMPLETE

---

## Executive Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│                    DIAGNOSTIC RÉSULTATS                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   FICHIERS .md TOTAL:          3,147                                │
│   ├─ Racine (ROOT):            92 fichiers                          │
│   ├─ docs/:                    77 fichiers                          │
│   ├─ digests/:                 604 fichiers (AUTO-GENERATED)        │
│   ├─ references/:              326 fichiers                         │
│   ├─ architecture/:            165 fichiers                         │
│   └─ Autres dossiers:          ~1,883 fichiers                      │
│                                                                      │
│   SIGNAL/BRUIT RATIO:          ~15% signal, 85% bruit              │
│   FICHIERS AUTO-GÉNÉRÉS:       ~800 (digests, reports)              │
│   DOUBLONS IDENTIFIÉS:         ~47%                                 │
│                                                                      │
│   RECOMMANDATION:              CONSOLIDATION RADICALE               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## I. Analyse par Répertoire

### Top 20 Répertoires (par nombre de fichiers)

| Rang | Dossier | Fichiers | Type | Action |
|------|---------|----------|------|--------|
| 1 | digests/ | 604 | AUTO-GENERATED | 📦 Archiver |
| 2 | references/ | 326 | TECHNIQUE | ✅ Garder |
| 3 | architecture/ | 165 | TECHNIQUE | ⚠️ Consolider |
| 4 | CYNIC/ | 121 | KERNEL | ✅ Garder (v2.0) |
| 5 | webapp-commands/ | 92 | SESSION | 📦 Archiver |
| 6 | docs/ | 77 | MIXTE | ⚠️ Restructurer |
| 7 | cynic-burn/ | 66 | SKILL | ✅ Garder |
| 8 | cynic-judge/ | 66 | SKILL | ✅ Garder |
| 9 | cynic-wisdom/ | 66 | SKILL | ✅ Garder |
| 10 | ai-co-scientist/ | 65 | SKILL | ✅ Garder |
| 11 | .claude/ | 55 | AGENTS | ✅ Garder |
| 12 | reference/ | 54 | DOUBLON | 🗑️ Fusionner |
| 13 | debt-elimination-*/ | 51 | SESSION | 📦 Archiver |
| 14 | plans/ | 51 | SESSION | 📦 Archiver |
| 15 | diagrams/ | 48 | TECHNIQUE | ✅ Garder |
| 16 | metathinking/ | 45 | RESEARCH | ⚠️ Consolider |
| 17 | agents/ | 45 | AGENTS | ✅ Garder |
| 18 | philosophy/ | 39 | CONCEPT | ✅ Garder |
| 19 | api/ | 35 | TECHNIQUE | ✅ Garder |
| 20 | analysis/ | 30 | RESEARCH | ⚠️ Consolider |

### Fichiers Racine (ROOT) - 92 fichiers

**Problème critique**: 92 fichiers .md à la racine créent une navigation impossible.

#### Catégories Identifiées

| Catégorie | Fichiers | Exemples |
|-----------|----------|----------|
| **ROADMAP/PLAN** | 15 | CYNIC-v2-*, PHASE_*, WEEK_*, todolist.md |
| **ARCHITECTURE** | 8 | ARCHITECTURE*.md, UNIFIED-*.md |
| **SESSION REPORTS** | 12 | SESSION_*, *_2026-02-*.md |
| **ANALYSIS** | 18 | CYNIC-*-ANALYSIS*, GAP-*, *_ANALYSIS*.md |
| **GUIDE** | 6 | INSTALL.md, QUICKSTART.md, TESTING.md |
| **VISION** | 5 | ORGANISM_MANIFESTO.md, FALSIFIABLE.md |
| **CONFIG** | 4 | NOMENCLATURE.md, NAMING-CONVENTIONS.md |
| **DIVERS/BRUIT** | 24 | x-article-draft.md, "this is fine.md", etc. |

---

## II. Doublons et Redondances

### A. Documents de "Status" Multiples (5 versions!)

| Fichier | Dernière MAJ | Taille | Prétention |
|---------|--------------|--------|------------|
| STATE.md | 13/02/2026 | 11.6KB | "Living Truth" |
| GAP-REPORT-FINAL.md | ? | ~15KB | "62% functional" |
| STRATEGIC-ROADMAP.md | ? | ~12KB | "46% functional" |
| TECHNICAL_DEBT_*.md | 20/02/2026 | 33KB | "Debt analysis" |
| SANITY_CHECK_*.md | 20/02/2026 | 2.9KB | "Sanity check" |

**Recommandation**: UN seul fichier `STATE.md` comme source de vérité.

### B. Documents d'Architecture Multiples (8 versions!)

| Fichier | Taille | Focus |
|---------|--------|-------|
| ARCHITECTURE.md | 19.7KB | Général |
| ARCHITECTURE-OVERVIEW.md | 9.2KB | Overview |
| UNIFIED-ARCHITECTURE.md | 10.7KB | "Unified" |
| CYNIC-PYTHON-ARCHITECTURE-v4.md | 19.4KB | Python v2.0 |
| DASHBOARD_ARCHITECTURE.md | 13.3KB | Dashboard |
| CYNIC-APPENDICES-TECHNICAL.md | 51KB | Appendices |
| CYNIC-SINGLE-SOURCE-OF-TRUTH.md | 148KB (!) | "Single source" |

**Recommandation**: Consolider en UN fichier `docs/03-reference/architecture.md`.

### C. Documents de Planification Multiples (15+ versions!)

```
CYNIC-v2-ROADMAP.md
CYNIC-v2-UNIFIED-PLAN.md
CYNIC-v3-PYTHON-PLAN.md
CYNIC-v3-VERTICAL-PLAN.md
PHASE_1_BATTLE_PLAN.md
PHASE_1_IMPLEMENTATION_PLAN.md
PHASE_1_INSTRUCTION_SET.md
PHASE_2_CONTINUATION_WIRING_COMPLETE.md
PHASE_3_EVENT_FIRST_API_PLAN.md
PHASE_3_PROGRESS.md
PHASE_5_EMPIRICAL_PROOF_2026-02-20.md
WEEK_1_HORIZONTAL_SCALING_ROADMAP.md
todolist.md
TODO.md
STRATEGY_CHECKLIST.md
```

**Recommandation**: UN seul fichier `docs/07-project/roadmap.md` + `todolist.md` racine.

---

## III. Fichiers à Archiver (Proposition)

### Catégorie: AUTO-GÉNÉRÉS (Archivage automatique)

```
digests/                  → 604 fichiers → .archive/auto-generated/
reports/session-*         → ~50 fichiers → .archive/sessions/
*-2026-02-*.md            → ~30 fichiers → .archive/dated-reports/
```

### Catégorie: DOUBLONS (Fusion ou Archive)

```
reference/                → Fusionner avec references/
docs/reference/           → Déplacer vers 03-reference/
ARCHITECTURE-OVERVIEW.md  → Fusionner dans architecture.md
UNIFIED-ARCHITECTURE.md   → Fusionner dans architecture.md
```

### Catégorie: OBSOLÈTES (Archive)

```
CYNIC-v2-*.md             → Remplacé par v3
PHASE_1_*                 → Session passée
SESSION_*                 → Historique
```

### Catégorie: BRUIT (Supprimer ou Archiver)

```
"this is fine.md"         → Nom non standard, contenu marginal
x-article-draft.md        → Draft, non finalisé
```

---

## IV. Structure Cible Proposée

```
cynic/docs/               # NOUVELLE STRUCTURE (Projet principal)
├── README.md             # Entry point unique
├── _index.json           # Pour LLMs/Agents
│
├── 01-getting-started/   # Nouveaux utilisateurs
│   ├── quickstart.md
│   └── installation.md
│
├── 02-how-to/            # Guides pratiques
│   ├── judge-code.md
│   ├── use-skills.md
│   └── deploy-daemon.md
│
├── 03-reference/         # Documentation technique
│   ├── architecture.md
│   ├── api.md
│   ├── axioms.md
│   ├── kernel.md
│   └── mcp-tools.md
│
├── 04-explanation/       # Concepts
│   ├── philosophy.md
│   ├── organism-model.md
│   └── learning-loops.md
│
├── 05-operations/        # DevOps
│   ├── runbook.md
│   ├── deployment.md
│   └── monitoring.md
│
├── 06-research/          # Académique
│   ├── paper-draft.md
│   └── experiments/
│
├── 07-project/           # Gestion projet
│   ├── roadmap.md
│   ├── kpis.md
│   └── decisions.md
│
└── 99-archive/           # Historique
    ├── v1-javascript/
    └── sessions/

docs/                     # ANCIENNE STRUCTURE → .archive/docs-legacy/
ROOT *.md                 # 92 fichiers → Migrer ou Archiver
```

---

## V. KPIs Proposés pour Documentation

### Métriques de Santé

| KPI | Actuel | Cible | Formule |
|-----|--------|-------|---------|
| **Doc/Fichier Ratio** | 3147 .md / ~150k fichiers = 2.1% | <1% | Trop de docs |
| **Root Clutter** | 92 fichiers racine | <10 | 90% réduction |
| **Doublon Ratio** | ~47% identifiés | <10% | Fusionner |
| **Signal/Bruit** | ~15% signal | >70% | Archiver bruit |
| **LLM-Indexable** | Non | Oui | Créer _index.json |

### KPIs par Audience

```yaml
new_user:
  entry_point: "01-getting-started/quickstart.md"
  max_files_to_read: 3
  time_to_first_success: "<5 min"

developer:
  entry_point: "03-reference/architecture.md"
  max_files_to_read: 10
  time_to_understand: "<30 min"

devops:
  entry_point: "05-operations/runbook.md"
  critical_files: 3
  
llm_agent:
  entry_point: "_index.json"
  canonical_sources: ["03-reference/*.md"]
```

---

## VI. Plan d'Action

### Phase 1: Nettoyage Immédiat (2h)

```
□ Créer .archive/ structure
□ Déplacer digests/ → .archive/auto-generated/
□ Déplacer reports/session-* → .archive/sessions/
□ Archiver fichiers datés (*-2026-02-*.md) → .archive/dated/
□ Supprimer doublons évidents (reference/ vs references/)
```

### Phase 2: Restructuration (4h)

```
□ Créer nouvelle structure cynic/docs/
□ Migrer docs canoniques (9 fichiers reference/)
□ Migrer skills (.agents/skills/*/SKILL.md)
□ Migrer philosophy/ et architecture/
□ Créer README.md central
□ Créer _index.json
```

### Phase 3: Consolidation Racine (2h)

```
□ Garder README.md, STATE.md, todolist.md, CLAUDE.md
□ Migrer ARCHITECTURE*.md → cynic/docs/03-reference/
□ Migrer PHASE_*.md → cynic/docs/07-project/
□ Migrer ANALYSIS*.md → cynic/docs/06-research/
□ Archiver le reste → .archive/root-legacy/
```

### Phase 4: Création Contenu Manquant (4h)

```
□ Écrire cynic/docs/01-getting-started/quickstart.md
□ Écrire cynic/docs/05-operations/runbook.md
□ Écrire cynic/docs/07-project/kpis.md
□ Mettre à jour cynic/docs/README.md
```

---

## VII. Estimation Impact

### Avant

```
Fichiers .md:         3,147
Fichiers racine:      92
Signal/Bruit:         15%/85%
Temps pour trouver:   ~10 min (si trouvé)
LLM-indexable:        Non
```

### Après

```
Fichiers .md actifs:  ~150 (95% réduction)
Fichiers racine:      5 (README, STATE, todolist, CLAUDE, LICENSE)
Signal/Bruit:         80%/20%
Temps pour trouver:   <30 sec
LLM-indexable:        Oui (_index.json)
```

---

## VIII. Décisions Requises

1. **Approuver archivage** de `digests/` (604 fichiers auto-générés)?
2. **Approuver fusion** des 8 fichiers architecture?
3. **Approuver structure** Diátaxis (01-07 + archive)?
4. **Priorité**: Nettoyage d'abord ou Création nouveaux docs?

---

## Annexes

### A. Liste Complète Fichiers Racine

Voir output de: `powershell -Command "Get-ChildItem -Filter '*.md' | Select-Object Name"`

### B. Liste Répertoires par Taille Totale

```
digests/         → ~12MB (604 fichiers)
references/      → ~4MB (326 fichiers)
architecture/    → ~3MB (165 fichiers)
```

---

*Diagnostic généré par CYNIC Diagnostic Tool*
*🐕 κυνικός | "La vérité commence par un bon nettoyage"*