use std::collections::HashSet;

use serde::Serialize;

use crate::rust_source::{line_count, relative_to_workspace, rust_files_under, workspace_root};

const HOTSPOT_LINE_THRESHOLD: usize = 400;

const KERNEL_AREAS: &[KernelAreaSpec] = &[
    KernelAreaSpec {
        name: "inference_stack",
        role: "Own model adapters, inference routing, embeddings, and model selection contracts.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-inference",
        target_surface: "ports, adapters, and runtime model ownership",
        extraction_priority: 1,
        status: "extract",
        rationale: "Provider adapters and inference routing are infrastructure around model ownership, not kernel coordination law.",
        anti_patterns: &[
            "provider adapters still live in kernel",
            "REST inference transport is coupled to model runtime decisions",
            "embedding and summarization contracts were only partially extracted",
        ],
        paths: &[
            "api/rest/inference_proxy.rs",
            "api/rest/inference_router.rs",
            "api/rest/soma.rs",
            "backends/auto_embed.rs",
            "backends/cli.rs",
            "backends/embedding.rs",
            "backends/model_discovery.rs",
            "backends/openai.rs",
            "backends/summarizer.rs",
            "domain/embedding.rs",
            "domain/inference.rs",
            "domain/summarization.rs",
            "domain/verdict_cache.rs",
        ],
    },
    KernelAreaSpec {
        name: "runtime_authority_and_lifecycle",
        role: "Own runtime authority, bootstrap policy, lifecycle mutation, host paths, and privileged activation evidence.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-host",
        target_surface: "runtime authority, lifecycle, principal, and bootstrap law",
        extraction_priority: 1,
        status: "extract",
        rationale: "Privilege, host layout, activation, restore, and upgrade flows belong with runtime authority, not the coordination kernel.",
        anti_patterns: &[
            "host lifecycle policy still exposed through kernel REST",
            "runtime paths and activation evidence are modeled under kernel domain",
            "bootstrap composition is still biased toward kernel ownership",
        ],
        paths: &[
            "bootstrap/",
            "api/rest/coord.rs",
            "domain/lifecycle_activation_evidence.rs",
            "domain/lifecycle_activation_requests.rs",
            "domain/lifecycle_diagnostics.rs",
            "domain/lifecycle_workflow_state.rs",
        ],
    },
    KernelAreaSpec {
        name: "node_and_network_control_plane",
        role: "Own node runtime surfaces, service observation, mesh state, and network/runtime formation.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-node",
        target_surface: "node runtime commands, service projection, and network control plane",
        extraction_priority: 2,
        status: "extract",
        rationale: "Service mesh and node runtime formation are closer to node authority than to kernel memory/coordination.",
        anti_patterns: &[
            "mesh and service endpoints are still published from kernel REST",
            "node observation logic leaks into kernel backends",
        ],
        paths: &[
            "api/rest/discovery.rs",
            "api/rest/mesh.rs",
            "api/rest/services.rs",
            "backends/node_probe.rs",
            "domain/zones.rs",
        ],
    },
    KernelAreaSpec {
        name: "mcp_projection",
        role: "Expose MCP tools as a thin projection over kernel or host authority without owning the transport inside kernel.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-mcp",
        target_surface: "thin MCP projection and tool transport",
        extraction_priority: 2,
        status: "extract",
        rationale: "Embedding an MCP server under kernel recreates authority overlap with the dedicated cynic-mcp crate.",
        anti_patterns: &[
            "kernel still owns MCP transport files",
            "projection and authority remain partially fused",
        ],
        paths: &["api/mcp/"],
    },
    KernelAreaSpec {
        name: "coordination_memory_core",
        role: "Keep durable task coordination, observations, and memory-backed workflow state.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-kernel",
        target_surface: "coordination, state, workflow memory, and event persistence",
        extraction_priority: 1,
        status: "stabilize",
        rationale: "This is the strongest remaining reason for the kernel to exist, but adapters and handlers must be split around ports and use cases.",
        anti_patterns: &[
            "REST handlers remain very large",
            "workflow state and storage concerns are spread across api/domain/infra/storage",
        ],
        paths: &[
            "api/rest/agent_tasks.rs",
            "api/rest/data.rs",
            "api/rest/event.rs",
            "api/rest/events.rs",
            "api/rest/observe.rs",
            "domain/coord.rs",
            "domain/events.rs",
            "domain/inference_queue.rs",
            "domain/state_log.rs",
            "domain/verdict_queue.rs",
            "infra/tasks/",
            "storage/",
        ],
    },
    KernelAreaSpec {
        name: "heuristics_and_data_pipeline",
        role: "Own organism scoring, heuristics, enrichment, and pipeline staging behind explicit data/inference ports.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-kernel::pipeline",
        target_surface: "internal pipeline/usecase layer pending dedicated stable ports",
        extraction_priority: 1,
        status: "stabilize",
        rationale: "The pipeline module is nearly empty while heuristics are dispersed across domain files, which blocks clean future extraction.",
        anti_patterns: &[
            "pipeline module is hollow",
            "heuristics are spread across domain files without a canonical pipeline boundary",
            "data evolution logic is mixed with transport and storage concerns",
        ],
        paths: &[
            "domain/enrichment.rs",
            "domain/kscore.rs",
            "domain/metabolism.rs",
            "domain/pattern_analysis.rs",
            "domain/probe.rs",
            "domain/stimulus.rs",
            "domain/token_postprocessor.rs",
            "domain/usage.rs",
            "pipeline/",
        ],
    },
    KernelAreaSpec {
        name: "product_domain_residual",
        role: "Product epistemology and market/domain heuristics that are not low-layer runtime baseline (wallet judgment, rug risk, phone, mail, dog verdicts, prompt domains).",
        current_owner: "cynic-kernel",
        target_owner: "product_domain_or_pipeline",
        target_surface: "explicit product domain modules outside L0-L3 runtime authority",
        extraction_priority: 2,
        status: "extract",
        rationale: "These surfaces inflate the kernel monologue and confuse agentic low-layer work with product judgment. They must be tiered as product_domain and eventually leave the coordination bus core.",
        anti_patterns: &[
            "product domain markdown and judgment logic sit beside coordination memory",
            "wallet/rug/phone/mail domains are treated as runtime proof surfaces",
            "dog verdict epistemology is confused with host/node health",
        ],
        paths: &[
            "domain/dog.rs",
            "domain/dog_health.rs",
            "domain/mail.rs",
            "domain/phone_number.rs",
            "domain/rug_prefilter.rs",
            "domain/rug_risk.rs",
            "domain/wallet_enrichment.rs",
            "domain/wallet_judgment/",
            "domain/wisdom/",
            "domains/",
            "escore/",
        ],
    },
    KernelAreaSpec {
        name: "observability_and_health_surfaces",
        role: "Own health reporting, introspection, runtime alerts, websocket observation, and internal metrics without leaking host/node authority.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-kernel",
        target_surface: "bounded observability and health usecases around coordination state",
        extraction_priority: 1,
        status: "stabilize",
        rationale: "Health and observation belong near kernel state, but they must be decomposed into bounded usecases instead of large transport-heavy modules.",
        anti_patterns: &[
            "health and observation transport types are oversized",
            "introspection mixes anomaly policy, metrics, probe interpretation, and alert rendering",
            "websocket/event observation remains transport-heavy inside kernel",
        ],
        paths: &[
            "api/rest/health.rs",
            "api/rest/types.rs",
            "api/websocket/",
            "domain/health_gate.rs",
            "domain/metrics.rs",
            "infra/health_loop.rs",
            "infra/task_health.rs",
            "introspection.rs",
        ],
    },
    KernelAreaSpec {
        name: "solana_and_external_data_adapters",
        role: "Own external Solana/Helius enrichment adapters behind domain ports, feeding the pipeline without contaminating core domain law.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-kernel::pipeline",
        target_surface: "external data adapters behind enrichment and probe ports",
        extraction_priority: 2,
        status: "stabilize",
        rationale: "These adapters are legitimate runtime capabilities, but they should hang off ports and dedicated adapter modules rather than bleed into generic backend/config ownership.",
        anti_patterns: &[
            "helius adapters still couple to broad infra config types",
            "external data adapter ownership is not distinguished from inference adapters",
            "probe and enrichment boundaries remain blurred",
        ],
        paths: &[
            "backends/dexscreener.rs",
            "backends/health_probe.rs",
            "backends/helius/",
            "backends/lofi.rs",
            "domain/ccm/",
            "domain/helius_credit.rs",
            "probe/llm.rs",
        ],
    },
    KernelAreaSpec {
        name: "storage_domain_boundary",
        role: "Own durable storage ports, null adapters, reconnectable behavior, and persistence mappings for coordination memory.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-kernel",
        target_surface: "storage ports and adapters separated from transport and workflow orchestration",
        extraction_priority: 1,
        status: "stabilize",
        rationale: "Storage is core kernel territory, but the boundary is currently smeared across domain, infra, and concrete Surreal adapters.",
        anti_patterns: &[
            "domain storage contracts are themselves oversized",
            "reconnectable and surreal adapters carry too much orchestration detail",
            "storage concerns are duplicated across api/domain/storage/infra",
        ],
        paths: &[
            "domain/storage/",
            "infra/reconnectable_storage.rs",
            "storage/",
        ],
    },
    KernelAreaSpec {
        name: "runtime_composition_and_organ_assembly",
        role: "Own kernel-local composition root concerns only after authority, inference, and node surfaces are delegated outward.",
        current_owner: "cynic-kernel",
        target_owner: "split:cynic-host+cynic-inference+cynic-kernel",
        target_surface: "thin composition root and bounded kernel-local assembly",
        extraction_priority: 1,
        status: "extract",
        rationale: "The remaining monolith is partly caused by mixed composition concerns: host/runtime config, inference config, organ assembly, and executable startup still meet in oversized shared files.",
        anti_patterns: &[
            "infra config aggregates unrelated subdomains",
            "main and organ assembly still carry too much wiring authority",
            "composition root is not yet thin enough for hexagonal scaling",
        ],
        paths: &[
            "bootstrap/",
            "domain/sanitize.rs",
            "infra/config/",
            "main.rs",
            "organ/",
        ],
    },
    KernelAreaSpec {
        name: "fleet_observation_and_inventory",
        role: "Own fleet inventory, host/node observation aggregation, and multi-node runtime visibility.",
        current_owner: "cynic-kernel",
        target_owner: "split:cynic-host+cynic-node",
        target_surface: "fleet inventory and observation projections derived from host/node runtime authority",
        extraction_priority: 1,
        status: "extract",
        rationale: "Fleet is an aggregate projection over hosts and nodes. Keeping it in kernel muddies the boundary between coordination memory and runtime authority observation.",
        anti_patterns: &[
            "fleet inventory is implemented as kernel infra instead of host/node projection",
            "multi-node observation logic is detached from runtime authority owners",
            "kernel accumulates operational inventory concerns that should be read from host/node proofs",
        ],
        paths: &["infra/probes/fleet.rs", "infra/probes/fleet_inventory.rs"],
    },
    KernelAreaSpec {
        name: "inference_capacity_and_slot_control",
        role: "Own backend slot accounting, queue priority, and runtime inference capacity control.",
        current_owner: "cynic-kernel",
        target_owner: "cynic-inference",
        target_surface: "capacity and queueing primitives for model runtime execution",
        extraction_priority: 1,
        status: "extract",
        rationale: "Slot and queue coordination are part of inference runtime ownership. Kernel consumers should depend on ports and reports, not own the primitive directly.",
        anti_patterns: &[
            "slot accounting lives beside kernel domain logic instead of inference runtime ownership",
            "capacity control is coupled to kernel-specific health loop updates",
            "queueing primitives are not yet exposed as inference-owned execution law",
        ],
        paths: &["domain/inference_queue.rs", "domain/slot_semaphore.rs"],
    },
];

#[derive(Serialize)]
pub(crate) struct KernelAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) crate_name: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) total_rust_file_count: usize,
    pub(crate) total_line_count: usize,
    pub(crate) covered_file_count: usize,
    pub(crate) uncovered_file_count: usize,
    pub(crate) next_steps: Vec<KernelAuditNextStep>,
    pub(crate) areas: Vec<KernelAreaAudit>,
    pub(crate) repeated_surface_findings: Vec<KernelRepeatedSurfaceFinding>,
    pub(crate) uncovered_hotspots: Vec<FileHotspot>,
}

#[derive(Serialize)]
pub(crate) struct KernelAuditNextStep {
    pub(crate) order: u8,
    pub(crate) area: &'static str,
    pub(crate) action: &'static str,
    pub(crate) target_owner: &'static str,
    pub(crate) rationale: &'static str,
}

#[derive(Serialize)]
pub(crate) struct KernelAreaAudit {
    pub(crate) name: &'static str,
    pub(crate) role: &'static str,
    pub(crate) current_owner: &'static str,
    pub(crate) target_owner: &'static str,
    pub(crate) target_surface: &'static str,
    pub(crate) extraction_priority: u8,
    pub(crate) status: &'static str,
    pub(crate) rationale: &'static str,
    pub(crate) anti_patterns: Vec<&'static str>,
    pub(crate) matched_file_count: usize,
    pub(crate) total_line_count: usize,
    pub(crate) hotspots: Vec<FileHotspot>,
    pub(crate) paths: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct FileHotspot {
    pub(crate) file_path: String,
    pub(crate) line_count: usize,
    pub(crate) status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct KernelRepeatedSurfaceFinding {
    pub(crate) id: &'static str,
    pub(crate) category: &'static str,
    pub(crate) status: &'static str,
    pub(crate) canonical_owner: &'static str,
    pub(crate) detail: &'static str,
    pub(crate) correction: &'static str,
    pub(crate) source_paths: Vec<&'static str>,
}

struct KernelAreaSpec {
    name: &'static str,
    role: &'static str,
    current_owner: &'static str,
    target_owner: &'static str,
    target_surface: &'static str,
    extraction_priority: u8,
    status: &'static str,
    rationale: &'static str,
    anti_patterns: &'static [&'static str],
    paths: &'static [&'static str],
}

pub(crate) fn audit_kernel_surfaces() -> KernelAuditReport {
    let kernel_root = workspace_root().join("crates/cynic-kernel");
    let files = rust_files_under(&kernel_root.join("src"));
    let indexed_files = files
        .iter()
        .filter_map(|path| {
            Some(IndexedRustFile {
                relative_path: relative_to_workspace(path),
                line_count: line_count(path)?,
            })
        })
        .collect::<Vec<_>>();
    let mut covered = HashSet::new();
    let areas = KERNEL_AREAS
        .iter()
        .map(|spec| audit_area(spec, &indexed_files, &mut covered))
        .collect::<Vec<_>>();
    let uncovered_hotspots = indexed_files
        .iter()
        .filter(|file| !covered.contains(&file.relative_path))
        .filter(|file| file.line_count >= HOTSPOT_LINE_THRESHOLD)
        .map(|file| FileHotspot {
            file_path: file.relative_path.clone(),
            line_count: file.line_count,
            status: "observe",
        })
        .collect::<Vec<_>>();
    let repeated_surface_findings = repeated_surface_findings();
    let total_line_count = indexed_files.iter().map(|file| file.line_count).sum();
    let covered_file_count = covered.len();
    let uncovered_file_count = indexed_files.len().saturating_sub(covered_file_count);
    let blocked_areas = areas
        .iter()
        .any(|area| matches!(area.status, "extract" | "stabilize"));
    let next_steps = vec![
        KernelAuditNextStep {
            order: 1,
            area: "fleet_observation_and_inventory",
            action: "move provider-facing fleet probes behind cynic-node and cynic-host observation surfaces",
            target_owner: "split:cynic-host+cynic-node",
            rationale: "fleet is already modeled outside kernel as a node-centric projection and should stop depending on kernel-local provider probes",
        },
        KernelAuditNextStep {
            order: 2,
            area: "inference_stack",
            action: "continue extracting model adapters, routing, and embedding runtime to cynic-inference",
            target_owner: "cynic-inference",
            rationale: "this remains the largest non-kernel authority embedded in kernel transport and backend modules",
        },
        KernelAuditNextStep {
            order: 3,
            area: "inference_capacity_and_slot_control",
            action: "transfer slot and queue primitives to cynic-inference while keeping only reporting and consumption ports in kernel",
            target_owner: "cynic-inference",
            rationale: "capacity control belongs with model runtime ownership and blocks a clean split between workflow and inference execution",
        },
        KernelAuditNextStep {
            order: 4,
            area: "runtime_composition_and_organ_assembly",
            action: "split infra config and organ assembly into bounded submodules after external owners are extracted",
            target_owner: "split:cynic-host+cynic-inference+cynic-kernel",
            rationale: "composition-root thinning becomes tractable only after authority-heavy surfaces stop terminating in kernel",
        },
        KernelAuditNextStep {
            order: 5,
            area: "heuristics_and_data_pipeline",
            action: "make pipeline a real internal layer with usecases and adapter ports before any new crate split",
            target_owner: "cynic-kernel::pipeline",
            rationale: "the heuristics surface is large, but extraction without an internal pipeline boundary would just relocate the monolith",
        },
        KernelAuditNextStep {
            order: 6,
            area: "product_domain_residual",
            action: "quarantine product epistemology (dogs, wallet/rug, phone, domain prompts) from L0-L3 runtime authority and stop treating it as low-layer proof",
            target_owner: "product_domain_or_pipeline",
            rationale: "agent fragmentation is worsened when product judgment shares the kernel bus with coordination and host proof",
        },
    ];
    KernelAuditReport {
        schema: "cynic.cli.kernel_audit.v1",
        status: if blocked_areas || !uncovered_hotspots.is_empty() {
            "blocked"
        } else {
            "ok"
        },
        crate_name: "cynic-kernel",
        rule: "kernel should retain coordination and memory authority while inference, runtime authority, node control plane, and MCP projection move to bounded owners",
        total_rust_file_count: indexed_files.len(),
        total_line_count,
        covered_file_count,
        uncovered_file_count,
        next_steps,
        areas,
        repeated_surface_findings,
        uncovered_hotspots,
    }
}

fn audit_area(
    spec: &'static KernelAreaSpec,
    files: &[IndexedRustFile],
    covered: &mut HashSet<String>,
) -> KernelAreaAudit {
    let matched = files
        .iter()
        .filter(|file| {
            spec.paths
                .iter()
                .any(|scope| matches_scope(&file.relative_path, scope))
        })
        .collect::<Vec<_>>();
    for file in &matched {
        covered.insert(file.relative_path.clone());
    }
    let total_line_count = matched.iter().map(|file| file.line_count).sum();
    let hotspots = matched
        .iter()
        .filter(|file| file.line_count >= HOTSPOT_LINE_THRESHOLD)
        .map(|file| FileHotspot {
            file_path: file.relative_path.clone(),
            line_count: file.line_count,
            status: "observe",
        })
        .collect::<Vec<_>>();
    KernelAreaAudit {
        name: spec.name,
        role: spec.role,
        current_owner: spec.current_owner,
        target_owner: spec.target_owner,
        target_surface: spec.target_surface,
        extraction_priority: spec.extraction_priority,
        status: spec.status,
        rationale: spec.rationale,
        anti_patterns: spec.anti_patterns.to_vec(),
        matched_file_count: matched.len(),
        total_line_count,
        hotspots,
        paths: spec
            .paths
            .iter()
            .map(|path| format!("crates/cynic-kernel/src/{path}"))
            .collect(),
    }
}

fn matches_scope(relative_path: &str, scope: &str) -> bool {
    let normalized_scope = format!("crates/cynic-kernel/src/{}", scope.trim_end_matches('/'));
    relative_path == normalized_scope || relative_path.starts_with(&(normalized_scope + "/"))
}

fn repeated_surface_findings() -> Vec<KernelRepeatedSurfaceFinding> {
    vec![
        KernelRepeatedSurfaceFinding {
            id: "storage_self_healing_python_projection",
            category: "obsolete_runtime_authority_projection",
            status: "blocked",
            canonical_owner: "cynic-host",
            detail: "kernel REST transport still exposes lifecycle remediation/provision surfaces, even though implicit legacy python execution has now been removed",
            correction: "finish the extraction by moving lifecycle remediation/provision authority to an explicit cynic-host surface and deleting the remaining kernel REST projection",
            source_paths: vec![
                "crates/cynic-kernel/src/api/rest/agent_tasks.rs",
                "crates/cynic-kernel/src/api/rest/coord.rs",
            ],
        },
        KernelRepeatedSurfaceFinding {
            id: "project_root_runtime_discovery_leak",
            category: "operator_worktree_dependency",
            status: "blocked",
            canonical_owner: "cynic-host runtime layout and repo access",
            detail: "kernel transport and composition still depend on repo/worktree discovery surfaces instead of a fully bounded runtime authority contract",
            correction: "continue routing repo/runtime discovery through typed runtime layout or repo access ports, and remove remaining kernel transport dependencies on project-root semantics",
            source_paths: vec![
                "crates/cynic-kernel/src/bootstrap/entry.rs",
                "crates/cynic-kernel/src/main.rs",
                "crates/cynic-kernel/src/api/rest/dispatch.rs",
            ],
        },
        KernelRepeatedSurfaceFinding {
            id: "surreal_process_control_in_rest_transport",
            category: "transport_owns_storage_operations",
            status: "blocked",
            canonical_owner: "cynic-host or bounded storage operations adapter",
            detail: "kernel REST coordination transport still sits adjacent to host-level lifecycle semantics, even after removing direct surreal shelling and local upgrade projections",
            correction: "delete or fully extract the remaining lifecycle semantics from coord REST so host/runtime authority lives only behind bounded host adapters",
            source_paths: vec![
                "crates/cynic-kernel/src/api/rest/coord.rs",
                "crates/cynic-kernel/src/api/rest/mod.rs",
            ],
        },
    ]
}

struct IndexedRustFile {
    relative_path: String,
    line_count: usize,
}

#[cfg(test)]
mod tests {
    use super::audit_kernel_surfaces;

    #[test]
    fn kernel_audit_reports_extraction_areas() {
        let report = audit_kernel_surfaces();
        assert_eq!(report.schema, "cynic.cli.kernel_audit.v1");
        assert_eq!(report.crate_name, "cynic-kernel");
        assert!(report.total_rust_file_count > 0);
        assert!(report.areas.iter().any(|area| {
            area.name == "inference_stack"
                && area.target_owner == "cynic-inference"
                && area.matched_file_count > 0
        }));
        assert!(report.areas.iter().any(|area| {
            area.name == "runtime_authority_and_lifecycle"
                && area.target_owner == "cynic-host"
                && area.matched_file_count > 0
        }));
        assert!(
            !report
                .repeated_surface_findings
                .iter()
                .any(|finding| { finding.id == "forge_task_registry_contract_duplication" })
        );
        assert!(report.repeated_surface_findings.iter().any(|finding| {
            finding.id == "storage_self_healing_python_projection"
                && finding
                    .source_paths
                    .iter()
                    .any(|path| path.ends_with("api/rest/agent_tasks.rs"))
        }));
    }
}
