# Crate Models — ownership & Separation of Concerns (canonique)

STATUS: spec d'ownership canonique par crate. Source de vérité pour
peupler `infra/surface-registry.json` et valider chaque extraction
(voir infra/CRATE_OWNERSHIP.md §7, infra/CRATE_MAP.md cartographie).

Ce document ne crée PAS de règle. Il assemble les règles DÉJÀ dans
`contracts/framework.json` + `contracts/foundation.json` en un modèle
d'ownership applicable à chaque crate, et fixe la forme de décision
quand on hésite entre "absorber" ou "créer un crate".

RÈGLES SOURCES (framework.json):
  modularity_rules:
    - protocol_is_pure (§468): cynic-protocol = schémas + règles pures,
      ne lit pas le host, ne mute pas le runtime.
    - cli_is_adapter (§472): cynic-cli câble in/out, ne possède pas
      l'orchestration.
    - proof_producers_are_low_layer (§476): host/node produisent les
      preuves, les couches hautes consomment.
    - one_boundary_one_reason (§481): ajouter crate/module/adapter/
      contrat SEULEMENT si une frontière existante ne peut pas absorber
      proprement la responsabilité.
  agnosticity_rules:
    - no_provider_in_domain (§454): types domaine sans cloud/hypervisor/
      LLM/DNS/host spécifique.
    - n_case_by_registry (§458): nouveau host/fleet/provider = registry
      data ou adapter config, pas branchement de code.
    - substrate_behind_ports (§462): systemd/k8s/lxc/ssh/rest/mcp/shell
      = adapters derrière ports portables.
  non_goals (§489):
    - "centralize all responsibilities in one orchestrator crate" INTERDIT.

================================================================
1. MODÈLE CANONIQUE PAR CRATE
================================================================

Chaque crate = UNE responsabilité primaire + ports vers le reste.
Forme: PRIMAIRE / POSSEDÉ / DÉLÉGUÉ / INTERDIT.

--- cynic-protocol (schémas + règles pures) ---
  PRIMAIRE: types, ports, policy pure, FrameworkSpec.
  POSSEDÉ: contrats de coordination (lease, federation), build, agent
           workload, runtime planning (PURES, pas d'I/O host).
  DÉLÉGUÉ: lecture host/mutation runtime → host/node.
  INTERDIT: lire le host, muter le runtime, posséder de l'I/O réseau
           réel (§468 protocol_is_pure).
  NOTE: build extrait vers cynic-build (2026-09-15, commits 0f6feae0+
        hexagonalisation 5c73f59b). Reste agent_workload (~4.9k): extraire
        vers cynic-workload dédié — PAS le noyer dans protocol.

--- cynic-host (host substrate + runtime authority) ---
  PRIMAIRE: observation host (disk/mem/cpu/net per AGENTS.md §125),
            bootstrap, lifecycle, forgejo, build runner, fleet health.
  POSSEDÉ: storage_* OBSERVATION (capabilities/census/identity),
           registry, update_lifecycle, runtime authority.
  DÉLÉGUÉ: preuves consommées par kernel/node; inference runtime →
           cynic-inference.
  INTERDIT: posséder la logique d'inférence (→ inference), posséder le
           coordination ledger (→ kernel).
  NOTE: 85k LOC. ecosystem_package(4573) à classifier: packaging vs
        host substrate? Si packaging → crate cynic-pkg ou reste host?

--- cynic-node (node-local agent tree + watchdog) ---
  PRIMAIRE: agent tree local, observation plan, build submission,
            watchdog, network control plane (node scope).
  POSSEDÉ: adapters (registry/headscale/wireguard/federation), usecases
           (build_scheduler), fleet_inventory (node-scoped).
  DÉLÉGUÉ: coordination globale → kernel; host substrate → host.
  INTERDIT: devenir un 2e kernel (coordination mémoire).
  NOTE: network_* (4 modules ~2.8k) cohérents → candidat crate
        cynic-network? Ou reste node si node = "network formation"?

--- cynic-kernel (coordination + memory + federation membrane) ---
  PRIMAIRE: coordination authority (leases/fencing), bootstrap,
            federation membrane, introspection, PoH anchor.
  POSSEDÉ: storage/surreal (coordination store), coord, event
           persistence, organ ORGANISM (domain::organ::OrganPort =
           introspection/senses — COEUR, NON extractible).
  DÉLÉGUÉ: inference backend health/registry → cynic-inference (voir
           slice A, infra/CRATE_OWNERSHIP.md §doublon1).
  INTERDIT: posséder l'inférence runtime (organ::InferenceOrgan →
           inference), posséder le host lifecycle (→ host).
  NOTE: organ a DEUX sens. `crate::organ::InferenceOrgan` (registry
        Dogs) = DOUBLON → inference. `domain::organ::OrganPort`
        (organisme/introspection) = COEUR kernel, reste.

--- cynic-inference (inference runtime) ---
  PRIMAIRE: model runtime, engine selection, profile bootstrap,
            backend registry/health, routing, slot/capacity.
  POSSEDÉ: adapters (local/process/systemd/hf/openai), application
           (chat/matrix/gateway), domain (backend_identity, queue,
           slot_semaphore), ports.
  DÉLÉGUÉ: host substrate → host; coordination → kernel.
  INTERDIT: posséder la coordination mémoire (→ kernel), posséder le
           host lifecycle (→ host).
  NOTE: DÉJÀ cible d'extraction kernel organ/ (KERNEL_AREAS
        inference_stack priority 1). backend_health types du kernel
        viennent ici (slice A).

--- cynic-cli (thin facade) ---
  PRIMAIRE: parse → inject ports → call crate API → render JSON/human.
  POSSEDÉ: surfaces (runtime_surface, dev_surface, chat_surface...).
  DÉLÉGUÉ: toute politique métier → crate métier.
  INTERDIT: posséder de la policy de phase, du classification de
           findings, du state schema (§472 cli_is_adapter).
  NOTE: runtime_surface(4324) domine le cli. Est-ce facade ou logique
        qui a fui? À auditer (peut-être slice vers crate métier).

--- cynic-build (pipeline de build souveraine, bounded context hexagonal) ---
  PRIMAIRE: admission, ordonnancement, preuve et publication des builds.
  POSSEDÉ: types (types.rs), ports pilotés BuildRunner/FilesystemPort/
           IdentityPort (ports.rs), décisions du runner (runner.rs),
           adapters OS + fake mémoire (adapters/), contrats purs
           build/*, release_publication, runtime_release.
  DÉLÉGUÉ: exécution des commandes → BuildRunner (implémenté par
           cynic-lifecycle); substrate host → host; coordination → kernel.
  INTERDIT: I/O ou spawn dans types/runner/lib (std::fs, Command
           réservés aux adapters/), dépendre de host/kernel/node/cli/mcp.
  NOTE: extrait de cynic-protocol 2026-09-15. lib.rs = facade fine,
        ratchet architecture_boundary.rs pin l'hexagone.

--- cynic-release (moteur de décision release & relais vitrine) ---
  PRIMAIRE: évaluation et décision pure d'éligibilité release et de relay.
  POSSEDÉ: types purs de décision (ReleaseVersion, ReleaseEligibilityVerdict,
           RelayDecisionProof), évaluation des contrats machine release-gates,
           relay-policy, release-versioning, staging-model.
  DÉLÉGUÉ: exécution des gates de compilation/test/artefacts → cynic-build;
           transport git, tags et push → cynic-vcs; observation hôte → cynic-host;
           façade opérateur → cynic-cli.
  INTERDIT: std::process::Command, I/O réseau direct, muter le filesystem,
           dépendre de host/kernel/node/cli/mcp (DAG strictement descendant).

--- cynic-observer / cynic-askesis / cynic-intel / cynic-zeroclaw-solana ---
  PRIMAIRE: observer = solana ingestion; askesis = companion reasoning
           isolé (framework askesis_companion); intel = INTELLIGENCE
           (tout raisonnement / ML / algorithmes / évaluation);
           zeroclaw = solana adapters.
  NOTE intel ÉLARGI (décision opérateur): kernel wallet_judgment /
           rug_* / dogs / ccm + host autoresearch appartiennent à intel
           (raisonnement/ML), pas au kernel/host. Voir CRATE_VOIE §slice6.
  POSSEDÉ: leur domaine étroit.
  INTERDIT: devenir des "mini-kernels" (tout-en-un).

================================================================
2. FORME DE DÉCISION (absorber vs créer crate)
================================================================

Quand on hésite, appliquer §481 one_boundary_one_reason dans l'ordre:

  (1) La responsabilité rentre-t-elle dans une PRIMAIRE existante
      (§1) SANS la diluer? → absorber dans ce crate.
  (2) Sinon, est-ce une FAMILLE cohérente qui mérite sa PRIMAIRE?
      → nouveau crate canonical (nom = cynic-<domaine>).
  (3) Sinon, est-ce un adapter/port d'un domaine existant? → module
      dans le crate qui possède le port (jamais nouveau crate).

INTERDIT de créer un crate "utilitaire fourre-tout" (ex: cynic-common)
— ça recrée la violation §489. Les types partagés vont dans le crate
qui POSSÈDE la responsabilité (ex: backend health → inference, pas un
crate shared).

================================================================
3. GRILLE DE SYNCHRO (machine-readable)
================================================================

Pour chaque crate, `infra/surface-registry.json` doit porter:
  owns:        [responsabilités primaires]
  delegates:   [crate:port orienté sortie]
  forbidden:   [ce qui viole §481/§489/§468/§472]

Cette entrée EST la projection de §1. Le `cynic dev ownership-audit`
et `crate-audit` la consomment comme attendu (voir
CRATE_OWNERSHIP.md §6 — les 3 audits sont DÉJÀ codés).

================================================================
4. RÉSIDUEL (honnete)
================================================================

- Le seuil "une responsabilité = un crate" est un idéal; en pratique
  un crate a 1 PRIMAIRE + des modules satellites (ports/adapters). La
  règle est: pas de DEUXIÈME PRIMAIRE non reliée.
- ecosystem_package(host) et network_*(node) sont des cas limites à
  trancher (voir CRATE_MAP.md §2/§4) — pas encore décidés ici.
- Ce doc est cité par AGENTS.md "Crate ownership & extraction"; il est
  donc autorité, pas prose périssable.

================================================================
RÉFÉRENCES
================================================================

- contracts/framework.json §454/§458/§462/§468/§472/§476/§481/§489
- contracts/foundation.json §481 one_boundary_one_reason
- infra/CRATE_OWNERSHIP.md  spec extraction + slice A (doublon organ)
- infra/CRATE_MAP.md        cartographie LOC mesurée
- AGENTS.md §125 canonical ownership per family
- crates/cynic-cli/src/kernel_boundary_audit.rs  KERNEL_AREAS (coded)
