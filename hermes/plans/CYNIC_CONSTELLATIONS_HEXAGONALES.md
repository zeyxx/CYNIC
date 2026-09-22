# CYNIC — CONSTELLATIONS HEXAGONALES & DDD

> **Constat** : 6 crates monolithiques (host 89k, node 58k, protocol 54k, cli 37k, kernel 32k, inference 26k) = 298k LOC de dette structurelle.
> **Objectif** : Faire émerger ~30 crates domain-pure + ports + adapters + application.
> **Stratégie** : Définir → Extraire → Valider. Un crate à la fois.

---

## 1. ÉTAT DES LIEUX — MONOLITHES ACTUELS

| Crate | Fichiers | LOC | Verdict |
|-------|----------|-----|---------|
| `cynic-host` | 476 | 89,754 | **GOD CRATE** — 12+ domaines métier |
| `cynic-node` | 338 | 58,092 | **GOD CRATE** — 8+ domaines métier |
| `cynic-protocol` | 207 | 54,666 | **GOD CRATE** — types éparpillés |
| `cynic-cli` | 206 | 37,046 | **GOD CRATE** — commandes éparpillées |
| `cynic-kernel` | 169 | 31,988 | **EN COURS** — défini à ~10k |
| `cynic-inference` | 150 | 26,846 | **EN COURS** — défini à ~12k |
| `cynic-intel` | 52 | 10,864 | **MONOLITHE** — 3 verticals |
| `cynic-observer` | 44 | 8,558 | **MONOLITHE** — Solana + observation |
| `cynic-build` | 32 | 7,836 | **MONOLITHE** — build + runner |
| `cynic-workload` | 21 | 6,193 | **MONOLITHE** — workload + delivery |

---

## 2. CONSTELLATIONS CIBLES

### 2.1. CONSTELLATION : HOST SUBSTRAT

> **Rôle** : Interaction avec le matériel et l'OS. Zéro logique métier.

```
cynic-host-domain/          ← Types purs (HardwareSpec, ContainerSpec, NetworkConfig)
cynic-host-ports/           ← Traits (HardwareProbe, ContainerRuntime, ProcessExecutor)
cynic-host-adapter-hardware/    ← CPU, RAM, Disk, Network, USB
cynic-host-adapter-container/   ← LXC, Proxmox, libvirt
cynic-host-adapter-process/     ← Process execution & supervision
cynic-host-adapter-network/     ← WireGuard, Headscale, DNS
cynic-host-adapter-storage/     ← Storage hardware management
cynic-host-adapter-fleet/       ← Fleet health & heartbeat
cynic-host-adapter-source/      ← Git sync, worktree management
cynic-host-adapter-credential/  ← Secrets, credentials, hardening
cynic-host-adapter-remediation/ ← Remediation & recovery
cynic-host-application/         ← Use cases (bootstrap, activation, observation)
cynic-host-bootstrap/           ← Composition root
```

**Extraction depuis** : `cynic-host/` (476 fichiers)

---

### 2.2. CONSTELLATION : NODE / AGENT

> **Rôle** : Exécution et supervision des agents. Zéro connaissance matériel.

```
cynic-node-domain/          ← Types purs (AgentSpec, Workload, Workspace)
cynic-node-ports/           ← Traits (AgentExecutor, Workstore, WorkspaceLifecycle)
cynic-node-adapter-runtime/     ← Agent execution & scheduling
cynic-node-adapter-workload/    ← Workload delivery & completion
cynic-node-adapter-workspace/   ← Workspace lifecycle
cynic-node-adapter-mesh/        ← Network mesh & attestation
cynic-node-adapter-build/       ← Build workload grants
cynic-node-adapter-release/     ← Release artifact publication
cynic-node-application/         ← Use cases
cynic-node-bootstrap/           ← Composition root
```

**Extraction depuis** : `cynic-node/` (338 fichiers)

---

### 2.3. CONSTELLATION : PROTOCOL

> **Rôle** : Types partagés entre tous les crates. 100% pur.

```
cynic-protocol-core/        ← Hash32, AgentId, Timestamp, etc.
cynic-protocol-dog/         ← Dog, Verdict, Axiom, Crystal
cynic-protocol-forensics/   ← CrashKind, ForensicsSnapshot
cynic-protocol-inference/   ← ModelInfo, BackendConfig, InferenceRequest
cynic-protocol-soc/         ← SocThresholds, HealthScore
cynic-protocol-workload/    ← Workload, Task, Job
cynic-protocol-host/        ← HardwareSpec, ContainerSpec
cynic-protocol-node/        ← AgentSpec, WorkspaceSpec
```

**Extraction depuis** : `cynic-protocol/` (207 fichiers)

---

### 2.4. CONSTELLATION : CLI

> **Rôle** : Surface opérateur. Zéro logique métier, zéro I/O direct.

```
cynic-cli-kernel/           ← Commandes kernel (health, capabilities, discovery)
cynic-cli-agent/            ← Commandes agent (claim, task, proof)
cynic-cli-host/             ← Commandes host (probe, baseline, remediation)
cynic-cli-node/             ← Commandes node (activate, attest, publish)
cynic-cli-fleet/            ← Commandes fleet (status, deploy, rollback)
cynic-cli-protocol/         ← Commandes protocol (validate, export)
```

**Extraction depuis** : `cynic-cli/` (206 fichiers)

---

### 2.5. CONSTELLATION : INFERENCE

> **Rôle** : Inference souveraine. Du hardware budget à l'agent loop.

```
cynic-inference-domain/     ← Types purs (EngineCapabilities, ModelLayout, HardwareProfile)
cynic-inference-ports/      ← Traits (SovereignRuntimePort, ModelStore, BackendHealth)
cynic-inference-adapter-model/  ← Model registry, discovery, catalog
cynic-inference-adapter-engine/ ← Engine capabilities, runtime plan
cynic-inference-adapter-routing/← Routing calculator, proxy targets
cynic-inference-application/← Use cases (inference gateway, proxy routing)
cynic-inference-bootstrap/  ← Composition root
```

**Extraction depuis** : `cynic-inference/` (150 fichiers)

---

### 2.6. CONSTELLATION : INTEL

> **Rôle** : Intelligence verticale (escore, futarchy, soc).

```
cynic-intel-domain/         ← Types purs (Score, Market, Threshold)
cynic-intel-escore/         ← EScore vertical
cynic-intel-futarchy/       ← Futarchy vertical
cynic-intel-soc/            ← SOC vertical
cynic-intel-application/    ← Use cases
```

**Extraction depuis** : `cynic-intel/` (52 fichiers)

---

### 2.7. CONSTELLATION : OBSERVER

> **Rôle** : Observation de l'écosystème (Solana, CYNIC interne).

```
cynic-observer-domain/      ← Types pur (Observation, Signal)
cynic-observer-solana/      ← Solana-specific (PoH, archive)
cynic-observer-cynic/       ← CYNIC-internal (health, metrics)
cynic-observer-application/ ← Use cases
```

**Extraction depuis** : `cynic-observer/` (44 fichiers)

---

### 2.8. CONSTELLATION : BUILD

> **Rôle** : Build pipeline & runner.

```
cynic-build-domain/         ← Types purs (BuildPlan, Artifact, Evidence)
cynic-build-adapter-runner/ ← Build runner harness
cynic-build-adapter-evidence/← Build evidence & proof
cynic-build-application/    ← Use cases
```

**Extraction depuis** : `cynic-build/` (32 fichiers)

---

### 2.9. CONSTELLATION : WORKLOAD

> **Rôle** : Workload delivery & tracking.

```
cynic-workload-domain/      ← Types purs (Workload, Task, Job)
cynic-workload-adapter-delivery/ ← Delivery & completion
cynic-workload-adapter-tracking/  ← Tracking & receipt
cynic-workload-application/ ← Use cases
```

**Extraction depuis** : `cynic-workload/` (21 fichiers)

---

### 2.10. CONSTELLATION : KERNEL (déjà défini)

```
cynic-kernel-domain/        ← Types purs (events, health, metrics, storage port)
cynic-kernel-api/           ← REST + WebSocket minimaux
cynic-kernel-infra/         ← Config, crypto, task_health
cynic-kernel-introspection/ ← Forensics orchestration
cynic-kernel-bootstrap/     ← Composition root
```

**Cible** : ~10k LOC, ~80 fichiers.

---

### 2.11. CONSTELLATION : STORAGE (déjà défini)

```
cynic-storage-ports/        ← StoragePort trait
cynic-storage-adapter-surreal/ ← SurrealDB implementation
cynic-storage-adapter-memory/  ← In-memory (testing)
```

**Cible** : ~2.5k LOC.

---

### 2.12. CONSTELLATION : IDENTITY (déjà défini)

```
cynic-identity/             ← ed25519 + node_id + BLAKE3
```

**Cible** : ~140 LOC.

---

### 2.13. CONSTELLATION : FORENSICS (déjà défini)

```
cynic-forensics-domain/     ← Types purs (CrashKind, ForensicsSnapshot)
cynic-forensics-adapter-probe/ ← Probe-based detection
cynic-forensics-adapter-local/  ← Local evidence collection
```

**Cible** : ~800 LOC.

---

### 2.14. CONSTELLATION : AGENT BUS (déjà fait)

```
cynic-agent-bus/            ← Domain pure (types, ports, ledger, claim)
cynic-agent-bus-server/     ← REST projection
cynic-agent-bus-coordinator/← Pyramid fleet
```

**Cible** : ~1.3k LOC.

---

## 3. DAG CIBLE

```
                    cynic-protocol-core
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
cynic-identity    cynic-protocol-dog    cynic-protocol-forensics
        │                  │                  │
        │                  ▼                  ▼
        │           cynic-kernel-domain   cynic-forensics-domain
        │                  │                  │
        │      ┌───────────┼───────────┐      │
        │      ▼           ▼           ▼      │
        │  cynic-host-  cynic-node-  cynic-   │
        │  ports        ports        storage- │
        │      │           │        ports    │
        │      ▼           ▼           │      │
        │  cynic-host-  cynic-node-    │      │
        │  adapter-*    adapter-*      │      │
        │      │           │           │      │
        │      ▼           ▼           ▼      │
        │  cynic-host-  cynic-node-  cynic-   │
        │  application  application  storage- │
        │      │           │        adapter-*│
        │      ▼           ▼           │      │
        │  cynic-host-  cynic-node-    │      │
        │  bootstrap    bootstrap      │      │
        │                         (SurrealDB)
        ▼
   cynic-cli-*
```

---

## 4. PLAN D'EXTRACTION

### Phase 1 — STABILISER LES FONDATIONS (cette session)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 1.1 | Tuer dead code kernel | `cynic-kernel` |
| 1.2 | Extraire `cynic-identity` | `cynic-identity` |
| 1.3 | Extraire `cynic-storage-ports` | `cynic-storage` |
| 1.4 | Valider kernel compile | `cynic-kernel` |

### Phase 2 — EXTRACTON LES VERTICALES (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 2.1 | Extraire `cynic-protocol-core` | `cynic-protocol-core` |
| 2.2 | Extraire `cynic-protocol-dog` | `cynic-protocol-dog` |
| 2.3 | Extraire `cynic-protocol-forensics` | `cynic-protocol-forensics` |
| 2.4 | Extraire `cynic-forensics-domain` | `cynic-forensics-domain` |
| 2.5 | Valider DAG | workspace |

### Phase 3 — EXTRACTON HOST (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 3.1 | Extraire `cynic-host-domain` | `cynic-host-domain` |
| 3.2 | Extraire `cynic-host-ports` | `cynic-host-ports` |
| 3.3 | Extraire `cynic-host-adapter-hardware` | `cynic-host-adapter-hardware` |
| 3.4 | Extraire `cynic-host-adapter-container` | `cynic-host-adapter-container` |
| 3.5 | Extraire `cynic-host-adapter-process` | `cynic-host-adapter-process` |
| 3.6 | Extraire `cynic-host-adapter-network` | `cynic-host-adapter-network` |
| 3.7 | Extraire `cynic-host-adapter-storage` | `cynic-host-adapter-storage` |
| 3.8 | Extraire `cynic-host-adapter-fleet` | `cynic-host-adapter-fleet` |
| 3.9 | Extraire `cynic-host-adapter-source` | `cynic-host-adapter-source` |
| 3.10 | Extraire `cynic-host-adapter-credential` | `cynic-host-adapter-credential` |
| 3.11 | Extraire `cynic-host-adapter-remediation` | `cynic-host-adapter-remediation` |
| 3.12 | Extraire `cynic-host-application` | `cynic-host-application` |
| 3.13 | Extraire `cynic-host-bootstrap` | `cynic-host-bootstrap` |
| 3.14 | Valider DAG | workspace |

### Phase 4 — EXTRACTON NODE (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 4.1 | Extraire `cynic-node-domain` | `cynic-node-domain` |
| 4.2 | Extraire `cynic-node-ports` | `cynic-node-ports` |
| 4.3 | Extraire `cynic-node-adapter-runtime` | `cynic-node-adapter-runtime` |
| 4.4 | Extraire `cynic-node-adapter-workload` | `cynic-node-adapter-workload` |
| 4.5 | Extraire `cynic-node-adapter-workspace` | `cynic-node-adapter-workspace` |
| 4.6 | Extraire `cynic-node-adapter-mesh` | `cynic-node-adapter-mesh` |
| 4.7 | Extraire `cynic-node-adapter-build` | `cynic-node-adapter-build` |
| 4.8 | Extraire `cynic-node-adapter-release` | `cynic-node-adapter-release` |
| 4.9 | Extraire `cynic-node-application` | `cynic-node-application` |
| 4.10 | Extraire `cynic-node-bootstrap` | `cynic-node-bootstrap` |
| 4.11 | Valider DAG | workspace |

### Phase 5 — EXTRACTON CLI + INFERENCE + INTEL (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 5.1 | Extraire `cynic-cli-kernel` | `cynic-cli-kernel` |
| 5.2 | Extraire `cynic-cli-agent` | `cynic-cli-agent` |
| 5.3 | Extraire `cynic-cli-host` | `cynic-cli-host` |
| 5.4 | Extraire `cynic-cli-node` | `cynic-cli-node` |
| 5.5 | Extraire `cynic-cli-fleet` | `cynic-cli-fleet` |
| 5.6 | Extraire `cynic-inference-domain` | `cynic-inference-domain` |
| 5.7 | Extraire `cynic-inference-ports` | `cynic-inference-ports` |
| 5.8 | Extraire `cynic-inference-adapter-*` | `cynic-inference-adapter-*` |
| 5.9 | Extraire `cynic-intel-domain` | `cynic-intel-domain` |
| 5.10 | Valider DAG | workspace |

---

## 5. RÈGLES D'EXTRACTION

1. **Un crate à la fois** — jamais 2 extractions en parallèle.
2. **Domain pur d'abord** — types + ports, puis adapters, puis application.
3. **Tests boundary obligatoires** — chaque nouveau crate a `tests/architecture_boundary.rs`.
4. **Validation DAG** — `cargo check --workspace` après chaque extraction.
5. **Zéro régression** — les consommateurs existants doivent compiler sans changement.
6. **Ratchet strict** — un crate extrait ne retourne jamais dans le monolithe.

---

## 6. MÉTRIQUES CIBLES

| Monolithe actuel | Fichiers | LOC | Cible | # Crates |
|------------------|----------|-----|-------|----------|
| `cynic-host` | 476 | 89,754 | ~12k | 12 |
| `cynic-node` | 338 | 58,092 | ~8k | 10 |
| `cynic-protocol` | 207 | 54,666 | ~5k | 7 |
| `cynic-cli` | 206 | 37,046 | ~3k | 6 |
| `cynic-kernel` | 169 | 31,988 | ~10k | 5 |
| `cynic-inference` | 150 | 26,846 | ~12k | 6 |
| `cynic-intel` | 52 | 10,864 | ~3k | 4 |
| `cynic-observer` | 44 | 8,558 | ~2k | 4 |
| `cynic-build` | 32 | 7,836 | ~2k | 3 |
| `cynic-workload` | 21 | 6,193 | ~2k | 3 |
| **TOTAL** | **1,695** | **331,833** | **~59k** | **~60** |

---

*Plan v1. Constellations hexagonales & DDD. Extraction progressive, validation continue.*
