"""Compatibility facade for lifecycle CLI command handlers."""

from organs.forge.src.entrypoints.lifecycle_activation_cli import (
    run_lifecycle_activate_node,
    run_lifecycle_activation_dispatch,
    run_lifecycle_activation_plan,
    run_lifecycle_activation_requests,
    run_lifecycle_apply_activation_evidence_patch,
    run_lifecycle_validate_activation_evidence_patch,
)
from organs.forge.src.entrypoints.lifecycle_diagnostic_cli import (
    lifecycle_blockers_payload,
    organism_gap_map_payload,
    organism_next_repair_batch_payload,
    lifecycle_root_install_sequence_payload,
    lifecycle_root_install_script,
    lifecycle_dispatch_error_payload,
    lifecycle_error_context,
    run_kernel_url,
    run_lifecycle_bootstrap_source_check,
    run_lifecycle_blockers,
    run_organism_gap_map,
    run_organism_next_repair_batch,
    run_lifecycle_root_install_sequence,
    run_lifecycle_root_install_script,
    run_lifecycle_check,
    run_lifecycle_constellation,
    run_lifecycle_health_accepted,
    run_lifecycle_lint,
    run_lifecycle_mcp_freshness,
    run_lifecycle_runtime_artifacts,
    run_lifecycle_runtime_paths,
    run_lifecycle_service_evidence_patch,
    run_lifecycle_verify_runtime_paths,
)
from organs.forge.src.core.lifecycle_diagnostic_payloads import parse_service_states_argument
from organs.forge.src.entrypoints.lifecycle_payload_cli import (
    lifecycle_profile_intents,
    run_lifecycle_profile_intents,
    run_lifecycle_profile_payloads,
    run_lifecycle_task_payload,
    run_lifecycle_validate_task_payload,
)
from organs.forge.src.entrypoints.lifecycle_transition_cli import (
    run_lifecycle_transition_contract,
    run_lifecycle_transition_debt,
)
from organs.forge.src.entrypoints.lifecycle_status_cli import run_lifecycle_status
from organs.forge.src.entrypoints.lifecycle_consumer_cli import run_lifecycle_consumer_status
