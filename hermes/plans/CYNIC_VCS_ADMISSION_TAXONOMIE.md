# CYNIC — VCS & ADMISSION : CARTOGRAPHIE DE DETTE & TAXONOMIE CIBLE

> **Autorité** : Cet document AUDITE l'état RÉEL du code au 2026-09-24.
>            Il complète `infra/CANONICAL_CRATE_MAP.md` (qui définit la cible)
>            en mesurant l'écart entre le souhaité et l'observé.
> **Pattern** : `cynic-<famille>-<rôle>` — tel que défini dans CANONICAL v2.
> **Méthode** : lecture exhaustive des fichiers sources, LOC réels, dépendances mesurées.

---

## 1. ÉTAT RÉEL — VCS / SOURCE SYNC / ADMISSION (2026-09-24)

### 1.1. Le chaînon manquant : `SourceControlAdmission`

```
OBSERVATION (CLI stub, cynic-cli/src/source_surface.rs:116) :

    fn blocked_source_mutation() -> SourceExit {
        SourceExit {
            status: "blocked",
            summary: "blocked_unsupported_mutation: source sync and worktree apply
                      require an admitted source-control transaction".to_string(),
        }
    }
```

**Verdict** : Le CLI appelle un gate d'admission qui n'existe nulle part dans le codebase.
Aucune struct, aucun module, aucun trait nommé `SourceControlAdmission`,
`SourceControlTransaction`, ou équivalent n'est implémenté.

Conséquence : `cynic source sync --apply` est un STUB. L'exécuteur réel
(`cynic_host::source_sync::apply::execute_plan`) existe, est testé, mais n'est
JAMAIS appelé depuis le CLI.

### 1.2. Fragmentation de la taxonomie VCS

Le domaine "source control / Vcs" est éclaté en AU MOINS 7 endroits :

| Endroit | Contenu | LOC | Problème |
|---|---|---|---|
| `cynic-vcs/src/sync/` | types, reconcile, plan (Cortex→Forgejo→GitHub) | ~300 | ✓ domaine pur, correct |
| `cynic-vcs/src/forgejo.rs` | plan_forgejo_convergence | 358 | ✓ domaine pur |
| `cynic-vcs/src/forgejo_token.rs` | ForgejoToken lifecycle | ~500 | ? doit aller dans cynic-vcs-forgejo |
| `cynic-vcs/src/source_control.rs` | SourceSnapshot, preflight, read-back | ~800 | ✓ |
| `cynic-vcs/src/source_release.rs` | Release evaluation | ~400 | ? domain |
| `cynic-host/src/source_sync/` | observe + apply (I/O) | ~700 | ✓ adapter host correct |
| `cynic-host/src/source_control/commit_scope.rs` | preflight commit | 80 | ✓ petit module |
| `cynic-host/src/forgejo_control.rs` | ForgejoControl — API adapter | 372 | ⚠️ DOIT aller dans cynic-vcs-forgejo |
| `cynic-host/src/forgejo_token.rs` | ForgejoToken adapter (I/O lourd) | 1052 | ⚠️ DOIT aller dans cynic-vcs-forgejo |
| `cynic-host/src/forgejo_webhook.rs` | Webhook parsing | 201 | ⚠️ DOIT aller dans cynic-vcs-forgejo |
| `cynic-host/src/forgejo_webhook_server.rs` | Webhook server | 401 | ⚠️ DOIT aller dans cynic-vcs-forgejo |
| `cynic-lifecycle/src/gate.rs` | LifecycleGate execution | 133 | ✓ lifecycle gate (pas VCS) |
| `cynic-lifecycle/src/promote.rs` | Phase promotion | 92 | ✓ lifecycle gate |
| `cynic-lifecycle/src/domain/artifact_publication.rs` | Publication transport | 151 | ⚠️ pub≠sync — différent mais adjacent |
| `cynic-host/src/lifecycle/` | host lifecycle (activation, diagnostics...) | 2478 | ⚠️ DOIT être dans cynic-host-lifecycle (renommer) |
| `cynic-host/src/network_actuation.rs` | PolicyAdmissionReport, execute_admitted_* | 924 | ⚠️ même pattern "admission" mais pas VCS |

**Synthèse** :
- Le domaine "admission" (`PolicyAdmissionReport`) existe DANS `network_actuation` — c'est le pattern pour `SourceControlAdmission`
- Le domaine "forgejo" (2026 LOC dans `cynic-host/src/`) n'a PAS été migré vers `cynic-vcs-forgejo` alors que CANONICAL v2 le demande depuis la VAGUE A
- `cynic-host/src/lifecycle/` (2478 LOC) est un doublon avec le crate `cynic-lifecycle` (3091 LOC)

---

## 2. DÉTTE DE TAXONOMIE / NOMENCLATURE

### 2.1. Divergence docs CANONICAL vs réalité fichiers

CANONICAL v2 §5 (2026-09-23) déclare :
> ✅ VAGUE A TERMINÉE 2026-09-24

Mais les fichiers sont TOUJOURS dans `cynic-host/src/` :

```
forgejo_control.rs       372 LOC  ← DOIT → cynic-vcs-forgejo
forgejo_token.rs        1052 LOC  ← DOIT → cynic-vcs-forgejo
forgejo_webhook.rs       201 LOC  ← DOIT → cynic-vcs-forgejo
forgejo_webhook_server.rs 401 LOC ← DOIT → cynic-vcs-forgejo
source_sync/             ~700 LOC  ← DOIT → cynic-vcs-git (adapter)
source_worktree*.rs      re-exports ← OK en façade
lifecycle/              2478 LOC  ← DOIT → cynic-host-lifecycle
```

**Cause probable** : La VAGUE A a été commitée mais concernait l'extraction de
`cynic-protocol/src/vcs/` vers `cynic-vcs/`. Les modules dans `cynic-host/`
n'ont PAS été migrés — ils restent dans le monolith.

### 2.2. Dead modules

| Module | Déclaré dans | Fichier | Statut |
|---|---|---|---|
| `source_change_transaction` | `cynic-node/src/lib.rs:64` | ❌ N'existe pas | DEAD — ne compile pas si invoqué |
| `cynic-host/src/source_worktree.rs` | `cynic-host/src/lib.rs:115` | ✅ Existe (re-export) | OK |

### 2.3. Nomenclature collision : "release"

Le concept de "release" est dispersé en 4 endroits :

| Endroit | Rôle |
|---|---|
| `cynic-release/src/evaluator.rs` | ReleaseGatesContract evaluation (domaine pur) |
| `cynic-release/src/contracts.rs` | Gate contract definitions |
| `cynic-lifecycle/src/promote.rs` | Phase promotion state machine |
| `cynic-cli/src/source_surface.rs` (Release cmd) | CLI release gate check |
| `cynic-cli/src/lifecycle_surface_execute.rs` | CLI lifecycle execute |

Ces 5 endroits NE PAS un cadre unifié. `cynic-release` évalue des gates,
`cynic-lifecycle` fait une phase machine, le CLI bricole entre les deux.

**Verdict** : Il manque un concept de "SourceAdmission" qui unifie
ReleaseGate + SyncPlan validation avant exécution.

### 2.4. Nomenclature collision : "lifecycle"

| Endroit | Contenu | LOC |
|---|---|---|
| `cynic-lifecycle/` (crate) | gate, promote, apply, repair, observation, artifact_publication | 3091 |
| `cynic-host/src/lifecycle/` | activation_evidence, workflow_state, diagnostics, forge_task_contract | 2478 |

Deux systèmes "lifecycle" sans pont. CANONICAL dit `cynic-lifecycle → cynic-host-lifecycle`
mais les 2480 LOC dans `cynic-host/src/lifecycle/` n'ont jamais été migrés.

---

## 3. MONOLITHES / SPOF IDENTIFIÉS

### 3.1. `cynic-host` — 89 754 LOC — GOD OBJECT

```
Sous-domaines identifiés (module tree) :
  agent/            — agent coordination, runtime, access
  agent_host/       — host-side agent management
  agent_runtime/    — process identity, probes
  audit_engine/     — audit
  autoresearch/     — auto-research
  build_runner_harness/ — build proof, toolchain
  container_runtime/ — container
  coordination_lease_store/ — agent lease
  credential_hardening/ — credential hardening
  detection_engine/ — detection
  execution/        — command execution
  framework_bootstrap/ — bootstrap
  host_exec_cli/    — CLI projection
  host_hardening/   — hardening
  lifecycle/        — ⚠️ 2478 LOC doublon
  lxc_exec_bridge/  — LXC
  mcp_runtime/      — MCP
  network_actuation.rs — ⚠️ 924 LOC — DOIT aller dans cynic-network
  one_shot_authority/ — authority
  organic_opportunity/ — opportunity
  os_health/        — health
  ports/            — port traits
  principal_prepare/ — principal
  privilege/        — privilege
  process_convergence/ — process
  profiles/         — profiles
  registry/         — registry
  runtime_seed/     — seed
  secret_lint/      — lint
  source_control/   — 80 LOC (commit_scope)
  source_sync/      — ⚠️ adapter VCS (observe+apply)
  source_worktree*  — ⚠️ re-exports
  space_gc/         — GC
  ssh_config/       — SSH
  ssh_trust/        — trust
  storage_capabilities/ — storage
  storage_census/   — census
  storage_credential_sync/ — sync
  storage_identity/ — identity
  storage_primary/ — primary
  toolchain/        — toolchain
  forgejo_*.rs      — ⚠️ 2026 LOC — DOIT aller dans cynic-vcs-forgejo
```

**Verdict** : `cynic-host` agrège 40+ sous-domaines. C'est un SPOF total.
Chaque modification nécessite de recompiler 90K LOC. Les domaines "forgejo",
"source_sync", "lifecycle", "network_actuation" sont clairement extractibles.

### 3.2. `cynic-node` — 58 087 LOC — MONOLITHE

```
Sous-domaines :
  network_actuator/ — mesh, ACL, route (DOIT aller dans cynic-network)
  dns_activation/   — DNS (DOIT aller dans cynic-node ou crate dédié)
  fleet_inventory/ — fleet
  fleet_observability/ — observability
  runtime_supervision/ — supervision
  watchdog/         — watchdog
  websocket/        — websocket
  source_change_transaction — ⚠️ DEAD (module déclaré, fichier absent)
  lifecycle.rs      — nœud lifecycle
  observer.rs       — node observation
```

**Verdict** : `cynic-network` devrait absorber `network_actuator/` + `network_actuation.rs` (host).

### 3.3. `cynic-protocol` — 54 687 LOC — GOD OBJECT FEUILLE

```
Zéro import workspace (zero dep) — mais 54K LOC.
La VAGUE A a extrait vcs/, storage/, solana/, harness/, peer/, network.rs,
forensics.rs → il reste encore 43K LOC.

Sous-domaines restants :
  agent_coordination/   — claim, lease, scaling
  agent_recursion/      — recursion
  agent_session/        — session
  cleanup/              — cleanup
  convergence/          — convergence
  development/          — development
  eval/                 — scoring
  fleet_inference/      — fleet inference
  health_signal/        — health
  host_substrate/       — ⚠️ hardware probes (DOIT aller dans cynic-host-substrate)
  kyon/                 — judgment
  pattern_gc/           — pattern GC
  provider/             — providers
  runtime/              — runtime types
```

**Verdict** : Toujours au-dessus du seuil 40K. Les vagues B/C/D ne sont pas faites.

---

## 4. LE PATTERN ADMISSION — EXISTANT MAIS ÉCLATÉ

Le codebase a déjà un pattern "admission" mais il est uniquement appliqué à la
network actuation :

```rust
// cynic-host/src/network_actuation.rs
pub struct PolicyAdmissionReport {
    pub intent: AdmissionIntent,
    pub mutation: AdmissionMutation,
    pub preconditions: Vec<String>,
    pub admitted: bool,
    pub blockers: Vec<String>,
}

pub fn execute_admitted_network_actuation(
    plan: &NetworkActuationPlan,
    admission_blockers: Vec<String>,
    executor: &dyn NetworkActuator,
) -> NetworkActuationReport {
    // n'exécute que si admission_blockers.is_empty()
}
```

**Ce pattern doit être unifié pour le source sync.**

Il manque :
1. Un `SourceAdmissionReport` dans `cynic-vcs/src/sync/` (pur, types only)
2. Un port `SourceAdmissionPort` qui valide le plan (gate pre-execution)
3. Une implémentation concrète dans `cynic-host/` qui :
   - collecte les gate proofs (depuis release_gate_adapter)
   - valide le sync plan
   - retourne `SourceAdmissionReport { admitted: true/false, blockers: [...] }`
4. Dans le CLI : si `admitted` → appeler `execute_plan`, sinon afficher les blockers

---

## 5. TAXONOMIE CIBLE — ALIGNÉE CANONICAL v2

### 5.1. Famille VCS (état souhaité)

```
cynic-vcs (résidu types only)         ← ~500 LOC après extraction
  sync/types.rs, sync/reconcile.rs, sync/plan.rs, forgejo.rs

cynic-vcs-git (À CRÉER)               ← adapter I/O Git
  source_sync/observe.rs, source_sync/apply.rs (depuis cynic-host)
  source_control/commit_scope.rs (depuis cynic-host)

cynic-vcs-forgejo (À CRÉER)           ← adapter I/O Forgejo
  forgejo_control.rs, forgejo_token.rs, forgejo_webhook.rs,
  forgejo_webhook_server.rs (depuis cynic-host)
  forgejo.rs (depuis cynic-vcs/src/ — plan_forgejo_convergence, domain pur)

cynic-host-lifecycle (RENFINIR cynic-lifecycle → cynic-host-lifecycle)
  Intégrer les 2478 LOC de cynic-host/src/lifecycle/
  Créer SourceControlAdmission ici (couplé au lifecycle gate)
```

### 5.2. Famille Admission (état souhaité)

```
SourceControlAdmission (NOUVEAU dans cynic-host-lifecycle)
  ├── SourceAdmissionReport (types dans cynic-vcs/src/sync/)
  ├── SourceAdmissionPort (trait dans cynic-vcs/src/ports.rs)
  ├── HostSourceAdmissionAdapter (dans cynic-host/src/adapters/)
  │     └── utilise release_gate_adapter::collect_host_gate_proofs
  └── CLI : cynic source sync --apply
        ├── observe_all_layers()
        ├── reconcile() → decision
        ├── plan() → steps
        ├── validate_source_admission(decision) → SourceAdmissionReport
        └── if admitted: execute_plan()
```

### 5.3. DAG cible pour le VCS uniquement

```
cynic-protocol (résidu ~3K)
      │
      ▼
cynic-vcs  (types sync + forgejo plan + SourceAdmissionReport)
      │
      ├──→ cynic-vcs-git     (observe, apply, commit_scope — I/O Git)
      │         uses SourceAdmissionPort (optionnel)
      │
      ├──→ cynic-vcs-forgejo (Forgejo adapters — I/O HTTP/SSH)
      │
      └──→ cynic-host-lifecycle (SourceControlAdmission)
                │
                └──→ uses cynic-release (gate evaluation)
                └──→ uses cynic-vcs::SourceAdmissionReport
```

---

## 6. REMÉDIATION — ORDRE BOTTOM-UP

### SESSION VCS-1 : Bâtir SourceControlAdmission (débloque le CLI)

```
Prérequis : aucun (on ne déplace pas de code encore)

1. Définir `SourceAdmissionReport` dans cynic-vcs/src/sync/types.rs
2. Définir `SourceAdmissionPort` dans cynic-vcs/src/ports.rs
3. Implémenter `HostSourceAdmissionAdapter` dans cynic-host/src/adapters/
   - collecte gate proofs via release_gate_adapter
   - valide decision.needs_review == false
   - valide decision.unknowns.is_empty()
   - retourne SourceAdmissionReport { admitted, blockers }
4. Connecter CLI `cynic source sync --apply` :
   - observe → reconcile → plan → validate_source_admission → execute_plan
5. Supprimer le stub `blocked_source_mutation`
6. Tests : admission bloquée si dirty files, gate non vert, etc.
```

### SESSION VCS-2 : Extraire cynic-vcs-forgejo

```
1. Créer crates/cynic-vcs-forgejo/
2. Déplacer forgejo_control.rs, forgejo_token.rs, forgejo_webhook.rs,
   forgejo_webhook_server.rs depuis cynic-host/src/
3. Déplacer forgejo.rs + forgejo_token.rs depuis cynic-vcs/src/
4. Adapter les imports (crate:: → cynic_vcs::)
5. Tests architecture_boundary.rs : zero dep on cynic-host/node/cli
6. Remplacer dans cynic-host/src/ par re-export ou adapter
```

### SESSION VCS-3 : Extraire cynic-vcs-git

```
1. Déplacer source_sync/observe.rs, source_sync/apply.rs depuis cynic-host/
2. Déplacer source_control/commit_scope.rs depuis cynic-host/
3. Adapter les imports
4. Re-exports dans cynic-host/src/ pour compatibilité
5. Supprimer le dead module source_change_transaction dans cynic-node
```

### SESSION VCS-4 : Unifier lifecycle

```
1. Renommer cynic-lifecycle → cynic-host-lifecycle (per CANONICAL)
2. Intégrer cynic-host/src/lifecycle/ (2478 LOC) dans cynic-host-lifecycle
3. Supprimer cynic-host/src/lifecycle/
4. SourceControlAdmission vit dans cynic-host-lifecycle
```

---

## 7. RÈGLES D'ENFORCEMENT POUR LE VCS

```
V1. SourceAdmission obligatoire
    Aucun `git push` vers forgejo/github sans SourceAdmissionReport.admitted == true.
    Le CLI ne contourne pas l'admission.

V2. Domaine pur séparé de l'I/O
    cynic-vcs (types + state machines) ne contient PAS de std::process::Command.
    L'I/O Git vit dans cynic-vcs-git et cynic-vcs-forgejo.

V3. Rollback automatique
    execute_plan garde son comportement actuel : rollback sur échec,
    pas de push partiel.

V4. Pas de re-export global
    cynic-host/src/source_sync/ ne fait PAS `pub use cynic_vcs::sync::*` en façade.
    Les imports sont explicites via cynic_vcs_git:: observe, apply.
```

---

## 8. INDICATEURS DE SUCCÈS

Avant (état actuel) :
- `cynic source sync --apply` → stub blocked ❌
- Aucune SourceControlAdmission ❌
- 2026 LOC de forgejo dans cynic-host ❌
- 2478 LOC de lifecycle doublon dans cynic-host ❌
- Dead module dans cynic-node ❌

Après remédiation :
- `cynic source sync --apply` → push Cortex→Forgejo→GitHub avec admission ✓
- SourceControlAdmission unifié ✓
- cynic-vcs-forgejo extrait ✓
- cynic-vcs-git extrait ✓
- cynic-host-lifecycle unifié ✓
- cynic-host < 40K LOC (ratchet) ✓

---

*v1 — 2026-09-24. Audit état réel. Aligné sur CANONICAL_CRATE_MAP v2.*
*Prochaine session : SESSION VCS-1 (bâtir SourceControlAdmission).*
