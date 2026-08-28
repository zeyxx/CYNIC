# Mapping CYNIC — Plan d'Isolation Agentique (vs Existant)

**Date**: 2026-08-27  
**Contexte**: Cross-reference entre le plan d'isolation (`docs/design/cynic-agent-isolation-plan.md`) et le code CYNIC réel  
**Objectif**: Identifier ce qui existe (réutilisable), les gaps (à créer), et le wiring manquant (à brancher)

---

## 1. Cartographie par Couche

### L0 — Contrats & Types Purs (`cynic-protocol`)

| Entité | Statut | Fichier | Notes |
|--------|--------|---------|-------|
| `OneShotCapability` | ✅ Existe | `cynic-protocol/src/one_shot_capability.rs` | Schema, signature ed25519, TTL, single-use. Prêt à être wiré. |
| `OneShotCapabilityConsumption` | ✅ Existe | `cynic-protocol/src/one_shot_capability.rs` | Preuve de consommation (anti-rejeu). |
| `AgentCapabilityGrant` | ✅ Existe | `cynic-protocol/src/agent_recursion.rs` | Champ `mutates: bool` — distinction read/write. |
| `AgentSessionRequest` | ⚠️ Partiel | `cynic-protocol/src/agent_session/types.ts` | Manque le champ `isolation: AgentIsolationProfile`. |
| `AgentBoundedAction` | ⚠️ Partiel | `cynic-protocol/src/agent_workload/types.ts` | `mutates: bool` existe, manque `one_shot_capability: Option<OneShotCapability>`. |
| `AgentCoordinationContinuity` | ✅ Existe | `cynic-protocol/src/agent_coordination_continuity.rs` | `retained_capabilities` vs `suspended_capabilities` (fail-closed). |
| `AgentIsolationProfile` | ❌ Manquant | — | Nouveau type à créer. |
| `cynic.agent_cot_stream_event.v1` | ❌ Manquant | — | Schéma événementiel CoT à créer. |

**Gaps L0** :
1. Créer `AgentIsolationProfile` (sandbox strategy, netns config, UDS path)
2. Étendre `AgentSessionRequest` avec `isolation`
3. Étendre `AgentBoundedAction` avec `one_shot_capability`
4. Créer le schéma `cynic.agent_cot_stream_event.v1`

---

### L1 — Substrat & Confinement Physique (`cynic-host`)

| Entité | Statut | Fichier | Notes |
|--------|--------|---------|-------|
| `lxc_exec_bridge` | ✅ Existe | `cynic-host/src/lxc_exec_bridge/` | LXC unprivileged via `pct exec`. Modèle pour workers longue durée. |
| `LxcBridgeAuthority` | ✅ Existe | `cynic-host/src/lxc_exec_bridge/types.rs` | Authority classes pour LXC. |
| `BreakglassConsumptionProof` | ✅ Existe | `cynic-host/src/lxc_exec_bridge/incident_authority.rs` | Preuve d'exécution incident. Réutilisable pour audit. |
| `InferenceSupervisorKind` | ✅ Existe | `cynic-inference/src/adapters/local/process_supervisor.rs` | `Process` / `SystemdUser` / `Docker` — dispatch supervisor. |
| `inference_gateway.rs` | ✅ Existe | `cynic-inference/src/application/inference_gateway.rs` | Gateway HTTP SSE. Doit recevoir le CoT tap. |
| `stream_metrics.rs` | ✅ Existe | `cynic-inference/src/domain/matrix_sweep/stream_metrics.rs` | Métriques token-level. Réutilisable pour CoT monitoring. |
| `bwrap` / bubblewrap adapter | ❌ Manquant | — | Nouveau : adaptateur pour agents éphémères locaux. |
| `UDS socket` (unix socket) | ❌ Manquant | — | Nouveau : socket local pour communication agent → inference runtime. |
| `Read-Only RootFS` setup | ❌ Manquant | — | Nouveau : montage bind RO pour binaires système dans bwrap/LXC. |
| `tmpfs` workspace | ❌ Manquant | — | Nouveau : workspace temporaire volatile par agent. |

**Gaps L1** :
1. Créer l'adaptateur bubblewrap (profile éphémère)
2. Créer le serveur UDS (`/run/cynic/inference.sock`)
3. Setup rootfs read-only + tmpfs workspace dans le pont d'exécution

---

### L2 — Réseau & Mesh de Nœud (`cynic-node` / `cynic-host`)

| Entité | Statut | Fichier | Notes |
|--------|--------|---------|-------|
| `runtime_network_preflight.rs` | ✅ Existe | `cynic-host/src/runtime_network_preflight.rs` | Vérifie la connectivité réseau. |
| `fleet_network_observer.rs` | ✅ Existe | `cynic-host/src/fleet_network_observer.rs` | Observation du mesh. |
| `local_network.rs` | ✅ Existe | `cynic-host/src/adapters/local_network.rs` | Adaptateur réseau local. |
| `remote_network.rs` | ✅ Existe | `cynic-host/src/adapters/remote_network.rs` | Adaptateur réseau distant. |
| `wireguard_readback.rs` | ✅ Existe | `cynic-host/src/wireguard_readback.rs` | Read-back WireGuard (mesh Synapse). |
| `peer_resolver_mesh.rs` | ✅ Existe | `cynic-host/src/peer_resolver_mesh.rs` | Résolution mesh. |
| `netns` provisionnement | ❌ Manquant | — | Nouveau : création automatique `ip netns add agent-<uuid>`. |
| `nftables` default-deny | ❌ Manquant | — | Nouveau : règles par-netns (drop tout sauf UDS). |
| Proxy egress capability-aware | ❌ Manquant | — | Nouveau : proxy HTTP local avec header `X-CYNIC-Capability-Token`. |
| `CapabilityEgressRoute` | ❌ Manquant | — | Nouveau : type pour routage egress validé par capability. |

**Gaps L2** :
1. Provisionnement netns automatique par session agentique
2. Règles nftables default-deny par netns
3. Proxy HTTP local d'inspection (capability-aware)
4. Type `CapabilityEgressRoute` dans cynic-protocol

---

### L3 — Mémoire & Vérification (`cynic-kernel`)

| Entité | Statut | Fichier | Notes |
|--------|--------|---------|-------|
| `kernel_proof_persistence.rs` | ✅ Existe | `cynic-host/src/kernel_proof_persistence.rs` | Persistance des preuves kernel. |
| `secret_material.rs` | ✅ Existe | `cynic-host/src/secret_material.rs` | Gestion des secrets (pas dans Git/logs). |
| `registry/validate.rs` | ✅ Existe | `cynic-host/src/registry/validate.rs` | Validation du registry. |
| `registry/persist.rs` | ✅ Existe | `cynic-host/src/registry/persist.rs` | Persistance du registry. |
| Registre admission ed25519 | ❌ Manquant | — | Nouveau : store d'admission pour OneShotCapability. |
| Anti-rejeu capability | ❌ Manquant | — | Nouveau : vérification unicité + consommation atomique. |
| `InferenceStatusProof` wiring | ⚠️ Partiel | `cynic-protocol/src/runtime/proofs.rs` | Schema existe, pas wiré au monitoring actif. |

**Gaps L3** :
1. Créer le registre d'admission ed25519 (store capabilities émises + consommées)
2. Wirer `InferenceStatusProof` au monitoring actif (pas juste schema)

---

### L4 — Exposition Agent & Gateway (`cynic-cli` / `cynic-inference`)

| Entité | Statut | Fichier | Notes |
|--------|--------|---------|-------|
| `inference_surface.rs` | ✅ Existe | `cynic-cli/src/runtime_surface/inference_surface.rs` | CLI `inference-start`, `status`, `vault`. |
| `local_runtime.rs` | ✅ Existe | `cynic-cli/src/runtime_surface/inference_surface/local_runtime.rs` | `execute_inference_start()` avec `--yes` gate. |
| `chat_service.rs` | ✅ Existe | `cynic-inference/src/application/chat_service.rs` | Streaming SSE. |
| `inference_dispatch.rs` | ✅ Existe | `cynic-cli/src/runtime_surface/inference_dispatch.rs` | Dispatch inference. |
| `--yes` capability token | ❌ Manquant | — | Nouveau : `--yes` sans token = refus. |
| `--capability-token <hex>` | ❌ Manquant | — | Nouveau : argv pour passer le token. |
| CoT middleware SSE | ❌ Manquant | — | Nouveau : intercepteur de tokens `<thinking>`. |
| Détection anomalie CoT | ❌ Manquant | — | Nouveau : pattern matching + risk score. |
| Kill switch auto | ❌ Manquant | — | Nouveau : SIGTERM agent sur anomalie. |
| Rate limiting | ❌ Manquant | — | Nouveau : token bucket par (appelant, action). |
| Action UUID | ❌ Manquant | — | Nouveau : identifiant unique propagé dans tout le flux. |
| Rapport signé | ❌ Manquant | — | Nouveau : `LocalRuntimeMutationReport` signé ed25519. |

**Gaps L4** :
1. Ajouter `--capability-token` argv + vérification avant exécution
2. Créer le middleware CoT (intercepteur SSE)
3. Créer le détecteur d'anomalies (pattern + score)
4. Créer le kill switch automatique
5. Ajouter rate limiting + UUID action + signature rapport

---

### L5 — Gouvernance & Alignement (`cynic-askesis` / `cynic-judge`)

| Entité | Statut | Fichier | Notes |
|--------|--------|---------|-------|
| `foundation.json` | ✅ Existe | `contracts/foundation.json` | Invariants constitutionnels. |
| `kyon-scenarios-v0.jsonl` | ✅ Existe | `cynic-askesis/datasets/kyon-scenarios-v0.jsonl` | Scénarios d'alignement (anger, coercive control, etc.). |
| `agent_loop.rs` | ✅ Existe | `cynic-askesis/src/agent_loop.rs` | Boucle agentique avec `harness kind, authority, isolation, target scope`. |
| Task admission policy | ⚠️ Partiel | `cynic-inference/src/domain/task_admission_port.rs` | Port existe, pas de policy d'impossibilité. |
| Reward model alignment | ❌ Manquant | — | Nouveau : scoring comportemental (pas juste output quality). |
| Token budget enforcement | ❌ Manquant | — | Nouveau : limite de reasoning tokens par tâche. |

**Gaps L5** :
1. Enforcer la task admission policy (rejeter les tâches impossibles)
2. Créer un reward model d'alignement (behavioral scoring)
3. Ajouter token budget enforcement

---

## 2. Gaps Transversaux (Wiring Manquant)

| Gap | Source | Cible | Description |
|-----|--------|-------|-------------|
| W1 | `OneShotCapability` (L0) | `--yes` gate (L4) | Wirer la vérification ed25519 au CLI |
| W2 | `AgentSessionRequest.isolation` (L0) | `lxc_exec_bridge` / `bwrap` (L1) | Créer le sandbox selon le profil |
| W3 | `netns` (L2) | `UDS socket` (L1) | Bind socket dans la netns agent |
| W4 | Proxy egress (L2) | `OneShotCapability` (L3) | Valider le header HTTP avec le registre |
| W5 | CoT stream (L4) | `InferenceStatusProof` (L3) | Générer preuve sur anomalie détectée |
| W6 | `AgentBoundedAction` (L0) | Kill switch (L4) | Déclencher SIGTERM sur anomalie |
| W7 | `stream_metrics.rs` (L1) | CoT monitoring (L4) | Corréler métriques + raisonnement |
| W8 | `InferenceSupervisorKind` (L1) | `bwrap` adapter (L1) | Ajouter `Bubblewrap` au dispatch |

---

## 3. Plan d'Action Bottom-Up (Revisé)

### Phase 1 — Contrats (L0)

| # | Action | Fichier cible | Dépendance |
|---|--------|---------------|------------|
| 1.1 | Créer `AgentIsolationProfile` | `cynic-protocol/src/agent_isolation.rs` | — |
| 1.2 | Étendre `AgentSessionRequest` | `cynic-protocol/src/agent_session/types.rs` | 1.1 |
| 1.3 | Étendre `AgentBoundedAction` | `cynic-protocol/src/agent_workload/types.rs` | — |
| 1.4 | Créer `cynic.agent_cot_stream_event.v1` | `cynic-protocol/src/agent_cot_stream.rs` | — |
| 1.5 | Créer `CapabilityEgressRoute` | `cynic-protocol/src/agent_isolation.rs` | 1.1 |

### Phase 2 — Runtime Confinement (L1)

| # | Action | Fichier cible | Dépendance |
|---|--------|---------------|------------|
| 2.1 | Créer adaptateur bubblewrap | `cynic-host/src/bwrap_bridge/` | 1.1 |
| 2.2 | Créer serveur UDS | `cynic-host/src/uds_server.rs` | — |
| 2.3 | Setup rootfs RO + tmpfs | `cynic-host/src/bwrap_bridge/` | 2.1 |
| 2.4 | Étendre `InferenceSupervisorKind` | `cynic-inference/src/adapters/local/process_supervisor.rs` | 2.1 |

### Phase 3 — Réseau (L2)

| # | Action | Fichier cible | Dépendance |
|---|--------|---------------|------------|
| 3.1 | Provisionnement netns auto | `cynic-host/src/netns_provisioner.rs` | 1.1 |
| 3.2 | Règles nftables default-deny | `cynic-host/src/netns_provisioner.rs` | 3.1 |
| 3.3 | Proxy HTTP capability-aware | `cynic-host/src/egress_proxy.rs` | 1.5 |
| 3.4 | Injection `HTTP_PROXY` dans netns | `cynic-host/src/netns_provisioner.rs` | 3.1, 3.3 |

### Phase 4 — Vérification (L3)

| # | Action | Fichier cible | Dépendance |
|---|--------|---------------|------------|
| 4.1 | Créer registre admission ed25519 | `cynic-kernel/src/capability_registry.rs` | 1.3 |
| 4.2 | Anti-rejeu capability | `cynic-kernel/src/capability_registry.rs` | 4.1 |
| 4.3 | Wire `InferenceStatusProof` | `cynic-kernel/src/monitoring.rs` | 4.1 |

### Phase 5 — Gateway & CLI (L4)

| # | Action | Fichier cible | Dépendance |
|---|--------|---------------|------------|
| 5.1 | Ajouter `--capability-token` argv | `cynic-cli/src/runtime_surface/inference_surface/local_runtime.rs` | 4.1 |
| 5.2 | Vérification token avant exécution | `cynic-cli/src/runtime_surface/inference_surface/local_runtime.rs` | 5.1 |
| 5.3 | Créer middleware CoT | `cynic-inference/src/application/cot_middleware.rs` | 1.4 |
| 5.4 | Créer détecteur anomalie | `cynic-inference/src/application/anomaly_detector.rs` | 5.3 |
| 5.5 | Créer kill switch | `cynic-host/src/kill_switch.rs` | 5.4 |
| 5.6 | Ajouter rate limiting | `cynic-cli/src/rate_limiter.rs` | — |
| 5.7 | Ajouter UUID action + signature rapport | `cynic-inference/src/application/local_runtime_service.rs` | — |

### Phase 6 — Alignement (L5)

| # | Action | Fichier cible | Dépendance |
|---|--------|---------------|------------|
| 6.1 | Enforcer task admission | `cynic-inference/src/domain/task_admission_port.rs` | — |
| 6.2 | Créer reward model | `cynic-askesis/src/reward_model.rs` | — |
| 6.3 | Token budget enforcement | `cynic-inference/src/application/inference_gateway.rs` | — |

---

## 4. Résumé des Priorités

| Priorité | Action | Couche | Impact |
|----------|--------|--------|--------|
| **P0** | Créer `AgentIsolationProfile` + étendre `AgentSessionRequest` | L0 | Fondation de tout le plan |
| **P0** | Étendre `AgentBoundedAction` avec `OneShotCapability` | L0 | Capability binding |
| **P0** | Ajouter `--capability-token` + vérification | L0→L4 | Gate immédiat |
| **P0** | Créer adaptateur bubblewrap | L1 | Isolation agents éphémères |
| **P1** | Provisionnement netns + nftables | L2 | Isolation réseau |
| **P1** | Registre admission ed25519 + anti-rejeu | L3 | Vérification capability |
| **P1** | Middleware CoT + détecteur anomalie | L4 | Monitoring temps réel |
| **P2** | Proxy egress capability-aware | L2 | Sorties contrôlées |
| **P2** | Kill switch automatique | L4 | Réponse anomalie |
| **P2** | Task admission policy | L5 | Anti-reward-hacking |
| **P3** | Reward model + token budget | L5 | Alignement long terme |

---

## 5. Ce qui est Réutilisable (Ne pas réinventer)

| Composant | Fichier | Usage dans le plan |
|-----------|---------|-------------------|
| `OneShotCapability` | `cynic-protocol/src/one_shot_capability.rs` | Capability token pour `--yes` |
| `AgentCapabilityGrant` | `cynic-protocol/src/agent_recursion.rs` | Scope binding (mutates flag) |
| `lxc_exec_bridge` | `cynic-host/src/lxc_exec_bridge/` | Workers longue durée |
| `BreakglassConsumptionProof` | `cynic-host/src/lxc_exec_bridge/incident_authority.rs` | Audit trail |
| `InferenceSupervisorKind` | `cynic-inference/src/adapters/local/process_supervisor.rs` | Dispatch sandbox |
| `stream_metrics.rs` | `cynic-inference/src/domain/matrix_sweep/stream_metrics.rs` | CoT monitoring |
| `runtime_network_preflight.rs` | `cynic-host/src/runtime_network_preflight.rs` | Vérif réseau |
| `fleet_network_observer.rs` | `cynic-host/src/fleet_network_observer.rs` | Observation mesh |
| `kyon-scenarios-v0.jsonl` | `cynic-askesis/datasets/` | Scénarios d'alignement |
| `agent_loop.rs` | `cynic-askesis/src/agent_loop.rs` | Harness kind + isolation + target scope |

---

**Prochaine étape** : Validation HITL de ce mapping, puis engagement Phase 1.1 (création `AgentIsolationProfile`).
