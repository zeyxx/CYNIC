import json
import tempfile
from pathlib import Path
from typing import Any


def write_base_files(root: Path) -> None:
    (root / "docs" / "foundation").mkdir(parents=True)
    (root / "AGENTS.md").write_text("# agents\n", encoding="utf-8")
    (root / "docs" / "foundation" / "CYNIC_META_LAYER_CONTRACTS.md").write_text("# meta\n", encoding="utf-8")
    (root / "docs" / "foundation" / "CYNIC_REPO_TOPOLOGY_POLICY.md").write_text("# policy\n", encoding="utf-8")
    (root / "docs" / "foundation" / "CYNIC_ORGANISM_POLICY_MATRIX.md").write_text("# matrix\n", encoding="utf-8")
    (root / "docs" / "foundation" / "CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md").write_text("# truth\n", encoding="utf-8")
    (root / "docs" / "foundation" / "CYNIC_FRACTAL_ABSTRACTION_PRINCIPLE.md").write_text("# fractal\n", encoding="utf-8")
    (root / "docs" / "architecture").mkdir(parents=True, exist_ok=True)
    (root / "docs" / "architecture" / "SOMA.md").write_text("# soma\n", encoding="utf-8")
    (root / "docs" / "architecture" / "CYNIC-NODE-SOMA-SYSTEM.md").write_text("# system\n", encoding="utf-8")
    (root / "docs" / "architecture" / "CYNIC-NODE-PHASE-B-LIFECYCLE.md").write_text("# lifecycle\n", encoding="utf-8")
    (root / "docs" / "architecture" / "TOPOLOGY.md").write_text("# topology\n", encoding="utf-8")
    (root / "docs" / "foundation" / "CYNIC_THEME_TOPOLOGY_POLICY.md").write_text("# theme policy\n", encoding="utf-8")
    (root / "crates" / "cynic-mcp" / "src").mkdir(parents=True)
    (root / "crates" / "cynic-mcp" / "src" / "proxy.rs").write_text(
        'let root = std::env::var("CYNIC_PROJECT_ROOT")?;\n',
        encoding="utf-8",
    )
    (root / "crates" / "cynic-node" / "src" / "domain").mkdir(parents=True)
    (root / "crates" / "cynic-node" / "src" / "adapters").mkdir(parents=True)
    (root / "crates" / "cynic-node" / "src" / "ports").mkdir(parents=True)
    (root / "crates" / "cynic-node" / "src" / "usecases").mkdir(parents=True)
    (root / "crates" / "cynic-node" / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
    (root / "crates" / "cynic-node" / "build.rs").write_text("fn main() {}\n", encoding="utf-8")
    (root / "infra").mkdir()
    (root / "infra" / "registry.json").write_text("{}", encoding="utf-8")
    (root / "organs" / "supervisor" / "entrypoints").mkdir(parents=True)
    (root / "organs" / "supervisor" / "state").mkdir()
    (root / "organs" / "supervisor" / "MANIFEST.yaml").write_text(
        "name: organ-supervisor\nstatus: canonical\ndescription: Test supervisor organ.\nstate_paths: [state/]\n",
        encoding="utf-8",
    )
    (root / "services").mkdir()
    (root / "organs" / "supervisor" / "entrypoints" / "runtime_principals.py").write_text(
        "choices=['status', 'plan', 'update', 'apply', 'install-defaults']\n"
        'PRIMARY_UPDATE_COMMANDS = {"codex": ["codex", "update"], "agy": ["agy", "update"]}\n'
        'help_text = "cynic update cynic update --dry-run"\n',
        encoding="utf-8",
    )
    (root / "infra" / "agent-policy.toml").write_text(
        """
[runtime]
authority = "axioms/runtime-principals-v1.json"
dedicated_user = "cynic"

[concerns.infra-write]
nopasswd = false

[concerns.network]
nopasswd = false

[agents.codex]
runtime_user = "cynic"
execution_posture = "bounded_yolo"

[agents.antigravity]
runtime_user = "cynic"
execution_posture = "bounded_yolo"

[deny]
commands = ["rm -rf /", "git reset --hard HEAD~"]
""",
        encoding="utf-8",
    )
    (root / "scripts").mkdir()
    (root / ".cortex").mkdir()
    (root / ".cynic").mkdir()


def write_api_discovery_files(root: Path) -> None:
    for path in ["docs/architecture", "docs/api", "crates/cynic-kernel/src/api/rest"]:
        (root / path).mkdir(parents=True, exist_ok=True)
    (root / "docs" / "api" / "openapi.json").write_text(
        json.dumps(
            {
                "openapi": "3.1.0",
                "x-cynic-api-topology-contract": {
                    "schema": "cynic.api_topology.v1",
                    "path": "axioms/api-topology-v1.json",
                },
                "x-cynic-api-layer-alignment-contract": {
                    "schema": "cynic.api_layer_alignment.v1",
                    "path": "axioms/api-layer-alignment-v1.json",
                },
                "paths": {
                    "/openapi.json": {"get": {"security": []}},
                    "/api/topology": {"get": {"security": []}},
                    "/api/layer-alignment": {"get": {"security": []}},
                },
            }
        ),
        encoding="utf-8",
    )
    (root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "discovery.rs").write_text(
        'const OPENAPI_JSON: &str = include_str!("../../../../../docs/api/openapi.json");\n'
        'const API_TOPOLOGY_JSON: &str = include_str!("../../../../../axioms/api-topology-v1.json");\n'
        'const API_LAYER_ALIGNMENT_JSON: &str = include_str!("../../../../../axioms/api-layer-alignment-v1.json");\n'
        "pub async fn openapi_handler() -> Response { json_contract(OPENAPI_JSON) }\n"
        "pub async fn api_topology_handler() -> Response { json_contract(API_TOPOLOGY_JSON) }\n"
        "pub async fn api_layer_alignment_handler() -> Response { json_contract(API_LAYER_ALIGNMENT_JSON) }\n",
        encoding="utf-8",
    )
    (root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "middleware.rs").write_text(
        'path == "/openapi.json" || path == "/api/topology" || path == "/api/layer-alignment"\n',
        encoding="utf-8",
    )
    (root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "mod.rs").write_text(
        'Router::new().route("/openapi.json", get(openapi_handler))\n'
        '    .route("/api/topology", get(api_topology_handler))\n'
        '    .route("/api/layer-alignment", get(api_layer_alignment_handler));\n',
        encoding="utf-8",
    )


def write_api_contracts(contracts: Path) -> None:
    (contracts / "api-layer-alignment-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.api_layer_alignment.v1",
                "status": "normative",
                "symbolic_vocabulary": {
                    "Keter": "source intent",
                    "Binah": "bounded form",
                    "Gevurah": "constraint",
                    "Tiferet": "integration",
                    "Hod": "interface",
                },
                "guardrails": ["analogy is subordinate to executable evidence"],
                "layers": {
                    "perception": {
                        "engineering_role": "test fixture",
                        "agent_question": "what is visible?",
                        "system_primitives": ["read", "poll", "file descriptor"],
                        "symbolic_alignment": ["Hod", "Tiferet"],
                        "must": ["test"],
                        "must_not": ["test"],
                    }
                },
                "vertical_alignment_rules": [{"id": "test", "rule": "test", "enforcement": "validator"}],
            }
        ),
        encoding="utf-8",
    )
    (contracts / "api-topology-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.api_topology.v1",
                "status": "normative",
                "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
                "allowed": {
                    "layers": ["perception"],
                    "mutation_levels": ["read"],
                    "statuses": ["canonical"],
                },
                "topology_budget": {
                    "max_root_splits": 0,
                    "max_transitional_operations": 0,
                    "max_legacy_operations": 0,
                    "compat_operations": {"allowed": False},
                },
                "public_discovery_endpoints": {
                    "GET /openapi.json": {"serves": "docs/api/openapi.json"},
                    "GET /api/topology": {"serves": "axioms/api-topology-v1.json"},
                    "GET /api/layer-alignment": {"serves": "axioms/api-layer-alignment-v1.json"},
                },
                "operations": {
                    "GET /openapi.json": {
                        "layer": "perception",
                        "mutation_level": "read",
                        "agent_safe_default": True,
                        "canonical_family": "/api",
                        "status": "canonical",
                    },
                    "GET /api/topology": {
                        "layer": "perception",
                        "mutation_level": "read",
                        "agent_safe_default": True,
                        "canonical_family": "/api",
                        "status": "canonical",
                    },
                    "GET /api/layer-alignment": {
                        "layer": "perception",
                        "mutation_level": "read",
                        "agent_safe_default": True,
                        "canonical_family": "/api",
                        "status": "canonical",
                    },
                },
            }
        ),
        encoding="utf-8",
    )


def write_minimal_peer_contracts(contracts: Path) -> None:
    (contracts / "organ-layer-topology-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.organ_layer_topology.v1",
                "status": "normative",
                "layers": [
                    {
                        "id": "foundational_contract",
                        "rank": 10,
                        "name": "Foundational contract",
                        "responsibility": "peer fixture",
                        "system_primitives": ["json schema", "contract validation", "git object"],
                        "symbolic_alignment": ["Keter", "Binah"],
                        "must_not_own": ["runtime_state"],
                        "canonical_examples": ["organs/contracts/"],
                        "parent_layer": None,
                        "projection_kind": "fractal_rank",
                    }
                ],
                "organs": [
                    {
                        "path": "organs/contracts/",
                        "layer_id": "foundational_contract",
                        "layer_rank": 10,
                        "owner": "organ-contracts",
                        "why": "peer fixture",
                        "evidence": ["MANIFEST.yaml"],
                    }
                ],
                "totals": {"layers": 1, "organs": 1, "by_layer": {"foundational_contract": 1}},
            }
        ),
        encoding="utf-8",
    )
    (contracts / "runtime-principals-v1.json").write_text(
        json.dumps({"schema": "cynic.runtime_principals.v1", "status": "normative"}),
        encoding="utf-8",
    )


def write_repo_topology_contract(contracts: Path) -> None:
    contract = {
        "schema": "cynic.repo_topology.v1",
        "status": "normative",
        "authorities": {
            "human_readable_policy": "docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md",
            "policy_matrix": "docs/foundation/CYNIC_ORGANISM_POLICY_MATRIX.md",
            "agent_entrypoint": "AGENTS.md",
        },
        "root_classes": [
            {
                "path": "docs/foundation/",
                "class": "policy",
                "owns": ["policy"],
                "must_not_own": ["runtime_state"],
            },
            {
                "path": "axioms/",
                "class": "semantic_contract",
                "owns": ["contracts"],
                "must_not_own": ["runtime_state"],
            },
            {
                "path": "infra/",
                "class": "organism_infrastructure",
                "owns": ["registry"],
                "must_not_own": ["organ_domain_logic"],
            },
            {
                "path": "crates/cynic-mcp/",
                "class": "portable_mcp_surface",
                "owns": ["mcp_tool_contract_projection"],
                "must_not_own": ["project_specific_hardcoded_paths"],
            },
        ],
        "organ_maturity_policy": {
            "allowed_statuses": ["canonical", "embryonic"],
            "status_meanings": {"canonical": "complete", "embryonic": "seed"},
            "minimum_exception_fields": ["path", "status", "owner", "replacement", "expiry_condition"],
            "shape_exception_fields": ["path", "status", "owner", "missing_paths", "replacement", "expiry_condition"],
            "rule": "known statuses only",
        },
        "rust_hexagonal_crate_policy": {
            "allowed_crates": [
                {
                    "path": "crates/cynic-node/",
                    "required_paths": [
                        "src/main.rs",
                        "src/domain/",
                        "src/adapters/",
                        "src/ports/",
                        "src/usecases/",
                    ],
                    "owner": "cynic-node",
                    "reason": "hexagonal crate shape under test",
                }
            ]
        },
        "organ_shape": {
            "state_policy": {
                "allowed_path": "organs/<organ>/state/",
                "commit_default": "forbidden",
                "manifest_state_paths": {
                    "field": "state_paths",
                    "allowed_prefixes": ["state/", "desired/"],
                    "require_relative_paths": True,
                    "reason": "test fixture",
                },
                "exceptions": ["fixtures", "operator_contracts_explicitly_declared_in_manifest"],
            }
        },
        "prohibited_script_entry_files": {
            "suffixes": [".sh", ".bat", ".cmd", ".ps1"],
            "shebang_interpreters": ["sh", "bash", "dash", "zsh", "ksh"],
            "owner": "cortex",
            "replacement": "organ entrypoint",
            "expiry_condition": "no shell entrypoints remain",
        },
        "tracked_executable_file_policy": {
            "owner": "cortex",
            "allowed_paths": [],
            "replacement": "explicit interpreter invocation",
            "expiry_condition": "no tracked executable source files",
        },
        "top_level_source_file_policy": {
            "owner": "cortex",
            "source_suffixes": [".py", ".js", ".ts", ".rs", ".go"],
            "allowed_paths": [],
            "replacement": "owned organ/crate/app/service boundary",
            "expiry_condition": "no root source files",
        },
        "generated_dependency_directory_policy": {
            "owner": "cortex",
            "directory_names": [".venv", "node_modules", ".pytest_cache", "__pycache__", ".gradle-user-home"],
            "scan_roots": ["apps/", "crates/", "organs/", "packages/", "services/"],
            "allowed_paths": [],
            "replacement": "external dependency cache",
            "expiry_condition": "no generated dependency directories in source membranes",
        },
        "entrypoint_thin_membrane_policy": {
            "owner": "cortex",
            "scan_globs": ["organs/*/src/entrypoints/*.py"],
            "max_lines": 160,
            "forbid_executable_bit": True,
            "exceptions": [],
            "replacement": "thin organ entrypoint",
            "expiry_condition": "entrypoint delegates to core",
        },
        "active_document_script_entrypoint_policy": {
            "owner": "cortex",
            "scan_paths": ["AGENTS.md", "docs/ops/"],
            "ignored_globs": [],
            "forbidden_patterns": [
                r"\b[\w./~$${}-]+\.sh\b",
                r"(^|[\s`(])(?:python3|python|bash|sh|node|npx|tsx)?\s*(?:\./)?scripts/[\w./-]+",
            ],
            "replacement": "organ entrypoint",
            "expiry_condition": "no active document script entrypoint references",
        },
    }
    (contracts / "repo-topology-v1.json").write_text(json.dumps(contract), encoding="utf-8")


def write_theme_and_evidence_contracts(contracts: Path, draft_governance: dict[str, str]) -> None:
    theme_contract = {
        "schema": "cynic.organism_theme_topology.v1",
        "status": "normative",
        "authorities": {
            "human_readable_policy": "docs/foundation/CYNIC_THEME_TOPOLOGY_POLICY.md",
            "repo_topology_contract": "axioms/repo-topology-v1.json",
        },
        "themes": [
            {
                "id": "runtime",
                "class": "runtime_topology",
                "owns": ["topology"],
                "canonical_sources": ["axioms/repo-topology-v1.json"],
                "allowed_projection_paths": ["docs/architecture/"],
                "must_not_own": ["legal_truth"],
                "drift_rule": "contracts first",
            },
            {
                "id": "data_and_evidence",
                "class": "evidence_theme",
                "owns": ["evidence"],
                "canonical_sources": ["axioms/evidence-capsule-v1.json"],
                "allowed_projection_paths": ["data/emergence/"],
                "must_not_own": ["legal_truth"],
                "drift_rule": "evidence first",
            }
        ],
        "doc_zones": [
            {
                "path": "axioms/",
                "class": "machine_contract",
                "may_own": ["contracts"],
                "must_not_own": ["drafts"],
                "canonicality": "canonical",
            }
        ],
        "promotion_rule": {"from": ["docs/internal/"], "to": ["docs/foundation/"], "requires": ["declared theme"]},
    }
    (contracts / "organism-theme-topology-v1.json").write_text(json.dumps(theme_contract), encoding="utf-8")
    document_inventory = {
        "schema": "cynic.document_theme_inventory.v1",
        "status": "draft",
        **draft_governance,
        "allowed_statuses": ["draft", "evidence"],
        "documents": [
            {
                "path": "data/emergence/probe/2026-06-24/",
                "theme": "data_and_evidence",
                "status": "evidence",
                "canonical_target": "data/emergence/probe/2026-06-24/MANIFEST.json",
            }
        ],
    }
    (contracts / "document-theme-inventory-v1.json").write_text(json.dumps(document_inventory), encoding="utf-8")
    (contracts / "evidence-capsule-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.evidence_capsule.v1",
                "status": "normative",
                "root": "data/emergence/",
                "capsule_depth": 2,
                "manifest_name": "MANIFEST.json",
                "manifest_required_fields": ["id", "created_at", "root", "purpose", "status", "sensitivity_policy"],
                "document_inventory_requirements": {
                    "theme": "data_and_evidence",
                    "status": "evidence",
                    "canonical_target": "<capsule>/MANIFEST.json",
                },
            }
        ),
        encoding="utf-8",
    )
    organ_maturity_inventory = {
        "schema": "cynic.organ_maturity_inventory.v1",
        "status": "draft",
        **draft_governance,
        "authorities": {"topology_contract": "axioms/repo-topology-v1.json", "topology_policy": "docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md"},
        "maturity_order": [
            "burn_candidate",
            "legacy_blocked",
            "quarantined",
            "embryonic",
            "transitional",
            "canonical_partial",
            "canonical",
        ],
        "organs": [{"path": "organs/supervisor/", "maturity": "canonical", "evidence": ["MANIFEST.yaml"], "next_action": "extend through boundary"}],
        "totals": {"organs": 1, "by_maturity": {"canonical": 1}},
    }
    (contracts / "organ-maturity-inventory-v1.json").write_text(json.dumps(organ_maturity_inventory), encoding="utf-8")
    organ_layer_topology = {
        "schema": "cynic.organ_layer_topology.v1",
        "status": "normative",
        "authorities": {
            "human_readable_policy": "docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md",
            "policy_matrix": "docs/foundation/CYNIC_ORGANISM_POLICY_MATRIX.md",
            "organ_maturity_inventory": "axioms/organ-maturity-inventory-v1.json",
        },
        "symbolic_vocabulary": {
            "Gevurah": "constraint",
            "Tiferet": "integration",
            "Hod": "interface",
            "Keter": "source intent",
            "Binah": "bounded form",
        },
        "layer_policy": {
            "ordering": "ascending_abstraction",
            "rank_meaning": "test fixture",
            "required_fields": [
                "id",
                "rank",
                "name",
                "responsibility",
                "system_primitives",
                "symbolic_alignment",
                "must_not_own",
                "canonical_examples",
            ],
        },
        "layers": [
            {
                "parent_layer": None,
                "projection_kind": "fractal_rank",
                "id": "governance_control_plane",
                "rank": 20,
                "name": "Governance control plane",
                "responsibility": "test fixture",
                "system_primitives": ["lease", "claim table", "policy evaluator"],
                "symbolic_alignment": ["Gevurah", "Tiferet", "Hod"],
                "must_not_own": ["test"],
                "canonical_examples": ["organs/supervisor/"],
            }
        ],
        "organs": [
            {
                "path": "organs/supervisor/",
                "layer_id": "governance_control_plane",
                "layer_rank": 20,
                "owner": "organ-supervisor",
                "why": "test fixture",
                "evidence": ["MANIFEST.yaml", "src/core", "src/entrypoints"],
            }
        ],
        "totals": {"layers": 1, "organs": 1, "by_layer": {"governance_control_plane": 1}},
    }
    (contracts / "organ-layer-topology-v1.json").write_text(json.dumps(organ_layer_topology), encoding="utf-8")


def write_surface_inventory_contracts(contracts: Path, draft_governance: dict[str, str]) -> None:
    for name in [
        "service-surface-inventory-v1.json",
        "diagnostic-surface-inventory-v1.json",
        "implementation-surface-inventory-v1.json",
        "infra-surface-inventory-v1.json",
        "data-control-surface-inventory-v1.json",
    ]:
        schema = {
            "service-surface-inventory-v1.json": "cynic.service_surface_inventory.v1",
            "diagnostic-surface-inventory-v1.json": "cynic.diagnostic_surface_inventory.v1",
            "implementation-surface-inventory-v1.json": "cynic.implementation_surface_inventory.v1",
            "infra-surface-inventory-v1.json": "cynic.infra_surface_inventory.v1",
            "data-control-surface-inventory-v1.json": "cynic.data_control_surface_inventory.v1",
        }[name]
        (contracts / name).write_text(
            json.dumps(
                {
                    "schema": schema,
                    "status": "draft",
                    **draft_governance,
                    **(
                        {"watched_root_globs": ["debug_*.py"]}
                        if name == "diagnostic-surface-inventory-v1.json"
                        else {"watched_globs": ["data/reports/**/.gitignore"]}
                        if name == "data-control-surface-inventory-v1.json"
                        else {}
                    ),
                    "surfaces": [],
                    "totals": {"surfaces": 0, "by_status": {}},
                }
            ),
            encoding="utf-8",
        )


def write_git_membrane_contract(contracts: Path, draft_governance: dict[str, str]) -> None:
    (contracts / "git-membrane-closure-plan-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.git_membrane_closure_plan.v1",
                "status": "draft",
                **draft_governance,
                "allowed_dispositions": ["track"],
                "execution_requirements": {
                    "requires_git_metadata_write": True,
                    "preflight_command": "make git-membrane-closure-preflight",
                    "claim_required": True,
                    "executor_authority": "test cortex",
                    "forbidden_when": ["test blocker"],
                },
                "post_closure_proof": {
                    "required_commands": ["make git-membrane-closure-check w=<wave>"],
                    "required_evidence": ["selected wave is closed"],
                },
                "waves": [
                    {
                        "wave": 1,
                        "theme": "contracts",
                        "disposition": "track",
                        "targets": [{"path": "axioms/", "classification": "contract_source", "reason": "contracts"}],
                    }
                ],
            }
        ),
        encoding="utf-8",
    )


def write_runtime_principals_contract(contracts: Path) -> None:
    runtime_principals_contract = {
        "schema": "cynic.runtime_principals.v1",
        "status": "normative",
        "dedicated_user": {"name": "cynic"},
        "postures": {
            "bounded_yolo": {
                "forbidden_nopasswd_concerns": ["infra-write", "network"],
                "forbidden_commands": ["rm -rf /", "git reset --hard HEAD~"],
            }
        },
        "cortex_cli_policy": {
            "required_agents": ["codex", "antigravity"],
            "required_runtime_user": "cynic",
            "required_posture": "bounded_yolo",
            "projection": "infra/agent-policy.toml",
        },
        "projection_requirements": {"agent_policy_toml": {"path": "infra/agent-policy.toml"}},
        "activation": {
            "script": "organs/supervisor/entrypoints/runtime_principals.py",
            "forbidden_legacy_sudoers": ["cynic ALL=(ALL) NOPASSWD: /usr/bin/pvesh"],
            "primary_dependency_updates": {
                "operator_command": "cynic update",
                "allowed_update_commands": {
                    "codex": ["codex", "update"],
                    "agy": ["agy", "update"],
                },
                "forbidden_update_patterns": ["curl | sh"],
            },
        },
    }
    (contracts / "runtime-principals-v1.json").write_text(json.dumps(runtime_principals_contract), encoding="utf-8")



def write_meta_layer_contract(contracts: Path) -> None:
    meta_layer_contract = {
        "schema": "cynic.meta_layer_contract.v1",
        "status": "normative",
        "version": 1,
        "purpose": "test fixture",
        "recursion_boundary": {
            "rule": "test root",
            "root_contract": "axioms/meta-layer-contract-v1.json",
            "registry_contract": "axioms/axiom-registry-v1.json",
            "allowed_meta_contracts": [
                "axioms/meta-layer-contract-v1.json",
                "axioms/axiom-registry-v1.json",
            ],
            "projection_rule": "test projection",
            "new_contract_admission": ["classify in registry"],
            "forbidden": ["competing contract root"],
        },
        "symbolic_vocabulary": {
            "Keter": "source intent",
            "Binah": "bounded form",
            "Hod": "interface",
            "Gevurah": "constraint",
            "Tiferet": "integration",
        },
        "precedence": ["policy"],
        "layers": [
            {
                "name": "policy",
                "canonical_home": "docs/foundation/",
                "engineering_role": "test fixture",
                "agent_question": "which policy applies?",
                "system_primitives": ["policy file", "review boundary", "admission criterion"],
                "symbolic_alignment": ["Keter", "Binah", "Hod"],
                "may_decide": ["test"],
            }
        ],
    }
    (contracts / "meta-layer-contract-v1.json").write_text(json.dumps(meta_layer_contract), encoding="utf-8")



def write_infrastructure_mcp_contract(contracts: Path) -> None:
    infrastructure_mcp_contract = {
        "schema": "cynic.infrastructure_mcp.v1",
        "status": "target_contract",
        "principles": [
            "deny_by_default",
            "registry_desired_state_first",
            "synapse_dns_is_the_service_contract",
            "mutating_operations_require_preflight_and_audit",
            "private_keys_never_leave_the_node",
        ],
        "operation_families": {
            "ipam": {
                "operations": {
                    "fleet.node.exec": {
                        "mcp_tool": "fleet_node_exec",
                        "mutates": True,
                        "lxc_bridge_contract": "axioms/lxc-exec-bridge-v1.json",
                        "current_coverage": [
                            "lxc_structured_precondition_missing_until_exec_bridge_exists_or_transport_configured"
                        ],
                    }
                },
                "lxc_exec_bridge.plan": {"mcp_tool": "lxc_exec_bridge_plan", "mutates": False},
                "lxc_exec_bridge.bundle": {
                    "mcp_tool": "lxc_exec_bridge_bundle",
                    "mutates": False,
                    "writes_local_output_dir": True,
                },
                "lxc_exec_bridge.status": {"mcp_tool": "lxc_exec_bridge_status", "mutates": False},
            },
            "mesh": {
                "operations": {
                    "network.mesh.status": {
                        "mcp_tool": "network_mesh_status",
                        "mutates": False,
                        "runtime_observation_contract": {
                            "schema": "cynic.network_mesh.runtime_observation.v1",
                            "required_fields": [
                                "schema",
                                "source",
                                "status",
                                "severity",
                                "blockers",
                                "missing",
                                "drift",
                                "blocking",
                            ],
                            "shared_between": [
                                "crates/cynic-node",
                                "organs/fleet",
                            ],
                        },
                    },
                    "network.mesh.dns.render": {"mcp_tool": "network_mesh_dns_render", "mutates": False},
                    "network.mesh.dns.activation_plan": {
                        "mcp_tool": "network_mesh_dns_activation_plan",
                        "mutates": False,
                    },
                    "network.mesh.dns.activation_bundle": {
                        "mcp_tool": "network_mesh_dns_activation_bundle",
                        "mutates": False,
                        "writes_local_output_dir": True,
                    },
                    "network.mesh.dns.activation_status": {
                        "mcp_tool": "network_mesh_dns_activation_status",
                        "mutates": False,
                    },
                }
            },
            "storage": {
                "operations": {
                    "storage.primary.plan": {"mcp_tool": "storage_primary_plan", "mutates": False},
                    "storage.primary.bundle": {
                        "mcp_tool": "storage_primary_bundle",
                        "mutates": False,
                        "writes_local_output_dir": True,
                    },
                    "storage.primary.status": {
                        "mcp_tool": "storage_primary_status",
                        "mutates": False,
                        "preconditions_reported": [
                            "registry_declares_storage_node",
                            "endpoint_is_synapse_dns",
                            "bridge_active",
                            "fleet_observes_node_running",
                            "mesh_reaches_storage",
                            "bundle_complete",
                            "service_health_healthy",
                        ],
                    },
                    "storage.primary.readiness": {
                        "mcp_tool": "storage_primary_readiness",
                        "mutates": False,
                        "composes": [
                            "storage.primary.status",
                            "lxc_exec_bridge.status",
                            "network.mesh.status",
                            "network.mesh.dns.activation_status",
                            "fleet.node.status",
                            "service.health",
                        ],
                    },
                }
            },
        },
    }
    (contracts / "infrastructure-mcp-v1.json").write_text(json.dumps(infrastructure_mcp_contract), encoding="utf-8")



def write_lxc_exec_bridge_contract(contracts: Path) -> None:
    lxc_exec_bridge_contract = {
        "schema": "cynic.lxc_exec_bridge.v1",
        "status": "normative",
        "authority": {
            "infra_contract": "axioms/infrastructure-mcp-v1.json",
            "organ": "organs/fleet",
            "policy_file": "organs/fleet/policies/lxc-exec-bridge.policy.json",
        },
        "principles": [
            "deny_by_default",
            "no_raw_shell",
            "no_agent_direct_pct_exec",
            "root_owned_helper_only",
            "sudoers_allows_helper_not_pct",
            "structured_json_request",
            "audit_every_attempt",
        ],
        "runtime_shape": {
            "helper_path": "/usr/local/libexec/cynic-lxc-exec-bridge",
            "source_path": "organs/fleet/src/entrypoints/lxc_exec_bridge_helper.py",
            "request_transport": "stdin_json",
            "response_transport": "stdout_json",
            "bundle_command": "make lxc-exec-bridge-bundle out=/tmp/cynic-lxc-bridge",
            "mcp_transport_env": {"CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET": "required"},
            "sudoers_subject": "cynic",
            "sudoers_rule_shape": "cynic ALL=(root) NOPASSWD: /usr/local/libexec/cynic-lxc-exec-bridge",
            "forbidden_sudoers_shape": "cynic ALL=(root) NOPASSWD: /usr/sbin/pct, /usr/bin/pct, /usr/bin/bash, /bin/sh",
        },
        "request": {"required_fields": ["node", "vmid", "purpose", "argv", "agent_id"]},
        "policy": {
            "allowed_command_shape": "argv_prefix_allowlist_per_purpose",
            "max_timeout_seconds": 60,
            "allowed_purposes": ["read_only", "service_control"],
        },
        "mcp_integration": {
            "operation": "fleet.node.exec",
            "lxc_transition": "fleet_node_exec returns precondition_missing until CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET is configured",
            "activation_evidence": [
                "source_helper_tests_pass",
                "helper_exists_root_owned",
                "sudoers_rule_matches_contract",
                "policy_validates",
                "dry_run_read_only_command_passes",
                "raw_pct_exec_not_allowed_to_agent",
                "activation_status_reports_active",
            ],
        },
    }
    (contracts / "lxc-exec-bridge-v1.json").write_text(json.dumps(lxc_exec_bridge_contract), encoding="utf-8")



def write_storage_primary_activation_contract(contracts: Path) -> None:
    storage_primary_activation_contract = {
        "schema": "cynic.storage_primary_activation.v1",
        "status": "normative",
        "authority": {
            "registry": "infra/registry.json",
            "storage_schema": "infra/surrealdb/schema.surql",
            "lxc_exec_bridge": "axioms/lxc-exec-bridge-v1.json",
            "network_mesh_dns": "axioms/infrastructure-mcp-v1.json:network.mesh.dns.activation_bundle",
            "organ": "organs/keep",
        },
        "target": {
            "node": "cynic-lxc-surrealdb",
            "role": "storage_primary",
            "service": "surrealdb",
            "canonical_endpoint": "http://organ-surrealdb.synapse.cynic:8000",
        },
        "status_contract": {
            "schema": "cynic.storage_primary_activation.status.v1",
            "active_requires": [
                "endpoint_is_synapse_dns",
                "bridge_active",
                "fleet_observes_node_running",
                "mesh_reaches_storage",
                "bundle_complete",
                "service_health_healthy",
            ],
        },
        "activation_request_contract": {
            "schema": "cynic.storage_primary_activation.request.v1",
            "bundles": {
                "lxc_exec_bridge": "/tmp/cynic-lxc-bridge",
                "network_mesh_dns": "/tmp/cynic-synapse-dns",
                "storage_primary": "/tmp/cynic-storage-primary",
            },
            "coord_targets": ["organs/fleet", "organs/keep", "infra/registry.json"],
            "required_sequence": [
                "activate lxc_exec_bridge",
                "activate network_mesh_dns",
                "activate storage_primary",
                "verify storage_primary_readiness.ready",
                "verify kernel health has no storage_down",
            ],
            "no_cortex_root_execution": True,
        },
        "forbidden_patterns": [
            "curl | sh",
            "root/root default credentials",
            "192.168.* service endpoint",
            "direct pct exec exposed to agents",
        ],
    }
    (contracts / "storage-primary-activation-v1.json").write_text(
        json.dumps(storage_primary_activation_contract),
        encoding="utf-8",
    )

def write_runtime_and_infra_contracts(contracts: Path) -> None:
    write_runtime_principals_contract(contracts)
    write_meta_layer_contract(contracts)
    write_infrastructure_mcp_contract(contracts)
    write_lxc_exec_bridge_contract(contracts)
    write_storage_primary_activation_contract(contracts)


def validate_with_fixture_paths(module: Any, root: Path, contracts: Path):
    old_values = (
        module.WORKSPACE_ROOT,
        module.CONTRACT_PATH,
        module.THEME_CONTRACT_PATH,
        module.DOCUMENT_THEME_INVENTORY_PATH,
        module.META_LAYER_CONTRACT_PATH,
        module.API_LAYER_ALIGNMENT_CONTRACT_PATH,
        module.API_TOPOLOGY_CONTRACT_PATH,
        module.OPENAPI_CONTRACT_PATH,
        module.KERNEL_REST_ROUTER_PATH,
        module.API_DISCOVERY_RS_PATH,
        module.API_MIDDLEWARE_RS_PATH,
        module.INFRASTRUCTURE_MCP_CONTRACT_PATH,
        module.LXC_EXEC_BRIDGE_CONTRACT_PATH,
        module.STORAGE_PRIMARY_ACTIVATION_CONTRACT_PATH,
        module.ORGAN_MATURITY_INVENTORY_PATH,
        module.ORGAN_LAYER_TOPOLOGY_PATH,
        module.EVIDENCE_CAPSULE_CONTRACT_PATH,
        module.SERVICE_SURFACE_INVENTORY_PATH,
        module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
        module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
        module.INFRA_SURFACE_INVENTORY_PATH,
        module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
        module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
        module.RUNTIME_PRINCIPALS_CONTRACT_PATH,
        module.AGENT_POLICY_PATH,
        module.GOVERNED_CONTRACT_PATHS,
        module.CONTRACTS_ROOT,
        module.MCP_SRC_ROOT,
        module.ORGANS_ROOT,
        module.SERVICES_ROOT,
        module.git_ls_files,
        module.git_ls_untracked,
    )
    try:
        module.WORKSPACE_ROOT = root
        module.CONTRACT_PATH = contracts / "repo-topology-v1.json"
        module.THEME_CONTRACT_PATH = contracts / "organism-theme-topology-v1.json"
        module.DOCUMENT_THEME_INVENTORY_PATH = contracts / "document-theme-inventory-v1.json"
        module.META_LAYER_CONTRACT_PATH = contracts / "meta-layer-contract-v1.json"
        module.API_LAYER_ALIGNMENT_CONTRACT_PATH = contracts / "api-layer-alignment-v1.json"
        module.API_TOPOLOGY_CONTRACT_PATH = contracts / "api-topology-v1.json"
        module.OPENAPI_CONTRACT_PATH = root / "docs" / "api" / "openapi.json"
        module.KERNEL_REST_ROUTER_PATH = root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "mod.rs"
        module.API_DISCOVERY_RS_PATH = root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "discovery.rs"
        module.API_MIDDLEWARE_RS_PATH = root / "crates" / "cynic-kernel" / "src" / "api" / "rest" / "middleware.rs"
        module.INFRASTRUCTURE_MCP_CONTRACT_PATH = contracts / "infrastructure-mcp-v1.json"
        module.LXC_EXEC_BRIDGE_CONTRACT_PATH = contracts / "lxc-exec-bridge-v1.json"
        module.STORAGE_PRIMARY_ACTIVATION_CONTRACT_PATH = contracts / "storage-primary-activation-v1.json"
        module.ORGAN_MATURITY_INVENTORY_PATH = contracts / "organ-maturity-inventory-v1.json"
        module.ORGAN_LAYER_TOPOLOGY_PATH = contracts / "organ-layer-topology-v1.json"
        module.EVIDENCE_CAPSULE_CONTRACT_PATH = contracts / "evidence-capsule-v1.json"
        module.SERVICE_SURFACE_INVENTORY_PATH = contracts / "service-surface-inventory-v1.json"
        module.DIAGNOSTIC_SURFACE_INVENTORY_PATH = contracts / "diagnostic-surface-inventory-v1.json"
        module.IMPLEMENTATION_SURFACE_INVENTORY_PATH = contracts / "implementation-surface-inventory-v1.json"
        module.INFRA_SURFACE_INVENTORY_PATH = contracts / "infra-surface-inventory-v1.json"
        module.DATA_CONTROL_SURFACE_INVENTORY_PATH = contracts / "data-control-surface-inventory-v1.json"
        module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = contracts / "git-membrane-closure-plan-v1.json"
        module.RUNTIME_PRINCIPALS_CONTRACT_PATH = contracts / "runtime-principals-v1.json"
        module.AGENT_POLICY_PATH = root / "infra" / "agent-policy.toml"
        module.GOVERNED_CONTRACT_PATHS = [
            module.CONTRACT_PATH,
            module.THEME_CONTRACT_PATH,
            module.DOCUMENT_THEME_INVENTORY_PATH,
            module.META_LAYER_CONTRACT_PATH,
            module.API_LAYER_ALIGNMENT_CONTRACT_PATH,
            module.API_TOPOLOGY_CONTRACT_PATH,
            module.OPENAPI_CONTRACT_PATH,
            module.KERNEL_REST_ROUTER_PATH,
            module.API_DISCOVERY_RS_PATH,
            module.API_MIDDLEWARE_RS_PATH,
            module.INFRASTRUCTURE_MCP_CONTRACT_PATH,
            module.LXC_EXEC_BRIDGE_CONTRACT_PATH,
            module.STORAGE_PRIMARY_ACTIVATION_CONTRACT_PATH,
            module.ORGAN_MATURITY_INVENTORY_PATH,
            module.ORGAN_LAYER_TOPOLOGY_PATH,
            module.EVIDENCE_CAPSULE_CONTRACT_PATH,
            module.SERVICE_SURFACE_INVENTORY_PATH,
            module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
            module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
            module.INFRA_SURFACE_INVENTORY_PATH,
            module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
            module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
            module.RUNTIME_PRINCIPALS_CONTRACT_PATH,
        ]
        module.CONTRACTS_ROOT = contracts
        module.MCP_SRC_ROOT = root / "crates" / "cynic-mcp" / "src"
        module.ORGANS_ROOT = root / "organs"
        module.SERVICES_ROOT = root / "services"
        module.git_ls_files = lambda *args: []
        module.git_ls_untracked = lambda *args: []
        return module.validate()
    finally:
        (
            module.WORKSPACE_ROOT,
            module.CONTRACT_PATH,
            module.THEME_CONTRACT_PATH,
            module.DOCUMENT_THEME_INVENTORY_PATH,
            module.META_LAYER_CONTRACT_PATH,
            module.API_LAYER_ALIGNMENT_CONTRACT_PATH,
            module.API_TOPOLOGY_CONTRACT_PATH,
            module.OPENAPI_CONTRACT_PATH,
            module.KERNEL_REST_ROUTER_PATH,
            module.API_DISCOVERY_RS_PATH,
            module.API_MIDDLEWARE_RS_PATH,
            module.INFRASTRUCTURE_MCP_CONTRACT_PATH,
            module.LXC_EXEC_BRIDGE_CONTRACT_PATH,
            module.STORAGE_PRIMARY_ACTIVATION_CONTRACT_PATH,
            module.ORGAN_MATURITY_INVENTORY_PATH,
            module.ORGAN_LAYER_TOPOLOGY_PATH,
            module.EVIDENCE_CAPSULE_CONTRACT_PATH,
            module.SERVICE_SURFACE_INVENTORY_PATH,
            module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
            module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
            module.INFRA_SURFACE_INVENTORY_PATH,
            module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
            module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
            module.RUNTIME_PRINCIPALS_CONTRACT_PATH,
            module.AGENT_POLICY_PATH,
            module.GOVERNED_CONTRACT_PATHS,
            module.CONTRACTS_ROOT,
            module.MCP_SRC_ROOT,
            module.ORGANS_ROOT,
            module.SERVICES_ROOT,
            module.git_ls_files,
            module.git_ls_untracked,
        ) = old_values


def validate_complete_contract_authority_fixture(module: Any):
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        contracts = root / "axioms"
        contracts.mkdir(parents=True)
        draft_governance = {
            "draft_reason": "test fixture",
            "promotion_condition": "test fixture promotes when validation passes",
            "decision_authority": "organ-supervisor",
        }
        write_base_files(root)
        (root / "data" / "emergence" / "probe" / "2026-06-24").mkdir(parents=True)
        (root / "data" / "emergence" / "probe" / "2026-06-24" / "MANIFEST.json").write_text(
            json.dumps(
                {
                    "id": "probe-2026-06-24",
                    "created_at": "2026-06-24T00:00:00Z",
                    "root": "data/emergence/probe/2026-06-24",
                    "purpose": "test capsule",
                    "status": "complete_v1",
                    "sensitivity_policy": {},
                }
            ),
            encoding="utf-8",
        )
        write_api_discovery_files(root)
        write_api_contracts(contracts)
        write_minimal_peer_contracts(contracts)
        write_repo_topology_contract(contracts)
        write_theme_and_evidence_contracts(contracts, draft_governance)
        write_surface_inventory_contracts(contracts, draft_governance)
        write_git_membrane_contract(contracts, draft_governance)
        write_runtime_and_infra_contracts(contracts)

        return validate_with_fixture_paths(module, root, contracts)
