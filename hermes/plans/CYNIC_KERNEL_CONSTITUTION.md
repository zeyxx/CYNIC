# CYNIC KERNEL — RESPONSABILITÉS CONSTITUTIONNELLES

> **Question fondamentale** : Qu'est-ce que le kernel ? Que doit-il porter ? Que ne doit-il PAS porter ?
> **Contexte** : Le kernel actuel = 220 fichiers, 17,103 LOC, 28 erreurs compilation, dette ~28%.
> **Objectif** : Définir la frontière constitutionnelle avant de refactorer.

---

## DÉFINITION CONSTITUTIONNELLE

### Le kernel EST :

| Responsabilité | Justification |
|----------------|---------------|
| **Runtime supervision** | Le kernel est le coordinateur d'exécution. Il orchestre les workloads d'inférence, gère les processus modèles, surveille la santé. C'est son cœur de métier. |
| **Storage** | Le kernel possède la persistance (SurrealDB + in-memory). Il définit le schéma, les tables, les queries. C'est le SSOT des données CYNIC. |
| **Host substrate** | Le kernel gère le hardware (probes, capacity detection). Il connaît l'OS, le GPU, la RAM, les processus. C'est la couche hôte. |
| **API surface** | Le kernel expose les endpoints REST + WebSocket. C'est l'interface opérateur de CYNIC. |
| **Agent session runtime** | Le kernel gère le cycle de vie des sessions agentiques (spawn, heartbeat, stagnation, handoff). C'est lui qui décide quand un agent tourne ou meurt. |

### Le kernel N'EST PAS :

| Hors périmètre | Justification |
|----------------|---------------|
| **Inference engine** | L'inférence GGUF/llama.cpp appartient à `cynic-inference`. Le kernel l'orchestre, il ne l'exécute pas. |
| **Agent coordination logic** | Les types de coordination (Task, Claim, Ledger) appartiennent à `cynic-agent-bus`. Le kernel les consomme, il ne les définit pas. |
| **Forensics SOC** | L'investigation forensique appartient à `cynic-forensics` / `cynic-intel`. Le kernel collecte les preuves, il n'enquête pas. |
| **Domain types (escore, futarchy, cpmm)** | Les types métier verticaux appartiennent à `cynic-intel-*`. Le kernel les héberge, il ne les définit pas. |
| **Solana-specific** | L'observation Solana, le Pay plugin appartiennent à `cynic-observer` / `cynic-zeroclaw-solana`. |

---

## PROPOSITION D'ARCHITECTURE CIBLE

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CYNIC RUNTIME                                │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    CYNIC-KERNEL (core)                         ││
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌───────────────┐  ││
│  │  │  bootstrap/     │  │  api/           │  │  storage/     │  ││
│  │  │  (composition   │  │  (REST + WS)    │  │  (SurrealDB   │  ││
│  │  │   root)         │  │                 │  │   + memory)   │  ││
│  │  └─────────────────┘  └─────────────────┘  └───────────────┘  ││
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌───────────────┐  ││
│  │  │  infra/         │  │  domain/        │  │  introspect/  │  ││
│  │  │  (probes,       │  │  (types purs,   │  │  (SOC engine, │  ││
│  │  │   config,       │  │   events,       │  │   forensics   │  ││
│  │  │   crypto,       │  │   storage port, │  │   evidence)   │  ││
│  │  │   health)       │  │   metrics)      │  │               │  ││
│  │  └─────────────────┘  └─────────────────┘  └───────────────┘  ││
│  │  ┌─────────────────┐  ┌─────────────────┐                     ││
│  │  │  probe/         │  │  senses/        │                     ││
│  │  │  (hardware      │  │  (location,     │                     ││
│  │  │   detection)    │  │   RTK)          │                     ││
│  │  └─────────────────┘  └─────────────────┘                     ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  Dépend de (DAG downward) :                                         │
│  ├── cynic-protocol (types purs, PoH, ed25519)                      │
│  ├── cynic-agent-bus (Task, Claim, Ledger, coordination)            │
│  ├── cynic-inference (runtime GGUF, model registry)                 │
│  ├── cynic-storage (client SurrealDB)                               │
│  └── cynic-host (probes hardware, OS substrate)                     │
│                                                                     │
│  Ne dépend PAS de :                                                 │
│  ├── cynic-forensics (le kernel collecte, forensics enquête)        │
│  ├── cynic-intel-* (types métier verticaux)                         │
│  ├── cynic-observer (Solana-specific)                               │
│  └── cynic-zeroclaw-solana (Solana Pay)                             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## FRONTIÈRES DU KERNEL

### Ce qui reste dans `cynic-kernel/` :

| Module | Raison |
|--------|--------|
| `bootstrap/` | Composition root — le point d'entrée de tout le système |
| `api/` | REST + WebSocket — interface opérateur |
| `storage/` | SSOT — le kernel possède les données |
| `domain/events.rs` | Types d'événements runtime (pas forensique) |
| `domain/metrics.ts` | Métriques runtime (CPU, RAM, GPU, processus) |
| `domain/storage/types.rs` | StoragePort trait (définit l'interface) |
| `domain/organ.rs` | OrganPort trait (contrat sensoriel) |
| `infra/config/` | Configuration loader |
| `infra/crypto.rs` | Crypto utilities (signing, verification) |
| `infra/probes/` | Hardware probes (CPU, GPU, RAM, disk, network) |
| `infra/tasks/` | Runtime loops (lifecycle, heartbeats, healing, perf) |
| `infra/task_health.rs` | Santé des tâches runtime |
| `infra/local_spool.rs` | Spool local pour les messages |
| `infra/runtime_secret.rs` | Secrets runtime |
| `probe/` | Hardware detection ( environnement, hardware, llm) |
| `senses/` | Location/RTK |
| `introspection/engine.rs` | SOC engine (evidence collection) |
| `introspection/soc.rs` | SOC thresholds |
| `introspection/probe.rs` | Forensic probe |

### Ce qui sort du kernel :

| Module | Destination | Raison |
|--------|-------------|--------|
| `domain/coord.rs` (ClaimEntry) | `cynic-agent-bus` | Type de coordination |
| `domain/storage/types.rs` (ProofOfHistory) | `cynic-agent-bus::Hash256` | Type de preuve |
| `storage/surreal/workspace_claims.rs` | `cynic-agent-bus` | Logique de claims |
| `storage/schema_bootstrap.sql` (work_claim) | `cynic-agent-bus` | Table dupliquée |
| `infra/alerts/slack.rs` | `cynic-host` | Alerting = concern hôte |
| `infra/remediation.rs` | `cynic-host` | Remédiation = concern hôte |
| `infra/runtime_takeover.rs` | `cynic-host` | Takeover = concern hôte |
| `infra/probes/fleet/` | `cynic-host` | Fleet management = concern hôte |
| `api/rest/data/` | `cynic-host` | Admin/monitoring = concern hôte |
| `api/websocket/` | `cynic-host` | WebSocket = concern hôte |
| `domain/chat.rs` | SUPPRIMÉ | Re-export cynic_inference, 0 refs |
| `domain/daily_budget.rs` | SUPPRIMÉ | Orphelin |
| `domain/epoche.rs` | SUPPRIMÉ | Orphelin |
| `domain/resource_guard.rs` | SUPPRIMÉ | Orphelin |
| `infra/health_loop.rs` | SUPPRIMÉ | 0 refs + cassé |
| `infra/node_enrollment_sqlite.rs` | SUPPRIMÉ | SQLite inutilisé |
| `infra/node_control_sqlite.rs` | SUPPRIMÉ | SQLite inutilisé |
| `infra/coordination_journal.rs` | SUPPRIMÉ | Orphelin |
| `infra/coordination_authority.rs` | SUPPRIMÉ | Orphelin |
| `infra/error_classifier.rs` | SUPPRIMÉ | Orphelin |
| `infra/pattern_model_seal.rs` | SUPPRIMÉ | Orphelin |
| `infra/evidence_projection_store.rs` | SUPPRIMÉ | Orphelin |
| `storage/memory.rs` | SUPPRIMÉ | In-memory, 0 refs |
| `build_scheduler.rs` | SUPPRIMÉ | 0 refs |
| `capability_issuer.rs` | SUPPRIMÉ | 0 refs |
| `bin/memory_benchmark.rs` | SUPPRIMÉ | Binaire inutilisé |

---

## RÈGLES CONSTITUTIONNELLES DU KERNEL

1. **Le kernel orchestre, il n'exécute pas** — l'inférence appartient à cynic-inference
2. **Le kernel stocke, il ne définit pas les types métier** — les types de coordination appartiennent au bus
3. **Le kernel expose, il ne forensique pas** — l'investigation appartient à cynic-forensics
4. **Le kernel ne connaît pas les verticals** — escore, futarchy, cpmm, Solana sont hors périmètre
5. **Le kernel ne réinvente pas le bus** — il consomme cynic-agent-bus, il ne le duplique pas
6. **Le kernel ne dépend pas de ses consommateurs** — DAG strict downward

---

## MÉTRIQUES CIBLES

| Métrique | Avant | Après | Delta |
|----------|-------|-------|-------|
| Fichiers | 220 | ~120 | -100 |
| LOC | 17,103 | ~10,000 | -7,100 |
| Erreurs compilation | 28 | 0 | -28 |
| Dead code LOC | 3,330 | 0 | -3,330 |
| Violations arch | 3 | 0 | -3 |
| Overlap bus | ~1,200 | 0 | -1,200 |
| Modules externes | 15+ | 5 | -10 |

---

*Définition constitutionnelle du kernel. À valider avant d'exécuter le refactor.*
