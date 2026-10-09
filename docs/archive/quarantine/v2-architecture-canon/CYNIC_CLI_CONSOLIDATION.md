# CYNIC — CLI Consolidation Audit

> **Version :** 0.1
> **Date :** 2026-09-05
> **Statut :** Audit de l'existant avant consolidation
> **Objectif :** Réduire le CLI de 40+ commandes à ~15 commandes claires

---

## Le problème

```
40+ commandes dans Command enum
  ↓
Beaucoup sont des stubs
Beaucoup se chevauchent
Pas de hiérarchie claire
Le CLI ment sur ses capacités
```

---

## Audit de l'existant

### Commandes racines (dans `Command enum`)

| Commande | Sous-commandes | Exécuteur | Statut | Problème |
|----------|----------------|-----------|--------|----------|
| `Agent` | AgentCommand | execute_agent | ✅ | OK |
| `AgentHost` | AgentHostCommand | (dans agent_host_surface) | ✅ | OK |
| `Audit` | AuditCommand | execute_audit_command | ✅ | OK |
| `Bootstrap` | BootstrapCommand | execute_bootstrap_command | ⚠️ Trop de sous-commandes | À consolider |
| `Branch` | BranchCommand | (dans branch_graph_surface) | ✅ | OK |
| `Build` | BuildCommand | execute_build | ✅ | OK |
| `Chat` | (inline) | execute_chat | ✅ | OK |
| `Commands` | - | execute_commands_manifest | ✅ | OK |
| `Deps` | Option<DepsCommand> | execute_deps_command | ✅ | OK |
| `Dev` | DevCommand | execute_dev_command | ⚠️ Trop de sous-commandes | À consolider |
| `Doctor` | (inline) | doctor::run_doctor | 🆕 | OK |
| `Errors` | (inline) | ❌ Pas de dispatch | ❌ STUB | À implémenter ou supprimer |
| `Federation` | FederationCommand | execute_federation | ✅ | OK |
| `Fleet` | FleetCommand | execute_fleet_command | ✅ | OK |
| `Forensics` | ForensicsCommand | execute_forensics | ✅ | OK |
| `Git` | GitCommand | execute_git_command | ✅ | OK |
| `Graph` | (inline) | ❌ Pas de dispatch | ❌ STUB | À supprimer |
| `Integrations` | IntegrationsCommand | execute_integrations_command | ✅ | OK |
| `Judge` | JudgeCommand | execute_judge | ✅ | OK |
| `Judgment` | JudgmentCommand | execute_judgment | ✅ | OK |
| `Lab` | LabCommand | execute_lab_command | ⚠️ Peu utilisé | À fusionner avec Runtime |
| `Lifecycle` | LifecycleCommand | execute_lifecycle | ✅ | OK |
| `Mask` | MaskCommand | ❌ Pas de dispatch | ❌ STUB | À supprimer |
| `Memory` | MemoryCommand | execute_memory | ✅ | OK |
| `Models` | OperatorModelsCommand | execute_operator_models | ✅ | OK |
| `Network` | NetworkCommand | execute_network | ✅ | OK |
| `NetworkAudit` | NetworkAuditCommand | execute_network_audit | ✅ | Redondant avec Audit ? |
| `NodeActivation` | NodeActivationCommand | execute_node_activation | ✅ | OK |
| `Peer` | PeerCommand | execute_peer | ✅ | OK |
| `Queue` | (inline) | execute_queue | ✅ | OK |
| `Runtime` | RuntimeCommand | execute_runtime_command | ⚠️ Trop de sous-commandes | À consolider |
| `Session` | SessionCommand | execute_session | ✅ | OK |
| `Solana` | SolanaCommand | execute_solana_command | ✅ | OK |
| `Source` | SourceCommand | execute_source | ✅ | OK |
| `Status` | (inline) | execute_deps | ⚠️ Redondant avec Deps | À fusionner |
| `Task` | TaskCommand | execute_task | ✅ | OK |
| `Transport` | TransportCommand | execute_transport_command | ⚠️ Peu utilisé | À fusionner avec Network |
| `Update` | (inline) | execute_update | ✅ | OK |
| `Upgrade` | - | ❌ Pas de dispatch | ❌ STUB | À supprimer |
| `Version` | (inline) | version_surface::run | ✅ | OK |

---

## Les stubs à supprimer

| Commande | Raison |
|----------|--------|
| `Errors` | Pas d'exécuteur — existe dans `operator_errors.rs` ? |
| `Graph` | Pas d'exécuteur — redondant avec Branch/Graph |
| `Mask` | Pas d'exécuteur — existe dans `mask_surface.rs` ? |
| `Upgrade` | Pas d'exécuteur — redondant avec Deps/Upgrade |

---

## Les fusions proposées

### 1. Lab → Runtime

`cynic lab` n'a que 3 sous-commandes (plan, provision, bundle). Toutes sont liées au runtime.

**Après :** `cynic runtime lab <subcommand>`

### 2. NetworkAudit → Audit

`cynic network-audit` est juste un type d'audit. Pas besoin d'une commande racine dédiée.

**Après :** `cynic audit network`

### 3. Transport → Network

Transport = réseau de transport (SSH, mesh). C'est un sous-ensemble de Network.

**Après :** `cynic network transport <subcommand>`

### 4. Status → Deps

`cynic status` affiche l'état des dépendances. C'est `cynic deps`.

**Après :** Supprimer `Status`, utiliser `cynic deps`

---

## La hiérarchie cible

```
cynic
├── agent          # Gestion des agents (claim, heartbeat, release)
├── audit          # Audits (crate, kernel, ownership, surface, network)
├── bootstrap      # Bootstrap (chain, host, framework)
├── build          # Build (run, discover, prove)
├── chat           # Chat (TUI, one-shot, export)
├── deps           # Dépendances (status, plan, audit, upgrade)
├── dev            # Dev (surfaces, framework, doctor, debt, opportunities)
├── errors         # Codes d'erreur (expliquer, remédiation)
├── fleet          # Flotte (report, discover, inventory)
├── forensics      # Forensics (evidence, timeline, post-mortem)
├── git            # Git (topology, hygiene, reconcile)
├── integrations   # Intégrations (status, plan, adopt, connect)
├── judge          # Jugement (patch, verdict)
├── lab            # Lab (plan, provision, bundle) — OU FUSIONNER AVEC RUNTIME
├── lifecycle      # Lifecycle (evidence, status, plan, gate, promote)
├── models         # Modèles (list, status, start, stop)
├── network        # Réseau (chain, runtime, proof, transport)
├── peer           # Peer (status, inspect, submit-tx)
├── queue          # File d'attente (plan, run, status)
├── runtime        # Runtime (paths, registry, validate, suite, plan)
├── session        # Session (list, export, delete)
├── solana         # Solana (plan, probe)
├── source         # Source (inventory, reconcile, status)
├── status         # ← SUPPRIMER (redondant avec deps)
├── task           # Tâches (list, status, cancel)
├── update         # Mise à jour (composants)
└── version        # Version
```

**Après consolidation :** ~25 commandes racines (au lieu de 40+)

---

## La hiérarchie cible (consolidée)

```
cynic
├── agent          # Agent management
├── audit          # All audits (unified)
├── bootstrap      # Bootstrap
├── build          # Build pipeline
├── chat           # Chat + TUI
├── deps           # Dependencies (status, plan, audit, upgrade)
├── dev            # Developer tools (surfaces, framework, doctor, debt)
├── errors         # Error codes
├── fleet          # Fleet observation
├── forensics      # Forensics
├── git            # Git supervision
├── integrations   # External integrations
├── judge          # Patch judgment
├── lifecycle      # Update lifecycle
├── models         # LLM model management
├── network        # Network + transport + peer
├── queue          # Task queue
├── runtime        # Runtime surfaces + lab
├── session        # Session management
├── solana         # Solana observation
├── source         # Source supervision
├── task           # Background tasks
└── version        # Version info
```

**Après consolidation :** ~20 commandes racines

---

## Plan d'action

### Phase 1 : Supprimer les stubs (1 jour)

- [ ] Vérifier si `Errors` existe dans `operator_errors.rs` → implémenter ou supprimer
- [ ] Vérifier si `Mask` existe dans `mask_surface.rs` → implémenter ou supprimer
- [ ] Supprimer `Graph` (redondant avec `Branch`)
- [ ] Supprimer `Upgrade` (redondant avec `Deps`)
- [ ] Supprimer `Status` (redondant avec `Deps`)

### Phase 2 : Fusionner les redondances (2-3 jours)

- [ ] Fusionner `Lab` → `Runtime` (ou supprimer Lab)
- [ ] Fusionner `NetworkAudit` → `Audit`
- [ ] Fusionner `Transport` → `Network`

### Phase 3 : Consolider les sous-commandes (1 semaine)

- [ ] `Dev` : réduire de 20+ sous-commandes à ~10
- [ ] `Runtime` : réduire de 15+ sous-commandes à ~8
- [ ] `Bootstrap` : réduire de 10+ sous-commandes à ~5

### Phase 4 : Refaire le `command_manifest.rs` (1 jour)

- [ ] Synchroniser le manifest avec la réalité
- [ ] Supprimer les commandes qui n'existent pas
- [ ] Ajouter les nouvelles commandes fusionnées

---

## Critères de décision

| Critère | Action |
|---------|--------|
| Pas d'exécuteur | Supprimer |
| Redondant avec une autre commande | Fusionner |
| < 3 sous-commandes | Fusionner dans la commande parente |
| Usage rare | Cacher (`hide = true`) ou supprimer |
| Trop de sous-commandes (> 10) | Scinder en sous-domaines |

---

*Ce document est la feuille de route pour la consolidation du CLI. Chaque phase doit être exécutée séparément avec des tests de non-régression.*
