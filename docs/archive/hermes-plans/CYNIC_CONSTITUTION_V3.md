# CYNIC — CONSTITUTION v3 (corrigée)

> **Version** : 3.0
> **Date** : 2026-09-22
> **Basée sur** : Solana layer isolation, adversarial review, CYNIC Net design, protocol evolution
> **Changements majeurs** : Tor→CYNIC Net, Nodes≠Agents, Gouvernement exécutable, Scaling protocolaire

---

## 1. DISTINCTION FONDAMENTALE : NODES vs AGENTS

### 1.1. NODE — Le substrat physique

> **Définition** : Un NŒUD est une entité matérielle ou virtuelle qui exécute le runtime CYNIC. Il est défini par ses ressources physiques, sa localisation, et son identité cryptographique.

| Attribut | Description |
|----------|-------------|
| **Identité** | ed25519 keypair → BLAKE3(pubkey)[..16] → node_id |
| **Hardware** | CPU, RAM, Disk, Network, GPU (optionnel) |
| **Localisation** | Coordonnées physiques ou réseau (IP, région) |
| **Ressources** | Compute (CU), Storage (GB), Bandwidth (Mbps) |
| **Statut** | Active, Inactive, Compromised, Decommissioned |
| **Rôle** | Validator, Relay, Observer, Bootstrap, Edge |

**Caractéristiques** :
- Un nœud est **borné physiquement** (consommation, latence, capacité)
- Un nœud peut héberger **plusieurs agents**
- Un nœud peut **défaillir** (panne matérielle, partition réseau, compromission)
- Un nœud est **remplaçable** (ses agents migrent vers un autre nœud)

**Types de nœuds** :

| Type | Rôle | Contraintes |
|------|------|-------------|
| **Validator** | Participe au consensus | Haute disponibilité, stake |
| **Relay** | Relaie les messages CYNIC Net | Bandwidth, uptime |
| **Observer** | Observe sans valider | Minimal (Raspberry Pi suffit) |
| **Bootstrap** | Initialise la confiance | Multisig, cérémonie |
| **Edge** | Nœud portable (Seeker) | <15W, batterie, intermittence |

---

### 1.2. AGENT — L'entité cognitive

> **Définition** : Un AGENT est un processus cognitif qui s'exécute SUR un nœud. Il est défini par ses objectifs, ses croyances, ses intentions, et son budget.

| Attribut | Description |
|----------|-------------|
| **Goals** | Ce que l'agent cherche à accomplir |
| **Beliefs** | Son modèle du monde (mis à jour par observation) |
| **Intentions** | Son plan d'action actuel |
| **Budget** | Ressources qu'il peut consommer (tokens, compute, time) |
| **Proof** | Historique de ses actions (askesis ledger) |
| **Scope** | Périmètre d'action (fichier, module, crate, système) |

**Caractéristiques** :
- Un agent est **borné logiquement** (budget, scope, compétences)
- Un agent peut **migrer** entre nœuds (état sérialisé, transporté, restauré)
- Un agent peut **déléguer** à des sous-agents (composition fractale)
- Un agent est **responsable** (chaque action est prouvée et attribuée)

**Types d'agents** :

| Type | Rôle | Exemples |
|------|------|----------|
| **Coordinateur** | Orchestre les autres agents | Planification, allocation, arbitrage |
| **Worker** | Exécute des tâches | Build, test, deploy, audit |
| **Reviewer** | Vérifie le travail | Code review, proof verification |
| **Auditor** | Audit la conformité | Constitutional Court, forensics |
| **Meta-agent** | Gère les autres agents | Self-reflection, architecture evolution |

---

### 1.3. RELATION NODE-AGENT

```
┌─────────────────────────────────────────────────────────────┐
│                        NODE (physique)                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Runtime CYNIC                                           │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐              │ │
│  │  │ AGENT A  │  │ AGENT B  │  │ AGENT C  │  ...         │ │
│  │  │ (Coord.) │  │ (Worker) │  │ (Reviewer)│              │ │
│  │  └──────────┘  └──────────┘  └──────────┘              │ │
│  │       │              │              │                   │ │
│  │       └──────────────┴──────────────┘                   │ │
│  │                      │                                  │ │
│  │              cynic-agent-bus                            │ │
│  │                      │                                  │ │
│  │       ┌──────────────┴──────────────┐                   │ │
│  │       ▼                             ▼                   │ │
│  │  Storage (SurrealDB)          CYNIC Net                 │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**Règles** :
1. Un agent s'exécute **sur exactement un nœud** à la fois
2. Un nœud peut héberger **0 à N** agents (selon ressources)
3. Un agent peut **migrer** vers un autre nœud (hand-off protocol)
4. Un agent **ne connaît pas** le matériel sous-jacent (abstraction)
5. Un nœud **ne connaît pas** la logique des agents (sandbox)

---

## 2. LES 15 DIMENSIONS (corrigées)

### 2.1. COGNITIF — Un OS qui pense

| Aspect | Implémentation |
|--------|----------------|
| **Apprentissage** | Fine-tuning continu, skill evolution, feedback loops |
| **Mémoire** | Mémoire de travail (RAM), long terme (SurrealDB), archive (askesis) |
| **Raisonnement** | Multi-modèle (LLM + symbolic + RL + court) |
| **Métacognition** | Le système évalue ses propres décisions et corrige |
| **Perception** | Probes hardware, observations réseau, stimuli externes |

**Correction v3** : La mémoire est **distribuée par nœud**, pas globale. Chaque nœud a sa mémoire locale ; la synchronisation est épidémique (gossip).

---

### 2.2. AGENTIQUE — Un OS d'agents

| Aspect | Implémentation |
|--------|----------------|
| **Autonomie** | Agents GOI (Goals, Observations, Intentions) |
| **Coordination** | Claims, proofs, handoffs via cynic-agent-bus |
| **Hiérarchie** | Pyramidale (span of control 3-7), failover par élection |
| **Auto-organisation** | Émergence par interaction, pas de contrôleur central |

**Correction v3** : Les agents sont **explicitement distincts des nœuds**. Un agent est un processus migratoire ; un nœud est un substrat fixe.

---

### 2.3. SOUVERAIN — Un OS qui s'autogouverne

| Aspect | Implémentation |
|--------|----------------|
| **Autogouvernance** | Constitutional Court (5 pouvoirs) |
| **Identité** | ed25519 + BLAKE3 node_id, propre à chaque nœud |
| **Preuve** | Proof-of-History (SHA-256 chaîné), askesis ledger |
| **Indépendance** | Aucun provider unique — multi-backend, fallback |

**Correction v3** : Le Constituant Humain n'a plus de **veto absolu**. Le veto est **temporaire** (72h) et doit être **justifié par preuve**. Après 72h sans preuve, le veto expire.

---

### 2.4. AGNOSTIQUE — Un OS sans dépendance

| Aspect | Implémentation |
|--------|----------------|
| **Hardware** | x86_64, ARM64, RISC-V |
| **Model** | Tout LLM (local, API, open-weight, fine-tuned) |
| **Provider** | OpenAI, Anthropic, Mistral, souverain local |
| **Protocol** | REST, gRPC, WebSocket, MCP |
| **Cloud** | Bare-metal, Proxmox, AWS, GCP, edge |

**Correction v3** : L'agnosticisme est **stratégique**, pas absolu. On privilégie les standards ouverts, mais on accepte les dépendances **tant qu'elles sont abstraites par des traits**.

---

### 2.5. PORTABLE — Un OS qui s'exécute partout

| Cible | Contraintes | Stratégie |
|-------|-------------|-----------|
| **Bare-metal** | x86_64 serveur | Installation directe, systemd |
| **VM** | Proxmox, QEMU, cloud VMs | Image QCOW2, cloud-init |
| **Cloud** | AWS, GCP, Azure | Terraform, Docker, Kubernetes |
| **Solana Seeker** | ARM64, <15W, batterie | Build ARM64, Q4/Q5, Tor→CYNIC Net |

**Correction v3** : Le Seeker est un **nœud Edge** — il exécute un runtime CYNIC **allégé** (pas de validation, juste observation + agents locaux).

---

### 2.6. BLOCKCHAIN-NATIVE — Un OS avec primitives décentralisées

| Primitive | Implémentation |
|-----------|----------------|
| **Bitcoin** | Unité de compte, timestamping, multi-sig |
| **Lightning** | Micropayments entre agents/nœuds |
| **CYNIC Net** | Transport onion-routed natif (remplace Tor) |
| **PoH** | Proof-of-History interne, causalité, consensus |
| **Merkle proofs** | Vérification d'intégrité (askesis) |

**Correction v3** : **Tor est remplacé par CYNIC Net** — protocole natif avec incitations BTC/LN, pas de directory authorities, consensus PoH pour le routage.

---

### 2.7. GOUVERNEMENT COMPUTATIONNEL — 5 pouvoirs exécutables

```
┌─────────────────────────────────────────────────────────────┐
│  POUVOIR CONSTITUANT (Humain + Preuve)                      │
│  ────────────────────────────────────                       │
│  • Détient les valeurs fondamentales                        │
│  • Veto temporaire (72h) avec preuve obligatoire            │
│  • Peut modifier la constitution (multisig 3-of-5)          │
├─────────────────────────────────────────────────────────────┤
│  POUVOIR EXÉCUTIF (Agent Bus + Coordinator)                 │
│  ──────────────────────────────────────────                 │
│  • Exécute les tâches selon les plans validés               │
│  • Gère les ressources (budget, span of control)            │
│  • Répond aux mandats du Judiciaire                         │
├─────────────────────────────────────────────────────────────┤
│  POUVOIR LÉGISLATIF (Policy Engine)                         │
│  ──────────────────────────────────                         │
│  • Définit les règles de fonctionnement                     │
│  • Met à jour les contracts                                 │
│  • Adapte aux nouveaux contextes                            │
├─────────────────────────────────────────────────────────────┤
│  POUVOIR JUDICIAIRE (Constitutional Court)                  │
│  ────────────────────────────────────────                   │
│  • Audit la conformité                                      │
│  • Émet des mandats de correction                           │
│  • Peut SUSPENDRE un agent/nœud (pas juste proposer)       │
│  • Le mandat est exécuté par l'Exécutif sous 24h            │
├─────────────────────────────────────────────────────────────┤
│  POUVOIR DE CONTRÔLE (Telemetry + Audit)                    │
│  ───────────────────────────────────────                    │
│  • Observe en continu                                       │
│  • Preuve de convergence/divergence                         │
│  • Alerte en cas d'anomalie                                 │
└─────────────────────────────────────────────────────────────┘
```

**Correction v3** : Le Judiciaire peut **suspendre** (pas juste proposer). L'Exécutif doit exécuter sous 24h. Le Constituant a un veto **temporaire et justifié**.

---

### 2.8. SCALING VERTICAL — Évolution protocolaire

| Stage | Nom | Nœuds | Protocole | Consensus | Innovation |
|-------|-----|-------|-----------|-----------|------------|
| **S1** | SEED | 1 | UDS + mmap | Single-authority | Zéro réseau |
| **S2** | SPROUT | 1-3 agents | TCP+TLS loopback | Kernel arbitration | Multi-agent |
| **S3** | SAPLING | 3-7 | QUIC/WireGuard | PBFT (3f+1) | Premier BFT |
| **S4** | TREE | 7-50 | QUIC priority | HotStuff (linear) | Cross-cluster |
| **S5** | FOREST | 50+ | CYNIC Net + datagrams | Tower BFT + PoH | Finalité économique |

**Règles** :
- Chaque stage est un **nouveau protocole** (pas "plus de nœuds")
- **Protocol strangling** : l'ancien protocole est supprimé, pas maintenu
- Chaque couche doit être **stress-testée** avant d'ajouter la suivante
- Pas de saut : on ne peut pas passer de S1 à S3 sans stabiliser S2

**Correction v3** : Le scaling est **protocolaire**, pas juste "plus de nœuds". Chaque stage émerge un nouveau protocole réseau.

---

### 2.9. AUTOPOIÉTIQUE — Un OS qui s'auto-répare

| Aspect | Implémentation |
|--------|----------------|
| **Auto-diagnostic** | Forensics engine, health probes, anomaly detection |
| **Auto-réparation** | Remediation, retry, fallback, graceful degradation |
| **Auto-scaling** | Ajustement dynamique des ressources |
| **Auto-mise à jour** | Release pipeline, rolling updates, rollback |

**Correction v3** : L'auto-réparation est **bornée**. Si le système ne peut pas se réparer seul en < 5 min, il **escalade vers l'humain** avec un diagnostic complet.

---

### 2.10. ÉVOLUTIF — Un OS qui fait évoluer son architecture

| Aspect | Implémentation |
|--------|----------------|
| **Mutation** | Les agents proposent des changements |
| **Sélection** | La Court valide/rejecte les mutations |
| **Héritage** | Les mutations réussies sont propagées |
| **Extinction** | Les branches mortes sont taillées (pattern-gc) |

**Correction v3** : Les mutations sont **progressives** (1 paramètre à la fois), pas globales. Chaque mutation est **isolée** et **réversible**.

---

### 2.11. ÉCONOMIQUE — Un OS avec économie de ressources

| Aspect | Implémentation |
|--------|----------------|
| **Budget** | Chaque agent a un budget (temps, tokens, compute) |
| **Allocation** | Le coordinateur alloue selon priorité + valeur |
| **Récompense** | Les agents producteurs reçoivent des crédits (BTC/LN) |
| **Pénalité** | Les agents dysfonctionnels perdent des crédits |
| **Réserve** | Fonds de réserve pour bear market (pas buyback aveugle) |

**Correction v3** : **Fonds de réserve obligatoire** (20% des revenus). Pas de buyback/burn si la réserve < 6 mois de fonctionnement.

---

### 2.12. EPISTÉMIQUE — Un OS avec vérité comme substrat

| Aspect | Implémentation |
|--------|----------------|
| **Preuve** | Chaque action produit une preuve (askesis, SHA-256) |
| **Vérifiabilité** | Toute affirmation est vérifiable |
| **Falsifiabilité** | Les preuves peuvent être invalidées par contre-preuve |
| **Consensus** | Les nœuds s'accordent via PoH |

**Correction v3** : La falsifiabilité est **implémentée** — chaque preuve a une fonction `verify()` et `falsify()`.

---

### 2.13. TEMPOREL — Un OS avec causalité comme primitive

| Aspect | Implémentation |
|--------|----------------|
| **PoH interne** | Chaque événement est horodaté et chaîné |
| **Causalité** | Les causes précèdent les effets (vérifiable) |
| **Consensus temporel** | Les nœuds s'accordent sur l'ordre |
| **Immabilité** | L'historique est append-only |

**Correction v3** : Le PoH est **local à chaque nœud**, synchronisé par consensus (pas de horloge globale).

---

### 2.14. DISTRIBUÉ — Un OS qui est un essaim

| Aspect | Implémentation |
|--------|----------------|
| **Pas de SPOF** | Multisig 3-of-5, pas un seul humain |
| **Consensus** | Tower BFT / temporal consensus |
| **Partition tolerance** | Tolérance aux partitions (CAP) |
| **Gossip** | Propagation épidémique |

**Correction v3** : Le multisig est **distribué** (3-of-5 avec des personnes indépendantes), pas Jean seul.

---

### 2.15. COMPOSABLE — Un OS fractal

| Aspect | Implémentation |
|--------|----------------|
| **Agents composites** | Un agent contient des sous-agents (récursif) |
| **Structures** | Pyramides, matrices, réseaux |
| **Interfaces** | Chaque agent expose des ports (traits) |
| **Réutilisation** | Les mêmes briques composent tout |

**Correction v3** : La composition est **bornée** (max 7 sous-agents par agent, loi de Miller).

---

## 3. STRESS TESTING — Principe de Solana

> **Règle** : Chaque couche doit être **prouvée et stress-testée** avant d'ajouter la suivante.

| Layer | Stress test criteria | Success metric |
|-------|---------------------|----------------|
| **Storage** | 1M writes, 10M reads | < 10ms p99 latency |
| **Agent Bus** | 1000 claims/sec | < 100ms resolution |
| **CYNIC Net** | 10MB/s onion-routed | < 500ms e2e latency |
| **Consensus** | 100 nodes, 33% Byzantine | Safety + Liveness |
| **Inference** | 100 concurrent requests | < 2s response |
| **Self-healing** | Kill 50% of nodes | Recovery < 30s |

**Règle de Solana** : Une couche n'est **jamais** considérée comme stable tant qu'elle n'a pas subi une **panne réelle** (testnet ou mainnet).

---

## 4. DÉFINITIONS LÉGALES (corrigées)

**Article 1** : CYNIC est un système d'exploitation agentique, souverain, agnostique et portable.

**Article 2** : Un **NŒUD** est une entité matérielle/virtuelle avec identité cryptographique et ressources physiques.

**Article 3** : Un **AGENT** est un processus cognitif avec objectifs, croyances, intentions, et budget, qui s'exécute sur un nœud.

**Article 4** : Les agents sont **distincts** des nœuds — un agent migre, un nœud persiste.

**Article 5** : CYNIC s'exécute sur baremetal, VM, cloud, et appareils portables (Solana Seeker).

**Article 6** : CYNIC maintient son intégrité via BTC, Lightning, CYNIC Net, et PoH.

**Article 7** : CYNIC est gouverné par 5 pouvoirs séparés, chacun ayant un pouvoir d'exécution.

**Article 8** : Le Judiciaire peut **suspendre** un agent/nœud ; l'Exécutif exécute sous 24h.

**Article 9** : Le Constituant a un veto **temporaire (72h)** avec preuve obligatoire.

**Article 10** : CYNIC évolue par **protocol strangling** — chaque stage est un nouveau protocole.

**Article 11** : Chaque couche doit être **stress-testée** avant d'ajouter la suivante.

**Article 12** : CYNIC maintient une **réserve économique** (6 mois minimum).

**Article 13** : Le multisig est **distribué** (3-of-5), pas un seul individu.

**Article 14** : La composition est **bornée** (max 7 sous-agents, loi de Miller).

---

## 5. ARCHITECTURE CIBLE — Vue d'ensemble

```
┌─────────────────────────────────────────────────────────────────────┐
│                          CYNIC OS                                    │
│  ═══════════════════════════════════════════════════════════════    │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    GOUVERNEMENT COMPUTATIONNEL                 │  │
│  │  Constituant │ Exécutif │ Législatif │ Judiciaire │ Contrôle  │  │
│  │  (Humain)    │ (Bus)    │ (Policy)   │ (Court)    │ (Telemetry)│ │
│  └───────────────────────────────────────────────────────────────┘  │
│                                   │                                 │
│  ┌────────────────────────────────▼───────────────────────────────┐  │
│  │                      COUCHE COGNITIVE                          │  │
│  │  Mémoire distribuée │ Raisonnement multi-modèle │ Métacognition│  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                   │                                 │
│  ┌────────────────────────────────▼───────────────────────────────┐  │
│  │                      COUCHE AGENTIQUE                          │  │
│  │  Agents GOI │ Claims │ Proofs │ Handoffs │ Pyramid Fleet      │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                   │                                 │
│  ┌────────────────────────────────▼───────────────────────────────┐  │
│  │                      COUCHE RÉSEAU                             │  │
│  │  CYNIC Net (onion) │ PoH Consensus │ BTC/LN │ NAT Traversal   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                   │                                 │
│  ┌────────────────────────────────▼───────────────────────────────┐  │
│  │                      COUCHE NŒUDS                              │  │
│  │  Validator │ Relay │ Observer │ Bootstrap │ Edge (Seeker)      │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                   │                                 │
│  ┌────────────────────────────────▼───────────────────────────────┐  │
│  │                      COUCHE STORAGE                            │  │
│  │  SurrealDB │ Askesis Ledger │ Vector Store │ Backup            │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 6. CYNIC NET — Protocole natif (résumé)

| Layer | Mechanism | Incentive | Consensus |
|-------|-----------|-----------|-----------|
| **Onion** | Sphinx 1366B, 5 hops | HTLC BTC/LN | PoH epoch (≥2/3) |
| **Transport** | QUIC + pluggable | Fee per packet | Stake-weighted QoS |
| **Discovery** | Kademlia DHT | BTC timelock stake | Signed descriptors |
| **NAT** | DCUtR + relay | Free first 100MB | — |

**Remplace** : Tor (directory authorities, pas d'incitation, vulnérable timing attacks).

---

## 7. PROTOCOLE ÉVOLUTION (résumé)

| Stage | Topology | Consensus | Transport | Strangling |
|-------|----------|-----------|-----------|------------|
| **SEED** | 1 node | Single-authority | UDS + mmap | — |
| **SPROUT** | 1-3 agents | Kernel arbitration | TCP+TLS | UDS supprimé |
| **SAPLING** | 3-7 nodes | PBFT | QUIC/WireGuard | TCP supprimé |
| **TREE** | 7-50 nodes | HotStuff | QUIC priority | PBFT supprimé |
| **FOREST** | 50+ nodes | Tower BFT + PoH | CYNIC Net | QUIC supprimé |

---

*Constitution CYNIC v3. Validée par Solana history, adversarial review, CYNIC Net design, protocol evolution.*
