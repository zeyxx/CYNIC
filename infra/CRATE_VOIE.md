# Carte de voie — extraction & ownership (reconcile, pas code)

STATUS: synthèse des 3 cartos (LOC / Topologie / Modèle SoC) en une
VOIE d'extraction proposée. Observe déjà fait (CRATE_MAP / TOPOLOGY /
MODELS). Ici = reconcile → propose l'ordre. Aucun code dans ce doc.

PRINCIPE DE VOIE (bottom-up, un couplage à la fois):
  Le défaut central n'est PAS la taille (LOC) — c'est le kernel qui
  importe du CONCRET partout (CRATE_TOPOLOGY §3-4: V1-V4). Chaque slice
  DOIT inverser un couplage kernel→concret derrière un PORT, selon le
  modèle CRATE_MODELS.md §1 (host/node/inference exposent des ports,
  kernel consomme). Pas extraire des LOC pour le plaisir.

PATTERN PAR SLICE (CYNIC coerant):
  desired: la responsabilité a un owner unique (Modèle §1)
  observe: grep use cynic_X (déjà fait, preuves dans TOPOLOGY)
  reconcile: décider port vs concret (Modèle §2)
  bounded action: déplacer l'impl, kernel garde un use de PORT
  read-back: `cynic dev crate-audit --json` + tests verts

================================================================
VOIE PROPOSÉE (ordre par risque croissant / ROI)
================================================================

SLICE 0 — NETTOYAGE (sans risque, préalable)
  - Cargo.toml fantôme host<-kernel / node<-kernel (0 usage réel,
    TOPOLOGY §1). Retirer du Cargo.toml.
  - Vérif: cargo check (builder non-ICE) ne casse rien.
  ROI: clarifie la topo, 0 logique touchée.

SLICE 1 — kernel/organ backend-health → inference (DÉJÀ PLANIFIÉ)
  - Plan: .hermes/plans/slice-a-organ-to-inference.md
  - Type: doublon #1 (TOPOLOGY V3 large). Risque MIN (types purs +
    re-export, kernel<-inference déjà une dep).
  - Garde-fou: tests organ/mod.rs:369 restent verts.

SLICE 2 — kernel lifecycle re-exports → host PORT
  - kernel domain/lifecycle_*.rs = `pub use cynic_host::lifecycle::*`
    (TOPOLOGY V2). Créer `host::lifecycle::LifecyclePort` (trait),
    kernel consomme le port au lieu du re-export.
  - Risque: moyen (couplage surface, mais déjà un re-export = facile
    à remplacer par un trait).

SLICE 3 — kernel→host runtime_registry/runtime_authority → PORT
  - kernel importe concret runtime_registry/authority (TOPOLOGY V1).
    Host expose `RuntimeRegistryPort` / `RuntimeAuthorityPort`.
  - Risque: moyen (bootstrap/* touché — à tracer main.rs).

SLICE 4 — kernel→inference routing/proxy/catalog → PORT
  - kernel importe LOGIQUE APP inference (TOPOLOGY V3):
    inference_router.rs (fleet_routing/model_catalog/node_runtime_ops),
    inference_proxy.rs (proxy_routing). Créer
    `inference::application::InferenceRoutingPort`, kernel consomme.
  - Risque: ÉLEVÉ (le kernel fait du routing — gros couplage). À faire
    APRÈS slice 1-3 pour avoir le pattern de ports rodé.

SLICE 5 — kernel→node fleet_observability/mesh → PORT
  - kernel importe concret node (TOPOLOGY V4): event.rs (fleet_obs),
    mesh.rs (mesh_registry). Node expose `FleetObservabilityPort` /
    `MeshRegistryPort`.
  - Risque: moyen.

SLICE 6 — CRATES ÉMERGENTES (matérialiser les frontières implicites)
  DÉCISIONS OPÉRATEUR (2026-08-25) consolidées dans CRATE_EMERGENTES.md:
  - cynic-network: CRATE. bin cynic-network-actuator existe déjà dans
    node mais le modèle réseau est DÉFAILLANT (kernel tire mesh_registry/
    fleet_observability de node en dur, V4). Créer crate = frontière
    nette (actuator privilégié + port contrôle + observation).
  - cynic-storage: CRATE. ~12.5k LOC (kernel 9.2k + host 3.4k) = autorité
    stockage à part entière (approche data-centric + SPOF SurrealDB
    F2). Séparer permet réplication HA sans toucher kernel.
  - cynic-pkg: CRATE, RENOMMÉ PROPREMENT (pas le split cynic-ecosystem/
    cynic-tooling débile). Scope = integration écosystème + tooling
    authority + install provenance (AGENTS.md §4). Le module
    ecosystem_package (4573 LOC) est 3 responsabilités mélangées →
    une seule crate propre porte ces modules.
  - cynic-intel: ÉLARGISSEMENT (pas nouvelle crate). intel = INTELLIGENCE
    → tout raisonnement/ML/algorithmes y vit. Rappatrier du kernel/host:
    wallet_judgment/, rug_*, dogs/, ccm/, host/autoresearch/. askesis
    RESTE à part (companion reasoning isolé per framework).
  - protocol build/agent_workload: ABSORBER (restent protocol, purs).
  Risque slice 6: faible (familles isolées, extraction mécanique).

================================================================
DÉCISIONS REQUISES (résolues par opérateur 2026-08-25):
  D1 storage: TRANCHÉ → cynic-storage crate dédié (data-centric + SPOF).
  D2 ecosystem-package: TRANCHÉ → cynic-pkg (renommé propre, scope =
     integration + tooling authority + install provenance).
  D3 seuil LOC: CRATE_OWNERSHIP.md §6 ACTION RESTANTE — ajouter
     blocage explicite >40k dans crate_audit. À faire une fois slice 0
     passée (env rustc OK).

================================================================
GARDES-FOU (ne pas régression)
================================================================

À chaque slice:
  - `cynic dev kernel-audit --json`  (status blocked→ok à la fin)
  - `cynic dev ownership-audit --json` (migration_targets résolus)
  - `cynic dev crate-audit --json` (hotspots跟踪)
  - tests unitaires de la zone touchée verts
  - commit isolé par slice (AGENTS.md git isolation)

ENV BLOCKANT: rustc 1.96 SIGSEGV sur libc/syn sur cette VM. Toute
exécution (cargo check/test) doit aller sur builder souverain
(10.10.10.119) ou après ICE résolu. Les docs de carto ne dépendent
pas du compile.

================================================================
RÉSUMÉ POUR VALIDATION
================================================================

Le kernel est le point chaud (importe concret de host/inference/node).
La voie = inverser SES couplages un par un derrière des ports, du plus
sûr (slice 0 nettoyage, slice 1 types) au plus lourd (slice 4 routing).
Les crates émergentes (§5) se matérialisent APRÈS, une fois le kernel
désagrégé et le modèle rodé.

Cette voie N'EST PAS un grand rewrite. C'est N slices de ~1 couplage,
chacune read-back prouvée. Conforme AGENTS.md + foundation §481/§489.

================================================================
RÉFÉRENCES
================================================================

- infra/CRATE_MAP.md        LOC mesurés
- infra/CRATE_TOPOLOGY.md   couplages réels, V1-V5, crates émergentes
- infra/CRATE_MODELS.md     grille SoC canonique + forme décision
- infra/CRATE_OWNERSHIP.md  plan extraction + slice A
- .hermes/plans/slice-a-organ-to-inference.md  plan slice 1 prêt
- contracts/framework.json §462/§468/§481/§489
- AGENTS.md "Crate ownership & extraction"
