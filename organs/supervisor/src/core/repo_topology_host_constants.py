#!/usr/bin/env python3
"""Constants used by topology-host validation."""

from __future__ import annotations

from pathlib import Path
import re
import sys

from organs.contracts.src.project_root import resolve_repo_root

WORKSPACE_ROOT = resolve_repo_root(Path(__file__).resolve())
if str(WORKSPACE_ROOT) not in sys.path:
    sys.path.insert(0, str(WORKSPACE_ROOT))

CONTRACT_PATH = WORKSPACE_ROOT / "axioms/repo-topology-v1.json"
THEME_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/organism-theme-topology-v1.json"
DOCUMENT_THEME_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/document-theme-inventory-v1.json"
META_LAYER_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/meta-layer-contract-v1.json"
API_LAYER_ALIGNMENT_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/api-layer-alignment-v1.json"
API_TOPOLOGY_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/api-topology-v1.json"
OPENAPI_CONTRACT_PATH = WORKSPACE_ROOT / "docs/api/openapi.json"
INFRASTRUCTURE_MCP_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/infrastructure-mcp-v1.json"
LXC_EXEC_BRIDGE_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/lxc-exec-bridge-v1.json"
STORAGE_PRIMARY_ACTIVATION_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/storage-primary-activation-v1.json"
ORGAN_MATURITY_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/organ-maturity-inventory-v1.json"
PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/python-organ-authority-inventory-v1.json"
ORGAN_LAYER_TOPOLOGY_PATH = WORKSPACE_ROOT / "axioms/organ-layer-topology-v1.json"
MULTISCALE_ORGANISM_PROPERTY_MAP_PATH = WORKSPACE_ROOT / "axioms/multiscale-organism-property-map-v1.json"
EVIDENCE_CAPSULE_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/evidence-capsule-v1.json"
SERVICE_SURFACE_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/service-surface-inventory-v1.json"
DIAGNOSTIC_SURFACE_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/diagnostic-surface-inventory-v1.json"
IMPLEMENTATION_SURFACE_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/implementation-surface-inventory-v1.json"
INFRA_SURFACE_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/infra-surface-inventory-v1.json"
DATA_CONTROL_SURFACE_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/data-control-surface-inventory-v1.json"
GIT_MEMBRANE_CLOSURE_PLAN_PATH = WORKSPACE_ROOT / "axioms/git-membrane-closure-plan-v1.json"
RUNTIME_PRINCIPALS_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/runtime-principals-v1.json"
ORGAN_MANIFEST_SCHEMA_PATH = WORKSPACE_ROOT / "axioms/schemas/organ-manifest-v1.schema.json"
AGENT_POLICY_PATH = WORKSPACE_ROOT / "infra/agent-policy.toml"
CONTRACTS_ROOT = WORKSPACE_ROOT / "axioms"
MCP_SRC_ROOT = WORKSPACE_ROOT / "crates/cynic-mcp/src"
KERNEL_REST_SRC_ROOT = WORKSPACE_ROOT / "crates/cynic-kernel/src/api/rest"
KERNEL_REST_ROUTER_PATH = KERNEL_REST_SRC_ROOT / "mod.rs"
API_DISCOVERY_RS_PATH = KERNEL_REST_SRC_ROOT / "discovery.rs"
API_MIDDLEWARE_RS_PATH = KERNEL_REST_SRC_ROOT / "middleware.rs"
ORGANS_ROOT = WORKSPACE_ROOT / "organs"
SERVICES_ROOT = WORKSPACE_ROOT / "services"

RUNTIME_STATE_GLOBS = [
    "infra/organ-anvil/*.json",
    "infra/organ-anvil/*.jsonl",
    "organs/*/state/*.json",
    "organs/*/state/*.jsonl",
    "organs/*/state/**/*.json",
    "organs/*/state/**/*.jsonl",
]

LOCAL_DATA_RUNTIME_OUTPUT_GLOBS = [
    "data/calibration_results_real.json",
    "data/population_stats.json",
    "data/token_profiles.jsonl",
    "data/market_snapshots/**",
    "data/obsidian/**",
]

PROVIDER_MUTATION_ROOTS = ["scripts", "infra/scripts", "organs/lifecycle/scripts"]
PROVIDER_MUTATION_SUFFIXES = (".py", ".sh")
TRANSITIONAL_WRAPPER_SUFFIXES = (".py", ".sh", ".ps1")

PROVIDER_MUTATION_RE = re.compile(
    r"\b("
    r"pct\s+(create|start|exec|stop|destroy|set)|"
    r"qm\s+(create|start|stop|destroy|set|clone|exec)|"
    r"provision_vm|ProxmoxAdapter|"
    r"/api2/json/nodes/[^\"'\\s]+/(qemu|lxc)/[^\"'\\s]+/(agent/exec|config|status)"
    r")\b",
    re.IGNORECASE,
)

TRANSITIONAL_PATH_REQUIRED_FIELDS = {
    "path",
    "owner",
    "replacement",
    "expiry_condition",
    "allowed_file_kinds",
    "forbidden_concerns",
}

MISPLACED_ORGAN_REQUIRED_FIELDS = {
    "path",
    "status",
    "owner",
    "target",
    "replacement",
    "expiry_condition",
}

LEGACY_HERMES_X_ROOT = Path("scripts") / "hermes-x"
LEGACY_HERMES_X_PATH_MARKER = LEGACY_HERMES_X_ROOT.as_posix()
LEGACY_HERMES_X_WRAPPER_MARKER = "from organs.twitter.src."
LEGACY_HERMES_X_FUNC_RE = re.compile(r"^\s*(def|class)\s+", re.MULTILINE)
INFRA_HERMES_X_REF_MARKER = f"@CYNIC_PROJECT_ROOT@/{LEGACY_HERMES_X_PATH_MARKER}"

LEGACY_HERMES_X_REFERENCE_ALLOWED_GLOBS = {
    "axioms/repo-topology-v1.json",
    "docs/superpowers/**",
    "docs/foundation/CYNIC_DEPRECATED_SURFACE_REPORT*.md",
    "data/reports/**",
    ".cortex/plans/**",
    "organs/*/state/*.jsonl",
    "organs/*/state/**/*.jsonl",
    "organs/twitter/MANIFEST.yaml",
    "organs/browser/MANIFEST.yaml",
    "organs/twitter/src/entrypoints/hermes_x.py",
    "organs/supervisor/src/core/repo_topology_host.py",
    "organs/supervisor/src/core/repo_topology_legacy.py",
    "organs/supervisor/src/core/repo_topology_policy.py",
    "organs/supervisor/controllers/validate_repo_topology.py",
    "organs/supervisor/tests/test_validate_repo_topology.py",
}

LEGACY_HERMES_X_INFRA_SCAN_SUFFIXES = {
    ".conf",
    ".service",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}

LEGACY_HERMES_X_SYSTEMD_PREFIX = "hermes-x-"
LEGACY_HERMES_X_SYSTEMD_DEP_RE = re.compile(
    r"^\s*(After|Before|Requires|RequiresMountsFor|Wants|PartOf|WantedBy|Also)\s*=\s*(.+)$"
)
LEGACY_HERMES_X_UNIT_TOKEN_RE = re.compile(r"\bhermes-x-[A-Za-z0-9_.@%-]+(?:\\.service)?\b")
LEGACY_HERMES_X_DOMAIN_ALIAS_RE = re.compile(r"\bHERMES_AGENT_DOMAIN_ALLOW=.*\bhermes-x\b")

LEGACY_HERMES_X_SCAN_SUFFIXES = {
    ".json",
    ".jsonl",
    ".md",
    ".py",
    ".rs",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}

DEPLOYMENT_PROJECTION_PATH_RE = re.compile(
    r"(?:/" + r"home/|/" + r"Users/|\\\\[A-Za-z]:\\|/Bureau/|%h/.*?/Bureau/)"
)

THEME_REQUIRED_FIELDS = {
    "id",
    "class",
    "owns",
    "canonical_sources",
    "allowed_projection_paths",
    "must_not_own",
    "drift_rule",
}

DOC_ZONE_REQUIRED_FIELDS = {
    "path",
    "class",
    "may_own",
    "must_not_own",
    "canonicality",
}

ROOT_CLASS_REQUIRED_FIELDS = {"path", "class", "owns", "must_not_own"}

DOCUMENT_INVENTORY_REQUIRED_FIELDS = {"path", "theme", "status", "canonical_target"}
DOCUMENT_INVENTORY_TRANSIENT_STATUSES = {"draft", "archive", "delete_candidate"}
DOCUMENT_INVENTORY_STATUS_TRANSITION_REQUIRED_FIELDS = {
    "owner",
    "rule",
    "exit_condition",
}

ORGAN_MATURITY_INVENTORY_REQUIRED_FIELDS = {"path", "maturity", "evidence", "next_action"}
ORGAN_MATURITY_NON_TERMINAL_STATUSES = {
    "canonical_partial",
    "transitional",
    "quarantined",
    "legacy_blocked",
    "embryonic",
}

ORGAN_MATURITY_TRANSITION_REQUIRED_FIELDS = {
    "owner",
    "rule",
    "promotes_to",
    "exit_condition",
}

ORGAN_LAYER_REQUIRED_FIELDS = {
    "id",
    "rank",
    "name",
    "responsibility",
    "system_primitives",
    "symbolic_alignment",
    "must_not_own",
    "canonical_examples",
}

ORGAN_LAYER_ENTRY_REQUIRED_FIELDS = {
    "path",
    "layer_id",
    "layer_rank",
    "owner",
    "why",
    "evidence",
}

ORGAN_UNTRACKED_BODY_EVIDENCE = "untracked organ body files"

SERVICE_SURFACE_REQUIRED_FIELDS = {"path", "class", "owner", "status", "evidence", "next_action"}
SURFACE_STATUS_TRANSITION_REQUIRED_FIELDS = {
    "owner",
    "rule",
    "exit_condition",
}

DIAGNOSTIC_SURFACE_REQUIRED_FIELDS = {
    "path",
    "status",
    "owner",
    "classification",
    "replacement",
    "expiry_condition",
}

IMPLEMENTATION_SURFACE_REQUIRED_FIELDS = {"path", "class", "owner", "status", "evidence", "next_action"}
INFRA_SURFACE_REQUIRED_FIELDS = {"path", "class", "owner", "status", "evidence", "next_action"}
DATA_CONTROL_SURFACE_REQUIRED_FIELDS = {
    "path",
    "status",
    "owner",
    "classification",
    "canonical_target",
    "expiry_condition",
}

GIT_MEMBRANE_CLOSURE_TARGET_REQUIRED_FIELDS = {"path", "reason", "classification"}
GIT_MEMBRANE_CLOSURE_TARGET_CLASSIFICATIONS = {
    "agent_adapter_projection",
    "application_source",
    "contract_source",
    "curated_evidence_capsule",
    "deployment_projection",
    "diagnostic",
    "developer_diagnostic_candidate",
    "document_projection",
    "generated_report_output_filter",
    "infra_contract_projection",
    "kernel_probe",
    "organ_test",
    "organ_body",
    "product_projection",
    "report_hygiene_control",
    "runtime_source",
    "topology_validator_source",
    "transitional_wrapper",
}

GIT_MEMBRANE_CLOSURE_WAVE_REQUIRED_FIELDS = {"wave", "theme", "disposition", "targets"}
GIT_MEMBRANE_CLOSURE_EXECUTION_REQUIRED_FIELDS = {
    "requires_git_metadata_write",
    "preflight_command",
    "claim_required",
    "executor_authority",
    "forbidden_when",
}
GIT_MEMBRANE_CLOSURE_PROOF_REQUIRED_FIELDS = {
    "required_commands",
    "required_evidence",
}

RUNTIME_PRINCIPALS_REQUIRED_FIELDS = {
    "schema",
    "status",
    "dedicated_user",
    "postures",
    "cortex_cli_policy",
    "projection_requirements",
    "activation",
}

EVIDENCE_CAPSULE_REQUIRED_FIELDS = {
    "root",
    "capsule_depth",
    "manifest_name",
    "manifest_required_fields",
    "document_inventory_requirements",
}

DRAFT_CONTRACT_REQUIRED_FIELDS = {
    "draft_reason",
    "promotion_condition",
    "decision_authority",
}

GOVERNED_CONTRACT_PATHS = [
    CONTRACT_PATH,
    THEME_CONTRACT_PATH,
    DOCUMENT_THEME_INVENTORY_PATH,
    ORGAN_MATURITY_INVENTORY_PATH,
    ORGAN_LAYER_TOPOLOGY_PATH,
    EVIDENCE_CAPSULE_CONTRACT_PATH,
    SERVICE_SURFACE_INVENTORY_PATH,
    DIAGNOSTIC_SURFACE_INVENTORY_PATH,
    IMPLEMENTATION_SURFACE_INVENTORY_PATH,
    INFRA_SURFACE_INVENTORY_PATH,
    DATA_CONTROL_SURFACE_INVENTORY_PATH,
    GIT_MEMBRANE_CLOSURE_PLAN_PATH,
    RUNTIME_PRINCIPALS_CONTRACT_PATH,
]

MANIFEST_SCRIPT_REFERENCE_SUFFIXES = {
    ".py",
    ".sh",
    ".ts",
    ".js",
    ".md",
    ".json",
    ".yaml",
    ".yml",
}

MANIFEST_UNTRACKED_SCRIPT_ALLOWED_STATUSES = {
    "organ_entrypoint",
    "transition_debt",
    "quarantined",
    "external_runtime",
}

MANIFEST_UNTRACKED_SCRIPT_REQUIRED_FIELDS = {
    "integration_status",
    "owner",
    "expiry_condition",
}

RUST_HEXAGONAL_CRATE_REQUIRED_FIELDS = {"path", "required_paths", "owner", "reason"}

DOCUMENT_INVENTORY_SENSITIVE_MARKDOWN_ROOTS = [
    "docs/foundation/",
    "docs/architecture/",
    "docs/ops/",
    "docs/internal/",
    "docs/hackathon/",
    "docs/research/",
    "docs/reference/",
    "data/emergence/",
    "data/reports/",
    "packages/",
]

DOCUMENT_CANONICAL_ZONE_VALUES = {
    "canonical_when_indexed",
    "canonical_by_schema_and_status",
}

GOD_OBJECT_LINE_LIMIT = 800
GOD_OBJECT_SCAN_GLOBS = [
    "organs/*/controllers/*.py",
    "organs/*/src/**/*.py",
    "organs/*/tests/test_*.py",
]
