# Cycle 2.2 — Arbitrage `agent_access`

> Phase Mesurer + proposition uniquement. Aucun code métier déplacé, aucun commit.
> Branche : `cycle/2.2-agent-access` — worktree `~/projects/cynic-2.2`.
> Note : `docs/plans/CYCLE_CHECKLIST_TDD.md` annoncé dans la consigne est
> **absent du repo** (recherche `find` vide) ; la méthode ci-dessous suit la
> checklist décrite dans la consigne (cartographier → classer → arbitrer).

## 1. Cartographie

Périmètre réel : **1 291 lignes** (et non ~1 291 dans un `lib.rs` de dossier).

| Fichier | Lignes | Rôle |
|---|---:|---|
| `crates/cynic-host/src/agent_access.rs` (façade, pas `agent_access/lib.rs`) | 27 | `pub use` des 7 modules |
| `agent_access/types.rs` | 62 | `AgentAccessPolicy/Observation/Plan` + schémas |
| `agent_access/decision.rs` | 184 | `reconcile_agent_access` (policy+observation → plan) |
| `agent_access/authority.rs` | 138 | `compose_agent_session_authority`, `AgentEnrollmentPolicy` |
| `agent_access/authority_bundle.rs` | 143 | `write_agent_authority_bundle` (écriture atomique + read-back) |
| `agent_access/authority_projection.rs` | 241 | `Spec/Observation/Plan`, `observe_*`, `plan_*` |
| `agent_access/authority_apply.rs` | 291 | `apply_agent_authority_projection` (admission + runner + read-back) |
| `agent_access/access_apply.rs` | 205 | `apply_agent_access` (idem, groupe/socket) |
| `adapters/agent_access.rs` (hors dossier, couplé) | 111 | `observe_agent_access` (`/etc/group`), `observe_linux_uid` (`/etc/passwd`) |

### 1.1 Qui appelle `agent_access` ? (grep repo entier)

**Personne hors `cynic-host` lui-même.**

- `grep -rn agent_access --include='*.rs' .` : 44 occurrences, toutes dans
  `crates/cynic-host/src/{agent_access.rs,agent_access/,adapters/agent_access.rs,lib.rs}`
  plus un champ homonyme **sans rapport** dans `cynic-registry/src/runtime_authority.rs`
  (`agent_access_group: "cynic-agent"` — simple donnée de layout, pas un appel).
- Aucun appel dans `cynic-cli`, `cynic-node`, `cynic-intel` (pourtant dépendants
  de `cynic-host`), aucune référence dans `crates/cynic-host/tests/`,
  `src/bin/cynic-host-exec.rs`, ni dans les usecases/bridge.
- Seuls consommateurs : `adapters/agent_access.rs` (observe) et les tests
  internes des modules (`#[cfg(test)]` dans chaque fichier).

**Conséquence Mesurer :** `agent_access` est aujourd'hui du code mort en
production (couvert uniquement par ses propres tests). L'extraction ne
cassera aucun appelant externe — mais déplacer du code mort sans d'abord
décider s'il doit être *branché* quelque part est un risque de conservation
inutile. Voir §4.

### 1.2 Dépendances sortantes de `agent_access`

| Dépendance | Utilisée par | Nature |
|---|---|---|
| `cynic-protocol` (`AgentSessionAuthority`, `agent_session_authority_digest`, `ExecutionAuthorization`, `Policy*`, `digest_json`) | `authority.rs`, `authority_projection.rs`, `access_apply.rs`, `authority_apply.rs` | **L0 pur**, déjà OK |
| `cynic-registry::runtime_authority::default_runtime_authority_paths()` | `types.rs::Policy::desired()`, `authority_projection.rs::Spec::installed()` | Défauts d'OS (paths `/run/cynic/kernel.sock`, `/etc/cynic`, groupe `cynic-agent`) |
| `crate::execution::{HostOperationPlan, admit_host_operation_plan, HOST_OPERATION_PLAN_SCHEMA}` | `access_apply.rs`, `authority_apply.rs` | Admission host (infrastructure) |
| `crate::ports::{HostOperation, HostCommandRunnerPort}` | `types.rs`, `decision.rs`, `authority_projection.rs`, `*_apply.rs` | Port host — **mais défini dans `cynic-host`**, pas en L0 |
| `crate::identity_database::resolve_identity_id` (lit `/etc/passwd`, `/etc/group`) | `authority_projection.rs::Spec::installed()` | Adapter OS, `pub(crate)` |
| `crate::adapters::agent_access::observe_agent_access` | `access_apply.rs` | Adapter OS (dépendance **inverse** : un module du dossier dépend d'un adapter) |
| `std::fs`, `std::os::unix::fs::{MetadataExt, PermissionsExt}`, `OpenOptions` | `authority_bundle.rs`, `authority_projection.rs`, `adapters/agent_access.rs` | I/O OS |

Points saillants :

- **Dépendance circulaire de fait** : `agent_access/` (supposé domaine)
  importe `crate::adapters::agent_access` — le domaine connaît son adapter.
- **Duplication** : deux parseurs `/etc/passwd` et `/etc/group` coexistent
  (`adapters/agent_access.rs::parse_passwd_uid/parse_group` vs
  `identity_database.rs::resolve_identity_id`), avec des sémantiques
  légèrement différentes (match exact vs `nth(1)`).
- `cynic-host` dépend **déjà** de `cynic-agent` (`Cargo.toml`), et
  `cynic-agent` dépend de `cynic-protocol` (L0 feuille, ne dépend ni de
  `cynic-host` ni de `cynic-registry`). Le sens d'extraction ne crée donc
  aucun cycle nouveau.
- `cynic-protocol::agent_session` possède déjà les types
  (`AgentSessionAuthority`, `AgentSessionEnrollment`, digest) et la décision
  de session — `authority.rs` n'est qu'un *constructeur* au-dessus.

## 2. Classification domaine pur vs adapter OS

| Fichier | Classe | Justification |
|---|---|---|
| `authority.rs` — `compose_agent_session_authority`, `default_local_agent_capabilities` | **Domaine pur** | Seules deps : `cynic-protocol` + `BTreeSet`. Aucun I/O, aucune OS. |
| `decision.rs` — `reconcile_agent_access` | **Domaine pur*** | Logique pure (policy+observation → plan), *mais* émet `crate::ports::HostOperation` (défini dans `cynic-host`). Pure une fois le type d'opération sorti de `cynic-host`. |
| `types.rs` — structs `Policy/Observation/Plan` | **Domaine pur*** | Idem (`Plan.operations: Vec<HostOperation>`). **Sauf** `AgentAccessPolicy::desired()` qui lit `cynic-registry` → constructeur d'adapter, à garder côté host. |
| `authority_projection.rs::plan_agent_authority_projection` | **Domaine pur*** | Pure (spec+observations → plan), mêmes réserves (`HostOperation`, invariants `0o640`/`root`). |
| `authority_projection.rs::observe_agent_authority_projection` | **Adapter OS** | `std::fs::read`, `symlink_metadata`, uid/gid/mode via `MetadataExt`. |
| `authority_projection.rs::Spec::installed()` | **Adapter OS** | `cynic-registry` + `identity_database` (`/etc/passwd`, `/etc/group`). |
| `authority_bundle.rs` — `write_agent_authority_bundle` + `atomic_write` | **Adapter OS** | Écriture fichier atomique (`create_new` + `sync_all` + `rename`), read-back. La *composition* qu'il appelle reste pure (voir `authority.rs`). |
| `access_apply.rs`, `authority_apply.rs` | **Orchestration / adapter** | Admission `HumanAdmin` (`cynic-protocol` — pur en soi), `admit_host_operation_plan` (`crate::execution`), exécution via `HostCommandRunnerPort`, observations avant/après. Couple domaine + host : ne peut pas vivre en domaine pur tel quel. |
| `adapters/agent_access.rs` | **Adapter OS** | `/etc/group`, `/etc/passwd`, métadonnées du socket Unix, `is_dir`. |

## 3. Arbitrage proposé

Principe (géométrie L0 → L2 → host déjà en place dans
`docs/plans/vertical-extraction-roadmap.md`) :

```
cynic-protocol (L0, feuille)        ports + types purs, zéro I/O, zéro dep cynic-*
        ↑
cynic-agent (L2 domaine)             décisions pures, ne dépend que de cynic-protocol
        ↑
cynic-host (adapters + orchestration)  I/O OS, admission, exécution, défauts registry
```

`cynic-host` dépend déjà de `cynic-agent` : l'extraction est dans le sens
autorisé et ne crée pas de cycle.

### 3.1 Domaine pur → `cynic-agent` (ou L0 pour un sous-ensemble)

| Quoi déplacer | Destination recommandée | Pourquoi |
|---|---|---|
| `AgentEnrollmentPolicy`, `compose_agent_session_authority`, `default_local_agent_capabilities`, constantes `DEFAULT_*` | **`cynic-protocol::agent_session`** (L0) | Ne font que construire `AgentSessionAuthority`, déjà en L0 avec son digest. Zéro I/O. C'est le déplacement le plus sûr et le plus petit. |
| `AgentAccessPolicy/Observation/Plan`, `reconcile_agent_access` (`decision.rs` + `types.rs` hors `desired()`) | **`cynic-agent`** (nouveau module, ex. `agent_access` ou dans `session.rs`) | Décision d'accès agent = cœur du bounded context agent. Ne doit dépendre que de `cynic-protocol`. |
| `plan_agent_authority_projection` + `AgentAuthorityProjection{Spec,Observation,Plan}` (hors `installed()`/`observe_*`) | **`cynic-agent`** (même module) | Même nature : spec + 2 observations → plan borné. |

### 3.2 Ports définis en L0 (`cynic-protocol`)

Aujourd'hui `HostOperation` (13 variantes, `ports/mod.rs`) et
`HostCommandRunnerPort` (`execution.rs:40`) vivent dans `cynic-host` — le
domaine pur ne peut pas les nommer sans dépendre du host. Définir en L0,
dans `cynic-protocol` (qui héberge déjà des ports provider, voir
`ports/mod.rs` ré-exportant `cynic_protocol::provider`) :

```rust
// cynic-protocol — énumération fermée, portée agent_access uniquement
pub enum AgentAccessOperation {
    EnsureSystemGroup { group: String },
    EnsureSupplementaryGroupMember { user: String, group: String },
    EnsureOwnedDir { path: String, user: String, group: String, mode: u32 },
    InstallOwnedFile { source: String, target: String, user: String, group: String, mode: u32 },
}

pub trait AgentAccessObservationPort {
    fn observe_agent_access(&self, policy: &AgentAccessPolicy) -> AgentAccessObservation;
    fn observe_authority_projection(&self, path: &str) -> AgentAuthorityProjectionObservation;
}

pub trait AgentAccessExecutionPort {
    fn run(&self, operation: &AgentAccessOperation) -> AgentAccessExecutionResult;
}
```

`AgentAccessPlan.operations` et `AgentAuthorityProjectionPlan.operations`
deviennent `Vec<AgentAccessOperation>`. `cynic-host` implémente les ports
en traduisant vers `HostOperation`/`HostCommandRunnerPort` (traduction
exhaustive, `match` sans wildcard — tout nouveau variant `HostOperation`
ne fuit pas silencieusement).

Alternative écartée : déplacer tout `HostOperation` en L0. Il est utilisé
par `domain/{services,privilege,packages,principal}.rs` de `cynic-host`
(cycle intra-couche documenté dans `ports/mod.rs`) ; un sous-ensemble
`AgentAccessOperation` évite de rouvrir ce chantier.

### 3.3 Adapters → `cynic-host` (restent, à nettoyer)

- `adapters/agent_access.rs` : reste, devient l'implémentation de
  `AgentAccessObservationPort`. Corriger la dépendance inverse :
  `access_apply.rs` ne doit plus importer `crate::adapters::…` mais passer
  par le port injecté.
- `authority_bundle.rs` : reste (I/O atomique). Sa composition délègue à la
  fonction pure désormais en L0.
- `access_apply.rs` / `authority_apply.rs` : restent comme orchestration
  host, mais réduites à : observer (port) → planifier (domaine) → admettre
  (`crate::execution`) → exécuter (port) → read-back. L'admission
  `HumanAdmin`/`PolicyDecision::Allow` (100 % `cynic-protocol`) peut rester
  dans ces fichiers.
- `AgentAccessPolicy::desired()` et `Spec::installed()` : restent côté host
  comme constructeurs de défauts (`cynic-registry` + `identity_database`),
  non exportés par le domaine. Les types purs prennent leurs champs en
  explicite.
- `identity_database.rs` vs `adapters/agent_access.rs` : fusionner les deux
  parseurs `/etc/passwd`/`/etc/group` en un seul adapter (dédup à faire
  avant ou pendant l'extraction, pas après).

## 4. Risques

1. **Port trop pauvre = fuite.** Si `AgentAccessExecutionPort::run` ne
   renvoie que `success: bool`, les invariants qui font la sécurité du
   module (`socket_mode == 0o660`, `mode == 0o640`, owner `root`,
   admission `HumanAdmin`, read-back `converged` obligatoire avant
   `completed`) risquent de se réimplémenter — ou de se perdre — côté
   adapter. Le port doit donc porter le résultat structuré
   (`command`, `exit_code`, `stderr`) et l'orchestration doit garder
   admission + read-back. À l'inverse, un port trop riche (exposer
   `HostOperationPlan`/`admit_host_operation_plan`) réintroduit `cynic-host`
   dans le domaine.
2. **Dérive du `match` de traduction.** `AgentAccessOperation` → `HostOperation`
   est une correspondance manuelle : un nouveau variant côté host (ou une
   sémantique `InstallOwnedFile` qui change) peut diverger silencieusement.
   Mitigation : `match` exhaustif sans `_`, test de correspondance dans
   `cynic-host`.
3. **Code mort.** Aucun appelant hors `cynic-host` (§1.1). Extraire avant de
   brancher, c'est risquer de conserver ~1 300 lignes inutilisées dans un
   nouveau crate. Décision préalable requise : brancher (usecase CLI/node
   qui appelle `apply_*`) ou supprimer. L'arbitrage ci-dessus reste valide
   dans les deux cas, mais le brancher est le seul qui justifie le coût.
4. **Invariants de mode en dur dans le domaine.** `0o660` (socket),
   `0o640` + `owner == "root"` (projection) sont des invariants d'OS
   codés dans `decision.rs`/`authority_projection.rs`. En domaine pur, ce
   sont des constantes de *politique* — acceptables si documentées comme
   telles, mais elles empêchent de réutiliser la décision hors Linux.
   Ne pas les « généraliser » en paramètres sans y être forcé.
5. **Deux sources de vérité pour l'identité.** `identity_database` et
   `adapters/agent_access` résolvent uid/gid différemment ; après
   extraction, `Spec::installed()` (host) et `observe_*` (port) doivent
   partager le même adapter, sinon `plan` compare des ids résolus par deux
   parseurs différents.
6. **Cycle de dépendances à surveiller.** `cynic-agent` ne doit **pas**
   dépendre de `cynic-registry` (défauts de paths) ni de `cynic-host`
   (ports). La seule dérive possible est `AgentAccessPolicy::desired()` :
   si elle part avec les types, elle embarque `cynic-registry` dans le
   domaine. D'où §3.3 : elle reste côté host.
7. **Surface de test.** Les tests actuels sont internes aux fichiers
   (dont un `CopyRunner` factice dans `authority_apply.rs`) ; après
   déplacement, les tests de décision pure doivent suivre le code en
   `cynic-agent`/`cynic-protocol` sans dépendance `tempfile`/fs, et les
   tests d'orchestration restent en `cynic-host` avec un runner factice.
   Ne pas déplacer les tests sans trier selon cette ligne.

## 5. Ordre d'exécution suggéré (cycles suivants, pas 2.2)

1. Trancher §4.3 (brancher ou supprimer) — bloque tout le reste.
2. Fusionner les parseurs passwd/group (§4.5).
3. Déplacer `authority.rs` en L0 (`cynic-protocol::agent_session`) — plus
   petit, sans port à définir.
4. Définir `AgentAccessOperation` + ports en L0 ; traduire dans `cynic-host`.
5. Déplacer `decision.rs`/`types.rs`/`plan_*` en `cynic-agent`.
6. Réduire `*_apply.rs` à l'orchestration via ports ; supprimer l'import
   `crate::adapters::…` depuis le domaine.

## Observation — le pattern "domain pull chaotique" fait émerger socles et déduplications

Arbitrage 2.2 : agent_access n'a aucun appelant hors cynic-host (code mort), mais
3 consommateurs attendent un socle canonique d'autorisation OS : cynic-agent
(claims/sessions), cynic-node (admission mesh), cynic-host (bootstrap).
Le domain pull "chaotique" (on tire, on voit ce qui casse) fait émerger :
1. les capacités manquantes (socle d'autorisation OS)
2. l'architecture (port L0 + adapter hôte, inversion des dépendances)
3. les déduplications (parse_passwd_uid x3, atomic_write x14, runners dupliqués)
Pattern répété sur tout le codebase : chaos -> pull -> socle canonique -> consommateurs -> unification.
