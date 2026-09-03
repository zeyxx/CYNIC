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
**Status** : Fermé 2026-09-03 (branche agent/hermes/host-proto-clean)

| Capacité | Fichier | Statut |
|----------|---------|--------|
| Git pin au SHA | `build_runner_adapter.rs` | ✅ Read-back SHA (`fa898703`) |
| Cargo via traducteur | `build_runner_adapter.rs` | ✅ `cargo:check`/`cargo:build` + preuve sortie |
| Tests scriptés | `build_runner_adapter.rs` (tests) | ✅ 9 cas (`e64ded5d`, `35ca17a7`) |
| Webhook HMAC | `forgejo_webhook_server.rs` | ✅ Vérif si secret + passthrough (`10cb24b1`) |
| Secret partagé | `forgejo_webhook.rs` + CLI | ✅ Env `CYNIC_WEBHOOK_SECRET`, jamais en flag |

**Preuve** : trigger live vert sur builder au SHA apparié, preuve JSON au commit exact.

---

### Vertical 3 — BuildRunner

**Owner** : A1 build_runner
**Status** : Trigger fermé 2026-09-03 ; admission restante

| Capacité | Fichier | Statut |
|----------|---------|--------|
| Trigger SSH | `build_runner_adapter.rs` | ✅ argv typé, sans script (`fa898703`) |
| SHA distant observé | `build_runner.rs` | ✅ `binary_sha` + parse strict (`35ca17a7`) |
| **MANQUANT** : Lease guard | `build_workflow.rs` | ❌ `run_build` exige `kernel_endpoint` + `snapshot` |
| **MANQUANT** : Source snapshot | `build_surface.rs` | ❌ Aucun snapshot observé |

**Preuve** : trigger live vert. Admission (bail + enregistrement) = prochaine slice.

---

### Vertical 4 — Artifact Lifecycle

**Owner** : A1 build_runner (fermé 2026-09-03)
**Status** : Fermé sauf registre

| Capacité | Fichier | Statut |
|----------|---------|--------|
| Digest lu | `build.rs:303` | ✅ `fs::read` digest |
| Install vers chemin canonique | `build.rs` | ✅ `install_artifact` + flag `--install-path` (`b8ebdd53`) |
| Rollback | `build.rs` | ✅ Backup `.prev` + restauration sur écart |
| Registry | `build_surface.rs:438` | ⚠️ `serve_webhook` ne prend plus `_registry` : forward vers node qui admet |

**Preuve** : 9 tests build verts dont 3 install, au SHA apparié.

---

### Vertical 5 — Runtime Deployment

**Owner** : A1 build_runner (partiel 2026-09-03)
**Status** : Lecture fermée, imposition restante

| Capacité | Fichier | Statut |
|----------|---------|--------|
| SHA embarqué (kernel) | `cynic-kernel/build.rs:33` | ✅ `CYNIC_GIT_SHA` |
| SHA embarqué (cli) | `cynic-cli/build.rs` + `version_surface.rs` | ✅ `cynic version --json` (`489f1138`) |
| SHA distant observé | `build_runner_adapter.rs` | ✅ `binary_sha` (`35ca17a7`) |
| Binaire canonique | `/var/lib/cynic/bin/cynic` | ✅ Principal cynic, symlink, rollback `.prev` (deploy manuel prouvé) |
| **MANQUANT** : Gate de fraîcheur | — | ❌ Écart SHA jamais imposé |
| **MANQUANT** : Rollback runtime auto | — | ❌ `.prev` manuel seul |

**Preuve** : binaire builder rapporte le SHA tip exact, digest apparié.

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

1. **Cette branche** (`agent/hermes/host-proto-clean`, 29 ahead / 0 behind
   main au 2026-09-03) porte I1 + A1 + webhook + version + install.
   0 behind : pas de rebase requis.
2. **Résiduel bloquant** : `make check` rouge sur 3 erreurs kernel
   étrangères (`dexscreener.rs:37` E0308 du commit intel `ebf231af`,
   `probe/hardware.rs:21-22` E0609 du commit kernel `b8784c0a`).
   Scope protocol/host/node/cli prouvé vert par slice au SHA apparié.
3. **Tranche étrangère à bord** (`buildrunner-closure`, 5 commits) :
   compile vert sur builder, comportement non relu. Merger avec
   acceptation écrite ou re-review.

Fichiers de conflit potentiels : aucun (0 behind). Risque restant :
dette kernel en vol au moment du merge.

---

## Prochaines étapes

1. **Gate** : kernel vert par ses owners, ou résiduel explicite accepté.
2. **Re-review** : tranche `buildrunner-closure` comportementale.
3. **Merge explicite** de cette branche vers main (décision opérateur).
4. **V1 complète** : admission sur trigger (bail + enregistrement),
   gate de fraîcheur imposée, GC corrélé workload, webhook live.

---

*Plan mis à jour au fil des merges. Chaque verticale fermée = PR mergée + preuve builder.*
