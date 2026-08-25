# Crate Map — état actuel mesuré (observe, pas reconcile)

STATUS: données brutes + observation. Aucune décision de seuil LOC,
aucune voie d'extraction décidée ici. Sert de base de preuve pour
choisir la voie (voir infra/CRATE_OWNERSHIP.md §7).

MÉTHODE: `find crates/<c>/src -name '*.rs' -not -path '*/tests/*'
| xargs wc -l`. Mesuré 2026-08-25, branch agent/hermes/pattern-gc-l1.

================================================================
1. TOTAUX PAR CRATE (src/, hors tests)
================================================================

    84813  cynic-host
    58452  cynic-kernel
    51770  cynic-node
    50791  cynic-protocol
    29269  cynic-cli
    24162  cynic-inference
    8225  cynic-observer
    5753  cynic-askesis
    4236  cynic-intel
     399  cynic-zeroclaw-solana
    ------
   318119  TOTAL

RÉPARTITION: 4 crates > 50k (host/kernel/node/protocol) = 245k LOC
= 77% du total. Le "gros" n'est pas le kernel seul.

================================================================
2. HOST (84813) — breakdown par sous-domaine
================================================================

    domain              9304   dont ecosystem_package 4573 (49% du domain!)
    framework_bootstrap 9062   dont:
                              machine_enrollment 1022
                              propagation        1053
                              provider_enrollment 831
                              runtime_binding    676
                              lifecycle          998
                              source_snapshot    755
                              bundle             645
    registry            2714
    update_lifecycle    2841
    lifecycle           2859
    host_exec_cli       2161
    storage_census       819
    storage_capabilities 660
    storage_identity    1189
    storage_primary      509
    adapters            6020
    framework_bootstrap total 9062 (voir dessus)
    < 1500 chacun: agent_access, agent_runtime, audit_engine, bridge,
      container_runtime, coordination_lease_store, credential_hardening,
      detection_engine, execution, lab, lxc_exec_bridge, mcp_runtime,
      one_shot_authority, os_health, ports, principal_prepare,
      process_convergence, profiles, release_bot, runtime_seed,
      secret_lint, source_control, source_sync, space_gc, ssh_trust,
      storage_credential_sync, toolchain

OBSERVATION (pas décision):
  - ecosystem_package (4573) est un SOUS-DOMAINE unique qui domine le
    domain host. Est-ce "host substrate" ou "packaging logic" ? À
    classifier avant extraction.
  - framework_bootstrap (9062) = 7 sous-concerns de ~1k chacun. C'est
    une famille "bootstrap" cohérente ou 7 responsabilités distinctes ?
  - storage_* (5 modules, ~3366 total) = famille storage déjà identifiée
    dans kernel aussi → DOUBLON cross-crate possible.

================================================================
3. KERNEL (58452) — breakdown
================================================================

    api         12812
    domain      12786   dont ccm 2085, storage 1716, wisdom 295
    infra       12678   dont probes 2491, tasks 2065, config_loader 968,
                          judge 464, agent_session_sqlite 547
    storage      9230
    organ        2339
    backends     3103
    bootstrap    2004
    probe        1168
    introspection 885
    senses        700
    pipeline        4   (creux — voir ownership doc §heuristics)
    bin           176

OBSERVATION:
  - api(12.8k) + domain(12.8k) + infra(12.7k) + storage(9.2k) = 47.5k
    = 81% du kernel. Ce sont les 4 "piliers" déjà ciblés par
    KERNEL_AREAS (storage_domain_boundary, coordination_memory_core,
    runtime_composition, heuristics_and_data_pipeline).
  - ccm(2085) + storage(1716) dans domain = coordination store.

================================================================
4. NODE (51770) — breakdown
================================================================

    adapters    13650   dont registry 1109, le reste < 250/chacun
                         (headscale 211, wireguard 212, dns_renderer 133,
                          host 62, local_* 18-159, proxmox 9, surrealdb 117,
                          websocket_federation 163, provider_fallback 154)
    usecases    11455   dont build_scheduler 1663, le reste morcelé
    ports        2242
    domain       3256
    config       1085
    dns_activation 1224
    network_actuator 1183
    network_mesh_apply_plan 1297
    fleet_inventory 643
    watchdog      938
    websocket     685
    network_actuator_status 284, network_mesh_plan 140,
      network_root_policy 122, runtime_supervision 317,
      source_change_transaction 442, takeover_activation 91,
      agent_workload_delivery_uc 145

OBSERVATION:
  - adapters(13.6k) dominé par registry(1109) + une pléthore de petits
    adapters (<250). Pas un monolithe, une accumulation.
  - usecases(11.4k) dominé par build_scheduler(1663).
  - family "network_*" = 4 modules ~2.8k → cohérente, candidate à un
    crate cynic-network? Ou reste node ?

================================================================
5. PROTOCOL (50791) — breakdown
================================================================

    build           3926
    agent_workload  4324
    runtime         1076
    pattern_gc      1565
    convergence      947
    agent_session   1173
    provider         562
    agent_recursion  326
    agent_skill      380
    fleet_inference  413
    kyon            411
    agent_coordination 203
    development      275

OBSERVATION:
  - build(3926) + agent_workload(4324) = 82% du crate. Deux familles
    dominent. "protocol" est-il devenu un fourre-tout pour build +
    agent workload ? À challenger (foundation §481).

================================================================
6. INFERENCE (24162) / CLI (29269) / AUTRES
================================================================

inference:
    application  9425   dont agent_harness 1829
    domain       6273
    adapters     6232
    ports         602
    math          674
  → cohort équilibré (app/domain/adapters ~6-9k). Bonne forme.

cli:
    dev_surface       2225
    runtime_surface   4324   <- LE plus gros du cli
    chat_tui          2058
    bootstrap_surface 1806
    chat_surface      1212
    agent_surface      865
    surface_audit      840
    fleet_divergence   761
    network_surface    437
    agent_exposure_audit 660
    judgment_surface   166
    lifecycle_surface  185
  → runtime_surface(4324) domine = 15% du cli. Est-ce que la surface
    runtime porte trop de logique ? (framework §472 cli_is_adapter)

observer 8225 / askesis 5753 / intel 4236 / zeroclaw 399:
  → tous < 10k, sains.

================================================================
7. GRILLE D'OBSERVATION (responsabilité dominante → crate cible?
   PROPOSITION EN SUSPENS, pas décision)
================================================================

Ces propositions ne sont PAS des ordres d'exécution. Elles découlent
de la cartographie + KERNEL_AREAS existant. La voie se décide APRÈS
validation humaine de cette carte.

    RESPONSABILITÉ (LOC)          CRATE ACTUEL    CIBLE PROBABLE
    -----------------------------------------------------------------
    host ecosystem_package (4573)  cynic-host      ? (packaging vs host)
    host storage_* (~3366)         cynic-host      doublon kernel storage
    kernel ccm (2085)              cynic-kernel    cynic-protocol? askesis?
    kernel storage (1716+9230)     cynic-kernel    cynic-storage? (coordination)
    kernel organ (2339)            cynic-kernel    cynic-inference (DÉJÀ!)
    node network_* (~2800)         cynic-node      cynic-network? reste node?
    protocol build (3926)          cynic-protocol  crate build dédié?
    protocol agent_workload (4324) cynic-protocol  crate agent_workload?
    cli runtime_surface (4324)     cynic-cli       logique vs facade?

DOUBLONS CROSS-CRATE CONFIRMÉS (violation §481):
    - kernel organ/inference  vs cynic-inference (24k) → DÉJÀ cible
    - kernel storage  vs host storage_* → À confirmer (même nature?)
    - kernel fleet probes vs host fleet → DÉJÀ dans KERNEL_AREAS

================================================================
8. RÉSIDUEL (honnete)
================================================================

- Cartographie LOC ne dit PAS "qualité". Un fichier de 400 LOC peut
  être une table de données pure (sain) ou une fonction de 400 LOC
  (anti-pattern). Il faut le breakdown PAR FICHIER (hotspots) pour
  juger — disponible via `cynic dev crate-audit --json` (hotspots
  >400 LOC listés) une fois l'env rustc débloqué.
- "ecosystem_package" host = 49% du domain: à classifier (packaging
  ou host substrate?) avant toute extraction.
- "protocol build + agent_workload = 82%": à challenger si protocol
  doit rester "typed contract boundaries only" (framework §184).
- Aucun seuil LOC décidé. La voie d'extraction se déduit de CETTE
  carte + des doublons, pas d'un nombre arbitraire.

================================================================
RÉFÉRENCES
================================================================

- Mesure: find crates/<c>/src -name '*.rs' (hors tests) | wc -l
- infra/CRATE_OWNERSHIP.md  spec d'ownership (reconcile)
- crates/cynic-cli/src/kernel_boundary_audit.rs  KERNEL_AREAS (coded)
- contracts/foundation.json §481 one_boundary_one_reason
