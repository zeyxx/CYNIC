use serde::Serialize;

<<<<<<< HEAD
use crate::capability_responsibility::{
    CapabilityResponsibilityCatalog, capability_responsibility_catalog,
};

=======
>>>>>>> github/main
pub const FRAMEWORK_SPEC_SCHEMA: &str = "cynic.framework_spec.v1";

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkSpec {
    pub schema: &'static str,
    pub status: &'static str,
    pub identity: FrameworkIdentity,
    pub operating_model: Vec<FrameworkPrinciple>,
<<<<<<< HEAD
    pub ambition_translation: Vec<FrameworkAmbitionTranslation>,
    pub canonical_layers: Vec<FrameworkLayer>,
    pub vertical_contracts: Vec<FrameworkVerticalContract>,
    pub responsibility_contracts: CapabilityResponsibilityCatalog,
=======
    pub canonical_layers: Vec<FrameworkLayer>,
    pub vertical_contracts: Vec<FrameworkVerticalContract>,
>>>>>>> github/main
    pub primitive_families: Vec<FrameworkPrimitiveFamily>,
    pub target_use_cases: Vec<FrameworkUseCase>,
    pub agnosticity_rules: Vec<FrameworkRule>,
    pub modularity_rules: Vec<FrameworkRule>,
    pub non_goals: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkIdentity {
    pub name: &'static str,
    pub purpose: &'static str,
    pub short_definition: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPrinciple {
    pub id: &'static str,
    pub rule: &'static str,
}

<<<<<<< HEAD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkAmbitionClass {
    Core,
    UpperLayer,
    ExternalAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkAmbitionStatus {
    Absent,
    Partial,
    Proved,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkAmbitionTranslation {
    pub id: &'static str,
    pub classification: FrameworkAmbitionClass,
    pub current_owner: Vec<&'static str>,
    pub runtime_status: FrameworkAmbitionStatus,
    pub translated_primitives: Vec<&'static str>,
    pub forbidden_substitutions: Vec<&'static str>,
    pub required_proof: Vec<&'static str>,
}

=======
>>>>>>> github/main
#[derive(Debug, Clone, Serialize)]
pub struct FrameworkLayer {
    pub id: &'static str,
    pub responsibility: &'static str,
    pub owned_by: Vec<&'static str>,
    pub evidence: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkVerticalContract {
    pub id: &'static str,
    pub desired_state: &'static str,
    pub observer: &'static str,
    pub reconciliation: &'static str,
    pub bounded_action: &'static str,
    pub read_back_proof: &'static str,
    pub owner: Vec<&'static str>,
    pub coverage: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPrimitiveFamily {
    pub id: &'static str,
    pub role: &'static str,
    pub examples: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkUseCase {
    pub id: &'static str,
    pub description: &'static str,
    pub required_primitives: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkRule {
    pub id: &'static str,
    pub rule: &'static str,
}

pub fn framework_spec() -> FrameworkSpec {
    FrameworkSpec {
        schema: FRAMEWORK_SPEC_SCHEMA,
        status: "normative",
        identity: FrameworkIdentity {
            name: "CYNIC",
            purpose: "Build a sovereign, agentic, OS-like runtime framework from portable low-level primitives.",
            short_definition: "CYNIC is a Rust-first organism runtime: registries declare desired state, host/node/kernel crates observe real state, protocol crates score evidence, and adapters expose bounded commands to humans and agents.",
        },
        operating_model: vec![
            FrameworkPrinciple {
                id: "bottom_up_runtime_truth",
                rule: "Runtime claims start from host, service, network, storage, and kernel read-back; prose cannot promote a layer to healthy.",
            },
            FrameworkPrinciple {
                id: "desired_observed_reconcile",
                rule: "Every orchestration loop must compare desired state, observed state, and the next safe action.",
            },
            FrameworkPrinciple {
                id: "agent_as_bounded_workload",
                rule: "An agent session is a workload with declared surface, evidence, and validation; it is not an unlimited authority.",
            },
            FrameworkPrinciple {
                id: "node_local_fractal_agents",
                rule: "Every node may host a recursive agent tree whose child authority and budgets only decrease; cross-node delegation is explicit federation.",
            },
            FrameworkPrinciple {
<<<<<<< HEAD
                id: "property_and_domain",
                rule: "Every capability has one accountable owner for state, lifecycle, recovery, retention, and proof, plus an explicit domain owner for semantics and invariants; delegation grants bounded execution and never transfers ownership.",
            },
            FrameworkPrinciple {
=======
>>>>>>> github/main
                id: "kubernetes_like_not_kubernetes_bound",
                rule: "CYNIC may use Kubernetes-like semantics, but the domain model must also run on one host, systemd, VMs, LXC, or another substrate.",
            },
        ],
<<<<<<< HEAD
        ambition_translation: vec![
            FrameworkAmbitionTranslation {
                id: "sovereign_membrane",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-host", "cynic-node"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec![
                    "typed crossings between independently authoritative planes",
                    "identity, capability, policy, adapter, and proof boundaries",
                    "truth-upward and intent-downward propagation",
                ],
                forbidden_substitutions: vec![
                    "one global controller as owner of every source of truth",
                    "a shared database presented as proof of external reality",
                    "projection-layer policy replacing lower-layer contracts",
                ],
                required_proof: vec![
                    "one request crossing source, build, deployment, and runtime planes with linked lower-layer read-back evidence",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "askesis_companion",
                classification: FrameworkAmbitionClass::UpperLayer,
                current_owner: vec!["cynic-askesis"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "content-addressed context assembly",
                    "source-visible suggestion, decision, and action separation",
                    "reversible private memory policy",
                ],
                forbidden_substitutions: vec![
                    "raw chat history as durable memory or authority",
                    "provider session state as user identity",
                    "an omnipotent agent with implicit mutation rights",
                ],
                required_proof: vec![
                    "a companion session reconstructed from evidence refs that performs a bounded delegated action and remains usable after provider replacement",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "accessible_personal_ml",
                classification: FrameworkAmbitionClass::UpperLayer,
                current_owner: vec!["cynic-inference", "cynic-askesis"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "task-oriented ML ports independent of provider APIs",
                    "typed data authority, consent, retention, cost, and evaluation",
                    "operator recipes with observable inputs and outputs",
                ],
                forbidden_substitutions: vec![
                    "a provider-specific API presented as a universal ML contract",
                    "simplicity that hides provenance, consent, cost, or uncertainty",
                    "automatic training or publication from merely observable data",
                ],
                required_proof: vec![
                    "a non-expert completes one personal-data ML task through the same port with two interchangeable adapters and receives provenance and evaluation evidence",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "sovereign_inference",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-inference"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec![
                    "portable inference capability ports",
                    "local and remote runtime adapters",
                    "model revision, resource, cost, and output evidence",
                ],
                forbidden_substitutions: vec![
                    "local-only execution as the definition of sovereignty",
                    "one model, engine, accelerator, or provider embedded in policy",
                    "successful generation presented as evaluated truth",
                ],
                required_proof: vec![
                    "the same typed inference request runs through local and remote adapters with bounded secrets, measured resources, and comparable output evidence",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "talaria_code_evolution",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["source", "build", "admission", "deployment", "outcome"],
                runtime_status: FrameworkAmbitionStatus::Blocked,
                translated_primitives: vec![
                    "typed evolution hypothesis and attributed source mutation",
                    "isolated workspace, reproducible build, and comparative conformance",
                    "independent admission, canary, outcome read-back, and rollback",
                ],
                forbidden_substitutions: vec![
                    "Talaria writing directly into an authoritative runtime worktree",
                    "a code-producing agent approving its own authority increase",
                    "tests optimized as a substitute for measured runtime outcomes",
                ],
                required_proof: vec![
                    "a Talaria-produced CYNIC change traverses workspace, source authority, build, admission, canary, outcome comparison, and rollback without bypass",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "opt_in_disclosure",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec![
                    "unowned: target cynic-protocol policy plus publication adapters",
                ],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "typed data classification and disclosure proposal",
                    "explicit consent or pre-authorized publication policy",
                    "minimization, redaction, commitment, publication, and read-back",
                ],
                forbidden_substitutions: vec![
                    "technical observability treated as permission to publish",
                    "blanket consent without subject, purpose, audience, or expiration",
                    "raw private evidence copied into a public proof",
                ],
                required_proof: vec![
                    "one private event is minimized under an inspectable consent policy, published through an adapter, read back, and linked without leaking the source payload",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "solana_public_settlement",
                classification: FrameworkAmbitionClass::ExternalAuthority,
                current_owner: vec!["unowned: target CYNIC Solana adapter"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "offline-first idempotent commitment plan",
                    "wallet authority separated from runtime machine identity",
                    "transaction submission, confirmation, and account read-back",
                ],
                forbidden_substitutions: vec![
                    "transaction inclusion presented as proof of an off-chain fact",
                    "one RPC, indexer, wallet, or SDK embedded in domain policy",
                    "Solana availability required for private local operation",
                ],
                required_proof: vec![
                    "a consented evidence commitment survives RPC loss, is replayed idempotently, confirmed on Solana, and linked to its private source proof",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "futard_audience_projection",
                classification: FrameworkAmbitionClass::ExternalAuthority,
                current_owner: vec!["Futard.io integration", "CYNIC publication port"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "public-safe evidence and decision projection",
                    "audience interaction imported as sourced events",
                    "projection read-back without ownership of private runtime truth",
                ],
                forbidden_substitutions: vec![
                    "a Futard.io page or database as CYNIC runtime authority",
                    "audience engagement presented as factual outcome",
                    "market visibility used as implicit disclosure consent",
                ],
                required_proof: vec![
                    "a minimized CYNIC publication is displayed and read back through Futard.io while private evidence and authority remain locally verifiable",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "one_person_futarchy",
                classification: FrameworkAmbitionClass::UpperLayer,
                current_owner: vec![
                    "Talaria decision loop",
                    "CYNIC evidence contracts",
                    "CYNIC publication contracts",
                ],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "typed thesis, alternatives, metric, resolver, and time window",
                    "pre-decision commitment and bounded execution evidence",
                    "post-decision outcome observation, dispute, and policy update",
                ],
                forbidden_substitutions: vec![
                    "market price or popularity presented as universal success",
                    "a market outcome presented as proof that execution occurred",
                    "an unverifiable KPI selected after the decision",
                ],
                required_proof: vec![
                    "one personal decision links a precommitted resolution rule to bounded execution, independently observed outcome, resolution, and subsequent policy update",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "sovereign_runtime",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-host", "cynic-node"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec![
                    "runtime principal",
                    "portable host ports",
                    "bounded mutation",
                    "read-back proof",
                ],
                forbidden_substitutions: vec![
                    "operator-home execution presented as installed authority",
                    "provider control plane presented as organism authority",
                ],
                required_proof: vec![
                    "fresh-node standalone bootstrap",
                    "steady-state execution without HumanAdmin secrets",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "closed_evolution",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-host", "cynic-node", "cynic-kernel"],
                runtime_status: FrameworkAmbitionStatus::Blocked,
                translated_primitives: vec![
                    "source claim",
                    "build admission",
                    "artifact release",
                    "runtime proof",
                ],
                forbidden_substitutions: vec![
                    "commit presented as deployment",
                    "successful build presented as runtime convergence",
                ],
                required_proof: vec![
                    "intent-to-source-to-build-to-release-to-runtime closure",
                    "rollback and final claim release",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "contestable_judgment",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-kernel", "cynic-askesis"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec![
                    "evidence envelopes",
                    "proof bundles",
                    "unknown states",
                ],
                forbidden_substitutions: vec![
                    "kernel interpretation presented as primary truth",
                    "single validator result presented as unchallengeable fact",
                ],
                required_proof: vec![
                    "versioned judge identity and rule",
                    "challenge, correction, and judgment lineage",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "multi_source_truth",
                classification: FrameworkAmbitionClass::ExternalAuthority,
                current_owner: vec!["cynic-protocol"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec![
                    "evidence refs",
                    "registry intent",
                    "runtime observation",
                ],
                forbidden_substitutions: vec![
                    "runtime observation overriding legal, protocol, Git, or market authority",
                    "untyped evidence ref presented as verified primary evidence",
                ],
                required_proof: vec![
                    "typed source authority and claim binding",
                    "source-specific verification and freshness",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "resource_metabolism",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-host", "cynic-node"],
                runtime_status: FrameworkAmbitionStatus::Blocked,
                translated_primitives: vec!["runtime budgets", "host metrics"],
                forbidden_substitutions: vec![
                    "unbounded cache growth",
                    "cleanup without protected-artifact classification",
                ],
                required_proof: vec![
                    "resource admission before mutation",
                    "bounded reclamation and capacity read-back",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "replayable_memory",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-kernel", "cynic-askesis"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec![
                    "content-addressed evidence",
                    "runtime proof history",
                    "persistence records",
                ],
                forbidden_substitutions: vec![
                    "chat history presented as organism memory",
                    "append-only accumulation without retention or replay policy",
                ],
                required_proof: vec![
                    "schema-aware replay",
                    "reference-aware retention, compaction, and migration",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "node_recursion",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-node", "cynic-kernel"],
                runtime_status: FrameworkAmbitionStatus::Blocked,
                translated_primitives: vec![
                    "delegation envelope",
                    "completion envelope",
                    "monotonic budgets",
                ],
                forbidden_substitutions: vec![
                    "prompt-only delegation",
                    "child authority greater than parent authority",
                ],
                required_proof: vec![
                    "durable scheduler with aggregate sibling budgets",
                    "cancellation, recovery, and completion evidence",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "fleet_coordination",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-node", "cynic-kernel"],
                runtime_status: FrameworkAmbitionStatus::Blocked,
                translated_primitives: vec!["registry placement", "node proof", "lease fencing"],
                forbidden_substitutions: vec![
                    "declared node presented as live node",
                    "single-node proof presented as fleet rollout",
                ],
                required_proof: vec![
                    "two-node canary with rollback",
                    "aggregate rollout history and residual unknown budget",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "runtime_federation",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-node", "cynic-kernel"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec!["peer identity", "reduced delegation authority"],
                forbidden_substitutions: vec![
                    "LAN reachability presented as federation",
                    "shared root credential presented as peer trust",
                ],
                required_proof: vec![
                    "explicit cross-node transport and trust contract",
                    "remote completion evidence bound to peer identity",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "operator_federation",
                classification: FrameworkAmbitionClass::UpperLayer,
                current_owner: vec!["future operator and protocol adapters"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec!["node roles", "runtime metrics"],
                forbidden_substitutions: vec![
                    "runtime federation presented as economic decentralization",
                    "payment or slashing without published service proof rules",
                ],
                required_proof: vec![
                    "external operator admission and dispute process",
                    "measured service, payment, and slashing receipts",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "generalized_ownership",
                classification: FrameworkAmbitionClass::Core,
                current_owner: vec!["cynic-protocol", "cynic-host"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec![
                    "Unix principal ownership",
                    "workspace claims",
                    "artifact authority",
                ],
                forbidden_substitutions: vec![
                    "filesystem owner presented as policy authority",
                    "resource without named duties, budget, and receipts",
                ],
                required_proof: vec![
                    "typed owner, custodian, authority, duties, and retention",
                    "ownership transfer and read-back evidence",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "decision_outcome_loop",
                classification: FrameworkAmbitionClass::UpperLayer,
                current_owner: vec!["cynic-protocol", "cynic-kernel"],
                runtime_status: FrameworkAmbitionStatus::Partial,
                translated_primitives: vec!["outcome spec", "KPI score", "runtime observations"],
                forbidden_substitutions: vec![
                    "activity presented as outcome",
                    "plan or artifact existence presented as adoption or value",
                ],
                required_proof: vec![
                    "hypothesis, metric, deadline, execution, and observation window",
                    "falsification and policy consequence",
                ],
            },
            FrameworkAmbitionTranslation {
                id: "public_legibility",
                classification: FrameworkAmbitionClass::UpperLayer,
                current_owner: vec!["cynic-cli", "future product projection"],
                runtime_status: FrameworkAmbitionStatus::Absent,
                translated_primitives: vec!["JSON reports", "proof refs", "explicit unknowns"],
                forbidden_substitutions: vec![
                    "UI presented as runtime proof",
                    "private build activity presented as public usage",
                ],
                required_proof: vec![
                    "inspectable claim bound to primary source",
                    "forbidden reading and public action",
                ],
            },
        ],
=======
>>>>>>> github/main
        canonical_layers: vec![
            FrameworkLayer {
                id: "l0_protocol",
                responsibility: "Schemas, pure planning rules, proof envelopes, KPI scoring, and framework contracts.",
                owned_by: vec!["cynic-protocol"],
                evidence: vec![
                    "cynic.framework_spec.v1",
                    "cynic.outcome_spec.v1",
                    "cynic.kpi_score.v1",
                ],
            },
            FrameworkLayer {
                id: "l1_host",
                responsibility: "Local OS substrate observation and bounded preparation.",
                owned_by: vec!["cynic-host"],
                evidence: vec!["host_identity_observed", "service_supervisor_observation"],
            },
            FrameworkLayer {
                id: "l2_node_network",
                responsibility: "Node, mesh, DNS, provider fallback, and network runtime proof.",
                owned_by: vec!["cynic-node"],
                evidence: vec!["network_runtime_observed"],
            },
            FrameworkLayer {
                id: "l3_kernel_memory",
                responsibility: "Persist observations, coordination, runtime proof history, and task memory.",
                owned_by: vec!["cynic-kernel"],
                evidence: vec!["kernel_health_observed", "runtime_proof_persisted"],
            },
            FrameworkLayer {
                id: "l4_agent_exposure",
                responsibility: "Expose backed capabilities to agents without duplicating domain authority.",
                owned_by: vec!["cynic-mcp", "cynic-cli"],
                evidence: vec!["agent_surface_projected", "cynic.cli.surface_manifest.v1"],
            },
        ],
        vertical_contracts: vec![
<<<<<<< HEAD
            FrameworkVerticalContract {
                id: "process",
                desired_state: "principal, executable, capabilities",
                observer: "process and identity probes",
                reconciliation: "admit only declared principal and capability subset",
                bounded_action: "start or stop one declared process",
                read_back_proof: "pid, uid, executable digest, health",
                owner: vec!["cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "principal",
                desired_state: "runtime user, groups, secret placement",
                observer: "OS identity and filesystem probes",
                reconciliation: "derive ownership and privilege gaps",
                bounded_action: "create or repair declared principal-owned paths",
                read_back_proof: "owner, group, mode, secret presence",
                owner: vec!["cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "filesystem",
                desired_state: "declared paths, modes, atomicity",
                observer: "filesystem and mount observer",
                reconciliation: "classify absent, mismatched, read-only, unknown",
                bounded_action: "atomic write, install, or rollback of one path",
                read_back_proof: "stat, digest, fsync/read-back",
                owner: vec!["cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "source",
                desired_state: "repository, worktree, lease, revision",
                observer: "Git and worktree observer",
                reconciliation: "admit only fenced agent-scoped changes",
                bounded_action: "staged commit or provider push",
                read_back_proof: "revision, diff, lease, remote evidence",
                owner: vec!["cynic-protocol", "cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "build",
                desired_state: "toolchain, inputs, reproducible command",
                observer: "build and dependency observer",
                reconciliation: "block drift or unproven inputs",
                bounded_action: "run canonical repository gate",
                read_back_proof: "exit status, input digest, artifact digest",
                owner: vec!["cynic-host", "cynic-cli"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "artifact",
                desired_state: "versioned runtime basket and activation target",
                observer: "install layout and manifest observer",
                reconciliation: "select only attested compatible artifact",
                bounded_action: "stage, atomically switch, or rollback",
                read_back_proof: "manifest, checksum, active symlink, version",
                owner: vec!["cynic-host", "cynic-node"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "service",
                desired_state: "supervisor, unit, health policy",
                observer: "service supervisor observer",
                reconciliation: "separate installed, active, healthy, and proven",
                bounded_action: "bounded start, stop, reload, or repair",
                read_back_proof: "service state and health probe",
                owner: vec!["cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "host_os",
                desired_state: "OS policy and declared root capabilities",
                observer: "host substrate and privilege observer",
                reconciliation: "preserve unknown and reject unscoped root work",
                bounded_action: "root helper argv from allowlisted policy",
                read_back_proof: "postflight host observation and rollback intent",
                owner: vec!["cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "network",
                desired_state: "interfaces, routes, firewall, connectivity",
                observer: "network surface and root firewall observer",
                reconciliation: "distinguish management, secondary, and default paths",
                bounded_action: "one explicit network profile through helper",
                read_back_proof: "routes, firewall, connectivity, rollback",
                owner: vec!["cynic-host", "cynic-node"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "node",
                desired_state: "node identity, runtime basket, capabilities",
                observer: "node-local observation producer",
                reconciliation: "admit node only from registry plus read-back",
                bounded_action: "node-local plan or bounded activation",
                read_back_proof: "node observation and proof bundle",
                owner: vec!["cynic-node"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "provider",
                desired_state: "provider resource and control capability",
                observer: "provider API or hypervisor adapter",
                reconciliation: "use provider truth without leaking provider policy",
                bounded_action: "one declared provider mutation",
                read_back_proof: "provider-side resource read-back",
                owner: vec!["cynic-node", "cynic-host"],
                coverage: "unknown",
            },
            FrameworkVerticalContract {
                id: "orchestrator",
                desired_state: "workload, placement, policy, identity",
                observer: "orchestrator API and workload observer",
                reconciliation: "respect orchestrator authority and scope",
                bounded_action: "namespace/node/workload-scoped action",
                read_back_proof: "API conditions and workload health",
                owner: vec!["cynic-node", "cynic-mcp"],
                coverage: "unknown",
            },
            FrameworkVerticalContract {
                id: "project",
                desired_state: "project governance profile and source authority",
                observer: "project registry and repository observer",
                reconciliation: "bind generic framework policy to project adapters",
                bounded_action: "project-scoped change or release",
                read_back_proof: "project proof bundle and revision",
                owner: vec!["cynic-protocol", "cynic-host"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "fleet",
                desired_state: "placement, rollout, fencing, residual unknown budget",
                observer: "fleet inventory and coordination observer",
                reconciliation: "build node-scoped plans and bounded rollout",
                bounded_action: "canary or staged node action",
                read_back_proof: "node proofs, lease events, rollout history",
                owner: vec!["cynic-kernel", "cynic-node"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "federation",
                desired_state: "peer identity, delegation, transport, completion",
                observer: "transport and coordination observer",
                reconciliation: "separate local recursion from node federation",
                bounded_action: "delegated action within reduced authority",
                read_back_proof: "peer identity and completion envelope",
                owner: vec!["cynic-protocol", "cynic-kernel"],
                coverage: "partial",
            },
            FrameworkVerticalContract {
                id: "human_boundary",
                desired_state: "consent, bootstrap, breakglass policy",
                observer: "authorization and audit observer",
                reconciliation: "keep HumanAdmin out of steady state",
                bounded_action: "explicit approval or incident breakglass",
                read_back_proof: "authorization event and resulting proof",
                owner: vec!["cynic-host", "cynic-mcp"],
                coverage: "partial",
            },
        ],
        responsibility_contracts: capability_responsibility_catalog(),
=======
            FrameworkVerticalContract { id: "process", desired_state: "principal, executable, capabilities", observer: "process and identity probes", reconciliation: "admit only declared principal and capability subset", bounded_action: "start or stop one declared process", read_back_proof: "pid, uid, executable digest, health", owner: vec!["cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "principal", desired_state: "runtime user, groups, secret placement", observer: "OS identity and filesystem probes", reconciliation: "derive ownership and privilege gaps", bounded_action: "create or repair declared principal-owned paths", read_back_proof: "owner, group, mode, secret presence", owner: vec!["cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "filesystem", desired_state: "declared paths, modes, atomicity", observer: "filesystem and mount observer", reconciliation: "classify absent, mismatched, read-only, unknown", bounded_action: "atomic write, install, or rollback of one path", read_back_proof: "stat, digest, fsync/read-back", owner: vec!["cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "source", desired_state: "repository, worktree, lease, revision", observer: "Git and worktree observer", reconciliation: "admit only fenced agent-scoped changes", bounded_action: "staged commit or provider push", read_back_proof: "revision, diff, lease, remote evidence", owner: vec!["cynic-protocol", "cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "build", desired_state: "toolchain, inputs, reproducible command", observer: "build and dependency observer", reconciliation: "block drift or unproven inputs", bounded_action: "run canonical repository gate", read_back_proof: "exit status, input digest, artifact digest", owner: vec!["cynic-host", "cynic-cli"], coverage: "partial" },
            FrameworkVerticalContract { id: "artifact", desired_state: "versioned runtime basket and activation target", observer: "install layout and manifest observer", reconciliation: "select only attested compatible artifact", bounded_action: "stage, atomically switch, or rollback", read_back_proof: "manifest, checksum, active symlink, version", owner: vec!["cynic-host", "cynic-node"], coverage: "partial" },
            FrameworkVerticalContract { id: "service", desired_state: "supervisor, unit, health policy", observer: "service supervisor observer", reconciliation: "separate installed, active, healthy, and proven", bounded_action: "bounded start, stop, reload, or repair", read_back_proof: "service state and health probe", owner: vec!["cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "host_os", desired_state: "OS policy and declared root capabilities", observer: "host substrate and privilege observer", reconciliation: "preserve unknown and reject unscoped root work", bounded_action: "root helper argv from allowlisted policy", read_back_proof: "postflight host observation and rollback intent", owner: vec!["cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "network", desired_state: "interfaces, routes, firewall, connectivity", observer: "network surface and root firewall observer", reconciliation: "distinguish management, secondary, and default paths", bounded_action: "one explicit network profile through helper", read_back_proof: "routes, firewall, connectivity, rollback", owner: vec!["cynic-host", "cynic-node"], coverage: "partial" },
            FrameworkVerticalContract { id: "node", desired_state: "node identity, runtime basket, capabilities", observer: "node-local observation producer", reconciliation: "admit node only from registry plus read-back", bounded_action: "node-local plan or bounded activation", read_back_proof: "node observation and proof bundle", owner: vec!["cynic-node"], coverage: "partial" },
            FrameworkVerticalContract { id: "provider", desired_state: "provider resource and control capability", observer: "provider API or hypervisor adapter", reconciliation: "use provider truth without leaking provider policy", bounded_action: "one declared provider mutation", read_back_proof: "provider-side resource read-back", owner: vec!["cynic-node", "cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "orchestrator", desired_state: "workload, placement, policy, identity", observer: "orchestrator API and workload observer", reconciliation: "respect orchestrator authority and scope", bounded_action: "namespace/node/workload-scoped action", read_back_proof: "API conditions and workload health", owner: vec!["cynic-node", "cynic-mcp"], coverage: "partial" },
            FrameworkVerticalContract { id: "temporal", desired_state: "consistent epoch progression, verifiable checkpoints, and bounded rollback windows", observer: "epoch clock probe, checkpoint observer, temporal snapshot producer", reconciliation: "admit only certificate-backed epoch advancement; request rollback on divergence", bounded_action: "accept epoch certificate, request rollback to certified base_epoch, or emit new snapshot", read_back_proof: "epoch certificate, temporal proof bundle, state digest match", owner: vec!["cynic-protocol", "cynic-kernel", "cynic-node"], coverage: "partial" },
            FrameworkVerticalContract { id: "project", desired_state: "project governance profile and source authority", observer: "project registry and repository observer", reconciliation: "bind generic framework policy to project adapters", bounded_action: "project-scoped change or release", read_back_proof: "project proof bundle and revision", owner: vec!["cynic-protocol", "cynic-host"], coverage: "partial" },
            FrameworkVerticalContract { id: "fleet", desired_state: "placement, rollout, fencing, residual unknown budget", observer: "fleet inventory and coordination observer", reconciliation: "build node-scoped plans and bounded rollout", bounded_action: "canary or staged node action", read_back_proof: "node proofs, lease events, rollout history", owner: vec!["cynic-kernel", "cynic-node"], coverage: "partial" },
            FrameworkVerticalContract { id: "federation", desired_state: "peer identity, delegation, transport, completion", observer: "transport and coordination observer", reconciliation: "separate local recursion from node federation", bounded_action: "delegated action within reduced authority", read_back_proof: "peer identity and completion envelope", owner: vec!["cynic-protocol", "cynic-kernel"], coverage: "partial" },
            FrameworkVerticalContract { id: "human_boundary", desired_state: "consent, bootstrap, breakglass policy", observer: "authorization and audit observer", reconciliation: "keep HumanAdmin out of steady state", bounded_action: "explicit approval or incident breakglass", read_back_proof: "authorization event and resulting proof", owner: vec!["cynic-host", "cynic-mcp"], coverage: "partial" },
        ],
>>>>>>> github/main
        primitive_families: vec![
            FrameworkPrimitiveFamily {
                id: "registry",
                role: "Declare desired placement, roles, services, and environment-specific topology.",
                examples: vec![
                    "infra/registry.json",
                    "infra/registries/single-node.json",
                    "infra/registries/lab.json",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "proof_bundle",
                role: "Carry observed runtime evidence between layers without relying on chat or stale files.",
                examples: vec![
<<<<<<< HEAD
                    crate::runtime::types::RUNTIME_PROOF_BUNDLE_SCHEMA,
=======
                    "cynic.runtime_proof_bundle.v1",
>>>>>>> github/main
                    "host_identity_observed",
                    "network_runtime_observed",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "planner",
                role: "Turn desired state and proof bundles into safe read-only, dry-run, and apply boundaries.",
                examples: vec![
<<<<<<< HEAD
                    crate::runtime::types::RUNTIME_EXECUTION_PLAN_SCHEMA,
=======
                    "cynic.runtime_execution_plan.v1",
>>>>>>> github/main
                    "can_apply=false until executor exists",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "adapter",
                role: "Bind portable ports to concrete substrates without leaking provider logic into domain rules.",
                examples: vec![
                    "systemd",
                    "local shell",
                    "MCP",
                    "future Kubernetes adapter",
                    "future VM/LXC adapter",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "agentic_surface",
                role: "Give human and LLM agents bounded, inspectable, validation-backed commands.",
                examples: vec![
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                    "cynic dev kpi-score --json",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "agent_recursion_envelope",
                role: "Carry node-local lineage, content-addressed context, decreasing capabilities and budgets, and upward completion proof across recursive agents.",
                examples: vec![
                    "cynic.agent_delegation_envelope.v1",
                    "cynic.agent_completion_envelope.v1",
                    "cynic.agent_recursion_admission.v1",
                ],
            },
        ],
        target_use_cases: vec![
            FrameworkUseCase {
                id: "single_machine_bootstrap",
                description: "Run the organism on one machine for development and proof iteration.",
                required_primitives: vec!["registry", "proof_bundle", "planner", "adapter"],
            },
            FrameworkUseCase {
                id: "portable_lab_fleet",
                description: "Validate N-case topology without binding the framework to one hostname, DNS zone, or provider.",
                required_primitives: vec!["registry", "planner", "adapter"],
            },
            FrameworkUseCase {
                id: "production_service_supervision",
                description: "Measure uptime, restart budgets, health latency, proof freshness, and registry/runtime convergence.",
                required_primitives: vec!["proof_bundle", "planner", "adapter"],
            },
            FrameworkUseCase {
                id: "agentic_development_runtime",
                description: "Let agents develop safely through explicit surfaces, claims, proofs, validation commands, and next-step scoring.",
                required_primitives: vec![
                    "agentic_surface",
                    "agent_recursion_envelope",
                    "proof_bundle",
                    "planner",
                ],
            },
            FrameworkUseCase {
                id: "kubernetes_like_orchestration",
                description: "Provide desired-state reconciliation semantics that can later target Kubernetes, VMs, LXC, systemd, or custom hosts.",
                required_primitives: vec!["registry", "planner", "adapter"],
            },
        ],
        agnosticity_rules: vec![
            FrameworkRule {
                id: "no_provider_in_domain",
                rule: "Domain types must not require a specific cloud, hypervisor, LLM, browser, DNS zone, or host identity.",
            },
            FrameworkRule {
                id: "n_case_by_registry",
                rule: "A new host, fleet, provider, or deployment model must be represented as registry data or adapter config before code branching.",
            },
            FrameworkRule {
                id: "substrate_behind_ports",
                rule: "Systemd, Kubernetes, LXC, SSH, REST, MCP, and shell execution are adapters behind portable ports.",
            },
        ],
        modularity_rules: vec![
            FrameworkRule {
                id: "protocol_is_pure",
                rule: "cynic-protocol owns schemas and pure rules; it must not read the host or mutate runtime state.",
            },
            FrameworkRule {
                id: "cli_is_adapter",
                rule: "cynic-cli wires inputs and outputs; it must not own orchestration semantics.",
            },
            FrameworkRule {
                id: "proof_producers_are_low_layer",
                rule: "Host and node crates produce runtime proofs; higher layers consume and persist them.",
            },
            FrameworkRule {
                id: "one_boundary_one_reason",
                rule: "Add a crate, module, adapter, or contract only when an existing boundary cannot absorb the responsibility cleanly.",
            },
        ],
        non_goals: vec![
            "be a UX portal first",
            "recreate deleted Python organs as durable authorities",
            "require Kubernetes before single-node execution works",
            "treat LLM session memory as canonical state",
            "centralize all responsibilities in one orchestrator crate",
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn framework_spec_has_unique_layers_and_use_cases() {
        let spec = framework_spec();
        let mut layers = BTreeSet::new();
        let mut use_cases = BTreeSet::new();
        for layer in &spec.canonical_layers {
            assert!(layers.insert(layer.id), "duplicate layer {}", layer.id);
            assert!(!layer.owned_by.is_empty(), "layer owner required");
            assert!(!layer.evidence.is_empty(), "layer evidence required");
        }
        for use_case in &spec.target_use_cases {
            assert!(
                use_cases.insert(use_case.id),
                "duplicate use case {}",
                use_case.id
            );
            assert!(
                !use_case.required_primitives.is_empty(),
                "use case primitives required"
            );
        }
        let mut verticals = BTreeSet::new();
        for vertical in &spec.vertical_contracts {
<<<<<<< HEAD
            assert!(
                verticals.insert(vertical.id),
                "duplicate vertical {}",
                vertical.id
            );
=======
            assert!(verticals.insert(vertical.id), "duplicate vertical {}", vertical.id);
>>>>>>> github/main
            assert!(!vertical.owner.is_empty(), "vertical owner required");
            assert!(!vertical.desired_state.is_empty());
            assert!(!vertical.observer.is_empty());
            assert!(!vertical.read_back_proof.is_empty());
        }
<<<<<<< HEAD
        let responsibility_ids = spec
            .responsibility_contracts
            .contracts
            .iter()
            .map(|contract| contract.capability_id)
            .collect::<BTreeSet<_>>();
        assert_eq!(verticals, responsibility_ids);
        assert!(spec.responsibility_contracts.validate().is_empty());
=======
>>>>>>> github/main
    }

    #[test]
    fn framework_spec_preserves_substrate_agnosticity() {
        let spec = framework_spec();
        assert!(
            spec.agnosticity_rules
                .iter()
                .any(|rule| rule.id == "substrate_behind_ports")
        );
        assert!(
            spec.non_goals
                .contains(&"require Kubernetes before single-node execution works")
        );
    }

    #[test]
    fn framework_spec_declares_node_local_fractal_recursion() {
        let spec = framework_spec();
        assert!(
            spec.operating_model
                .iter()
                .any(|principle| principle.id == "node_local_fractal_agents")
        );
        assert!(
            spec.primitive_families
                .iter()
                .any(|family| family.id == "agent_recursion_envelope")
        );
        let agent_runtime = spec
            .target_use_cases
            .iter()
            .find(|use_case| use_case.id == "agentic_development_runtime")
            .expect("agentic runtime use case");
        assert!(
            agent_runtime
                .required_primitives
                .contains(&"agent_recursion_envelope")
        );
    }
<<<<<<< HEAD

    #[test]
    fn framework_spec_traces_foundational_ambitions_without_false_convergence() {
        let spec = framework_spec();
        let mut ids = BTreeSet::new();
        for ambition in &spec.ambition_translation {
            assert!(
                ids.insert(ambition.id),
                "duplicate ambition {}",
                ambition.id
            );
            assert!(!ambition.current_owner.is_empty());
            assert!(!ambition.translated_primitives.is_empty());
            assert!(!ambition.forbidden_substitutions.is_empty());
            assert!(!ambition.required_proof.is_empty());
        }
        for required in [
            "sovereign_runtime",
            "sovereign_membrane",
            "askesis_companion",
            "accessible_personal_ml",
            "sovereign_inference",
            "talaria_code_evolution",
            "opt_in_disclosure",
            "solana_public_settlement",
            "futard_audience_projection",
            "one_person_futarchy",
            "closed_evolution",
            "contestable_judgment",
            "multi_source_truth",
            "resource_metabolism",
            "replayable_memory",
            "node_recursion",
            "fleet_coordination",
            "runtime_federation",
            "operator_federation",
            "generalized_ownership",
            "decision_outcome_loop",
            "public_legibility",
        ] {
            assert!(ids.contains(required), "missing ambition {required}");
        }
        assert!(
            spec.ambition_translation
                .iter()
                .any(|ambition| { ambition.runtime_status == FrameworkAmbitionStatus::Blocked })
        );
        assert!(
            spec.ambition_translation
                .iter()
                .any(|ambition| { ambition.runtime_status == FrameworkAmbitionStatus::Absent })
        );
    }
=======
>>>>>>> github/main
}
