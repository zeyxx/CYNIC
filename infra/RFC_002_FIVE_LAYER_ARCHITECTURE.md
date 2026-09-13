# RFC-002: L'Architecture Décentralisée en 5 Couches de CYNIC
## De l'Ancre Matérielle (L0) au Marché Souverain d'Intents Multi-Agents (L4)

**Statut** : Normatif / Architecture de Référence  
**Date** : 13 Septembre 2026  
**Auteurs** : Titouan Corneloup (zeyxx), Hermes Agent  
**Invariants Référents** : `contracts/foundation.json`, `infra/runtime-authority.json`  
**Sources Théoriques & Fondations Externes** :
- *Hexagonal Architecture (Ports & Adapters)* — Alistair Cockburn ([cockburn.us](https://alistair.cockburn.us/hexagonal-architecture/))
- *Bitcoin Lightning Network & HTLC Contracts* — Poon & Dryja (2016)
- *Tower BFT & Verifiable Delay Functions* — Solana Labs / Anza Architecture
- *Multi-Party Virtual State Channels* — Dziembowski et al. (EUROCRYPT 2019)
- *Multi-agent systems and decentralized computing fog* — Ponomarev & Voronkov (arXiv:1702.08529)

---

## 1. Contexte & Le Problème du Trilemme Agentique

Dans un système multi-agents autonome et distribué sur plusieurs machines physiques (Cortex, Builder, Proxmox, Edge), trois contraintes entrent en collision directe :
1. **L'Immutabilité & La Preuve (Auditabilité L1)** : Tout acte doit être vérifiable, signé cryptographiquement et persisté (zéro mensonge, pas de succès simulé).
2. **La Vitesse d'Exécution & Zéro Latence Disque (Fluidité L2)** : Deux agents collaborant sur du code (ex: itérations de compilation/test) ne peuvent pas payer le coût d'une synchronisation disque CAS ou d'un consensus de bloc à chaque micro-délégation.
3. **Le Passage à l'Échelle & L'Économie Énergétique (Routage L3/L4)** : Les capacités physiques (GPU VRAM, cœurs CPU, thermie, bande passante) doivent être allouées dynamiquement sans point unique de défaillance (SPOF) ni serveur maître central.

---

## 2. La Matrice des 5 Couches Fractales

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Layer 4 : MARCHÉ SOUVERAIN & INTENT D'AGENTS                           │
│ - Enchères énergétiques (Joules / Tokens) & Priorisation Mempool       │
│ - Réputation cryptographique & Slashing automatique                    │
│ - Arbitrage de litiges par rejeu déterministe                          │
├────────────────────────────────────────────────────────────────────────┤
│ Layer 3 : ROUTAGE MULTI-CANAUX & TOPOLOGIE DE PISCINE                  │
│ - Sphinx / Onion Routing pour délégations multi-sauts confidentielles  │
│ - Virtual State Channels (Dziembowski et al. 2019)                     │
│ - Agrégation de preuves condensées (Recursive Proof Batching)          │
├────────────────────────────────────────────────────────────────────────┤
│ Layer 2 : CANAUX D'ÉTATS BILATÉRAUX (P2P State Channels)               │
│ - Micro-délégations sub-milliseconde en mémoire vive pure              │
│ - HTLC de Tâches : Délégation contre Preimage de Preuve                │
│ - Baux temporaires (LeaseGuard) et Clés de Révocation Anti-Triche      │
├────────────────────────────────────────────────────────────────────────┤
│ Layer 1 : RÈGLEMENT, CONSENSUS BFT & MAGASIN CAS                       │
│ - Séquenceur d'Époques Monotones (cynic-consensus, Quorum 2/3)         │
│ - Magasin CAS immuable (cynic-storage / SQLite TypedBooks / WAL)       │
│ - Git SSOT (Forgejo souverain) & Clôture de Baux (Settlement)          │
├────────────────────────────────────────────────────────────────────────┤
│ Layer 0 : SUBSTRAT PHYSIQUE & SÉCURITÉ MATÉRIELLE                      │
│ - Maillage WireGuard Zero-Trust (wg0) & Pare-feu nftables pur          │
│ - Paires de clés matérielles Ed25519 (MachineEnrollment)               │
│ - Horloge Monotone Matérielle (CLOCK_MONOTONIC_RAW) anti-timetravel    │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Détail des Couches & Mécanismes Cryptographiques

### Layer 0 : Substrat Physique & Sécurité Bas-Niveau
- **Ancre Cryptographique** : Chaque machine possède une clé asymétrique Ed25519 scellée localement.
- **Filtrage Zero-Trust (`cynic-network`)** : Règles nftables natives atomiques (`policy drop`, `ct state invalid drop`, rate-limiting anti-DoS sur WireGuard 44196).
- **Temps Matériel Inviolable** : Exclusion de `SystemTime` pour les décisions de consensus ; utilisation exclusive de ticks monotones scellés.

### Layer 1 : Le Règlement (Settlement), Consensus & SSOT
- **Inspirations** : Solana Tower BFT / Bitcoin L1.
- **Rôle** : Tribunal de dernière instance et mémoire persistante.
- **Mécanisme** :
  - Un commit Git ou une release binaire n'est proclamée qu'après évaluation du quorum strict 2/3 (`cynic-consensus::evaluate_fleet_consensus`).
  - L'ouverture d'un bail racine (`cynic agent claim`) et sa libération scellée (`cynic agent release`) sont les deux seuls événements persistés sur la L1.

### Layer 2 : Canaux d'États Bilatéraux (State Channels Inter-Agents)
- **Inspirations** : Lightning Network (Poon & Dryja 2016), Virtual Channels.
- **Rôle** : Permettre à des agents (ex: Hermes et le Builder) d'échanger des centaines de micro-actions de test à latence sub-milliseconde.
- **Le Mécanisme Hash Read-Back Contract (HTLC de Calcul)** :
  1. L'Agent A délègue un workload à l'Agent B : `Hash(TaskPayload)`.
  2. L'Agent B exécute le calcul en mémoire ou conteneur isolé.
  3. L'Agent B transmet la preimage de la preuve : `Preimage = SHA256(OutputArtefact + ReadBackMetrics)`.
  4. Le canal bilatéral est mis à jour instantanément sans I/O disque.
  5. En cas de contestation ou de triche, la clé de révocation permet de fermer unilatéralement le canal sur le Layer 1 et de révoquer l'enrôlement du nœud coupable.

### Layer 3 : Routage Multi-Canaux & Réseau de Piscine
- **Inspirations** : Sphinx Onion Routing / Réseaux de canaux virtuels.
- **Rôle** : Permettre à un agent sur Cortex de mobiliser un GPU sur un nœud distant non directement appairé, à travers des nœuds intermédiaires.
- **Confidentialité & Zero-Leakage** : Le nœud intermédiaire valide et achemine le contrat sans avoir accès au prompt ou au code source métier transporté.

### Layer 4 : Le Marché Souverain & Priorisation d'Intents
- **Inspirations** : Mempool Bitcoin, Fee-market EIP-1559 adapté à l'énergie.
- **Rôle** : Arbitrer la rareté des ressources matérielles du cluster.
- **Mécanisme** :
  - Les workloads en attente dans le Mempool sont classés par ratio d'utilité énergétique : $\text{Priorité} = \frac{\text{Budget Alloué}}{\text{Coût Thermique/CPU estimé}}$.
  - Les nœuds de la flotte se déclarent dynamiquement disponibles pour les tâches correspondant exactement à leurs profils matériels observés.

---

## 4. Revue Multi-Prismes & Red-Teaming de l'Architecture

### Prisme 1 : L'Auditeur DevSecOps & Cryptographe
- *Question Critique* : Que se passe-t-il si un nœud intermédiaire dans le Layer 3 altère la tâche ou ment sur les métriques d'exécution ?
- *Garantie CYNIC* : Le contrat de workload porte un digest d'entrée inviolable. Le demandeur vérifie la preuve de sortie par ré-exécution probabiliste ou vérification d'empreinte binaire déterministe. Tout écart entraîne le slashing immédiat du worker au niveau L1.

### Prisme 2 : L'Ingénieur Systèmes Distribués & SRE
- *Question Critique* : Comment éviter le blocage lors d'un partitionnement réseau (Split-Brain) ?
- *Garantie CYNIC* : Règle du Fail-Closed Quorum 2/3. Si un nœud est isolé du reste du cluster, ses canaux L2 se gèlent automatiquement en lecture seule (read-only) après expiration de son bail de 30 secondes. Aucun état dissident ne peut être committé.

### Prisme 3 : L'Économiste de Ressources & HPC (Calcul Haute Performance)
- *Question Critique* : Pourquoi ne pas simplement utiliser un scheduler centralisé classique comme Slurm ou Kubernetes ?
- *Garantie CYNIC* : Slurm et Kubernetes imposent des serveurs maîtres (API Server, etcd) qui constituent des SPOF, des cibles d'attaque majeures et des couches de dépendances écrasantes. Le modèle CYNIC à 5 couches est **100% autonome, peer-to-peer, compilé en Rust statique et sans master**.

---

## 5. Feuille de Route d'Implémentation & Jalons Techniques

1. **Jalon L0/L1 (Actuel - Complété à 95%)** :
   - `cynic-network` (nftables, WireGuard mesh, DNS).
   - `cynic-consensus` (Quorum BFT 2/3, séquenceur d'époques).
   - Admission native HTTP 9090 sur le builder.
2. **Jalon L2 (Prochain Chantier)** :
   - Spécification de `cynic-channel` : Baux en mémoire et validation de preimage de preuve (HTLC de build).
3. **Jalon L3/L4 (Horizon Flotte Étendue)** :
   - Routage de tâches multi-nœuds et ordonnancement de Mempool par budget énergétique.
