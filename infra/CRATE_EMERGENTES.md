# Crates émergentes — grille de jugement (décisions validées)

STATUS: décisions de l'opérateur (2026-08-25) consolidées. Basé sur
CRATE_TOPOLOGY.md §5 + carto LOC (CRATE_MAP.md) + feedback opérateur:
  - intel = intelligence → tout raisonnement/ML/algorithmes y vit
  - cynic-pkg = OK mais RENOMMÉ proprement (single crate, pas le split
    cynic-ecosystem/cynic-tooling débile)
  - cynic-storage = crate dédié (stockage = autorité à part entière,
    approche data-centric)
  - cynic-network = crate dédié (modèle réseau actuel défaillant)

CRITÈRE (CRATE_MODELS.md §2): créer crate si famille COHÉRENTE avec
PRIMAIRE distincte. Sinon absorber.

================================================================
1. cynic-intel — INTELLIGENCE (élargi, décision opérateur)
================================================================
PRIMAIRE (NOUVEAU): tout ce qui touche au RAISONNEMENT, aux
ALGORITHMES, au ML, à l'évaluation épistémique. Pas seulement
wallet-judgment.

PREUVES (code réel):
  cynic-intel possède DÉJÀ:
    escore/ (aggregator/consensus/crystal/merkle/decay) 1024 LOC
    futarchy/ (cpmm/vault) 199 LOC
    rug_risk.rs 379 + rug_prefilter.rs 232
    wallet_judgment.rs 259 + wallet_enrichment.rs 242
    kscore.rs 318, token_postprocessor.rs 317, solana_safety_dog.rs 142
  → intel est DÉJÀ une crate d'évaluation épistémique + futarchique
    complète. Le nom "wallet judgment" est un sous-ensemble.

CE QUI DOIT Y VENIR (extract du kernel/host):
  - kernel/domain/wallet_judgment/ + rug_risk.rs + rug_prefilter.rs
    (DÉJÀ dupliqués dans intel → doublon #1 large, KERNEL_AREAS
    product_domain_residual)
  - kernel/domain/dogs/* (dog health/verdicts = ML eval) → intel
  - kernel/domain/ccm/ (credit monitoring = scoring) → intel
  - host/autoresearch/ (reflection/reasoning) → intel (raisonnement)
  - askesis RESTE à part: framework.json askesis_companion = companion
    reasoning isolé, pas le moteur ML central.

VERDICT: ÉLARGISSEMENT (pas nouvelle crate — intel existe, on y
  rappatre tout le raisonnement/ML du kernel/host).

================================================================
2. cynic-build + cynic-agent-workload — ABSORBER (inchangé)
================================================================
build(3.9k) + agent_workload(4.3k) = 82% de protocol, mais PUR
(types, pas d'I/O). Restent dans protocol. Re-juger si protocol
déborde après slice 0.

================================================================
3. cynic-pkg — CRATE DÉDIÉ, RENOMMÉ PROPREMENT (décision opérateur)
================================================================
SIGNAL (LU le terrain, pas supposé): host/domain/ecosystem_package/
= 4573 LOC:
  integration_* (apply/connect/plan/port/recipe/types) ~75% du module
    → INTÉGRATION D'ÉCOSYSTÈME (connecter/planifier/appliquer services)
  tooling_authority.rs 6138, python_tooling.rs 2475, execution.rs 1921
    → AUTORITÉ D'EXÉCUTION D'OUTILS (python, sudoers-like)
  probe.rs 7626, upstream_probe.rs 4371, registry.rs 5131
    → OBSERVATION UPSTREAM
  voisins: package_provenance.rs 354 (PackagePromotionPolicy),
    packages.rs 670 (PackagePolicy, activation plan)
    → PROVENANCE D'INSTALL (distro, promotion)

DONC le module est TROIS responsabilités mélangées:
  (a) intégration écosystème  (b) autorité tooling  (c) provenance pkg

DÉCISION: une SEULE crate (pas le split cynic-ecosystem/cynic-tooling
que j'avais proposé — débile). NOM PROPRE À CONFIRMER (operator):
  proposition: `cynic-pkg` (sa portée = integration + tooling
  authority + install provenance, AGENTS.md §4 install authority).
  Le module interne `ecosystem_package` sera renommé pour matéher
  (ex: `integration`, ou le crate porte directement ces modules).

POUR: AGENTS.md §4 exige une install authority explicite (canal,
  provenance, mutateur, read-back). Aujourd'hui noyé dans host/domain
  (4573 + 1024 voisins) sans frontière. Une crate force la modélisation.
VERDICT: CRATE (renommé proprement, scope = integration + tooling
  authority + install provenance).

================================================================
4. cynic-storage — CRATE DÉDIÉ (décision opérateur)
================================================================
POURQUOI (approche data-centric + SPOF):
  - storage-audit.json (F2): SurrealDB mélange ops mutable + ledger
    immuable signé, 28 tables, PoH. C'est LE SPOF critique.
  - Stockage mérite sa propre AUTORITÉ: rétention, réplication,
    intégrité — pas un module du kernel.
  - 9.2k kernel/storage + 3.4k host/storage_* = ~12.5k LOC de logique
    stockage → assez pour PRIMAIRE distincte.
  - Séparer permet d'appliquer l'audit SPOF (replicas, HA) SANS toucher
    au kernel.

NOTE: host/storage_* = OBSERVATION (pas doublon de kernel/storage =
  IMPL). Les deux vont dans cynic-storage: impl + observation host
  comme ports/sources.

VERDICT: CRATE (coordination store dédié, lié audit SPOF).

================================================================
5. cynic-network — CRATE DÉDIÉ (décision opérateur: modèle défaillant)
================================================================
POURQUOI (modèle réseau actuel DÉFAILLANT):
  - bin cynic-network-actuator DÉJÀ dans node (actuator_main.rs,
    sudoers, privileged helper) mais PAS une crate.
  - kernel tire mesh_registry/fleet_observability de node EN DUR
    (TOPOLOGY V4) → contrôle réseau ÉCLATÉ entre node (actuation) et
    kernel (consommation), sans PORT propre.
  - C'est le défaut kernel hot-spot appliqué au réseau.

VERDICT: CRATE (frontière nette: actuator privilégié + port de contrôle
  + observation, consommée par kernel/node via port). Nettoyer le
  Cargo de node pour héberger le bin dans la nouvelle crate.

================================================================
SYNTHÈSE (décisions validées)
================================================================

  CRATE                       VERDICT           ACTION
  -----------------------------------------------------------------
  cynic-intel (élargi)        ÉLARGISSEMENT     kernel wallet/rug/dogs/
                                               ccm + host autoresearch → intel
  cynic-build                 ABSORBER          reste protocol
  cynic-agent-workload        ABSORBER          reste protocol
  cynic-pkg (renommé)         CRATE              integration + tooling
                                               authority + provenance
  cynic-storage               CRATE              coordination store dédié
  cynic-network              CRATE              actuator + port + obs réseau
  inference::backend_health  EXTRAIRE           slice A (déjà prêt)

RÈGLE: une crate < 5k n'est pas auto-justifiée, sauf intention déjà
prouvée (network a son bin) ou autorité à part entière (storage/pkg
méritent leur crate par design, pas par taille).

================================================================
RÉFÉRENCES
================================================================

- infra/CRATE_TOPOLOGY.svg          diagramme couplages réels
- infra/CRATE_TOPOLOGY.md §5        crates émergentes (brut)
- infra/CRATE_MAP.md §2             host/domain (ecosystem_package 4573)
- infra/CRATE_MODELS.md §1,§2       grille SoC + forme décision
- contracts/framework.json          askesis_companion, protocol_is_pure
- AGENTS.md §4                      install authority (pkg provenance)
- infra/storage-audit.json          F2 (SurrealDB SPOF)
