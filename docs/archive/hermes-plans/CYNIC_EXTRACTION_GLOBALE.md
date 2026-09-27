# CYNIC — PLAN D'EXTRACTION GLOBAL v1

> **Problème** : kernel, host, protocol, inference forment un graphe cyclique de dépendances.
> **Solution** : Extraire TOUT ce qui est partagé en micro-crates indépendantes, puis reconstruire.
> **Durée** : 5-7 sessions
> **Objectif** : Zéro cycle, frontières pures, chaque crate testable isolément.

---

## 1. DIAGNOSTIC DES CYCLES

| Cycle | Chemin | Types en conflit |
|-------|--------|------------------|
| **K ↔ H** | kernel → host (runtime_registry, resolve_runtime_registry_path) | BackendConfig, StorageConfig, DogThresholds |
| **K ↔ P** | kernel → protocol (types, schemas) | NodeControlPlan, AgentSessionRecord |
| **H ↔ P** | host → protocol (types) | BuildArtefact, HardwareSpec |
| **H ↔ I** | host → inference (config_loader, routing) | BackendConfig, RoutingCalculator |
| **K ↔ I** | kernel → inference (config_loader, organ) | ModelCatalog, InferenceRequest |

---

## 2. EXTRACTION PHASE 0 — Types atomiques (session suivante)

### 2.1. `cynic-types` — Types fondamentaux

| Type | Source | Destination |
|------|--------|-------------|
| `Hash32` | agent-bus, protocol | cynic-types |
| `AgentId` | agent-bus, kernel | cynic-types |
| `TaskId`, `ClaimId` | agent-bus, kernel | cynic-types |
| `Timestamp` | partout | cynic-types |
| `Scope`, `Budget` | agent-bus, kernel | cynic-types |

### 2.2. `cynic-protocol-core` — Protocole de communication

| Type | Source | Destination |
|------|--------|-------------|
| `Verdict`, `DogScore` | protocol | cynic-protocol-core |
| `Axiom`, `Crystal` | protocol | cynic-protocol-core |
| `Observation` | protocol, kernel | cynic-protocol-core |
| `InferenceRequest` | protocol, inference | cynic-protocol-core |

### 2.3. `cynic-identity` — Identité cryptographique

| Type | Source | Destination |
|------|--------|-------------|
| `NodeIdentity` | kernel, host | cynic-identity |
| `ed25519 keypair` | kernel, host | cynic-identity |
| `BLAKE3(node_id)` | kernel, host | cynic-identity |

### 2.4. `cynic-storage-ports` — Traits de stockage

| Type | Source | Destination |
|------|--------|-------------|
| `StoragePort` | kernel, host | cynic-storage-ports |
| `VerdictStorage` | kernel | cynic-storage-ports |
| `ActivityStorage` | kernel | cynic-storage-ports |
| `TaskStorage` | kernel | cynic-storage-ports |

### 2.5. `cynic-config` — Configuration partagée

| Type | Source | Destination |
|------|--------|-------------|
| `BackendConfig` | host, kernel, inference | cynic-config |
| `StorageConfig` | host, kernel | cynic-config |
| `DogThresholds` | host, kernel | cynic-config |
| `OrganRemediation` | host, kernel | cynic-config |

---

## 3. DAG CIBLE (final)

```
                    ┌─────────────────┐
                    │  cynic-types    │  (Hash32, AgentId, TaskId, Timestamp)
                    └────────┬────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
          ▼                  ▼                  ▼
   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
   │ cynic-      │   │ cynic-      │   │ cynic-      │
   │ identity    │   │ protocol-   │   │ config      │
   │             │   │ core        │   │             │
   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
          │                 │                  │
          │    ┌────────────┴────────────┐     │
          │    │                         │     │
          ▼    ▼                         ▼     ▼
   ┌─────────────┐               ┌─────────────┐
   │ cynic-      │               │ cynic-      │
   │ storage-    │               │ host-       │
   │ ports       │               │ domain      │
   └──────┬──────┘               └──────┬──────┘
          │                             │
          │         ┌───────────────────┤
          │         │                   │
          ▼         ▼                   ▼
   ┌─────────────┐               ┌─────────────┐
   │ cynic-      │               │ cynic-      │
   │ kernel-     │               │ host-       │
   │ domain      │               │ adapters    │
   └──────┬──────┘               └──────┬──────┘
          │                             │
          ▼                             ▼
   ┌─────────────┐               ┌─────────────┐
   │ cynic-      │               │ cynic-      │
   │ kernel-     │◄──────────────│ host-       │
   │ bootstrap   │  (runtime)    │ bootstrap   │
   └─────────────┘               └─────────────┘
```

---

## 4. PLAN D'EXTACTION

### Session 1 : Types atomiques

| Étape | Action | Crate |
|-------|--------|-------|
| 1.1 | Extraire `cynic-types` | cynic-types |
| 1.2 | Extraire `cynic-identity` | cynic-identity |
| 1.3 | Valider DAG (pas de cycle) | workspace |

### Session 2 : Protocole + Config

| Étape | Action | Crate |
|-------|--------|-------|
| 2.1 | Extraire `cynic-protocol-core` | cynic-protocol-core |
| 2.2 | Extraire `cynic-config` | cynic-config |
| 2.3 | Valider DAG | workspace |

### Session 3 : Storage ports + Kernel domain

| Étape | Action | Crate |
|-------|--------|-------|
| 3.1 | Extraire `cynic-storage-ports` | cynic-storage-ports |
| 3.2 | Extraire `cynic-kernel-domain` | cynic-kernel-domain |
| 3.3 | Valider DAG | workspace |

### Session 4 : Host domain + Adapters

| Étape | Action | Crate |
|-------|--------|-------|
| 4.1 | Extraire `cynic-host-domain` | cynic-host-domain |
| 4.2 | Extraire `cynic-host-ports` | cynic-host-ports |
| 4.3 | Extraire `cynic-host-adapter-*` (12 crates) | cynic-host-adapter-* |
| 4.4 | Valider DAG | workspace |

### Session 5 : Inference + Node + CLI

| Étape | Action | Crate |
|-------|--------|-------|
| 5.1 | Extraire `cynic-inference-domain` | cynic-inference-domain |
| 5.2 | Extraire `cynic-inference-ports` | cynic-inference-ports |
| 5.3 | Extraire `cynic-inference-adapter-*` | cynic-inference-adapter-* |
| 5.4 | Extraire `cynic-node-domain` + `cynic-node-ports` | cynic-node-* |
| 5.5 | Extraire `cynic-cli` (adapter sortant) | cynic-cli |
| 5.6 | Valider DAG | workspace |

### Session 6 : Bootstrap + Application

| Étape | Action | Crate |
|-------|--------|-------|
| 6.1 | Extraire `cynic-kernel-bootstrap` | cynic-kernel-bootstrap |
| 6.2 | Extraire `cynic-host-bootstrap` | cynic-host-bootstrap |
| 6.3 | Extraire `cynic-host-application` | cynic-host-application |
| 6.4 | Extraire `cynic-inference-application` | cynic-inference-application |
| 6.5 | Valider DAG final | workspace |

---

## 5. RÈGLES D'EXTRACTION

1. **Un crate à la fois** — jamais 2 extractions en parallèle.
2. **Domain pur d'abord** — types + ports, puis adapters, puis application.
3. **Tests boundary obligatoires** — chaque nouveau crate a `tests/architecture_boundary.rs`.
4. **Validation DAG** — `cargo check --workspace` après chaque extraction.
5. **Zéro régression** — les consommateurs existants doivent compiler sans changement.
6. **Ratchet strict** — un crate extrait ne retourne jamais dans le monolithe.
7. **Feature flags** — gate les modules lourds (SurrealDB, inference, crypto).

---

## 6. MÉTRIQUES CIBLES

| Monolithe | LOC actuel | LOC cible | # Crates |
|-----------|------------|-----------|----------|
| cynic-host | 89,754 | ~12K | 12 |
| cynic-node | 58,092 | ~10K | 10 |
| cynic-protocol | 54,666 | ~5K | 4 |
| cynic-kernel | 9,406 | ~5K | 5 |
| cynic-inference | 26,846 | ~12K | 7 |
| cynic-cli | 37,046 | ~3K | 1 |
| **TOTAL** | **275,710** | **~47K** | **~39** |

---

## 7. ANTI-PATTERNS À ÉVITER

| Anti-pattern | Pourquoi | Solution |
|--------------|----------|----------|
| **Extraction partielle** | Crée des cycles temporaires | Extraire TOUT le domain d'un coup |
| **Ré-exports globaux** | Masque les dépendances réelles | Imports explicites |
| **Tests d'intégration dans domain** | Pollue la pureté | Tests dans `tests/` séparés |
| **Features mal utilisées** | Compilation conditionnelle cassée | Features = compilation units, pas runtime flags |
| **Version mismatch** | 5 versions de `serde` | `[workspace.dependencies]` |

---

*Plan v1. Extraction globale en 6 sessions. Validation DAG continue.*
