# Cycle 2.2 — Inventaire des idées à sauver : `agent_access`

> **Phase Mesurer uniquement.** Aucun code déplacé, aucun commit.
> Branche `cycle/2.2-agent-access`, worktree `~/projects/cynic-2.2`.
> Périmètre : `crates/cynic-host/src/agent_access/` = **1 264 lignes**
> (7 fichiers) + façade `agent_access.rs` (27 l.) + adapter couplé
> `crates/cynic-host/src/adapters/agent_access.rs` (111 l.).
> Référence méthode : `docs/plans/CYCLE_CHECKLIST_TDD.md` (3 échelles).
> Voir aussi `docs/plans/CYCLE_2.2_ARBITRAGE.md` (arbitrage de déplacement ;
> le présent document est l'**inventaire des idées**, pas la décision).

## 0. Mesure de base (vérifiée par grep, pas par impression)

- **Appelants hors `cynic-host` : aucun.** `grep -rn agent_access|AgentAccess|
  compose_agent_session|write_agent_authority|apply_agent_ --include='*.rs' crates/`
  ne retourne, hors du dossier lui-même, que `lib.rs:5` (`pub mod agent_access`),
  `adapters/mod.rs` + `adapters/agent_access.rs` (l'adapter interne), et le
  champ homonyme sans rapport `cynic-registry/src/runtime_authority.rs:36`
  (`agent_access_group`, simple donnée de layout). Aucun appel dans
  `cynic-cli`, `cynic-node`, `cynic-intel` (pourtant dépendants de
  `cynic-host`), aucune référence dans `crates/cynic-host/tests/` ni
  `src/bin/cynic-host-exec.rs`.
- **Couverture : 10 tests, tous internes** (`#[cfg(test)]` : decision 3,
  authority 2, authority_projection 1, authority_bundle 1, authority_apply 1,
  adapters/agent_access 2 ; access_apply et types : 0).
- **Conclusion Mesurer : code mort en production.** Tout verdict « sauver »
  ci-dessous signifie *sauver l'idée pour un usage futur branché*, pas
  *conserver parce qu'il est utilisé*. Si aucun usecase ne doit le brancher,
  tout est purgeable — voir Risques §5.1.

## 1. Inventaire par fichier : idées à sauver, invariants, verdict

### 1.1 `types.rs` — 62 l. — verdict : **SAUVER (purifier puis extraire)**

Idées/invariants à sauver :
- Les trois structs valeur `AgentAccessPolicy / AgentAccessObservation /
  AgentAccessPlan` + constantes de schéma `cynic.host.agent_access_{policy,
  observation,plan}.v1` : le vocabulaire partagé policy→observation→plan.
- `AgentAccessObservation` en `Option<bool>`/`Option<u32>` (présence inconnue
  ≠ absence) : c'est ce qui permet les `residual_unknowns` de `decision.rs`.

Impuretés (à purger du type, pas de l'idée) :
- `AgentAccessPlan.operations: Vec<HostOperation>` — le type pur nomme le
  port du host (`crate::ports::HostOperation`, `ports/mod.rs:64`).
- `AgentAccessPolicy::desired()` lit
  `cynic_registry::runtime_authority::default_runtime_authority_paths()`
  (socket `/run/cynic/kernel.sock`, groupe `cynic-agent`, user `cynic`,
  `socket_mode: 0o660`) : constructeur d'adapter OS, pas du domaine.

Destination : structs + schémas → `cynic-agent` (nouveau module `agent_access`,
ne dépendant que de `cynic-protocol`) ; `operations` → nouvel enum fermé L0
(voir §4) ; `desired()` → reste dans `cynic-host` comme défaut d'adapter.

### 1.2 `decision.rs` — 184 l. — verdict : **SAUVER (quasi pur)**

Idées/invariants à sauver (le cœur du module) :
- `reconcile_agent_access(policy, observation) -> plan` : fonction pure,
  sans I/O, déterministe (tri + dédup des blockers/unknowns).
- Invariant `socket_mode == 0o660` exigé dans la policy, sinon
  `agent_access_policy_invalid`.
- **Blocage dur (pas d'opération de réparation)** si le socket existe mais
  `socket_group_id != access_group_id` (`agent_socket_group_mismatch`) ou
  mode ≠ policy (`agent_socket_mode_mismatch`) : le domaine refuse de
  « réparer » un socket vivant, il bloque.
- **Convergence sans socket** : si groupe + membres + répertoire sont bons
  mais socket absent, plan = `converged`, avec
  `agent_socket_post_start_read_back_pending` en unknown résiduel (le socket
  naîtra au démarrage du kernel ; la vérification se fera au read-back).
- Ordre borné des opérations : groupe système → membres supplémentaires
  (runtime_user + authorized_principals, via `BTreeSet`) → répertoire
  runtime `0o755` si absent. `None` (inconnu) ne génère jamais d'opération,
  seulement un unknown résiduel.

Impureté : émet `HostOperation` (4 variantes utilisées sur 13 :
`EnsureSystemGroup`, `EnsureSupplementaryGroupMember`, `EnsureOwnedDir`).
Une fois l'opération définie en L0, le fichier est 100 % pur.

Destination : `cynic-agent`, même module que §1.1.

### 1.3 `authority.rs` — 138 l. — verdict : **SAUVER en totalité (le plus sûr)**

Idées/invariants à sauver :
- `compose_agent_session_authority(node_id, revision, peer_observation_max_age_ms,
  policies)` : validation complète (node non vide, revision ≠ 0, max_age ≠ 0,
  ≥1 policy, uid uniques via `BTreeSet`, `max_ttl_ms ≠ 0`, types/capabilities
  non vides/blank) puis construction + digest de contenu
  (`agent_session_authority_digest` de `cynic-protocol`). Dépendances :
  `cynic-protocol` + `BTreeSet` uniquement. **Aucun I/O, aucune OS.**
- `AgentEnrollmentPolicy` : l'entrée « uid Linux observé » (`observed_linux_uid`)
  → principal `linux-uid:<uid>` ; transport forcé
  `LocalUnixPeer` / `LocalPeerBinding` (pas de choix libre = pas de downgrade).
- `default_local_agent_capabilities()` : les 5 grants du contrat MCP local
  (`coord.read`, `coord.write`, `observation.write`, `runtime.read`,
  `agent_workload.delivery`, avec leur `mutates`) — contrat figé, testé.
- Constantes `DEFAULT_AGENT_SESSION_TTL_MS = 900_000`,
  `DEFAULT_PEER_OBSERVATION_MAX_AGE_MS = 30_000`.

Destination : **`cynic-protocol::agent_session`** (L0) — les types qu'il
construit (`AgentSessionAuthority`, `AgentSessionEnrollment`, digest) y vivent
déjà (`agent_session/types.rs`, `agent_session/decision.rs:17`
`reconcile_agent_session`). C'est le déplacement le plus petit et le plus sûr
du cycle. Rien à purger.

### 1.4 `authority_bundle.rs` — 143 l. — verdict : **SAUVER l'idée, PURGER du domaine**

Idées/invariants à sauver (pattern, pas le fichier) :
- **Écriture atomique** : `create_new` dans un fichier temporaire
  `.<name>.<pid>.tmp` + `write_all` + `sync_all` + `rename`, nettoyage du
  temporaire en cas d'échec (`atomic_write`).
- **Read-back obligatoire** : relecture + désérialisation + comparaison
  d'égalité stricte avec l'autorité composée ; toute divergence = erreur
  (`agent authority read-back differs from desired content`).
- Refus si le répertoire parent est absent (pas de création implicite).

À purger : le fichier en tant que module du « domaine » — c'est un adapter
OS (`std::fs`, `OpenOptions`). Il reste dans `cynic-host` (ou rejoint les
adapters), et sa composition délègue à `authority.rs` extrait (§1.3).
Note inter-crates : `create_new`+`sync_all` est déjà copié dans ≥10 adapters
de `cynic-host` (`jsonl_append`, `local_evidence_store`,
`framework_bootstrap/*/apply`, `one_shot_authority`, …) et 4 de `cynic-agent` :
c'est une SSOT d'adapter manquante, pas une raison d'extraire ce fichier.

### 1.5 `authority_projection.rs` — 241 l. — verdict : **SAUVER (scinder en 2)**

À sauver (pur) :
- `AgentAuthorityProjectionSpec / Observation / Plan` + schéma
  `cynic.host.agent_authority_projection.v1`.
- `plan_agent_authority_projection(spec, source, installed)` : invariants
  `mode == 0o640`, `owner == "root"`, `owner_id`/`group_id` résolus,
  digest de la source prouvé avant toute installation
  (`agent_authority_projection_source_unproved`), convergence exigeant
  contenu + digest + mode + uid + gid, une seule opération bornée
  `InstallOwnedFile`, preuves attendues
  (`installed_authority_digest_matches`, `installed_authority_mode_0640`,
  `kernel_bootstrap_reads_installed_authority`), rollback = « restaurer le
  fichier d'autorité content-addressed précédent ».

À purger du domaine (adapter OS) :
- `observe_agent_authority_projection` (`std::fs::read`, `symlink_metadata`,
  `MetadataExt` uid/gid/mode, validation du digest lu).
- `AgentAuthorityProjectionSpec::installed()` : lit `cynic-registry`
  (chemin `config_root/agent-session-authority.json`, groupe runtime) **et**
  `crate::identity_database::resolve_identity_id("/etc/passwd"|"/etc/group")`.

Destination : `plan_*` + structs → `cynic-agent` ; `observe_*` + `installed()`
→ `cynic-host` (devient l'implémentation du port d'observation).

### 1.6 `authority_apply.rs` — 291 l. — verdict : **SAUVER le pattern, PURGER la copie**

Idées/invariants à sauver (le gabarit d'orchestration, partagé avec §1.7) :
1. Observation **avant** (`source` + `before`) → plan.
2. Porte `explicit_apply` : sans lui, statut `planned` + blocker
   `explicit_apply_required`, zéro exécution.
3. **Admission `PolicyAuthority::HumanAdmin`** (`cynic-protocol`,
   `PolicyAdmissionReport` + `ExecutionAuthorization`, `requires_explicit_apply`,
   `requires_privilege`, `mutates`) via `admit_host_operation_plan` avant
   toute exécution ; refus d'admission → `blocked`, pas d'exécution.
4. Exécution séquentielle avec **arrêt à la première erreur**.
5. Observation **après** + **read-back obligatoire** : `completed` seulement
   si tout a réussi **et** le re-plan est `converged`, sinon
   `agent_authority_install_read_back_failed`.
6. `operation_plan` : id content-addressed
   (`agent-authority-install:<digest>`), préconditions nommées, rollback intent.

À purger : le fichier tel quel — c'est un orchestrateur host (admission +
`HostCommandRunnerPort`), et ~80 % de sa structure (dont `admission()`
copiée à l'identique) est dupliquée avec `access_apply.rs`. Les deux
`admission()` doivent devenir **une** SSOT (paramétrée par `POLICY_ID`/kind),
et l'orchestration ne doit plus dépendre de `crate::execution` en dur mais
d'un port (voir §4).

### 1.7 `access_apply.rs` — 205 l. — verdict : **SAUVER le pattern, PURGER la dépendance inverse**

Idées à sauver : exactement le même gabarit que §1.6, avec
`reconcile_agent_access` comme planificateur, `POLICY_ID =
"cynic.host.agent_access.apply"`, rollback intent « retirer uniquement les
membres supplémentaires et le groupe nouvellement déclarés après revue
opérateur », et les mêmes règles de statut (`completed`/`blocked`/`planned`).
C'est la **meilleure preuve** que le gabarit admission→exécution→read-back
est un pattern de DnA à factoriser, pas deux implémentations.

Impureté à purger en priorité : `use crate::adapters::agent_access::
observe_agent_access;` — **le « domaine » importe son adapter** (dépendance
inverse, interdite par l'invariant documenté dans `ports/mod.rs`). Après
extraction, l'observation passe par le port injecté, jamais par un import
d'adapter. Zéro test dans ce fichier (le gabarit n'est testé que côté
`authority_apply.rs` grâce à `CopyRunner`) : tout test de gabarit doit être
fusionné, pas dupliqué.

## 2. Échelle 1 — Intra-crate : FCIS / hexagonal / DDD

| Fichier | Classe réelle | Justification |
|---|---|---|
| `authority.rs` | **Domaine pur (FCIS)** | `cynic-protocol` + `BTreeSet`, zéro I/O |
| `decision.rs`, `types.rs` (structs), `authority_projection.rs::plan_*` | **Domaine pur*** | purs sauf le type `HostOperation` qu'ils émettent/nomment |
| `authority_projection.rs::observe_*`, `::installed()`, `authority_bundle.rs`, `adapters/agent_access.rs` | **Adapters OS** | `std::fs`, `/etc/group`, `/etc/passwd`, `MetadataExt`, `cynic-registry` |
| `access_apply.rs`, `authority_apply.rs` | **Orchestration couplée** | admission + exécution + observation : ni domaine pur, ni adapter propre |

Impuretés structurelles :
- `agent_access/` n'a **aucune** séparation domaine/ports/adapters en son
  sein (contrairement à `domain/`, `ports/`, `adapters/` du reste de
  `cynic-host`) : la façade `agent_access.rs` ré-exporte domaine, adapters
  et orchestrateurs pêle-mêle.
- Dépendance inverse `agent_access → adapters::agent_access` (§1.7).
- `admission()` dupliquée à l'identique dans les deux `*_apply.rs`
  (violation ARCH-002/ARCH-006 : deux patterns pour un même geste).
- DDD : deux agrégats dans un même bounded context — *accès groupe/socket*
  (appartenance) et *projection d'autorité* (fichier content-addressed) —
  qui ne partagent que le gabarit d'apply. Après extraction, ce sont deux
  modules distincts de `cynic-agent`, pas un dossier fourre-tout.
- Les modes `0o660`/`0o640`/`0o755` et `owner == "root"` sont des invariants
  de politique OS codés dans le domaine : à conserver comme constantes
  documentées (ne pas les « généraliser » en paramètres sans y être forcé),
  en sachant qu'elles ancren
...[truncated 6154 chars]
## Socle canonique agent_access (verticale)
Port L0 (cynic-protocol) : AgentAccessOperation (enum fermée : Observe, Project, Apply),
traits Observation et Execution. Adapter hôte (cynic-host) : implémentation OS
(/etc/passwd, chown/chmod fichiers d'autorité, read-back). Consommateurs en attente :
cynic-agent (claims/sessions), cynic-node (admission mesh), cynic-host (bootstrap).
