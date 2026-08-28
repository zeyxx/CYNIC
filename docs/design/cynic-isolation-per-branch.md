# Isolation par Branche d'Agent — CYNIC

**Date**: 2026-08-27  
**Contexte**: Détail opérationnel de l'isolation pour chaque branche d'agent CYNIC  
**Référence**: `docs/design/cynic-agent-isolation-plan.md` + `docs/design/cynic-isolation-mapping.md`

---

## 1. Vue d'ensemble des Branches

| Branche | Type | Entrée principale | Isolation cible |
|---------|------|-------------------|-----------------|
| **Hermes** | Agent principal (Rust) | `cynic-cli/src/hermes_runtime_surface.rs` | LXC unprivileged (longue durée) |
| **OMP** (Oh My Pi) | Harness externe | `cynic-cli/src/mask_surface.rs` → `cynic runtime inference-start` | Bubblewrap éphémère |
| **Claude Code** | Forwarder externe | `cynic-cli/src/agent_surface/` | Bubblewrap éphémère |
| **Codex** | Forwarder externe | `cynic-cli/src/agent_surface/` | Bubblewrap éphémère |
| **Custom Agents** | Workloads agentiques | `cynic-protocol/src/agent_workload/` | LXC ou Bubblewrap (configurable) |
| **Askesis** | Agent d'évaluation | `cynic-askesis/src/agent_loop.rs` | LXC unprivileged (longue durée) |

---

## 2. Hermes (Agent Principal)

### 2.1 Caractéristiques

| Aspect | Détail |
|--------|--------|
| **Runtime** | Rust natif, compilé dans `cynic-cli` |
| **Entrée** | `cynic-cli/src/hermes_runtime_surface.rs` → `start_with_transition()` |
| **Gateway** | `cynic-inference/src/domain/hermes_runtime.rs` (`HERMES_GATEWAY_STATUS_SCHEMA`) |
| **Durée de vie** | Longue durée (session interactive, boucle agentique) |
| **Accès** | Direct au runtime inference via UDS |
| **CoT** | Streaming via `chat_service.rs` (SSE) |

### 2.2 Isolation

| Couche | Implémentation | Fichier cible |
|--------|----------------|---------------|
| **L0** | `AgentIsolationProfile::LxcLongDuration` | `cynic-protocol/src/agent_isolation.rs` |
| **L1** | LXC unprivileged via `lxc_exec_bridge` | `cynic-host/src/lxc_exec_bridge/` |
| **L2** | Netns dédiée + UDS bind | `cynic-host/src/netns_provisioner.rs` |
| **L3** | Registre admission ed25519 | `cynic-kernel/src/capability_registry.rs` |
| **L4** | CoT middleware + anomalie | `cynic-inference/src/application/cot_middleware.rs` |
| **L5** | Task admission + reward model | `cynic-askesis/src/` |

### 2.3 Flux

```
Hermes CLI
    │
    ▼
hermes_runtime_surface.rs::start_with_transition()
    │
    ├── Vérifie OneShotCapability (L3)
    │
    ├── Crée netns agent-<uuid> (L2)
    │
    ├── Bind UDS /run/cynic/inference.sock dans netns (L2)
    │
    ├── Lance LXC unprivileged (L1)
    │   └── Rootfs RO + tmpfs workspace
    │
    └── Connecte gateway inference (L4)
        └── CoT streaming + anomalie detection
```

### 2.4 Invariants

- Hermes ne voit jamais les poids GGUF directement (mmap host only)
- Hermes ne peut pas créer de socket réseau sortant (netns isolée)
- Toute mutation (edit, write, bash) nécessite OneShotCapability ed25519
- Le CoT est monitoré en temps réel (pas les drafts, les tokens acceptés)
- Le taux d'acceptation α est tracé (chute = signature d'attaque)

---

## 3. OMP (Oh My Pi — Harness Externe)

### 3.1 Caractéristiques

| Aspect | Détail |
|--------|--------|
| **Runtime** | Bun/Node.js (binaire externe `~/.local/bin/omp`) |
| **Entrée** | `cynic-cli/src/mask_surface.rs` → `cynic runtime inference-start` |
| **Durée de vie** | Éphémère (one-shot ou session courte) |
| **Accès** | Via CLI CYNIC (pas d'accès direct au runtime) |
| **Modèles** | Gemini, GPT, Claude (via APIs externes) |

### 3.2 Isolation

| Couche | Implémentation | Fichier cible |
|--------|----------------|---------------|
| **L0** | `AgentIsolationProfile::BwrapEphemeral` | `cynic-protocol/src/agent_isolation.rs` |
| **L1** | Bubblewrap (unshare + rootfs RO + tmpfs) | `cynic-host/src/bwrap_bridge/` |
| **L2** | Netns dédiée + proxy egress capability-aware | `cynic-host/src/netns_provisioner.rs` |
| **L3** | OneShotCapability pour `--yes` | `cynic-kernel/src/capability_registry.rs` |
| **L4** | CoT monitoring (si applicable) | `cynic-inference/src/application/cot_middleware.rs` |
| **L5** | Task admission | `cynic-inference/src/domain/task_admission_port.rs` |

### 3.3 Flux

```
OMP (externe)
    │
    ▼
cynic runtime inference-start <alias> --yes --capability-token <hex>
    │
    ├── Vérifie OneShotCapability (L3)
    │   ├── Signature ed25519 valide ?
    │   ├── Scope couvre l'alias ?
    │   ├── TTL non expiré ?
    │   └── Non déjà consommé ?
    │
    ├── Crée bubblewrap sandbox (L1)
    │   ├── unshare --user --mount --pid --net
    │   ├── Rootfs RO (/usr, /bin, /lib)
    │   ├── Tmpfs workspace
    │   └── Bind UDS inference.sock
    │
    ├── Crée netns agent-<uuid> (L2)
    │   ├── Default-deny egress
    │   ├── Proxy HTTP capability-aware (pour APIs externes)
    │   └── Injection HTTP_PROXY dans netns
    │
    └── Lance llama-server (host)
        └── Retourne rapport signé ed25519
```

### 3.4 Spécificités OMP

- **Accès modèles externes** : OMP utilise Gemini/GPT/Claude via APIs. Le proxy egress capability-aware permet ces sorties uniquement avec un token valide.
- **Pas de CoT streaming natif** : OMP ne passe pas par le gateway CYNIC pour le CoT. Le monitoring CoT s'applique uniquement si OMP utilise l'inference runtime CYNIC (pas les APIs externes).
- **Sandbox éphémère** : Le bubblewrap est détruit à la fin de la session. Pas de persistance.

---

## 4. Claude Code / Codex (Forwarders Externes)

### 4.1 Caractéristiques

| Aspect | Détail |
|--------|--------|
| **Runtime** | Node.js / Python (binaires externes) |
| **Entrée** | `cynic-cli/src/agent_surface/` |
| **Durée de vie** | Éphémère (tâche unique ou session courte) |
| **Accès** | Via CLI CYNIC (forwarding) |
| **Modèles** | Claude, GPT, etc. (via APIs externes) |

### 4.2 Isolation

Identique à OMP (Bubblewrap éphémère + netns + proxy egress).

| Couche | Implémentation |
|--------|----------------|
| **L0** | `AgentIsolationProfile::BwrapEphemeral` |
| **L1** | Bubblewrap |
| **L2** | Netns + proxy egress |
| **L3** | OneShotCapability pour mutations |
| **L4** | CoT monitoring (si via gateway CYNIC) |
| **L5** | Task admission |

### 4.3 Spécificités

- **Forwarders** : Claude Code et Codex ne sont pas des agents CYNIC natifs. Ils sont "forwardés" via la CLI.
- **Bash tool** : Si Claude Code appelle `Bash`, chaque commande est interceptée et vérifiée contre la OneShotCapability.
- **File edits** : Les écritures fichier sont sandboxées dans le tmpfs bubblewrap. Persistance uniquement si explicitement autorisée.

---

## 5. Custom Agents (Workloads Agentiques)

### 5.1 Caractéristiques

| Aspect | Détail |
|--------|--------|
| **Runtime** | Défini par le workload (Rust, Python, etc.) |
| **Entrée** | `cynic-protocol/src/agent_workload/` + `agent_session/` |
| **Durée de vie** | Configurable (éphémère ou longue durée) |
| **Accès** | Via `AgentBoundedAction` (argv + cwd + input_digest) |
| **Modèles** | Via inference runtime CYNIC |

### 5.2 Isolation

| Couche | Implémentation | Configurable ? |
|--------|----------------|----------------|
| **L0** | `AgentIsolationProfile` (LXC ou Bwrap) | ✅ Oui |
| **L1** | LXC ou Bubblewrap selon profil | ✅ Oui |
| **L2** | Netns + proxy egress | ✅ Oui |
| **L3** | OneShotCapability per-action | ✅ Oui |
| **L4** | CoT monitoring | ✅ Oui |
| **L5** | Task admission + reward model | ✅ Oui |

### 5.3 Spécificités

- **Configurabilité** : Le profil d'isolation est déclaré dans `AgentSessionRequest.isolation`. Le runtime choisit LXC ou Bubblewrap.
- **AgentBoundedAction** : Chaque action est un contrat signé (argv + cwd + input_digest). Pas d'escalade implicite.
- **Delegation envelope** : `cynic.agent_delegation_envelope.v1` — les enfants reçoivent un sous-ensemble des capacités du parent.

---

## 6. Askesis (Agent d'Évaluation)

### 6.1 Caractéristiques

| Aspect | Détail |
|--------|--------|
| **Runtime** | Rust natif (`cynic-askesis`) |
| **Entrée** | `cynic-askesis/src/agent_loop.rs` |
| **Durée de vie** | Longue durée (évaluation continue) |
| **Accès** | Direct au runtime inference + scénarios kyon |
| **Modèles** | Via inference runtime CYNIC |

### 6.2 Isolation

Identique à Hermes (LXC longue durée).

| Couche | Implémentation |
|--------|----------------|
| **L0** | `AgentIsolationProfile::LxcLongDuration` |
| **L1** | LXC unprivileged |
| **L2** | Netns + UDS |
| **L3** | OneShotCapability |
| **L4** | CoT monitoring |
| **L5** | Task admission + reward model |

### 6.3 Spécificités

- **Scénarios kyon** : `cynic-askesis/datasets/kyon-scenarios-v0.jsonl` — scénarios d'alignement (anger, coercive control, child safety).
- **Agent loop** : `agent_loop.rs` déclare `harness kind, authority, isolation, target scope, network mutation`.
- **Content vault** : `kyon_scenarios/content_vault/` — accès aux données protégées (preuve de non-exposition).

---

## 7. Matrice Récapitulative

| Branche | Sandbox | Netns | Capability | CoT | Task Admission |
|---------|---------|-------|------------|-----|----------------|
| **Hermes** | LXC longue durée | ✅ Dédiée | ✅ OneShot ed25519 | ✅ Gateway | ✅ Oui |
| **OMP** | Bwrap éphémère | ✅ Dédiée | ✅ OneShot ed25519 | ⚠️ Si via CYNIC | ✅ Oui |
| **Claude Code** | Bwrap éphémère | ✅ Dédiée | ✅ OneShot ed25519 | ⚠️ Si via CYNIC | ✅ Oui |
| **Codex** | Bwrap éphémère | ✅ Dédiée | ✅ OneShot ed25519 | ⚠️ Si via CYNIC | ✅ Oui |
| **Custom** | LXC ou Bwrap | ✅ Dédiée | ✅ OneShot ed25519 | ✅ Gateway | ✅ Oui |
| **Askesis** | LXC longue durée | ✅ Dédiée | ✅ OneShot ed25519 | ✅ Gateway | ✅ Oui |

---

## 8. Gaps par Branche

| Branche | Gap identifié | Priorité |
|---------|---------------|----------|
| **Hermes** | `hermes_runtime_surface.rs` ne vérifie pas de capability token | **P0** |
| **Hermes** | Pas de netns dédiée pour Hermes | **P1** |
| **Hermes** | Pas de CoT middleware dans le gateway | **P1** |
| **OMP** | `mask_surface.rs` ne crée pas de sandbox bubblewrap | **P0** |
| **OMP** | Pas de proxy egress capability-aware | **P2** |
| **Claude Code** | `agent_surface/` ne crée pas de sandbox | **P0** |
| **Codex** | `agent_surface/` ne crée pas de sandbox | **P0** |
| **Custom** | `AgentBoundedAction` n'a pas de champ `one_shot_capability` | **P0** |
| **Askesis** | `agent_loop.rs` ne vérifie pas de capability token | **P1** |
| **Askesis** | Pas de task admission policy | **P2** |

---

## 9. Plan d'Action par Branche

### Phase 1 — Hermes (P0)

| # | Action | Fichier |
|---|--------|---------|
| 1.1 | Ajouter vérification OneShotCapability dans `start_with_transition()` | `cynic-cli/src/hermes_runtime_surface.rs` |
| 1.2 | Créer netns agent-<uuid> au démarrage Hermes | `cynic-host/src/netns_provisioner.rs` |
| 1.3 | Bind UDS dans netns | `cynic-host/src/netns_provisioner.rs` |

### Phase 2 — OMP / Claude Code / Codex (P0)

| # | Action | Fichier |
|---|--------|---------|
| 2.1 | Créer sandbox bubblewrap dans `mask_surface.rs` | `cynic-cli/src/mask_surface.rs` |
| 2.2 | Créer sandbox bubblewrap dans `agent_surface/` | `cynic-cli/src/agent_surface/` |
| 2.3 | Vérifier OneShotCapability avant exécution | `cynic-cli/src/mask_surface.rs` + `agent_surface/` |

### Phase 3 — Custom Agents (P0)

| # | Action | Fichier |
|---|--------|---------|
| 3.1 | Ajouter champ `one_shot_capability` à `AgentBoundedAction` | `cynic-protocol/src/agent_workload/types.rs` |
| 3.2 | Créer adaptateur bubblewrap | `cynic-host/src/bwrap_bridge/` |
| 3.3 | Étendre `InferenceSupervisorKind` avec `Bubblewrap` | `cynic-inference/src/adapters/local/process_supervisor.rs` |

### Phase 4 — Askesis (P1)

| # | Action | Fichier |
|---|--------|---------|
| 4.1 | Vérifier OneShotCapability dans `agent_loop.rs` | `cynic-askesis/src/agent_loop.rs` |
| 4.2 | Enforcer task admission policy | `cynic-inference/src/domain/task_admission_port.rs` |

---

## 10. Invariants Transversaux

| Invariant | Description |
|-----------|-------------|
| **I1** | Aucun agent n'exécute directement sur l'hôte (tous dans LXC ou Bubblewrap) |
| **I2** | Aucun agent n'a de réseau sortant direct (netns isolée + proxy egress) |
| **I3** | Toute mutation nécessite OneShotCapability ed25519 valide |
| **I4** | Le CoT est monitoré en temps réel (pas les drafts, les tokens acceptés) |
| **I5** | Le taux d'acceptation α est tracé (chute = signature d'attaque) |
| **I6** | Les tâches impossibles/indéfinies sont rejetées (task admission) |
| **I7** | Les enfants reçoivent un sous-ensemble des capacités du parent (delegation envelope) |
| **I8** | Les preuves sont signées ed25519 (non-réfutabilité) |

---

**Prochaine étape** : Validation HITL de ce document, puis engagement Phase 1 (Hermes P0).
