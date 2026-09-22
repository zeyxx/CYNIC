# RAPPORT D'ANALYSE DE CHEVAUCHEMENTS — CYNIC MONOLITHIC CRATES

**Date:** 2026-09-22  
**Crates analysés:** cynic-host, cynic-node, cynic-kernel, cynic-protocol, cynic-cli, cynic-inference  
**Total:** ~298k LOC, 1546 fichiers

---

## 1. TABLEAU RÉCAPITULATIF

| Paire | Chev. LOC (estimé) | Types sig. dup. | Types totaux dup. | Imports c1→c2 | Imports c2→c1 | Frontière |
|-------|-------------------|-----------------|-------------------|---------------|---------------|-----------|
| cynic-host ↔ cynic-node | ~3,712 | 16 | 148 | 0 | 135 | 🔴 MAJEURE → refonte |
| cynic-node ↔ cynic-kernel | ~1,324 | 12 | 53 | 0 | 5 | 🟠 Modérée → domain |
| cynic-host ↔ cynic-kernel | ~1,325 | 8 | 53 | 0 | 5 | 🟠 Modérée → domain |
| cynic-node ↔ cynic-protocol | ~1,328 | 7 | 61 | 146 | 0 | 🟠 Modérée → domain |
| cynic-protocol ↔ cynic-cli | ~595 | 5 | 24 | 0 | 67 | 🟠 Modérée → domain |
| cynic-node ↔ cynic-inference | ~1,349 | 4 | 57 | 2 | 0 | 🟡 Mineure → shared |
| cynic-host ↔ cynic-protocol | ~2,026 | 3 | 93 | 254 | 0 | 🟡 Mineure → shared |
| cynic-host ↔ cynic-cli | ~1,490 | 3 | 53 | 0 | 128 | 🟡 Mineure → shared |
| cynic-node ↔ cynic-cli | ~1,124 | 3 | 40 | 0 | 25 | 🟡 Mineure → shared |
| cynic-kernel ↔ cynic-protocol | ~802 | 3 | 37 | 44 | 0 | 🟡 Mineure → shared |
| cynic-host ↔ cynic-inference | ~1,373 | 2 | 58 | 1 | 0 | 🟡 Mineure → shared |
| cynic-kernel ↔ cynic-inference | ~755 | 1 | 32 | 19 | 0 | 🟡 Mineure → shared |
| cynic-protocol ↔ cynic-inference | ~713 | 1 | 35 | 0 | 25 | 🟡 Mineure → shared |
| cynic-cli ↔ cynic-inference | ~854 | 1 | 32 | 34 | 0 | 🟡 Mineure → shared |
| cynic-kernel ↔ cynic-cli | ~392 | 0 | 14 | 0 | 0 | ✅ Claire |

---

## 2. DÉTAIL PAR PAIRE

### 2.1 cynic-host ↔ cynic-node — 🔴 MAJEUR

**Types dupliqués (struct/enum/trait):**
- `FakeCommandRunner`, `FakeRunner`, `ForgejoPayload`, `HostExecRequest`, `MockObserver`
- `NetworkPolicy`, `Node`, `Registry`, `RegistryLifecycle`, `RegistryNetworkPolicy`, `RegistryNode`
- `RuntimeArtifact`, `RuntimeBasket`, `StubPort`
- ⚠️ `can` (trait dans host, enum dans node) — **incohérence de kind**
- ⚠️ `the` (type dans host, struct dans node) — **incohérence de kind**

**Imports croisés:** node→host: 135 imports (host ne dépend pas de node)

**Responsabilités partagées à tort:**
- **Registry/Node lifecycle** — les deux crates gèrent l'enregistrement des nœuds
- **NetworkPolicy** — défini dans les deux, devrait être un type partagé
- **RuntimeArtifact/RuntimeBasket** — chevauchement de gestion d'artefacts

**Frontière proposée:**
- `cynic-host` = adapter d'exécution host (IO, filesystem, processus)
- `cynic-node` = adapter de contrôle node (activation, réseau, DNS)
- Extraire `NetworkPolicy`, `Node`, `Registry` dans `cynic-types` ou `cynic-domain-fleet`

---

### 2.2 cynic-node ↔ cynic-kernel — 🟠 MODÉRÉ

**Types dupliqués:**
- `AppState`, `EnrollmentResponse`, `FleetInventoryDetails`, `FleetInventoryProviderSummary`
- `FleetInventorySummary`, `FleetNodeObservation`, `KernelIdentity`, `NodeConfig`, `SurrealResponse`
- ⚠️ `to` (type dans node, trait dans kernel)
- ⚠️ `would` (enum dans node, struct dans kernel)

**Imports croisés:** kernel→node: 5 imports

**Responsabilités partagées à tort:**
- **FleetInventory** — le kernel expose des types d'inventaire qui devraient être dans node
- **EnrollmentResponse** — le kernel définit la réponse d'enrollment
- **SurrealResponse** — type de réponse DB partagé

**Frontière proposée:**
- `cynic-kernel` = API kernel (routing, backends, storage, health probes)
- `cynic-node` = runtime node (actuation, network, supervision)
- Migrer `FleetInventory*`, `EnrollmentResponse` vers `cynic-domain-fleet`

---

### 2.3 cynic-host ↔ cynic-kernel — 🟠 MODÉRÉ

**Types dupliqués:**
- `CircuitState`, `CryptoError`, `IdentityCapability`
- ⚠️ `MetabolicState` (enum dans host, struct dans kernel) — **incohérence**
- `NetworkProbe`, `OsHealthFinding`, `ProbeStatus`, `SecretMaterial`

**Imports croisés:** kernel→host: 5 imports

**Responsabilités partagées à tort:**
- **Health/Monitoring** — les deux gèrent la santé (OsHealthFinding, ProbeStatus)
- **Crypto/Secrets** — `CryptoError`, `SecretMaterial` dupliqués
- **MetabolicState** — devrait être un type de domaine unique

**Frontière proposée:**
- `cynic-host` = exécution host (processus, filesystem, hardware)
- `cynic-kernel` = API kernel (routing, backends, storage)
- Extraire `MetabolicState`, `OsHealthFinding`, `ProbeStatus` dans `cynic-domain-health`

---

### 2.4 cynic-node ↔ cynic-protocol — 🟠 MODÉRÉ

**Types dupliqués:**
- `AllowanceGrant`, `ReleaseProof`, `RuntimeBasketArtifact`
- `SolanaPayRequest`, `StorageDecision`, `StorageObservation`

**Imports croisés:** node→protocol: 146 imports

**Responsabilités partagées à tort:**
- **Storage** — `StorageDecision`, `StorageObservation` dans les deux
- **Release/Allowance** — protocoles de libération dupliqués

**Frontière proposée:**
- `cynic-protocol` = contrats de communication (messages, preuves)
- `cynic-node` = runtime node (pas de types de protocole inline)
- Migrer `StorageDecision`, `StorageObservation` vers `cynic-domain-storage`

---

### 2.5 cynic-protocol ↔ cynic-cli — 🟠 MODÉRÉ

**Types dupliqués:**
- `AgentBudgets`, `AuditSummary`, `AutoresearchReport`, `Observation`, `OneShotCapability`

**Imports croisés:** cli→protocol: 67 imports

**Responsabilités partagées à tort:**
- **Observation** — type de base dupliqué
- **Audit** — `AuditSummary` dans les deux

**Frontière proposée:**
- `cynic-protocol` = contrats uniquement
- `cynic-cli` = surface CLI (pas de types métier inline)
- Extraire `Observation`, `AuditSummary` dans `cynic-domain-observation`

---

### 2.6 cynic-kernel ↔ cynic-cli — ✅ CLAIRE

**Types dupliqués:** 0 (uniquement des fonctions utilitaires triviales)

**Imports croisés:** 0

**Frontière:** Claire. Aucune refonte nécessaire.

---

## 3. TYPES CRITIQUES MULTI-CRATES (RESPONSABILITÉ PARTAGÉE À TORT)

| Type | Crates concernés | Problème |
|------|-----------------|----------|
| `the` | host, node, protocol | type/struct — nom ambigu, devrait être renommé ou unifié |
| `NetworkPolicy` | cli, host, node | Défini dans 3 crates — devrait être dans `cynic-types` |
| `Registry` | cli, host, node | Défini dans 3 crates — devrait être dans `cynic-domain-fleet` |
| `MetabolicState` | host, kernel, protocol | enum/struct — incohérence de kind, devrait être dans `cynic-domain-metabolism` |
| `Observation` | cli, inference, protocol | struct/enum — incohérence, devrait être dans `cynic-domain-observation` |

---

## 4. MAPPAGE HEXAGONAL PROPOSÉ

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    ARCHITECTURE HEXAGONALE CIBLE                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  DOMAIN CRATES (pure business logic, no I/O):                           │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │ cynic-domain-   │  │ cynic-domain-   │  │ cynic-domain-   │         │
│  │ lifecycle       │  │ fleet           │  │ health          │         │
│  │ (claim/release/ │  │ (Node/Registry/ │  │ (OsHealthFinding│         │
│  │  enrollment)    │  │  FleetInventory)│  │  ProbeStatus)   │         │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │ cynic-domain-   │  │ cynic-domain-   │  │ cynic-domain-   │         │
│  │ metabolism      │  │ observation     │  │ inference       │         │
│  │ (MetabolicState │  │ (Observation    │  │ (AgentRuntime   │         │
│  │  budget)        │  │  AuditSummary)  │  │  Mode)          │         │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
│                                                                         │
│  PORT CRATES (interfaces/contracts):                                    │
│  ┌─────────────────┐  ┌─────────────────┐                              │
│  │ cynic-protocol  │  │ cynic-types     │  (shared types: the,          │
│  │ (ReleaseProof   │  │ (NetworkPolicy, │   MetabolicState,             │
│  │  AllowanceGrant)│  │  Registry,      │   Observation)                │
│  └─────────────────┘  │  Node)          │                              │
│                       └─────────────────┘                              │
│                                                                         │
│  ADAPTER CRATES (infrastructure/IO):                                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │ cynic-host      │  │ cynic-node      │  │ cynic-kernel    │         │
│  │ (host adapter)  │  │ (node adapter)  │  │ (kernel adapter)│         │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
│  ┌─────────────────┐  ┌─────────────────┐                              │
│  │ cynic-cli       │  │ cynic-inference │                              │
│  │ (CLI adapter)   │  │ (inference      │                              │
│  └─────────────────┘  │  adapter)       │                              │
│                       └─────────────────┘                              │
│                                                                         │
│  APPLICATION CRATES (orchestration):                                    │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ cynic-core (orchestration, wire-up, minimal logic)          │       │
│  └─────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 5. RECOMMANDATIONS

### Priorité 1 — Refonte majeure (host ↔ node)
- Extraire `NetworkPolicy`, `Node`, `Registry` dans `cynic-domain-fleet`
- Unifier `RuntimeArtifact`, `RuntimeBasket` dans `cynic-domain-lifecycle`
- Supprimer les 135 imports node→host (inverser la dépendance)

### Priorité 2 — Migration modérée
- `MetabolicState` → `cynic-domain-metabolism` (unifier enum/struct)
- `OsHealthFinding`, `ProbeStatus` → `cynic-domain-health`
- `FleetInventory*`, `EnrollmentResponse` → `cynic-domain-fleet`
- `Observation`, `AuditSummary` → `cynic-domain-observation`

### Priorité 3 — Nettoyage
- Renommer ou supprimer le type `the` (ambigu, présent dans 3 crates)
- Supprimer les types `would`, `of`, `to` dupliqués (noms ambigus)
- Unifier `Observation` (struct vs enum)

### Priorité 4 — Stabilisation
- `cynic-kernel` et `cynic-cli` ont une frontière claire — garder tel quel
- `cynic-protocol` ne dépend de personne — c'est un bon crate de base
- `cynic-inference` a un chevauchement mineur — migration simple

---

## 6. MÉTRIQUES FINALES

| Crate | LOC | Fichiers | Modules | Types sig. | Imports cross-crates |
|-------|-----|----------|---------|------------|---------------------|
| cynic-host | 89,754 | 476 | 137 | 893 | 11 imports (principalement protocol) |
| cynic-node | 58,092 | 338 | 60 | 637 | 6 imports (host, protocol, workload) |
| cynic-kernel | 31,988 | 169 | 13 | 293 | 9 imports (protocol, inference, node, host) |
| cynic-protocol | 54,666 | 207 | 133 | 939 | 0 imports cross-crates |
| cynic-cli | 37,046 | 206 | 86 | 233 | 8 imports (host, protocol, inference, node) |
| cynic-inference | 26,846 | 150 | 16 | 355 | 2 imports (protocol, workload) |

**Total chevauchement estimé:** ~17,500 LOC à migrer vers des crates de domaine.
