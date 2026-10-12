# Roadmap Technique CYNIC : Refonte en Profondeur (FCIS / DDD)

**Date d'entrée en vigueur :** 12 Octobre 2026  
**Auteur :** Hermes Agent & Titouan Corneloup (zeyxx)  
**Objectif :** Éradication de la dette technique, conformité stricte aux principes FCIS/DDD, amincissement radical du CLI, standardisation réseau L3/L4/L7, souveraineté cloud et persistance multi-modèle.

---

## Sommaire des Chantiers

1. [Chantier 1 : Refonte Radicale de `cynic-cli` (FCIS / DDD & Thin Surface)](#chantier-1--refonte-radicale-de-cynic-cli)
2. [Chantier 2 : Réseau Souverain (QUIC/HTTP/3 L7 & Mesh WireGuard L3)](#chantier-2--réseau-souverain-quichttp3-l7--mesh-wireguard-l3)
3. [Chantier 3 : Cloud Computing Privé Proxmox & Runners Hybrides](#chantier-3--cloud-computing-privé-proxmox--runners-hybrides)
4. [Chantier 4 : Data System, Triade Épistémique & Télémétrie](#chantier-4--data-system-triade-épistémique--télémétrie)

---

## Chantier 1 : Refonte Radicale de `cynic-cli`

### Diagnostic factuel
- **Volume actuel :** 29 936 LOC réparties sur 149 fichiers (seuil god-object : 5 000 LOC / 30 modules).
- **Nombre de commandes racine :** 24 commandes non structurées (seuil constitutionnel : 7 familles max).
- **Violations FCIS/DDD :**
  - Use cases complets codés dans le CLI (`node_surface/network_surfaces_uc.rs` : 1 386 LOC).
  - Orchestration de compilation et paquets dans le CLI (`build_surface.rs` : 1 280 LOC).
  - Gestion Git, worktrees et PR Forgejo dans le CLI (`source_surface.rs` : 1 166 LOC).
  - Moteur de coordination agentique dans le CLI (`agent_surface/coordination.rs` : 1 037 LOC).
  - Moteur de chat, sessions et inférence dans le CLI (`chat_surface/runtime.rs` : 852 LOC).
  - Fourre-tout de développement (`runtime_surface.rs` et sous-modules : ~4 000 LOC).
  - 19 appels `std::process::Command::new`, 6 clients HTTP `reqwest`, 41 écritures directes `fs::write`.

### Objectif d'arrivée
- Volume cible : < 4 000 LOC réparties sur des façades minces (< 80 LOC par `mod.rs`).
- ZÉRO logique métier dans `cynic-cli` : uniquement parseur clap, appel de ports vers les crates de domaine, et projection (`render_text` ou `emit_json`).
- Convergence stricte vers les **7 familles racine canoniques** :
  1. `status` (absorbe `doctor`, `models`, `errors`, `version`)
  2. `agent` (absorbe `session`, `task`, `mask`, `chat`)
  3. `node` (absorbe les sondes hôtes : `cpu`, `memory`, `disk`, `capacity`, `harness`, `privilege`, `forensics`)
  4. `storage` (absorbe `ping`, `query`, `topology`, `credentials`, `capabilities` via `cynic-surreal`)
  5. `network` (absorbe `mesh`, `mesh coordination`, `peer`, `discovery`, `federation`)
  6. `source` (absorbe `git`, `vcs`, `pr`, `worktree`, `deploy`, `judge`, `dependency`, `update`)
  7. `build` & `audit` (absorbe `court`, `secrets`, `conformance`, `crates`, `proofs`)

### Plan d'exécution par tranches
- [x] **Tranche 1.1 — Expulsion des use cases réseau hors de la surface nœud :** (Complété 2026-10-12)
  - `network_surfaces_uc.rs` (1 386 LOC) expulsé de `node_surface` et reclassé sous `network_surface/execution.rs`.
  - Suppression de la sous-commande doublon `cynic node network`.
- [x] **Tranche 1.4 — Création de `cynic storage` et migration `node host` :** (Complété 2026-10-12)
  - Création de la famille canonique racine `cynic storage` (`ping`, `query`, `capabilities`, `credential`) câblée sur `cynic-surreal`.
  - Migration des sondes matérielles hôtes vers `cynic node host {cpu, memory, disk, capacity}`.
  - Migration des audits de conformance, metrics et harness vers `cynic audit {conformance, metrics, harness}`.
  - Déploiement du binaire release sur la flotte (hash `944753f2...`).
- [ ] **Tranche 1.2 — Expulsion de la gestion de sources et VCS vers `cynic-vcs` :**
  - Déplacer la logique Git/PR de `source_surface.rs` vers `cynic-vcs`.
- [ ] **Tranche 1.3 — Expulsion de l'orchestration de build vers `cynic-build` :**
  - Déplacer la logique de `build_surface.rs` vers `cynic-build`.
- [ ] **Tranche 1.5 — Finalisation du cap des 7 racines et nettoyage des façades :**
  - Réduction finale des LOC restantes sous 4 000.

---

## Chantier 2 : Réseau Souverain (QUIC/HTTP/3 L7 & Mesh WireGuard L3)

### Diagnostic factuel
- Le maillage inter-nœuds repose sur WireGuard (`synapse` 10.10.10.0/24), ce qui garantit un chiffrement L3 robuste.
- Les communications applicatives inter-agents utilisent actuellement TCP (HTTP/1.1 et HTTP/2), exposées au Head-of-Line blocking lors de micro-coupures ou d'instabilités réseau (laptops, runners mobiles, NAT traversal).

### Objectif d'arrivée
- Standardisation de **QUIC (RFC 9000)** et **HTTP/3 (RFC 9114)** pour toutes les communications de service inter-agents CYNIC.
- Multiplexage de flux sans blocage en tête de ligne (HoL).
- Support natif de la migration de connexion (Connection ID) pour les nœuds nomades sans rupture de session TLS 1.3.

### Plan d'exécution par tranches
- [x] **Tranche 2.1 — Intégration de la stack QUIC pure Rust (FCIS) :** (Complété 2026-10-12)
  - Modèle pur de streams multiplexés (RFC 9000 section 19.14) avec ALPN `cynic-agent/1`.
  - Canaux typés : `Control` (bidirectionnel), `Telemetry` (unidirectionnel), `Artifact` (unidirectionnel chunked 64KB).
  - Port asynchrone natif `QuicTransportPort` sans dépendance I/O directe.
  - Validation test unitaire et de frontière `quic_domain_tests` vert sur le builder 10.10.10.119.
- [x] **Tranche 2.2 — Protocole RPC agentique et multiplexage de trames (FCIS) :** (Complété 2026-10-12)
  - Cadrage binaire compact `AgentFrame` (`CYNIC_FRAME_MAGIC` 0x43, version wire 1).
  - Canaux typés : `Control` (commandes de dispatch, leases, heartbeats), `Telemetry` (métriques de charge continue), `Artifact` (chunks découpés avec somme de contrôle BLAKE3 vérifiée).
  - Encodage et décodage purs avec intégrité cryptographique BLAKE3 (`crates/cynic-network/src/quic/framing.rs`).
  - Validation test unitaire et de roundtrip `quic_domain_tests` vert sur le builder 10.10.10.119 (8/8 tests passés).
- [ ] **Tranche 2.3 — Automatisation de l'admission Headscale & CoreDNS :**
  - Intégrer la génération et la révocation dynamique des clés Headscale via le coordinateur `10.10.10.40:8080`.
  - Assurer la résolution DNS dynamique `*.synapse.cynic` sans fichiers hosts statiques.

---

## Chantier 3 : Cloud Computing Privé Proxmox & Runners Hybrides

### Diagnostic factuel
- L'infrastructure on-premise est opérationnelle sur l'hyperviseur Proxmox PVE (`192.168.0.37`) avec conteneurs LXC spécialisés (100, 101, 102, 103, 105).
- L'ancien service orphelin `cynic-cloud-api` a été mis en quarantaine le 2026-10-06.
- Les runners distants (Hetzner, GPU cloud) nécessitent un bootstrap manuel pour rejoindre la flotte.

### Objectif d'arrivée
- Cloud privé souverain 100% piloté en Rust via `cynic-proxmox` et `cynic-lxc`.
- Remplacement légitime de `cynic-cloud-api` par une façade de contrôle intégrée à `cynic-node`.
- Admission zero-touch pour les runners GPU et serveurs de calcul hybrides via WireGuard + pre-auth keys.

### Plan d'exécution par tranches
- [ ] **Tranche 3.1 — Cycle de vie natif des conteneurs LXC :**
  - Industrialiser les profils d'agents sous forme de templates LXC légers avec `cynic-proxmox`.
  - Contrôle dynamique de l'allocation CPU/RAM/Vulkan selon les besoins de charges (`cynic-node`).
- [ ] **Tranche 3.2 — Spécification et déploiement de l'API de contrôle Cloud :**
  - Concevoir l'API de contrôle souveraine remplaçant `cynic-cloud-api` avec authentification Ed25519 et RBAC.
- [ ] **Tranche 3.3 — Fédérateur de calcul hybride (Edge / Cloud) :**
  - Connecter les nœuds de calcul externes (ex: instances GPU RunPod, serveurs Hetzner) au coordinateur Headscale avec isolation stricte.

---

## Chantier 4 : Data System, Triade Épistémique & Télémétrie

### Diagnostic factuel
- SurrealDB 3.1.5 tourne sur le CT 103 (`10.10.10.103:8000`) avec moteur de stockage persistant `surrealkv`.
- La crate pure `cynic-surreal` a été créée selon le patron FCIS/DDD avec séparation étanche (domain Zero-I/O / adapters).
- Le risque d'encombrer SurrealDB avec des séries temporelles numériques haute fréquence a été identifié.

### Objectif d'arrivée
- Cloisonnement strict des responsabilités de données :
  1. **SurrealDB (`cynic-surreal`)** : Persistance des sessions agentiques (`data_system`), mémoire épistémique (Triade : Faits, Preuves, Réfutations), graphe relationnel inter-agents.
  2. **Prometheus / OpenMetrics (`cynic-host`)** : Métriques numériques, séries temporelles de performance matérielle et temps d'inférence.
  3. **Solana / Git Forgejo** : Ancrage immuable des preuves de déploiement et des consensus.

### Plan d'exécution par tranches
- [ ] **Tranche 4.1 — Projection des sessions agentiques (`data_system`) :**
  - Brancher le cycle de vie des sessions agents (démarrage, tours de parole, terminaison, artefacts) sur `cynic-surreal`.
- [ ] **Tranche 4.2 — Triade Épistémique et Graphe de Connaissances :**
  - Implémenter les tables de faits, preuves et réfutations dans SurrealDB avec validation déterministe.
- [ ] **Tranche 4.3 — Exportateur Prometheus natif :**
  - Exposer un endpoint HTTP `/metrics` standard pour la collecte des métriques matérielles et du runtime sans surcharger SurrealDB.

---

## Règles d'Exécution & Invariants
1. **Build Gate SSOT :** Toutes les compilations et vérifications s'exécutent sur le builder distant `10.10.10.119`. Aucune compilation locale sur `cortex`.
2. **Isolation Git :** Chaque tranche s'exécute dans un worktree dédié `agent/hermes/<nom-tranche>`, merge `--no-ff` sur `main` après `make check` vert, puis push immédiat.
3. **100% Rust, Zéro Script Shell :** Tout le code de déploiement et d'orchestration reste en Rust pur.
