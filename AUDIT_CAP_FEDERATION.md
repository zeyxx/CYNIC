# Audit Théorème CAP — Fédération de Nœuds CYNIC

## Contexte

Application du théorème CAP (Consistency, Availability, Partition tolerance) à la fédération de nœuds CYNIC. Le théorème stipule qu'en présence de partitions réseau (P), un système ne peut garantir qu'un seul des deux : Cohérence (C) ou Disponibilité (A).

Scope : fédération de nœuds (kernel → node → host → fleet), surfaces de découverte, de résolution de pairs, d'autonomie locale, de réconciliation federation, et de sélection de build node. Hors scope : services d'inférence, orchestration Solana.

## 1. Cartographie des Surfaces de Fédération

| Couche | Module | Surface | Rôle |
|---|---|---|---|
| Protocol | `cynic_protocol::fleet_discovery` | `fleet_discovery.rs` | Types purs + `classify_node()` (pure function) |
| Host | `cynic_host::fleet_discovery` | `fleet_discovery.rs` | Adapter I/O : lit registry, probe TCP |
| Protocol | `cynic_protocol::federation` | `federation.rs` | États réconciliation, modes (Coordinated/Autonomous/Promoted/Degraded) |
| Host | `cynic_host::federation_runtime` | `federation_runtime.rs` | Orchestrateur : observe → décide → applique (ports injectables) |
| Protocol | `cynic_protocol::peer_resolver` | `peer_resolver.rs` | Traits `PeerResolver`, types `PeerEndpoint` |
| Host | `cynic_host::peer_resolver_mesh` | `peer_resolver_mesh.rs` | Impl `MeshPeerResolver` : probe TCP + OS health |
| Protocol | `cynic_protocol::local_autonomy` | `local_autonomy.rs` | Décision pure `decide_autonomy` |
| Host | `cynic_host::local_autonomy_runtime` | `local_autonomy_runtime.rs` | Boucle tick : observe OS + kernel + mesh |
| Protocol | `cynic_protocol::build_node` | `build_node.rs` | Résolveur pur fail-closed `resolve_build_node` |
| Host | `cynic_host::os_health` | `os_health/observe.rs` | Observer portable `/proc/sysctl/kmsg` |
| Protocol | `cynic_protocol::node_control` | `node_control.rs` | Plan de contrôle nœud v2 |
| Protocol | `cynic_protocol::fleet_heartbeat` | `fleet_heartbeat.rs` | Envelope heartbeat v1 (orpheline) |
| Host | `cynic_host::registry` | `registry.rs` | Résolution transport SSH depuis registry (SSOT) |
| Host | `cynic_host::runtime_authority` | `runtime_authority.rs` | Injection paths runtime |
| CLI | `cynic_cli::fleet_surface` | `fleet_surface.rs` | CLI : Report/Discover/Health |
| CLI | `cynic_cli::federation_surface` | `federation_surface.rs` | CLI : `federation status` |
| CLI | `cynic_cli::fleet_report` | `fleet_report.rs` | Rapport santé flotte |
| MCP | `cynic_mcp::routers::local::fleet` | `routers/local/fleet.rs` | MCP : `run_federation_status`, `run_fleet_discover` |
| MCP | `cynic_mcp::local_tools` | `local_tools.rs` | Outils MCP : `run_federation_status` (duplication) |

## 2. Application du Théorème CAP par Surface

### Surface A : `fleet_discovery` (protocol + host)

**Architecture :**
- Protocol : types purs + `classify_node()`
- Host : `FleetDiscoveryAdapter` lit `infra/registry.json` + probe TCP connect

**Trade-off exercé : AP (Availability + Partition tolerance)**

| Critère CAP | Niveau | Justification |
|---|---|---|
| **Consistency** | WEAK | Registry statique (snapshot), classification locale instantanée |
| **Availability** | STRONG | Probe TCP non-bloquant (timeout 3000ms), fallback sur registry corrompu → rapport vide + `residual_unknowns` |
| **Partition tolerance** | PARTIAL | Probe indépendant par nœud, mais pas de retry/cache distribué. Partition persistante → nœuds "Unreachable" |

**Preuves :**
- `fleet_discovery.rs` host, lignes 37-49 : erreur registry → rapport vide, pas d'échec
- `fleet_discovery.rs` host, ligne 27 : timeout 3000ms explicite
- `fleet_discovery.rs` protocol, ligne 66 : `classify_node()` pure function, classification locale

### Surface B : `federation_runtime` (host)

**Architecture :**
- `FederationRuntime` orchestre via ports injectables (KernelObserver, MeshObserver, OsHealthObserver, etc.)
- `FederationReconciliation` enum : Coordinated / Autonomous / Promoted / Degraded

**Trade-off exercé : C ↔ A configurable (mode-dependent)**

| Critère CAP | Niveau | Justification |
|---|---|---|
| **Consistency** | FORT (Coordinated) | Exige kernel joignable + lease valide pour converger |
| **Availability** | FORT (Autonomous/Degraded) | Boucle `tick()` continue même si kernel mort |
| **Partition tolerance** | EXPLICITE | Transition Coordinated → Autonomous = partition détectée |

**Preuves :**
- `federation_runtime.rs` ligne 92 : `reconcile()` observe tous les ports → décide
- `federation_runtime.rs` test ligne 461 : `autonomous_when_kernel_dead` — mode change sur unreachable kernel
- `local_autonomy.rs` : `decide_autonomy()` pure function → `AutonomyMode`

**Observation :** C'est le point central du modèle CAP. Le runtime ne choisit pas un seul mode — il a un spectre. C'est correct architecturalement.

### Surface C : `peer_resolver` / `peer_resolver_mesh`

**Architecture :**
- Protocol : traits abstraits
- Host : `MeshPeerResolver` implémente probe TCP + OS health locale

**Trade-off exercé : AP (Availability prioritaire)**

| Critère CAP | Niveau | Justification |
|---|---|---|
| **Consistency** | FAIBLE | `fallback_peers()` retourne pairs déclarés sans liveness check, pas de timestamp/version |
| **Availability** | FORTE | `fallback_peers()` conçu pour "when the live mesh is unreachable" |
| **Partition tolerance** | PARTIAL | Pas de mécanisme de retry/cache, mais fallback disponible |

**Vulnérabilité critique :**
- `fallback_peers()` ne signe pas les endpoints (`signature_ed25519: String::new()`, ligne 161)
- Sous partition, un attaquant pourrait injecter des endpoints fallback non authentifiés

**Preuves :**
- `peer_resolver_mesh.rs` ligne 140 : commentaire "Fallback: ... when the live mesh is unreachable"
- `peer_resolver_mesh.rs` ligne 161 : `signature_ed25519: String::new()` (vide = non authentifié)

### Surface D : `local_autonomy_runtime`

**Architecture :**
- Boucle `tick()` / `run_continuous()` : observe OS + kernel + mesh → `decide_autonomy()`

**Trade-off exercé : A + P (Availability + Partition, locale C sacrifiée)**

| Critère CAP | Niveau | Justification |
|---|---|---|
| **Consistency** | FAIBLE (locale) | Observation locale, pas de replication d'état entre nœuds |
| **Availability** | FORTE | Boucle continue indéfiniment, même si kernel == 0 |
| **Partition tolerance** | EXPLICITE | Mode `Autonomous` = réaction à partition |

**Preuves :**
- `local_autonomy_runtime.rs` ligne 113 : `run_continuous()` loop infinie
- `local_autonomy_runtime.rs` ligne 48 : `tick()` → `AutonomyDecision`
- `local_autonomy.rs` : `decide_autonomy()` pure function

### Surface E : `build_node` (protocol)

**Architecture :**
- `resolve_build_node()` : fonction PURE, fail-closed

**Trade-off exercé : C prioritaire (dans décision de sélection)**

| Critère CAP | Niveau | Justification |
|---|---|---|
| **Consistency** | FORT (fail-closed) | Aucun nœud défectueux jamais sélectionné |
| **Availability** | STRONG (fail-closed) | Retourne `None` plutôt que nœud cassé |
| **Partition tolerance** | NE S'APPLIQUE PAS | Fonction pure, pas de I/O |

**Preuves :**
- `build_node.rs` ligne 96 : `resolve_build_node()` fails closed → `None` si pas de candidat healthy
- Tests lignes 154-170 : failover primary → secondary, mais `None` si aucun reachable+healthy

### Surface F : `registry.rs` (host)

**Architecture :**
- Résout transports SSH, rôles build_runner depuis `infra/registry.json`

**Trade-off exercé : C > A (Consistency > Availability)**

| Critère CAP | Niveau | Justification |
|---|---|---|
| **Consistency** | FORT | Registry = SSOT, ambiguïté = erreur explicite |
| **Availability** | LIMITÉE | Registry corrompu = échec, pas de fallback |
| **Partition tolerance** | NON | Registry local, pas de concept de partition |

**Preuves :**
- `registry.rs` ligne 304 : `node_substrate_target_node_ambiguous` → erreur
- `registry.rs` ligne 399 : `build_runner_role_missing` → erreur

## 3. Duplications et Faiblesses Structurelles

### Duplication 1 : `peer_resolver_mesh.rs` — `resolve_peers()` vs `resolve_all_peers()`

**Niveau de sévérité : CRITIQUE**

`resolve_peers()` (ligne 94) et `resolve_all_peers()` (ligne 171) partagent 95% du code. La logique de probe de nœud est dupliquée.

**Impact :** Un correctif de sécurité dans `resolve_peers` pourrait ne pas être répercuté dans `resolve_all_peers`.

**Localisation du code dupliqué :**
- Lignes 100-135 (`resolve_peers`) vs lignes 177-208 (`resolve_all_peers`) — boucles identiques
- Lignes 105-116 vs 181-190 — probe_ports loop identique
- Lignes 118-124 vs 192-197 — os_health identique

**Fix :** Extraire `fn probe_node(&self, node: &DeclaredPeer) -> PeerEndpoint`.

### Duplication 2 : Probe OS health — `peer_resolver_mesh.rs` vs `os_health/observe.rs`

**Niveau de sévérité : CRITIQUE**

`MeshPeerResolver` réimplémente les fonctions `read_bool`, `read_available_mem_mib`, `read_reclaim_pressure_pct` (lignes 301-350) — le code est identique à `os_health/observe.rs` (lignes 65-121).

**Impact :** La logique de probe OS est dupliquée. Une mise à jour du probe (ex: nouveau seuil mémoire) risque de ne pas être répercutée.

**Preuve :** `peer_resolver_mesh.rs` ligne 301 : `fn read_bool(path: &str) -> bool` — identique à `os_health/observe.rs` ligne 65.

### Duplication 3 : MCP `local_tools.rs` vs `routers/local/fleet.rs`

**Niveau de sévérité : MOYENNE**

`run_federation_status()` est défini dans `local_tools.rs` (lignes 499-519) et semble également présent dans `routers/local/fleet.rs`. Il faut vérifier si c'est un `include!` (même code) ou une vraie duplication.

**Investigation :** `routers/local.rs` ligne 2 effectue `include!("local/fleet.rs")`. Si `local_tools.rs` a sa propre définition, c'est une vraie duplication.

### Duplication 4 : `os_health/observe.rs` vs `peer_resolver_mesh.rs` — `read_bool` conservateur

**Niveau de sévérité : FAIBLE (mais incohérent)**

- `os_health/observe.rs` ligne 68 : `read_bool` retourne `false` en cas d'erreur (conservateur "does not panic")
- `peer_resolver_mesh.rs` ligne 304 : `read_bool` retourne `false` — identique

C'est un commentaire contradictoire : `os_health/observe.rs` dit "conservative: assume 'does not panic'", mais `peer_resolver_mesh.rs` ligne 60 dit `if !softlockup_panic` (traite `false` comme anormal). Ces deux implémentations sont cohérentes mais la duplication cache un risque de divergence.

### Faiblesse 1 : `fallback_peers()` ne signe pas les endpoints

**Niveau de sévérité : CRITIQUE (sécurité)**

`fallback_peers()` (peer_resolver_mesh.rs ligne 140-169) retourne des `SignedPeerEndpoint` avec `signature_ed25519: String::new()` (vide).

**Impact :** Sous partition réseau, un attaquant pourrait forcer un nœud à utiliser un endpoint fallback malveable. L'absence de signature signifie qu'il n'y a aucune authentification cryptographique des pairs de secours.

**Contrôle :** Le trait `PeerResolver` (peer_resolver.rs) définit `fallback_peers() -> Vec<SignedPeerEndpoint>` — le type dit "signé" mais l'implémentation ne signe pas.

### Faiblesse 2 : `fleet_heartbeat.rs` orphelin

**Niveau de sévérité : MOYENNE**

Le schéma `FLEET_HEARTBEAT_SCHEMA` et les types dans `fleet_heartbeat.rs` ne sont utilisés par aucune surface active. La découverte fonctionne via probes TCP synchrones, pas via heartbeats distribués.

**Impact :** Un schéma de coordination distribuée existe mais n'est pas branché. Cela crée une confusion sur l'architecture réelle.

### Faiblesse 3 : Absence de mécanisme de cache distribué

**Niveau de sévérité : FAIBLE**

Aucune surface n'implémente de cache distribué pour la découverte ou la résolution de pairs. Chaque tick de `local_autonomy_runtime` répète les probes TCP.

**Impact :** Charge réseau inutile, latence accrue sous partition.

**Note :** `fleet_heartbeat.rs` définit déjà un schéma pour cela — il faudrait le brancher.

### Faiblesse 4 : `federation_runtime.rs` — `observed_at_unix_s: 0`

**Niveau de sévérité : FAIBLE**

Ligne 101 : `observed_at_unix_s: 0` — le timestamp est codé en dur à zéro dans `FederationRuntime::reconcile()`.

**Impact :** L'observation de federation n'a pas de timestamp réel, ce qui casse la preuve de fraîcheur.

**Preuve :** `federation_runtime.rs` ligne 99-101 :
```rust
let autonomy_obs = AutonomyObservation {
    schema: "cynic.local_autonomy_state.v1".to_string(),
    observed_at_unix_s: 0,  // ← HARDCODED
```

### Faiblesse 5 : `MeshPeerObserver` — `emergence_state: None` (TODO)

**Niveau de sévérité : MOYENNE**

`federation_runtime.rs` ligne 221 : `emergence_state: None, // TODO: build from gossip state`

**Impact :** La réconciliation de federation n'a pas accès à l'état d'émergence du mesh — les élections de rôle secondaires ne peuvent pas se faire via le runtime standard.

### Faiblesse 6 : CLI `fleet discover` requiert `--json`

**Niveau de sévérité : FAIBLE**

`fleet_surface.rs` ligne 53-55 :
```rust
FleetCommand::Discover { json, registry } => {
    if !json {
        return ExitCode::FAILURE;
    }
```

**Impact :** Aucun rendu humain pour `fleet discover` — doit toujours passer par `--json`. C'est une surface d'observation qui ne sert que pour la machine.

## 4. Recommandations Priorisées

### Priorité 1 : Refactor dupliqué OS probe (`peer_resolver_mesh.rs`)

**Action :**
1. Extraire `read_bool`, `read_available_mem_mib`, `read_reclaim_pressure_pct` depuis `peer_resolver_mesh.rs`
2. Utiliser `crate::os_health::observe::LocalOsHealthObserver` via injection
3. `MeshPeerResolver` devrait recevoir un `OsHealthObserver` (trait déjà défini dans `federation_runtime.rs`)

**Preuve attendue :**
- `cargo test -p cynic-host -- peer_resolver_mesh` passe
- `peer_resolver_mesh.rs` ne contient plus de fonctions dupliquées
- Code ligne < 280 (actuel: 389)

### Priorité 2 : Fusion `resolve_peers` / `resolve_all_peers`

**Action :**
1. Extraire `fn probe_node(&self, node: &DeclaredPeer) -> PeerEndpoint` dans `MeshPeerResolver`
2. `resolve_peers(role)` et `resolve_all_peers()` deviennent des wrappers
3. Ajouter tests de non-régression divergence

**Preuve attendue :**
- `cargo test -p cynic-host -- peer_resolver` — tous les tests existants passent + nouveau test "resolve_peers and resolve_all_peers return identical endpoints for same node"

### Priorité 3 : Signature des fallback endpoints

**Action :**
1. `fallback_peers()` doit soit ne plus retourner `SignedPeerEndpoint` (utiliser un type `UnsignedPeerEndpoint`), soit signer vraiment via `cynic_capability_issuance`
2. Rendre explicite dans le trait que les fallback ne sont pas signés (sécurité par l'opaque)

**Preuve attendue :**
- Pas de `SignedPeerEndpoint` avec `signature_ed25519: String::new()` dans le codebase (grep vérification)

### Priorité 4 : Brancher `fleet_heartbeat` ou le quarantine

**Action :**
1. Brancher le schéma `fleet_heartbeat` dans `fleet_discovery` (heartbeat comme source d'observation supplémentaire)
2. OU le quarantaine : déplacer `fleet_heartbeat.rs` dans un dossier `quarantined/` avec README expliquant l'abandon

**Preuve attendue :**
- `grep -r "FLEET_HEARTBEAT_SCHEMA" --include="*.rs"` : soit utilisé soit quarantaine

### Priorité 5 : Fix timestamp `observed_at_unix_s`

**Action :**
- Ligne 101 : remplacer `0` par `SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)`

**Preuve attendue :**
- Test : `reconcile()` retourne un `AutonomyObservation` avec `observed_at_unix_s > 0` en production

### Priorité 6 : Cache distribué pour la découverte

**Action :**
1. Ajouter un cache TTL (10s) dans `FleetDiscoveryAdapter` et `MeshPeerResolver`
2. Utiliser le schéma `fleet_heartbeat` pour la propagation gossip

**Preuve attendue :**
- Deux appels `discover()` consécutifs avec le même registry → second appel < 50ms (cache hit)

## 5. Résumé CAP par Surface

| Surface | C | A | P | Trade-off |
|---|---|---|---|---|
| `fleet_discovery` | WEAK | STRONG | PARTIAL | AP |
| `federation_runtime` | FORT (Coordinated) | FORT (Autonomous) | EXPLICITE | Configurable C↔A |
| `peer_resolver` | FAIBLE | STRONG | PARTIAL | AP (vulnérabilité signature) |
| `local_autonomy_runtime` | FAIBLE (locale) | STRONG | EXPLICITE | AP local |
| `build_node` | FORT (fail-closed) | STRONG (fail-closed) | N/A | C prioritaire |
| `registry.rs` | FORT | LIMITÉE | N/A | C > A |

## 6. Conclusion

**Point fort :** L'architecture hexagonale du `federation_runtime` (ports injectables + adapters + tests fakes) est exemplaire. Le modèle CAP est **explicite et configurable** — le runtime n'exerce pas C ou A dans le vide, il a un spectre de modes réagissant à l'état du réseau. C'est architecturalement correct.

**Point faible 1 :** La duplication de code OS-probe (`peer_resolver_mesh.rs` vs `os_health/observe.rs`) et le doublement de `resolve_peers`/`resolve_all_peers` sont des dettes techniques critiques qui risquent de diverger. Priorité absolue au refactor.

**Point faible 2 :** `fallback_peers()` retourne des `SignedPeerEndpoint` non signés — une faille de sécurité sous partition. Le type dit "signé" mais l'implémentation ne signe pas. Correction immédiate.

**Point faible 3 :** Le hardcodage `observed_at_unix_s: 0` et `emergence_state: None` sont des bugs logiques qui cassent la chaîne de preuve de fraîcheur.

L'approche globale est cohérente avec le théorème CAP : le système privilégie la disponibilité et la tolérance aux partitions (P+ A) dans les surfaces d'observation, tout en offrant la cohérence forte (C) au niveau de la source de vérité (registry) et de la décision de sélection (build_node fail-closed). La configuration CAP n'est pas exposée comme des flags, mais comme une machine à états (`FederationReconciliation`).
