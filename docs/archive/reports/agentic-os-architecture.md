# Rapport d'Analyse : Architecture d'un OS Agentique

**Date** : Septembre 2026  
**Contexte** : CYNIC (331k LOC, 6 crates monolithiques → 60 crates hexagonaux)  
**Question** : Comment un OS agentique devrait-il être structuré ? Quelle est la séparation kernel/node/host ? Comment gérer capabilities et isolation ?

---

## 1. Synthèse des Systèmes Étudiés

### 1.1 seL4 — Le microkernel vérifié formellement

| Aspect | Pattern |
|--------|---------|
| **Kernel** | Minimale (~10k SLOC) : TCBs, Scheduling Contexts, Page Directories, Endpoints, CNodes, Frames, Interrupt Objects, Notifications, Untyped Memory |
| **User-level** | RootTask (bootstrap) détient toutes les ressources physiques ; alloue aux enfants via capabilities |
| **Capabilities** | 10 types d'objets kernel, tous désignables uniquement via un CNode (arbre de capabilities). CSpace par processus. Unforgeable, communicable via IPC |
| **IPC** | Synchronous Protected Procedure Call (Call/ReplyRecv), MCS (Mixed-Criticality System) pour isolation temporelle |
| **Politique** | Zéro politique dans le kernel — toute politique est en user-space |
| **Mémoire** = | Frames alloués à des PDs ; gestion mémoire entièrement en user-space par le root task |

**Leçon CYNIC** : Le kernel ne garantit que mécanismes (isolation, communication, allocation physique). La politique (qui peut faire quoi) vit en user-space. CYNIC doit séparer son *moteur d'isolation* de ses *politiques d'allocation*.

---

### 1.2 Fuchsia — Capability routing et Job Tree

| Aspect | Pattern |
|--------|---------|
| **Kernel** | Zircon : objets Thread, Process, Job, VMO, Channel, Socket, Stream, Event, FIFO |
| **Hiérarchie** | Job tree (racine → enfants), chaque process est sous un job, contrôle de ressources hiérarchique |
| **Capabilities** | Handles (tokens 32-bit en user-space). Types : Process Handle, Thread Handle, Job Handle, VMO Handle, Channel Handle, etc. Rights associés (READ/WRITE/DUPLICATE/TRANSFER/GRANT) |
| **IPC** | FIDL protocols sur Channels (data + handle passing) — capability transfer par channel |
| **Composants** | Component Framework (CF) : composants sandboxés, capacité routing (discovery → broker → injection dans namespace) |
| **Mise à jour** | Composants indépendamment updatable, signed packages |

**Leçon CYNIC** : Le "job tree" est le bon modèle pour les agents CYNIC — un agent parent contrôle l'isolation et les ressources de ses sous-agents. Le "capability routing" de Fuchsia correspond au besoin de CYNIC de dynamiquement connecter des runtimes d'exécution.

---

### 1.3 Redox OS — Microkernel Rust, Scheme-based IPC

| Aspect | Pattern |
|--------|---------|
| **Kernel** | Microkernel Rust, minimal, IRQ management, IPC dispatch, memory management |
| **IPC** | "Scheme" system — tout est un URI (`scheme://path`), namespace par processus, résolution par daemon nsmgr |
| **Capabilities** | Namespace file descriptor + CWD comme capability ; openat(dir_fd, path) — pas de chemins globaux |
| **Sécurité** | Capability-based : un processus ne voit que ses schemes enregistrés dans sa namespace |
| **Drivers** | En user-space, namespace spécial permettant l'accès hardware interrupts |

**Leçon CYNIC** : Le modèle "namespace = ensemble de capabilities accessible" est directement transposable : un agent CYNIC ne devrait voir que les "protocoles" (schemes) autorisés dans son namespace de runtime.

---

### 1.4 Linux — Monolithique, POSIX capabilities, namespaces, cgroups

| Aspect | Pattern |
|--------|---------|
| **Kernel** | Monolithique (~30M SLOC), drivers in-kernel, syscall ABI stable |
| **Capabilities** : | POSIX capabilities (CAP_SYS_ADMIN, CAP_NET_ADMIN, etc.) — 3 sets par task (Effective, Permitted, Inheritable) |
| **Isolation** | Namespaces (mnt, pid, net, user), cgroups, seccomp, LSM (SELinux/AppArmor) |
| **Limitation** | Capabilities Linux = *pas* des object capabilities — ce sont des flags de privilège, pas des tokens communicables |
| **Modèle** | Retrofitted security sur un design monolithique — "everything is a file" + permission bits |

**Leçon CYNIC (négative)** : Les POSIX capabilities ne sont PAS des capabilities au sens seL4/Fuchsia. CYNIC doit éviter le piège "flag-based" et utiliser de vraies object capabilities communicables.

---

### 1.5 Frameworks Multi-Agents (LangGraph, Autogen, CrewAI)

| Framework | Pattern |
|-----------|---------|
| **LangGraph** | Graphe orienté : agents = nœuds, transitions = arêtes. Stateful, checkpointable, contrôle explicite des transitions. Mémoire partagée (state) + mémoires privées par agent |
| **Autogen** | Conversationnel : agents s'échangent des messages, orchestration par groupes/chat. Code executor natif |
| **CrewAI** | Rôle-based : chaque agent a un rôle, des goals, des tools. Orchestration séquentielle ou hiérarchique, délégation autonome |
| **Commun** | Prompts spécialisés par agent, outils dédiés, outputs structurés, mémoire multi-niveaux |

**Leçon CYNIC** : Les agents CYNIC devraient être modélisés comme un **graphe de nœuds avec transitions explicites** (LangGraph) + **rôles isolés avec budgets de capabilities** (CrewAI). L'isolation se fait par sandbox + scope de capabilities.

---

## 2. Architecture Proposée pour un OS Agentique

### 2.1 Les Trois Couches Fondamentales

```
┌─────────────────────────────────────────────────────────────────────┐
│  AGENT LAYER — Pure domain logic, policies, protocols               │
│  (n agents = n isolated runtimes, chacun avec son CSpace)           │
├─────────────────────────────────────────────────────────────────────┤
│  KERNEL LAYER — Isolation engine, capability verifier, scheduler    │
│  (moteur d'allocation, vérification, IPC, temporal budgeting)        │
├─────────────────────────────────────────────────────────────────────┤
│  HOST LAYER — Hardware abstraction, drivers, network fabric         │
│  (bare-metal, LXC, cloud — adapters vers le kernel)                  │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Séparation KERNEL / NODE / HOST

| Entité | Responsabilité | Inspiration |
|--------|----------------|-------------|
| **KERNEL** | • Vérification de toute opération (capability check)<br>• Allocation des ressources physiques (CPU, RAM, GPU)<br>• IPC atomique entre agents<br>• Ordonnancement temporel (budgets)<br>• Aucune politique — que des mécanismes | seL4 (minimale), Zircon (objets kernel) |
| **NODE** | • Exécute un sous-ensemble d'agents<br>• Gère la mémoire locale et le scheduling local<br>• Identity, attestation, heartbeat<br>• Capability brokering entre agents locaux<br>• Gestion de la persistence (SurrealDB locale) | Fuchsia (Job Tree), Redox (Scheme daemon) |
| **HOST** | • Abstraction hardware (CPU, GPU, NIC, Disk)<br>• Container runtime (LXC/gVisor)<br>• Network fabric (WireGuard mesh)<br>• Bootstrap chain<br>• Metrics & telemetry | Linux (cgroups/namespaces), ColonyOS (executor) |

**Principe fondamental** : Le kernel ne connaît ni les agents, ni les politiques métier, ni les protocoles réseau. Il ne connaît que : des **objets** (capabilities), des **budgets** (temps/mémoire), et des **canaux** (IPC).

### 2.3 Modèle de Capabilities pour CYNIC

```
Capability = (Object Reference, Rights, Generation)

Types d'objets CYNIC :
  • AgentHandle    → droits : SPAWN, SEND, KILL, INSPECT
  • StorageHandle  → droits : READ, WRITE, DELETE, QUOTA_SET
  • InferenceHandle → droits : SUBMIT, CANCEL, BUDGET_QUERY
  • NetworkHandle  → droits : BIND, CONNECT, LISTEN, ACCEPT
  • DomainHandle   → droits : RESOLVE, REGISTER, RENEW
  • MoneyHandle    → droits : PROPOSE, APPROVE, EXECUTE (budget guard)
  • DelegateHandle → droits : DELEGATE (capability forwarding)
```

**CSpace par agent** (inspiré seL4) :
- Chaque agent possède un CNode racine (sa capability space)
- Un agent parent peut déléguer un sous-ensemble de ses capabilities aux enfants
- Le "revocation anchor" permet de révoquer en cascade (pattern membrane)

### 2.4 Modèle de Namespace (inspiré Redox/Fuchsia)

```
Agent Namespace = {
  // Capabilities accessibles par URI-like scheme
  "storage://tasks/{task_id}"     → StorageHandle (READ | WRITE)
  "inference://models/{model_id}" → InferenceHandle (SUBMIT)
  "network://peers/{peer_id}"     → NetworkHandle (CONNECT)
  "agent://children/{child_id}"   → AgentHandle (SEND | INSPECT)
}
```

**Règle d'or** : Un agent ne peut *jamais* fabriquer une capability — il ne peut que les recevoir par :
1. Héritage du parent (bootstrap)
2. Réception via IPC (channel/handle transfer)
3. Brokering explicite par le capability router

### 2.5 Architecture Hexagonale (60 crates cibles)

```
cynic-domain/                    # Pure domain logic (zero dependencies)
  ├── domain-agent/              # Agent lifecycle, state machine
  ├── domain-workload/           # Task types, pipelines, DAG
  ├── domain-federation/         # Protocol, membership, consensus
  ├── domain-capability/         # Capability types, rights, policy DSL
  ├── domain-economics/          # Budgets, markets, incentives
  ├── domain-law/                # Constitutional court, policy
  └── domain-intel/              # Intelligence, detection, forensics

cynic-port/                      # Interfaces (traits uniquement)
  ├── port-storage/              # StorageReader, StorageWriter
  ├── port-inference/            # InferenceEngine, ModelRegistry
  ├── port-network/              # Transport, Discovery, Gossip
  ├── port-agent/                # AgentSpawner, AgentCommunicator
  ├── port-crypto/               # Signer, Verifier, Encryptor
  └── port-time/                 # Clock, TemporalOracle

cynic-adapter/                   # Implémentations concrètes
  ├── adapter-surrealdb/         # SurrealDB storage backend
  ├── adapter-llamacpp/          # llama.cpp inference backend
  ├── adapter-vllm/              # vLLM inference backend
  ├── adapter-wireguard/         # WireGuard transport
  ├── adapter-libp2p/            # libp2p transport (federation)
  ├── adapter-gvisor/            # gVisor sandbox
  ├── adapter-lxc/               # LXC container backend
  └── adapter-cloudflare/        # Cloudflare Workers backend

cynic-kernel/                    # Composition root (wiring)
  ├── kernel-isolation/          # CSpace, capability verifier, IPC
  ├── kernel-scheduler/          # Temporal budgeting, MCS-like
  ├── kernel-allocation/         # Resource allocation, quota enforcement
  └── kernel-bootstrap/          # Genesis, handoff, recovery

cynic-node/                      # Node runtime (l'agent "OS" local)
  ├── node-facade/               # API publique du node
  ├── node-supervisor/           # Supervision des agents locaux
  ├── node-attestation/          # Hardware attestation, TPM
  ├── node-mesh/                 # Federation, peer discovery
  └── node-watchdog/             # Health, recovery, upgrade

cynic-host/                      # Host OS adapters (hardware)
  ├── host-baremetal/            # Bare-metal deployment
  ├── host-linux/                # Linux host integration
  ├── host-cloud/                # Cloud VM adapter
  └── host-metrics/              # Observability, telemetry

cynic-cli/                       # User-facing command surface
cynic-protocol/                  # Wire protocol (already partially split)
cynic-inference/                 # Inference orchestration
cynic-storage/                   # Storage orchestration
cynic-network/                   # Network orchestration
cynic-mcp/                       # MCP projection
cynic-vcs/                       # Version control integration
cynic-build/                     # Build & release pipeline
```

---

## 3. Leçons Clés pour CYNIC

### Leçon 1 : Le kernel ne fait qu'ALLER et VÉRIFIANT — jamais de politique

Inspiration : seL4 (Liedtke's minimality principle)  
Application CYNIC : Le "kernel agentique" ne doit contenir :
- Capability verification (est-ce que cette operation est autorisée ?)
- Resource accounting (budget CPU/mémoire)
- IPC routing (passer un message d'un agent à un autre)
- Scheduling (quel agent tourne quand ?)

TOUT le reste (politique de spawn, règles de budget, protocoles fédéraux) vit en user-space.

### Leçon 2 : Hiérarchie de capabilities = Hiérarchie de contrôle

Inspiration : Fuchsia Job Tree, seL4 CNode tree  
Application CYNIC : Les "agents parents" contrôlent les ressources de leurs enfants via la délégation de capabilities. Un agent compromis ne peut pas escalader au-delà de son CSpace.

### Leçon 3 : Namespace comme confinement

Inspiration : Redox Scheme namespaces, Fuchsia component sandbox  
Application CYNIC : Chaque agent n'a qu'un namespace restreint de capabilities visibles. Pas de chemins globaux, pas d'URLs absolues sans capability correspondante.

### Leçon 4 = Hexagonal Architecture pour les crates

Inspiration : Alistair Cockburn's hexagonal, `hex` (AIOS), `cohesix`  
Application CYNIC : Chaque crate métier doit avoir :
- Un **domain** (pure logic, zéro deps)
- Des **ports** (traits Rust, interfaces)
- Des **adapters** (implémentations concrètes)
- Un **composition root** (wiring, main)

La règle : un crate du domaine ne peut PAS dépendre d'un adapter. L'inversion de contrôle est obligatoire.

### Leçon 5 = Séparation temporelle (MCS)

Inspiration : seL4 MCS (Mixed-Criticality System)  
Application CYNIC : Les agents critiques (forensics, constitutional court, secrets management) doivent avoir des budgets temporels garantis, indépendamment de la charge des agents non-critiques. Pas de starvation par un agent gourmand.

### Leçon 6 = Zero-Trust entre nœuds

Inspiration : ColonyOS zero-trust executor protocol, Fuchsia capability transfer  
Application CYNIC : Chaque nœud doit prouver son identité et ses autorisations à chaque interaction inter-nœuds. "Never trust, always verify" — pas de TLS implicitement trusted.

---

## 4. Décisions Architecturales Recommandées

### Décision 1 : Modèle de capabilities unifié

CYNIC doit adopter un modèle d'object capabilities (pas des POSIX capabilities). Chaque ressource (storage, inference, network, autre agent) est un "handle" communicable et vérifiable.

**Format** : Capability = { object_id: UUID, rights: Bitmap, generation: u64, revocation_anchor: Option<UUID> }

### Décision 2 : Les agents sont les "processus" de l'OS agentique

Un agent CYNIC = un process seL4/Fuchsia :
- Son propre CSpace (capabilities)
- Son propre scheduling context (budget CPU)
- Son propre namespace (ressources visibles)
- Son propre sandbox (isolation mémoire)

### Décision 3 : Le nœud CYNIC = le "Job" Fuchsia

Un nœud CYNIC gère un arbre d'agents (job tree). Il est responsable :
- Du brokering de capabilities entre agents locaux
- De l'attestation matérielle
- De la persistence locale (SurrealDB)
- De la fédération avec d'autres nœuds

### Décision 4 : Le kernel = seL4-style, minimal, vérifié

Le kernel CYNIC doit être aussi petit que possible (~5-10k LOC). Seulement :
- Capability table management
- IPC endpoint management  
- Resource frame allocation
- Scheduling context management

Idéalement, une partie devrait être vérifiée formellement (au moins le capability verifier + l'IPC fastpath).

### Décision 5 : 60 crates organisées en couches hexagonales

```
cynic-domain-*     (6 crates)  → pure logic, zéro deps workspace
cynic-port-*       (6 crates)  → traits uniquement
cynic-adapter-*    (8 crates)  → implémentations concrètes
cynic-kernel-*     (4 crates)  → mécanismes fondamentaux
cynic-node-*       (5 crates)  → runtime local d'agents
cynic-host-*       (4 crates)  → abstraction hardware
cynic-cli          (1 crate)   → surface de commande
cynic-protocol     (1 crate)   → wire protocol
```

Soit ~35 crates de base. Les 25 restants pour des domaines métier spécifiques (intel, forensics, market, etc.).

---

## 5. Risques et Limitations

| Risque | Mitigation |
|--------|------------|
| **Surcharge IPC** | Fastpath inspiré seL4 : appel direct entre agents dans le même nœud, pas de serialization |
| **Complexité du CSpace** | Bibliothèque helper "capspace" qui abstrait la gestion d'arbre |
| **Revocation cascade** | Pattern membrane : chaque delegation passe par un "membrane" qui peut révoquer en cascade |
| **Interop Linux/legacy** | Adapter POSIX pour les tools legacy, mais les nouveaux agents sont capability-native |
| **Performance inference** | GPU passthrough direct via VFIO — pas de copie via le kernel |

---

## 6. Conclusion

L'architecture optimale pour CYNIC combine :
1. **seL4** pour le kernel (minimale, vérifiée, capability-based)
2. **Fuchsia** pour le Job Tree et le capability routing entre agents
3. **Redox** pour les namespaces comme confinement par scheme
4. **Hexagonal Architecture** pour les crates Rust (domain pur ↔ adapters)
5. **LangGraph** pour les workflows multi-agents (graphe orienté explicite)

La séparation KERNEL/NODE/HOST correspond à :
- **KERNEL** = isolation engine + capability verifier + scheduler (métal)
- **NODE** = orchestrateur local d'agents + persistence + federation
- **HOST** = abstraction hardware + drivers + deployment target

---

*Fin du rapport*
