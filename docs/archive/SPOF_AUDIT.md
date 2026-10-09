# SPOF Audit — Points de défaillance uniques CYNIC

STATUS: audit des points de défaillance uniques (Single Points of
Failure) identifiés dans le runtime CYNIC. Basé sur le code live (pas
les docs), mesuré 2026-08-25. Chaque SPOF = impact + preuve file:line +
état de la remédiation.

MÉTHODE: lecture du code kernel/host/node + grep des endpoints hardcodés
+ vérification de l'absence de réplication/failover.

================================================================
1. SurrealDB = SPOF de stockage et de coordination (CRITIQUE)
================================================================

IMPACT: si l'endpoint SurrealDB tombe, TOUT s'arrête — coordination,
preuves cryptographiques (PoH), leases, audit, agent sessions.

PREUVES:
- crates/cynic-kernel/src/storage/client.rs:56 → init(config) prend UN
  StorageConfig avec un seul `url`. PAS de liste de replicas.
- crates/cynic-kernel/src/storage/surreal/coord.rs → TOUTE la
  coordination (claims, leases, verify, finalize, audit, PoH, heartbeat)
  est implémentée PAR SurrealHttpStorage. Un seul writer.
- crates/cynic-kernel/src/storage/reconnectable/reconnector.rs:62 →
  try_reconnect() repointe vers LE MÊME url. C'est du "reconnect",
  pas du failover. Zéro réplication.
- crates/cynic-kernel/src/bootstrap/storage.rs:39 → si init échoue,
  fallback NullStorage (mode dégradé, UnavailableCoord). Pas de quorum.
- infra/storage-audit.json:51-60 → 28 tables, PoH + ops mutable mélangés
  dans la même DB (F2: deux natures incompatibles, une seule histoire
  backup/rétention).

ÉTAT REMÉDIATION: aucune réplication active. Le skill cynic-infra-
diagnostics mentionne "secondary SurrealDB replica on different LXC/
subnet" comme mitigation, mais non implémentée.

================================================================
2. Kernel single-process (port 3030) = SPOF d'orchestration
================================================================

IMPACT: si le binaire kernel s'éteint (OOM, panic, host reboot),
il n'y a pas de reprise. Le watchdog constate, il ne failover pas.

PREUVES:
- crates/cynic-kernel/src/main.rs:7 → `#[tokio::main]` un seul process.
- crates/cynic-kernel/src/bootstrap/server.rs:94 → UN TcpListener + UN
  UnixListener. Pas de multi-instance, pas de VIP/HA.
- crates/cynic-node/src/watchdog/kernel_bridge.rs → le watchdog ne fait
  QUE observer /health et POST une evidence. Aucune bascule vers un
  kernel secondaire. kernel_primary est un rôle unique.
- crates/cynic-kernel/src/domain/constants.rs:13 → DEFAULT_REST_ADDR
  = "127.0.0.1:3030" (loopback, pas de VIP).

ÉTAT REMÉDIATION: aucune. Le watchdog (cynic-node) observe mais ne
redémarre pas un kernel secondaire.

================================================================
3. Forgejo (10.10.10.101) = SPOF de vérité git (SSOT)
================================================================

IMPACT: pas de Forgejo = pas de SSOT candidate, pas de bootstrap de
build reproductible, pas de coordinated commit.

PREUVES:
- infra/registry.json:137 → forgejo = node unique 10.10.10.101.
- infra/storage-audit.json:95 → "Forgejo = SSOT candidate, GitHub relay".
- Le push mirror GitHub existe (crates/cynic-host/src/source_sync/
  reconcile.rs:232 teste forgejo_ahead → push github) mais c'est un
  relay à 1 semaine de lag, pas un failover actif.

ÉTAT REMÉDIATION: mirror GitHub passif (1 semaine de lag). Pas de
failover automatique.

================================================================
4. StorageConfig.url unique = SPOF de configuration
================================================================

IMPACT: un seul endpoint storage hardcodé. Pas de bascule possible
sans redémarrer le kernel avec une nouvelle config.

PREUVES:
- crates/cynic-kernel/src/storage/client.rs:77-83 → init_with(url, ns, db)
  prend un seul url. Pas de Vec<String> de replicas.
- Le reconnecteur (reconnector.rs) ne pointe que vers ce même url.

ÉTAT REMÉDIATION: aucune. Le pattern pour remédier = StorageConfig.urls:
Vec<String> + leader election (voir CRATE_VOIE slice 0 + audit SPOF).

================================================================
5. PatternAnalyzer /events = SPOF de boucle de reconnexion
================================================================

IMPACT: après un restart du kernel, le PatternAnalyzer (qui s'abonne
au flux SSE /events du kernel) peut tourner en boucle de reconnexion
pendant 5-6 min (premier cycle d'introspection).

PREUVES:
- skill cynic-infra-diagnostics: "[PatternAnalyzer] /events stream
  closed" (repeated every ~30s), digestion_starvation alert.
- Cause: le broadcast channel event_tx peut ne pas émettre avant le
  premier cycle d'introspection (60s warmup + 5min interval).

ÉTAT REMÉDIATION: wait-for-first-cycle (passif). Pas de HA kernel pour
éviter le restart.

================================================================
6. Bootstrap Chain = SPOF de confiance
================================================================

IMPACT: si le bootstrap (boot.rs, runtime_takeover) échoue, le kernel
ne peut pas démarrer en mode dégradé avec une identité valide.

PREUVES:
- crates/cynic-kernel/src/bootstrap/storage.rs:33-61 →
  connect_storage_runtime() = point d'entrée unique. Si SurrealDB est
  injoignable au boot → NullStorage + UnavailableCoord (mais le boot
  continue en dégradé, pas de retry bloquant).
- Le bootstrap chain (machine enrollment → storage → coord) est
  séquentiel, pas de quorum.

ÉTAT REMÉDIATION: mode dégradé (NullStorage) mais pas de véritable
retry/election.

================================================================
RÉSUMÉ DES SPOFs
================================================================

  #   SPOF                        CRITIQUE  REMÉDIATION
  ----------------------------------------------------------------
  1   SurrealDB (storage+coord)   OUI       aucune (replica prévu)
  2   Kernel single-process       OUI       aucune (watchdog passif)
  3   Forgejo (git SSOT)          OUI       mirror GitHub 1-semaine
  4   StorageConfig.url unique    OUI       aucune
  5   PatternAnalyzer /events     NON       wait-for-cycle
  6   Bootstrap chain             NON       mode dégradé NullStorage

RECOMMANDATION (ordre):
  P0 — StorageConfig.urls: Vec<String> + leader election (brise #1 et #4).
  P1 — Kernel: healthcheck actif + bascule watchdog vers kernel_secondary.
  P1 — Storage: séparer la nature "ledger immuable" (PoH) de "ops mutable"
       dans deux DB (déjà demandé par storage-audit F2).
  P2 — Forgejo: promouvoir le mirror GitHub en failover à RPO court.

================================================================
RÉFÉRENCES
================================================================

- crates/cynic-kernel/src/storage/client.rs:56-131
- crates/cynic-kernel/src/storage/reconnectable/reconnector.rs:57-106
- crates/cynic-kernel/src/bootstrap/storage.rs:33-82
- crates/cynic-kernel/src/main.rs:7
- crates/cynic-kernel/src/bootstrap/server.rs:94
- crates/cynic-node/src/watchdog/kernel_bridge.rs
- infra/registry.json:137
- infra/storage-audit.json:51-60, 95
- infra/CRATE_TOPOLOGY.md (couplages kernel)
- infra/CRATE_VOIE.md (slices 0-6, remédiation)
- skill cynic-infra-diagnostics (PatternAnalyzer, recovery paths)
