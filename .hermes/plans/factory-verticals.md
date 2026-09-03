# CYNIC Factory — Plan de convergence verticale

**Date** : 2026-09-03
**Contexte** : 3 flux parallèles (I1 industrialisation, A1 build_runner, Webhook/Artifact audit) convergent sur le worktree partagé. Risque de conflit. Ce plan aligne les travaux.

---

## Modèle usine

Chaque unité d'usine suit :

```
mandat → workload → claim → lease → workspace → action → artifact → preuve → rétention
```

| Terme | Forme canonique |
|-------|-----------------|
| mandat | typed request (CLI/MCP/webhook) |
| workload | `cynic.agent_workload_envelope.v1` |
| claim | `cynic.agent_claims.v2` (scope, budget, TTL) |
| lease | `cynic.coord_lease.v1` (fencing token, monotonic) |
| workspace | checkout isolé `checkouts/main-<sha>` |
| action | argv typé (`cargo check -p`, `rm -rf`, etc.) |
| artifact | binaire/digest signé |
| preuve | `cynic.build_artefact_report.v1` + read-back |
| rétention | GC corrélé workload/process/claim |

---

## 5 verticales à fermer

### Vertical 1 — Workspace/Target Lifecycle

**Owner** : I1 industrialisation (nous)
**Status** : Partiel (GC plan → queue → proof)

| Capacité | Fichier | Statut |
|----------|---------|--------|
| GC plan | `domain/build_artefact/probe.rs` | ✅ `build_gc_plan()` |
| Queue compose | `domain/queue/probe.rs` | ✅ `build_queue_plan()` |
| CLI surfaces | `cli/build_discovery_surface.rs` | ✅ `discover`, `branch graph`, `queue` |
| Adapters Local/Remote/Fleet | `adapters/local_*` + `fleet_*` | ✅ 9 adapters |
| Protobuf types | `protocol/build_discovery.rs`, `branch_graph.rs`, `queue.rs` | ✅ schemas v1 |
| **MANQUANT** : GC corrélé workload/process | `adapters/fleet_build_artefact_observer.rs` | 🔴 Observer ne corrèle pas les processus actifs |
| **MANQUANT** : Planner PSI | — | 🔴 Pas de choix `CARGO_BUILD_JOBS` selon pression mémoire/CPU |

**Preuve** : `cynic queue --json` retourne `cynic.queue_plan.v1` avec items typés.

---

### Vertical 2 — Git Event Pipeline

**Owner** : A1 build_runner
**Status** : Prêt pour review, non commité

| Capacité | Fichier | Statut |
|----------|---------|--------|
| Git pin au SHA | `build_runner_adapter.rs` | ✅ Read-back SHA |
| Cargo via traducteur | `build_surface.rs` | ✅ `make:check`/`make:build` |
| Preuve sortie | `build_runner_adapter.rs` | ✅ Code de sortie + logs |
| Tests scriptés | `build_runner_adapter.rs` (tests) | ✅ 7 cas |
| **MANQUANT** : Webhook HMAC | `forgejo_webhook_server.rs` | ❌ Pas de vérif signature |
| **MANQUANT** | `forgejo_webhook.rs` | ❌ Pas de secret partagé |

**Preuve** : node lib vert local, fmt propre. CLI à prouver sur builder.

---

### Vertical 3 — BuildRunner

**Owner** : A1 build_runner
**Status** : Partiel (trigger SSH fonctionnel, admission incomplète)

| Capacité | Fichier | Statut |
|----------|---------|--------|
| Trigger SSH | `build_runner_adapter.rs` | ✅ argv typé |
| HMAC sign/verify | `build_runner_adapter.rs` | ✅ `HmacKey` |
| Runner preflight | `build_runner_adapter.rs` | ✅ `RunnerPreflight` |
| **MANQUANT** : Lease guard | `build_workflow.rs` | ❌ `run_build` exige `kernel_endpoint` + `snapshot` |
| **MANQUANT** | `build_surface.rs` | ❌ `run_build` bloque sans guard |
| **MANQUANT** : Source snapshot | `build_surface.rs` | ❌ Aucun snapshot observé |

**Preuve** : trigger SSH émis mais bloqué en admission.

---

### Vertical 4 — Artifact Lifecycle

**Owner** : À assigner
**Status** : Manquant

| Capacité | Fichier | Statut |
|----------|---------|--------|
| Digest lu | `build.rs:303` | ✅ `fs::read` digest |
| **MANQUANT** : Install vers chemin canonique | `build.rs` | ❌ Pas de copy/install |
| **MANQUANT** | `build_surface.rs:46` | ❌ Défaut `target/` |
| **MANQUANT** | `build_surface.rs` | ❌ `Activate` = `build-release` sans install |
| **MANQUANT** : Rollback | — | ❌ Aucun chemin de rollback |
| **MANQUANT** : Registry | `build_surface.rs:438` | ❌ `serve_webhook` ignore `_registry` |

**Preuve** : artifact abandonné dans `target/`, jamais installé.

---

### Vertical 5 — Runtime Deployment

**Owner** : À assigner
**Status** : Manquant

| Capacité | Fichier | Statut |
|----------|---------|--------|
| SHA embarqué (kernel) | `cynic-kernel/build.rs:33` | ✅ `CYNIC_GIT_SHA` |
| **MANQUANT** : SHA embarqué (cli/host) | `cynic-cli/src/main.rs` | ❌ Aucune surface version |
| **MANQUANT** | `cynic-host/src/build.rs` | ❌ Pas de const SHA |
| **MANQUANT** : Gate de fraîcheur | — | ❌ Aucune comparaison SHA local vs distant |
| **MANQUANT** : Rollback runtime | — | ❌ Aucun chemin de rollback |

**Preuve** : `cynic version` inexistant. Seul kernel expose SHA via REST.

---

## Ownership par couche

| Couche | Crate | Responsabilité |
|--------|-------|----------------|
| Types purs | `cynic-protocol` | Schemas v1, invariants, `one_boundary_one_reason` |
| Observation | `cynic-host` | Substrate (disk/CPU/mem/net), processus, filesystem |
| Exécution | `cynic-node` | Workload, claim, lease, scheduler, preuves |
| Coordination | `cynic-kernel` | Claims, leases, transitions, SSOT |
| Projection | `cynic-cli` | Clap → inject adapter → host → JSON, 0 policy |

Règle : aucun dashboard, script, ou journal parallèle ne devient autorité.

---

## Merge order (éviter les conflits)

1. **I1 industrialisation** (base) — déjà push sur main via `ac7e304b`
2. **A1 build_runner** — PR depuis `agent/hermes/buildrunner-closure`
3. **Webhook/Artifact audit** — PR avec 6 manquants comme blockers

Fichiers de conflit potentiels :
- `crates/cynic-cli/src/lib.rs` (tous les 3 modifient)
- `crates/cynic-host/src/build_surface.rs` (A1 + Webhook)
- `crates/cynic-host/src/forgejo_webhook_server.rs` (Webhook audit)

Résolution : rebase contre main courant, re-claim si scope modifié.

---

## Prochaines étapes

1. **I1** : Merger `agent/hermes/host-proto-clean` sur main (PR)
2. **A1** : Commit + push `agent/hermes/buildrunner-closure`, ouvrir PR
3. **Webhook audit** : Ouvrir PR avec les 6 manquants comme blockers explicites
4. **Factory** : Activer lease fencing sur builder (claim avant build, release après proof)
5. **V1 complète** : Workspace/target lifecycle → rétention → GC corrélé workload

---

*Plan mis à jour au fil des merges. Chaque verticale fermée = PR mergée + preuve builder.*
