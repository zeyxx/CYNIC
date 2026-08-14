# Spécification d'Ingénierie Système : Fédération Réseau P2P Synapse & Synchronisation Merkle

**Status :** Normatif & Contractuel (Layer 2 Network Mesh Protocol — voir `contracts/foundation.json`).

---

## I. WHERE (Où vit ce sous-système ?)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. TOPOLOGIE & ENTITÉS P2P (crates/cynic-node/src/domain/mesh.rs)           │
│    • SynapseMeshTopology, PeerIdentity, SynapseP2PMessage, SynapsePeerState │
│    • Schéma : cynic.synapse_p2p.v1                                          │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. FÉDÉRATION RUNTIME (crates/cynic-node/src/domain/runtime_federation.rs)  │
│    • File d'attente hors-ligne & synchronisation à la reconnexion.          │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. REGISTRE RÉSEAU CANONIQUE (infra/registry.json)                          │
│    • Plan Synapse canonique (CIDR 10.10.10.0/24, réservations IP).          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## II. WHAT (Que fait ce système ? Invariants & Messages)

### 1. Protocole de Messages P2P Chiffrés
* **`Handshake`** : Échange d'identité et de version de protocole (`cynic.synapse_p2p.v1`).
* **`LeaseAnnouncement`** : Diffusion d'un nouveau bail de coordination acquis par un agent local vers tous les pairs du réseau mesh.
* **`MerkleRootBroadcast`** : Propagation de la racine de réputation 7D de l'époque $E$.
* **`Heartbeat`** : Maintien de liveness sans charge CPU excessive.

### 2. Invariants d'Ingénierie
1. **Isolation par Défaut** : Les échanges inter-nœuds transitent exclusivement dans le tunnel WireGuard chiffré (`synapse0`). Tout trafic non chiffré est bloqué par `cynic-firewall`.
2. **Tolérance aux Partitions Réseau** : Si un nœud est temporairement déconnecté, les messages sortants sont mis en mémoire tampon (`FederationQueue`) et rejoués dès le rétablissement de la connectivité.

---

## III. HOW (Comment s'exécute la synchronisation de deux nœuds ?)

```text
 Nœud Alpha (Hôte Proxmox)                                  Nœud Beta (Runner GPU)
       │                                                              │
       ├────────────── 1. WireGuard Handshake (10.10.10.101) ────────►│
       │◄───────────── 2. Handshake ACK (10.10.10.102) ──────────────┤
       │                                                              │
       ├────────────── 3. LeaseAnnouncement (cynic-inference) ────────►│ (Beta note que
       │                                                              │  cynic-inference
       │                                                              │  est occupé)
       │                                                              │
       │◄───────────── 4. MerkleRootBroadcast (Epoch 42, 0xABC...) ──┤ (Alpha met à jour
       │                                                              │  son état d'âme)
```

---

## IV. WHY (Pourquoi cette architecture ? Justification & Arbitrages)

1. **Pourquoi WireGuard P2P plutôt qu'un serveur de coordination centralisé ?**
   * *Risque prévenu* : Élimine le point de défaillance unique (*Single Point of Failure*). Même si un nœud tombe, les autres nœuds continuent de se coordonner localement.
2. **Pourquoi propager les baux sur le réseau ?**
   * Permet à un agent exécuté sur le Nœud Alpha (ex: Cortex) et à un agent de build sur le Nœud Beta (ex: Runner GPU) de respecter mutuellement leurs zones sans conflit.
