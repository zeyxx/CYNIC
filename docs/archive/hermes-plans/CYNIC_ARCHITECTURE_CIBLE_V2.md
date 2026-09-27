# CYNIC — ARCHITECTURE CIBLE v2 (corrigée)

> **Basé sur** : recherche externe (Bevy, Deno, Actix, Reth), analyse de chevauchements (17.5K LOC dupliqués), analyse OS agentique (seL4, Fuchsia, Redox), pureté DDD/Hexa (0% > 45%).
> **Objectif** : 6 monolithes (331K LOC) → ~40 crates hexagonaux (~8K LOC/crate).
> **Stratégie** : Stabiliser → Unifier → Extraire → Valider.

---

## 1. PRINCIPES DIRECTEURS (SOTA Rust)

| Principe | Source | Application CYNIC |
|----------|--------|-------------------|
| **Flat `crates/`** | matklad, rust-analyzer | Pas de hiérarchie profonde |
| **Taille 5K-15K LOC/crate** | Bevy, Deno, Actix | Découper tout > 15K |
| **Umbrella crate mince** | Bevy (2.4KB), Tokio | `cynic/` ré-exporte tout |
| **Features = firewall** | Tokio (50+ features) | Gate les modules lourds |
| **main.rs < 100 lignes** | Tous projets | Composition root uniquement |
| **[workspace.dependencies]** | Standard | Cohérence des versions |
| **DAG strict** | Tous projets | Zéro cycle, zéro import ascendant |

---

## 2. ARCHITECTURE CIBLE — 3 COUCHES

```
┌─────────────────────────────────────────────────────────────────┐
│  COUCHE 1 : HOST (abstraction matérielle)                       │
│  ──────────────────────────────────────────                     │
│  Hardware probes, containers, process exec, network, fleet      │
│  → Ne connaît ni le kernel ni les agents                       │
│  → Expose des traits (HardwareProbe, ContainerRuntime)          │
│  → Implémente les adapters (Linux, Proxmox, WireGuard)          │
│  → ~12K LOC, ~150 fichiers                                     │
├─────────────────────────────────────────────────────────────────┤
│  COUCHE 2 : KERNEL (superviseur de runtime)                     │
│  ──────────────────────────────────────────                     │
│  Capability verifier, IPC router, MCS scheduler                 │
│  → Ne connaît ni le matériel ni les agents                     │
│  → Alloue et vérifie uniquement (seL4 principle)               │
│  → ~8K LOC, ~80 fichiers                                       │
├─────────────────────────────────────────────────────────────────┤
│  COUCHE 3 : NODE (orchestrateur d'agents)                       │
│  ──────────────────────────────────────────                     │
│  Job Tree style Fuchsia, persistence locale, attestation        │
│  → Consomme les capabilities du kernel                         │
│  → Exécute les workloads via les agents                        │
│  → ~10K LOC, ~100 fichiers                                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. DAG CIBLE (strict, sans cycle)

```
                    cynic-types (shared kernel)
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
   cynic-host-      cynic-kernel-   cynic-node-
   domain           domain          domain
          │              │              │
          ▼              ▼              ▼
   cynic-host-      cynic-kernel-   cynic-node-
   ports            ports           ports
          │              │              │
          ▼              ▼              ▼
   cynic-host-      cynic-kernel-   cynic-node-
   adapters         adapters        adapters
          │              │              │
          ▼              ▼              ▼
   cynic-host-      cynic-kernel-   cynic-node-
   application      application     application
          │              │              │
          ▼              ▼              ▼
               cynic-cli (adapter sortant)
```

**Règle** : les flèches ne vont que vers le bas. Zéro cycle. Zéro import latéral.

---

## 4. DÉCOUPAGE PAR CONSTELLATION

### 4.1. CONSTELLATION : TYPES PARTAGÉS (extraction immédiate)

> **Objectif** : Unifier les 5 types multi-crates et éliminer les 17.5K LOC de chevauchement.

| Crate | Types | LOC |
|-------|-------|-----|
| `cynic-types` | NetworkPolicy, Registry, MetabolicState, Observation, Hash32, AgentId, Timestamp | ~2K |
| `cynic-protocol` | Dog, Verdict, Axiom, Crystal, InferenceRequest, BackendConfig | ~5K |

**Extraction depuis** : `cynic-protocol/` (207 fichiers → ~50 fichiers), types dupliqués depuis host/node/cli.

---

### 4.2. CONSTELLATION : HOST (89K → ~12K LOC)

> **Rôle** : Abstraction matérielle. Zéro logique métier.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-host-domain` | Types purs (HardwareSpec, ContainerSpec, NetworkConfig) | ~2K |
| `cynic-host-ports` | Traits (HardwareProbe, ContainerRuntime, ProcessExecutor) | ~1K |
| `cynic-host-adapter-hardware` | CPU, RAM, Disk, Network, USB | ~3K |
| `cynic-host-adapter-container` | LXC, Proxmox, libvirt | ~2K |
| `cynic-host-adapter-process` | Process execution & supervision | ~1K |
| `cynic-host-adapter-network` | WireGuard, Headscale, DNS | ~2K |
| `cynic-host-adapter-fleet` | Fleet health & heartbeat | ~1K |
| `cynic-host-adapter-source` | Git sync, worktree management | ~2K |
| `cynic-host-adapter-credential` | Secrets, credentials, hardening | ~1K |
| `cynic-host-adapter-remediation` | Remediation & recovery | ~1K |
| `cynic-host-application` | Use cases (bootstrap, activation) | ~2K |
| `cynic-host-bootstrap` | Composition root | ~1K |

**Extraction depuis** : `cynic-host/` (476 fichiers → ~150 fichiers).

---

### 4.3. CONSTELLATION : KERNEL (32K → ~8K LOC)

> **Rôle** : Superviseur de runtime. Capability verifier + IPC router + MCS scheduler.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-kernel-domain` | Types purs (KernelEvent, HealthGate, StoragePort) | ~2K |
| `cynic-kernel-ports` | Traits (StoragePort, CoordPort, HealthGate) | ~1K |
| `cynic-kernel-adapter-storage` | SurrealDB adapter | ~3K |
| `cynic-kernel-adapter-coord` | Coordination adapter | ~1K |
| `cynic-kernel-application` | Use cases (orchestration, scheduling) | ~2K |
| `cynic-kernel-bootstrap` | Composition root | ~1K |

**Extraction depuis** : `cynic-kernel/` (169 fichiers → ~80 fichiers).

---

### 4.4. CONSTELLATION : NODE (58K → ~10K LOC)

> **Rôle** : Orchestrateur local d'agents. Job Tree style Fuchsia.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-node-domain` | Types purs (AgentSpec, Workload, Workspace) | ~2K |
| `cynic-node-ports` | Traits (AgentExecutor, Workstore, WorkspaceLifecycle) | ~1K |
| `cynic-node-adapter-runtime` | Agent execution & scheduling | ~2K |
| `cynic-node-adapter-workload` | Workload delivery & completion | ~2K |
| `cynic-node-adapter-workspace` | Workspace lifecycle | ~2K |
| `cynic-node-adapter-mesh` | Network mesh & attestation | ~2K |
| `cynic-node-adapter-build` | Build workload grants | ~1K |
| `cynic-node-adapter-release` | Release artifact publication | ~1K |
| `cynic-node-application` | Use cases | ~2K |
| `cynic-node-bootstrap` | Composition root | ~1K |

**Extraction depuis** : `cynic-node/` (338 fichiers → ~100 fichiers).

---

### 4.5. CONSTELLATION : CLI (37K → ~3K LOC)

> **Rôle** : Surface opérateur. Adapter sortant vers le terminal.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-cli` | Commandes (health, capabilities, discovery) | ~3K |

**Note** : `cynic-cli` n'a pas de domain propre — c'est un **adapter sortant** (UI). Il consomme les traits des autres crates.

**Extraction depuis** : `cynic-cli/` (206 fichiers → ~50 fichiers).

---

### 4.6. CONSTELLATION : INFERENCE (26K → ~12K LOC)

> **Rôle** : Inference souveraine. Du hardware budget à l'agent loop.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-inference-domain` | Types purs (EngineCapabilities, ModelLayout, HardwareProfile) | ~3K |
| `cynic-inference-ports` | Traits (SovereignRuntimePort, ModelStore, BackendHealth) | ~1K |
| `cynic-inference-adapter-model` | Model registry, discovery, catalog | ~2K |
| `cynic-inference-adapter-engine` | Engine capabilities, runtime plan | ~2K |
| `cynic-inference-adapter-routing` | Routing calculator, proxy targets | ~2K |
| `cynic-inference-application` | Use cases (inference gateway, proxy routing) | ~2K |
| `cynic-inference-bootstrap` | Composition root | ~1K |

**Extraction depuis** : `cynic-inference/` (150 fichiers → ~80 fichiers).

---

### 4.7. CONSTELLATION : INTEL (10K → ~3K LOC)

> **Rôle** : Intelligence verticale (escore, futarchy, soc).

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-intel-domain` | Types purs (Score, Market, Threshold) | ~1K |
| `cynic-intel-escore` | EScore vertical | ~1K |
| `cynic-intel-futarchy` | Futarchy vertical | ~1K |
| `cynic-intel-soc` | SOC vertical | ~1K |

**Extraction depuis** : `cynic-intel/` (52 fichiers → ~30 fichiers).

---

### 4.8. CONSTELLATION : OBSERVER (8K → ~2K LOC)

> **Rôle** : Observation de l'écosystème (Solana, CYNIC interne).

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-observer-domain` | Types purs (Observation, Signal) | ~1K |
| `cynic-observer-solana` | Solana-specific (PoH, archive) | ~1K |
| `cynic-observer-cynic` | CYNIC-internal (health, metrics) | ~1K |

**Extraction depuis** : `cynic-observer/` (44 fichiers → ~20 fichiers).

---

### 4.9. CONSTELLATION : BUILD (7K → ~2K LOC)

> **Rôle** : Build pipeline & runner.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-build-domain` | Types purs (BuildPlan, Artifact, Evidence) | ~1K |
| `cynic-build-adapter-runner` | Build runner harness | ~1K |
| `cynic-build-adapter-evidence` | Build evidence & proof | ~1K |

**Extraction depuis** : `cynic-build/` (32 fichiers → ~20 fichiers).

---

### 4.10. CONSTELLATION : WORKLOAD (6K → ~2K LOC)

> **Rôle** : Workload delivery & tracking.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-workload-domain` | Types purs (Workload, Task, Job) | ~1K |
| `cynic-workload-adapter-delivery` | Delivery & completion | ~1K |
| `cynic-workload-adapter-tracking` | Tracking & receipt | ~1K |

**Extraction depuis** : `cynic-workload/` (21 fichiers → ~15 fichiers).

---

### 4.11. CONSTELLATION : STORAGE (2K → ~2K LOC)

> **Rôle** : Storage adapters (SurrealDB, in-memory).

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-storage-ports` | StoragePort trait | ~1K |
| `cynic-storage-adapter-surreal` | SurrealDB implementation | ~1K |
| `cynic-storage-adapter-memory` | In-memory (testing) | ~1K |

**Déjà extrait** : `cynic-storage/` existe (10 fichiers, 2.4K LOC).

---

### 4.12. CONSTELLATION : IDENTITY (~140 LOC)

> **Rôle** : ed25519 + node_id + BLAKE3.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-identity` | ed25519 + node_id + BLAKE3 | ~140 |

**Extraction depuis** : `cynic-kernel/infra/node_identity.rs`.

---

### 4.13. CONSTELLATION : FORENSICS (~800 LOC)

> **Rôle** : Détection crash & anomalies.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-forensics-domain` | Types purs (CrashKind, ForensicsSnapshot) | ~300 |
| `cynic-forensics-adapter-probe` | Probe-based detection | ~300 |
| `cynic-forensics-adapter-local` | Local evidence collection | ~200 |

**Déjà extrait** : `cynic-forensics/` existe (5 fichiers, 802 LOC).

---

### 4.14. CONSTELLATION : AGENT BUS (~1.3K LOC)

> **Rôle** : Coordination multi-agents.

| Sous-crate | Rôle | LOC |
|------------|------|-----|
| `cynic-agent-bus` | Domain pure (types, ports, ledger, claim) | ~774 |
| `cynic-agent-bus-server` | REST projection | ~91 |
| `cynic-agent-bus-coordinator` | Pyramid fleet | ~457 |

**Déjà extrait** : 3 crates existent.

---

## 5. MÉTRIQUES CIBLES

| Monolithe actuel | LOC actuel | Cible | # Crates |
|------------------|------------|-------|----------|
| `cynic-host` | 89,754 | ~12K | 12 |
| `cynic-node` | 58,092 | ~10K | 10 |
| `cynic-kernel` | 31,988 | ~8K | 6 |
| `cynic-inference` | 26,846 | ~12K | 7 |
| `cynic-cli` | 37,046 | ~3K | 1 |
| `cynic-intel` | 10,864 | ~3K | 4 |
| `cynic-observer` | 8,558 | ~2K | 3 |
| `cynic-build` | 7,836 | ~2K | 3 |
| `cynic-workload` | 6,193 | ~2K | 3 |
| `cynic-protocol` | 54,666 | ~5K | 2 |
| **TOTAL** | **331,833** | **~61K** | **~51** |

**Réduction** : 331K → 61K LOC (-82%), 1546 → ~500 fichiers (-68%).

---

## 6. PLAN D'EXTRACTION (révisé)

### Phase 0 — STABILISER (cette session, en cours)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 0.1 | Tuer dead code kernel | `cynic-kernel` |
| 0.2 | Fixer les mod.rs | `cynic-kernel` |
| 0.3 | Valider kernel compile | `cynic-kernel` |

### Phase 1 — UNIFIER LES TYPES (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 1.1 | Extraire `cynic-types` (NetworkPolicy, Registry, etc.) | `cynic-types` |
| 1.2 | Unifier `MetabolicState` (enum vs struct) | `cynic-types` |
| 1.3 | Unifier `Observation` (struct vs enum) | `cynic-types` |
| 1.4 | Valider DAG | workspace |

### Phase 2 — EXTRACTON LES VERTICALES (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 2.1 | Extraire `cynic-identity` | `cynic-identity` |
| 2.2 | Extraire `cynic-host-domain` + `cynic-host-ports` | `cynic-host-*` |
| 2.3 | Extraire `cynic-kernel-domain` + `cynic-kernel-ports` | `cynic-kernel-*` |
| 2.4 | Extraire `cynic-node-domain` + `cynic-node-ports` | `cynic-node-*` |
| 2.5 | Valider DAG | workspace |

### Phase 3 — EXTRACTON LES ADAPTERS (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 3.1 | Extraire `cynic-host-adapter-*` | `cynic-host-adapter-*` |
| 3.2 | Extraire `cynic-kernel-adapter-*` | `cynic-kernel-adapter-*` |
| 3.3 | Extraire `cynic-node-adapter-*` | `cynic-node-adapter-*` |
| 3.4 | Valider DAG | workspace |

### Phase 4 — EXTRACTON APPLICATION + BOOTSTRAP (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 4.1 | Extraire `cynic-host-application` + `cynic-host-bootstrap` | `cynic-host-*` |
| 4.2 | Extraire `cynic-kernel-application` + `cynic-kernel-bootstrap` | `cynic-kernel-*` |
| 4.3 | Extraire `cynic-node-application` + `cynic-node-bootstrap` | `cynic-node-*` |
| 4.4 | Valider DAG | workspace |

### Phase 5 — EXTRACTON CLI + INFERENCE + INTEL (session suivante)

| Étape | Action | Crate(s) |
|-------|--------|----------|
| 5.1 | Extraire `cynic-cli` (adapter sortant) | `cynic-cli` |
| 5.2 | Extraire `cynic-inference-*` | `cynic-inference-*` |
| 5.3 | Extraire `cynic-intel-*` | `cynic-intel-*` |
| 5.4 | Valider DAG | workspace |

---

## 7. RÈGLES D'EXTRACTION (SOTA)

1. **Un crate à la fois** — jamais 2 extractions en parallèle.
2. **Domain pur d'abord** — types + ports, puis adapters, puis application.
3. **Tests boundary obligatoires** — chaque nouveau crate a `tests/architecture_boundary.rs`.
4. **Validation DAG** — `cargo check --workspace` après chaque extraction.
5. **Zéro régression** — les consommateurs existants doivent compiler sans changement.
6. **Ratchet strict** — un crate extrait ne retourne jamais dans le monolithe.
7. **Feature flags** — gate les modules lourds (SurrealDB, inference, crypto).
8. **Umbrella crate** — `cynic/` ré-exporte tout avec features.

---

## 8. UMBRELLA CRATE (Bevy-style)

```rust
// crates/cynic/src/lib.rs (~200 lignes)
pub use cynic_types::*;
pub use cynic_protocol::*;

#[cfg(feature = "host")]
pub use cynic_host::*;

#[cfg(feature = "kernel")]
pub use cynic_kernel::*;

#[cfg(feature = "node")]
pub use cynic_node::*;

#[cfg(feature = "inference")]
pub use cynic_inference::*;

#[cfg(feature = "cli")]
pub use cynic_cli::*;
```

```toml
# crates/cynic/Cargo.toml
[features]
default = ["full"]
full = ["host", "kernel", "node", "inference", "cli"]
host = ["cynic-host"]
kernel = ["cynic-kernel"]
node = ["cynic-node"]
inference = ["cynic-inference"]
cli = ["cynic-cli"]
```

---

## 9. ANTI-PATTERNS À ÉVITER

| Anti-pattern | Pourquoi | Solution |
|--------------|----------|----------|
| **God crate** (> 15K LOC) | Recompilation totale | Split par domaine |
| **Dépendances circulaires** | Interdit par Cargo | Traits + shared crate |
| **Hiérarchie profonde** | Cargo ne comprend pas | Flat `crates/` |
| **async_trait dans domain** | Pollue la pureté | Traits Rust natifs (1.75+) |
| **Re-exports externes dans domain** | Fuites de dépendance | Domain = zéro deps externes |
| **Pas de composition root** | Wiring dispersé | `compose.rs` unique |
| **Pas de feature flags** | Compilation monolithique | Features par domaine |

---

*Architecture cible v2. Validée par recherche externe, analyse de chevauchements, OS agentique, pureté DDD/Hexa.*
