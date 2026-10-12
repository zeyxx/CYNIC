# CYNIC — Fédération Mycélienne & Architecture RBAC Data-Centric

## 1. Vision & Philosophie : Le Modèle Mycélium

L'architecture CYNIC rompt définitivement avec le modèle hiérarchique centralisé ("Cerveau maître / Kernel god-object").
À la place, elle adopte le modèle **biologique de la ruche et du mycélium** :

1. **Autonomie Local-First** :
   Chaque nœud (`cortex`, `builder`, `agent-host`, `surrealdb-store`) est un organisme complet. Il possède son stockage local (spool / WAL), son attestation binaire locale, son catalogue de capacités et sa boucle d'observation.
   Un nœud continue de fonctionner et de produire des preuves même en situation de coupure réseau totale (*partition tolerance*).

2. **Fédération sans SPOF (Single Point of Failure)** :
   Il n'existe aucun nœud central indispensable à la survie du système. Le "cerveau" de CYNIC est l'état émergent validé par consensus cryptographique (quorum BFT local et règlement Solana).

3. **Filtrage à la Source & Sobriété d'Émission** :
   Un nœud ne diffuse pas son flux brut. Il filtre localement ses observations et n'émet vers le réseau mesh que les synthèses normées nécessaires.

---

## 2. Modèle RBAC & ACL d'Ingénierie Stricte

Pour garantir l'étanchéité absolue entre nœuds, agents et processus, tout flux réseau entrant est soumis à une matrice d'admission stricte.

### 2.1. Niveaux de Rôles de Nœuds (`FleetRole`)

| Rôle Fédéré | Responsabilité | Capacités Admissibles | Périmètre d'Émission Réseau |
|---|---|---|---|
| **`WorkstationHost`** (`cortex`) | Station opérateur & pilotage de conception | Toutes (sous validation HITL) | Requêtes de build, admission de claims |
| **`HypervisorHost`** (`proxmox`) | Orchestration matérielle et conteneurs LXC | `pct-*`, `pvesh` | Rapports de métriques et statut hôte |
| **`BuildRunner`** (`builder`) | Compilation déterministe et `make check` | `cargo-build`, `git-checkout` | Dépôt d'artéfacts vers le store, receipts |
| **`AgentHost`** (`agent-host`) | Exécution étanche des sessions agents | `tmux-*`, `agent-session` | Heartbeats de baux, résumés d'exécution |
| **`ArtifactStore`** (`10.10.10.103`)| Distribution immuable des binaires release | Lecture HTTP publique | Checksums, catalogues de versions |
| **`StoragePrimary`** (`surrealdb`) | Persistance graphe des sessions et claims | Requêtes SurrealQL autorisées | Projections de réputation et historique |

### 2.2. Règles d'ACL & d'Admission Réseau

1. **Règle Zero-Implicit** : Tout paquet non associé à un bail ou une enveloppe cryptographique déclarée est rejeté au niveau L4 (`cynic-firewall` / WireGuard).
2. **Principe de Moindre Exposition** :
   - Un conteneur d'agent (`cynic-agent-host`) n'a aucun droit d'accès direct en écriture sur `cortex` ni sur l'hyperviseur Proxmox.
   - Le builder (`cynic-builder`) n'a de canal d'écriture que vers le répertoire `/data/runtime-releases` de l'Artifact Store (`10.10.10.103`).
   - L'Artifact Store est en **lecture seule stricte** pour les agents consommateurs via HTTP (port 8443).

---

## 3. Dimension Data-Centric : Enveloppes & Contrats

Fini le câblage aveugle en scripts ad-hoc. Chaque interaction inter-nœuds doit être portée par un schéma canonique sérialisé en JSON :

### 3.1. Les Enveloppes Canoniques

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. cynic.node_observation.v1                                                │
│    Attestation locale du nœud (version, SHA-256, RTT, ports ouverts).       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. cynic.coordination_lease.v1                                              │
│    Bail de travail exclusif revendiqué par un agent pour éviter les conflits.│
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. cynic.runtime_artifact_store.v1                                          │
│    Contrat de publication et d'intégrité des binaires release compilés.     │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. cynic.data_system.session_trace.v1                                       │
│    Trace structurée de session agent (normalisée vers SurrealDB).           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2. Intégration de `infra/data_system` vers SurrealDB

Le package `data_system` capture les sessions des différents moteurs d'agents (Hermes, Claude, etc.) dans `/vault/captures/`.
Dans l'architecture mycélienne :
1. La capture brute reste locale au nœud qui exécute l'agent.
2. Le modèle de données canonique extrait les faits, les claims et les commits.
3. Seule la synthèse normée est indexée dans la base SurrealDB partagée (`10.10.10.103:8000`) pour alimenter le graphe de réputation et la traçabilité de la flotte.
