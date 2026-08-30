# CYNIC — Atlas d'ownership (one-page executive summary)

STATUS: synthèse exécutive des 6 docs détaillés. Usage: entrée unique
pour nouveau contributeur ou rappel rapide. Chaque chiffre renvoie à
une preuve file:line dans les docs détaillés (liens en bas).

================================================================
1. CHIFFRES CLÉS
================================================================

  318 119 LOC total  (src/, hors tests, mesuré 2026-08-25)
  10 crates
  4 crates > 50k LOC = 77% du total (host 85k > kernel 58k > node 52k > protocol 51k)
  61 traits (ports) dans host + kernel + inference + protocol
  31 couplages CONCRETS sortant du kernel (host 12 + inference 18 + node 11)
  0 couplage concret entrant dans host ou node (Cargo.toml fantôme)
  6 crates émergentes identifiées (4 décidées, 2 en suspens)

================================================================
2. TOPOLOGIE (qui tire du concret de qui)
================================================================

  protocol (7 traits, 0 dep) = base pure
       ↑
  intel (1 dep: protocol)
       ↑
  inference (24 traits, protocol + intel)
       ↑
  host (74 traits, protocol + inference + intel) ← modèle port-rich
       ↑
  kernel (27 traits, host + inference + protocol + intel + node) ← HOT SPOT
       ↑
  node (host + inference + protocol)

  kernel → host      12 fichiers CONCRET  (runtime_registry, lifecycle, authority)
  kernel → inference 18 fichiers CONCRET  (routing, proxy, slots, catalog)
  kernel → node      11 fichiers CONCRET  (fleet_observability, mesh_registry)
  host   → inference  2 fichiers PORT     (ChatPort, TaskAdmissionPort) ← le modèle

  DÉFAUT CENTRAL: le kernel importe du CONCRET au lieu de consommer
  des ports. Host (74 traits) montre que le pattern est possible.

================================================================
3. ÉMERGENTES (décisions opérateur 2026-08-25)
================================================================

  CRATE                       VERDICT           ACTION
  -----------------------------------------------------------------------
  cynic-network              CRATE              bin existe, modèle défaillant
  cynic-storage              CRATE              ~12.5k, data-centric + SPOF
  cynic-pkg (renommé)        CRATE              integration + tooling + provenance
  cynic-intel                ÉLARGISSEMENT      tout raisonnement/ML y vit
  cynic-build                ABSORBER           reste protocol (pur)
  cynic-agent-workload       ABSORBER           reste protocol (pur)
  inference::backend_health  EXTRAIRE           slice A prêt

================================================================
4. VOIE D'EXTRACTION (slices, ordre risque croissant)
================================================================

  0. Nettoyage Cargo fantôme (sans risque)
  1. organ backend-health → inference (slice A, plan prêt)
  2. lifecycle re-exports → host LifecyclePort
  3. runtime_registry/authority → host ports
  4. routing/proxy/slots → InferenceRoutingPort + InferenceProxyPort + SlotManagementPort
  5. fleet_observability/mesh → node ports
  6. Matérialiser: network, storage, pkg + élargissement intel

  Chaque slice = pattern CYNIC complet + gardes-fou json verts + commit isolé.
  Précondition: inventaire des surfaces cibles AVANT tout code (pas de doublon).

================================================================
5. GARDES-FOU EXISTANTS (déjà codés)
================================================================

  cynic dev kernel-audit --json     KERNEL_AREAS (14 zones, schema v1)
  cynic dev ownership-audit --json  11 surfaces ownership (schema v1)
  cynic dev crate-audit --json      LOC/hotspots/entrypoints (schema v1)

================================================================
6. DOCS DÉTAILLÉS (liens)
================================================================

  CRATE_OWNERSHIP.md    spec d'ownership systémique (foundation §481/§489)
  CRATE_MODELS.md       grille SoC canonique (PRIMARY/OWNS/DELEGATES/FORBIDDEN)
  CRATE_MAP.md          LOC mesurés par crate/sous-domaine
  CRATE_TOPOLOGY.md     couplages réels + V1-V5 + inference + émergentes
  CRATE_TOPOLOGY.svg    diagramme visuel (couches + flèches port/concret)
  CRATE_EMERGENTES.md    grille de jugement + décisions opérateur
  CRATE_VOIE.md         slices 0-6 + précondition surfaces cibles
  SPOF_AUDIT.md          audit des points de défaillance uniques (SurrealDB, kernel, Forgejo)

================================================================
7. BLOQUANTS EXÉCUTION
================================================================

  - rustc 1.96 ICE (SIGSEGV libc/syn) sur cette VM
  - Builder souverain 10.10.10.119 injoignable
  → Toute exécution cargo attend un env sain. Plans prêts dans .hermes/plans/.

================================================================
RÉFÉRENCES
================================================================

- contracts/foundation.json §481 one_boundary_one_reason, §489 no centralize
- contracts/framework.json §462/§468/§472/§476
- AGENTS.md "Crate ownership & extraction"
