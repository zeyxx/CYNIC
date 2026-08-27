# Crate Topology — structures émergentes (observe, pas reconcile)

STATUS: cartographie des dépendances inter-crates RÉELLES (pas
Cargo.toml déclaratif) + classification des couplages (port vs concret).
Suite de infra/CRATE_MAP.md (LOC) et infra/CRATE_MODELS.md (SoC).
Aucune décision de voie ici — voir infra/CRATE_OWNERSHIP.md §7.

MÉTHODE: grep des `use cynic_X::` / `cynic_X::` réels dans src/ (hors
tests), vs la déclaration Cargo.toml. Mesuré 2026-08-25.

================================================================
1. TOPOLOGIE DÉCLARÉE (Cargo.toml) vs RÉELLE (grep use)
================================================================

DÉCLARÉE (Cargo.toml):
  cynic-protocol   <- []              (base)
  cynic-intel      <- [protocol]
  cynic-inference  <- [protocol, intel]
  cynic-askesis    <- []
  cynic-host       <- [protocol, inference, intel, KERNEL?]
  cynic-node       <- [protocol, host, inference, KERNEL?]
  cynic-kernel     <- [host, inference, protocol, intel, node]
  cynic-observer   <- [host, intel, protocol]
  cynic-cli        <- [host, inference, node, observer, protocol]
  cynic-zeroclaw   <- [intel, protocol]

RÉELLE (grep use cynic_X — 0 = n'importe jamais la crate):
  host    -> kernel : 0 usage   (Cargo dit host<-kernel: FANTÔME)
  node    -> kernel : 0 usage   (Cargo dit node<-kernel: FANTÔME)
  kernel  -> host   : ~12 fichiers (RÉEL)
  kernel  -> inference: ~12 fichiers (RÉEL)
  kernel  -> node   : 2 fichiers (RÉEL)
  host    -> inference: 2 fichiers, TOUS via TRAITS (PROPRE)
  inference-> host  : 0 usage (inference n'importe pas host) PROPRE

DÉCOUVERTE: les dépendances Cargo.toml host<-kernel et node<-kernel
sont des FANTÔMES (0 usage réel). Le mot "kernel" dans host/node est
un champ de CONFIG (cfg.kernel.url, kernel_signature_hex), pas une
import de crate. À nettoyer du Cargo.toml (dette).

================================================================
2. COUPLAGES CLASSÉS (port vs concret)
================================================================

NOMBRE DE TRAITS (ports) PAR CRATE (grep ^pub trait, hors tests):
  host       74 traits  ← LE plus riche en ports (pattern établi)
  kernel     27 traits
  inference  24 traits
  protocol    7 traits

NOTE: le kernel a 27 traits mais n'en CONSOME presque aucun — il
importe du concret. Host = le modèle à suivre (74 traits, port-rich).

PROTOCOLE (idéal, respecte framework §468 protocol_is_pure):
  kernel -> protocol : TYPES PUGS (BuildRecipe, LeaseVerification,
    PatternModel). ✓ Aucun use de logique runtime protocol.

INFERENCE:
  host -> inference   : PORTS uniquement (ChatPort, TaskAdmissionPort).
                        reflection.rs:78, local_task_admission.rs:15. ✓
  kernel -> inference : STATIQUE CONCRET (LOGIQUE APP):
    - api/rest/inference_router.rs:7  fleet_routing, model_catalog,
      node_runtime_ops  (TOUTE la logique routing inference DANS kernel)
    - api/rest/inference_proxy.rs:17,36,51,74  proxy_routing::ProxyTarget
    - api/rest/inference_observation.rs:6  model_catalog::ModelProbeObservation
    - api/rest/types.rs:25,597  SlotTracker, SlotSemaphoreMap, ProxyTargets
    - infra/config_types.rs:3  backend_config (concret)
    -> kernel "fait" de l'inférence au lieu de déléguer un port. ✗

NODE:
  kernel -> node : STATIQUE CONCRET:
    - api/rest/event.rs:9,35  cynic_node::fleet_observability
      (project_fleet_stats, validate_event_identity) + re-export
    - api/rest/mesh.rs:4  cynic_node::mesh_registry::validate_mesh_key
    -> kernel tire fleet/mesh de node au lieu de port. ✗

HOST:
  kernel -> host : STATIQUE CONCRET:
    - domain/lifecycle_*.rs (4 fichiers) = `pub use cynic_host::
      lifecycle::*` (RE-EXPORT surface lifecycle host) → doublon
    - api/forge_task_contract.rs:6,15  lifecycle::forge_task_contract
    - infra/probes/network.rs:14  runtime_layout::current_runtime_layout
    - bootstrap/*  runtime_authority::default_runtime_authority_paths
    - infra/config_loader/helpers.rs:1  runtime_registry::resolve_*
    -> kernel consomme runtime authority de host directement. ✗
       (Modèle dit: host observe, kernel coordonne — ici kernel "pioche"
        dans host. Dépendance DESCENDANTE anormale.)

================================================================
3. STRUCTURE ÉMERGENTE CENTRALE
================================================================

Le kernel est le POINT CHAUD: il importe du CONCRET partout
(host runtime_registry/lifecycle, inference routing/proxy, node
fleet/mesh) au lieu de consommer des PORTS. C'est l'incarnation du
"tous ses œufs dans le même panier" au niveau topologique: le kernel
est devenu un AGRÉGATEUR qui tire la logique de tous les autres crates.

    protocol (pur) ──────┐
        ↑                 │
    intel (pur)           │
        ↑                 │
    inference ──ports──► host  (host consomme inference PROPREMENT ✓)
        ↑                 │
        └────concret──── kernel (kernel tire inference/node/host EN DUR)
                          │
                   node (tiré par kernel en dur)

GRAPHE DE COUPLAGE (flèche = importe du concret):
  kernel ──► host      (12 fichiers, lifecycle/registry/authority)
  kernel ──► inference (12 fichiers, routing/proxy/catalog CONCRET)
  kernel ──► node      (2 fichiers, fleet_observability/mesh REGISTRY)
  host   ──► inference (2 fichiers, PORTS uniquement ✓)
  Tout   ──► protocol  (types purs ✓)

================================================================
4. VIOLATIONS ÉMERGENTES (vs CRATE_MODELS.md §1)
================================================================

V1. kernel dépend CONCRET de host (runtime authority) — devrait être
    un port host exposé à kernel. Inversion de couplage.
V2. kernel re-exporte host::lifecycle (domain/lifecycle_*.rs = pub use)
    — doublon de surface, au-delà du doublon organ (slice A).
V3. kernel importe la LOGIQUE APPLICATION inference (routing/proxy/
    catalog) au lieu d'un port — c'est le doublon #1 vu large.
V4. kernel importe fleet_observability/mesh_registry de node en dur.
V5. Dépendances Cargo.toml host<-kernel / node<-kernel = FANTÔMES
    (0 usage) — dette de nettoyage, à retirer.

Ces V1-V4 sont le MÊME défaut: kernel ne consomme pas de ports
externes, il importe du concret. C'est la cible structurelle de
l'extraction C (ton choix), pas seulement les doublons de types.

================================================================
5. CRATES ÉMERGENTES (familles cohérentes sans crate dédiée)
================================================================

Aucune crate orpheline: toute dep Cargo pointe vers une crate réelle
(cynic-network-actuator est un [[bin]] de node, pas une dep — erreur
de lecture initiale corrigée). MAIS des frontières IMPLICITES émergent
comme candidates canoniques (CRATE_MODELS.md §2: créer crate si famille
cohérente):

  FAMILLE              LOC    HÉBERGÉE DANS      SIGNAL D'ÉMERGENCE
  --------------------------------------------------------------------
  network (actuator/     ~3k    cynic-node        DÉJÀ un bin séparé
    plan/mesh/policy)                     cynic-network-actuator
                                         (actuator_main.rs) →
                                         frontière conceptualisée,
                                         pas encore crate
  build                3.9k    cynic-protocol    82% du crate avec
                                         agent_workload →
                                         candidate cynic-build
  agent-workload       4.3k    cynic-protocol    idem → candidate
                                         cynic-agent-workload
  ecosystem-package    4.5k    cynic-host/domain packaging isolable
                                         → candidate cynic-pkg?
  backend-health       ~1.3k   kernel organ →   slice A déjà planifié
                                 inference       → inference::backend_health

NOTE: le bin cynic-network-actuator prouve que "network" était prévu
comme surface autonome dans le modèle. Ces crates ne manquent pas par
oubli — ce sont des frontières IMPLICITES pas encore matérialisées.

================================================================
6. INFERENCE (24k) — cible slice 1 + slice 4
================================================================

INFERENCE = crate d'inférence runtime. 24 traits (ports) — pattern établi.

PORTS EXISTANTS (24 traits, 13 dans ports/ + application):
  inference_port.rs: BackendPort, InferPort
  chat_port.rs: ChatPort (extends BackendPort)
  embedding_port.rs: EmbeddingPort
  model_registry_port.rs: ModelRegistryPort, BackendCatalogPort
  model_store_port.rs: ModelStorePort
  chat_session_port.rs: ChatSessionStore
  model_publisher_port.rs: ModelPublisherPort
  hf_hub_port.rs: HfHubPort
  local_runtime_port.rs: LocalModelInventoryPort, LocalInferenceRuntimePort
  hermes_runtime_port.rs: HermesGatewayRuntimePort
  filesystem_capacity_port.rs: FilesystemCapacityPort
  empirical_store_port.rs: EmpiricalStorePort
  summarization_port.rs: SummarizationPort
  task_admission_port.rs: TaskAdmissionPort
  agent_harness/ports.rs: CoordinationPort, HarnessExecutor, HarnessObserver
  agent_harness/runtime_adapter.rs: AgentRuntimePort
  chat_service.rs: ChatCompletionClient, ToolInvoker
  adapters/local/process_control.rs: ProcessControlPort

CIBLE SLICE 1 (kernel organ backend-health → inference):
  Surface existante VÉRIFIÉE: domain/backend_identity.rs (33 LOC)
    BackendHealthSnapshot, BackendIdentity, BackendCapability, BackendStatus
  → Le kernel Backend/BackendHealth/BackendId (organ/registry.rs) doit
    soit fusionner dans backend_identity (BackendHealthSnapshot existe déjà),
    soit l'étendre. JAMAIS créer un module backend_health parallèle.
  Surface: domain/slot_semaphore.rs (SlotSemaphoreMap, SlotTracker ré-exportés
    dans lib.rs:97) — le kernel les importe déjà en concret (types concrets,
    pas un port). Slice 4 doit créer SlotManagementPort au-dessus.

CIBLE SLICE 4 (kernel routing/proxy → inference PORTS):
  Surfaces existantes VÉRIFIÉES:
    application/proxy_routing.rs (260 LOC): ProxyTargets, ProxyTarget,
      SelectedProxyTarget, OperationIntent, select_proxy_target()
    application/fleet_routing/: fleet routing logic
    application/routing_policy.rs: select_backend()
    application/model_selection.rs: select_model()
  → Créer InferenceProxyPort (au-dessus proxy_routing) +
    InferenceRoutingPort (au-dessus fleet_routing/routing_policy) +
    SlotManagementPort (au-dessus slot_semaphore). Le kernel consomme
    ces 3 traits au lieu des types concrets.

================================================================
7. RÉSIDUEL (honnete)
================================================================

- "port vs concret" classé par grep use; un trait d'inference importé
  en tant que TYPE (ex: EmbeddingPort à types.rs:24) est OK (trait),
  mais SlotTracker (concret) à types.rs:25 ne l'est pas.
- Certains `cynic_node::` peuvent être des ports déguisés en types —
  à confirmer par lecture de node::fleet_observability signature.
- Cargo.toml fantôme host<-kernel / node<-kernel: à vérifier que
  retirer ne casse RIEN (grep dit 0 usage). À faire sur builder non-ICE.
- Aucune décision de voie ici. Les V1-V5 + crates émergentes §5 +
  inference §6 alimentent CRATE_OWNERSHIP.md §7 (plan d'extraction)
  une fois validés.

================================================================
RÉFÉRENCES (preuves grep)
================================================================

- host<-kernel/node<-kernel fantômes: Cargo.toml vs grep use cynic_kernel
- kernel->host: domain/lifecycle_*.rs (pub use), api/forge_task_contract.rs:6,
  infra/probes/network.rs:14, bootstrap/*, infra/config_loader/helpers.rs:1
- kernel->inference: api/rest/inference_router.rs:7, inference_proxy.rs:17/36/51/74,
  inference_observation.rs:6, types.rs:25/597, config_types.rs:3
- kernel->node: api/rest/event.rs:9/35, mesh.rs:4
- host->inference: autoresearch/reflection.rs:78, adapters/local_task_admission.rs:15
- contracts/framework.json §462 substrate_behind_ports, §468 protocol_is_pure
- infra/CRATE_MODELS.md §1 grille PRIMARY/OWNS/DELEGATES/FORBIDDEN
