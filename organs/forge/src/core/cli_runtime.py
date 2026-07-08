"""
CLI runtime for organ-forge.

The process entrypoint is a thin membrane in src/entrypoints/cli.py.
"""
import sys
import argparse
import asyncio
import json
import os
from pathlib import Path


def _bootstrap_repo_path() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            return candidate
    raise RuntimeError("Could not resolve repository root for organ-forge cli")


PROJECT_ROOT = _bootstrap_repo_path()

from organs.forge.src.adapters.git_adapter import LocalGitPerceiver
from organs.forge.src.adapters.storage_adapter import JsonStateStorage
from organs.forge.src.adapters.kernel_adapter import KernelRestClient
from organs.forge.src.core.organ_memory import KernelOutboxDelivery, OrganMemoryStore
from organs.forge.src.entrypoints.lifecycle_cli import (
    lifecycle_error_context,
    lifecycle_profile_intents,
    parse_service_states_argument,
    run_kernel_url,
    run_lifecycle_activate_node,
    run_lifecycle_activation_dispatch,
    run_lifecycle_activation_plan,
    run_lifecycle_activation_requests,
    run_lifecycle_bootstrap_source_check,
    run_lifecycle_apply_activation_evidence_patch,
    run_lifecycle_blockers,
    run_organism_gap_map,
    run_organism_next_repair_batch,
    run_lifecycle_root_install_sequence,
    run_lifecycle_root_install_script,
    run_lifecycle_check,
    run_lifecycle_constellation,
    run_lifecycle_consumer_status,
    run_lifecycle_health_accepted,
    run_lifecycle_lint,
    run_lifecycle_mcp_freshness,
    run_lifecycle_runtime_artifacts,
    run_lifecycle_profile_intents,
    run_lifecycle_profile_payloads,
    run_lifecycle_runtime_paths,
    run_lifecycle_service_evidence_patch,
    run_lifecycle_status,
    run_lifecycle_task_payload,
    run_lifecycle_transition_contract,
    run_lifecycle_transition_debt,
    run_lifecycle_validate_activation_evidence_patch,
    run_lifecycle_validate_task_payload,
    run_lifecycle_verify_runtime_paths,
)
from organs.forge.src.core.services import HealthAuditor, BuildDelegationService


def run_audit(repo_path, state_file):
    # 1. Initialize Adapters
    perceiver = LocalGitPerceiver(repo_path)
    storage = JsonStateStorage(state_file)
    
    # 2. Initialize Domain Services
    auditor = HealthAuditor()
    
    print(f"Perceiving repo state at {repo_path}...")
    state = perceiver.gather_state()
    
    print("Auditing state...")
    audited_state = auditor.audit(state)
    
    print(f"Saving audited state to {state_file}...")
    storage.save_state(audited_state)
    
    print(f"Done. Health Score: {audited_state.health_score}/100")
    for alert in audited_state.alerts:
        print(f"[{alert.severity}] {alert.message}")
        
    return 0


def run_organ_memory_status(repo_path) -> int:
    state_root = Path(repo_path) / "organs" / "forge" / "state"
    print(
        json.dumps(
            OrganMemoryStore(state_root).status(),
            separators=(",", ":"),
        )
    )
    return 0


def run_organ_memory_record(repo_path, event_type: str, payload: str, storage_class: str) -> int:
    try:
        payload_obj = json.loads(payload)
        if not isinstance(payload_obj, dict):
            raise ValueError("payload must be a JSON object")
        state_root = Path(repo_path) / "organs" / "forge" / "state"
        event = OrganMemoryStore(state_root).append_journal(
            event_type=event_type,
            payload=payload_obj,
            storage_class=storage_class,
        )
        print(json.dumps({"status": "ok", "event": event.__dict__}, separators=(",", ":")))
        return 0
    except Exception as exc:
        print(
            json.dumps({"status": "error", "error": str(exc)}, separators=(",", ":")),
            file=sys.stderr,
        )
        return 1


def run_organ_memory_flush(
    repo_path,
    *,
    dry_run: bool,
    limit: int,
    max_attempts: int,
    kernel_url: str | None = None,
    api_key: str | None = None,
) -> int:
    try:
        state_root = Path(repo_path) / "organs" / "forge" / "state"
        store = OrganMemoryStore(state_root)
        if dry_run:
            payload = store.flush_outbox(
                delivery=KernelOutboxDelivery("", ""),
                dry_run=True,
                limit=limit,
                max_attempts=max_attempts,
            )
        else:
            base_url = kernel_url or os.environ.get("CYNIC_REST_ADDR", "")
            auth_key = api_key if api_key is not None else os.environ.get("CYNIC_API_KEY", "")
            payload = store.flush_outbox(
                delivery=KernelOutboxDelivery(base_url, auth_key),
                dry_run=False,
                limit=limit,
                max_attempts=max_attempts,
            )
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload.get("status") == "ok" else 1
    except Exception as exc:
        print(
            json.dumps({"status": "error", "error": str(exc)}, separators=(",", ":")),
            file=sys.stderr,
        )
        return 1

async def run_daemon(repo_path, state_file):
    from organs.forge.controllers.lifecycle_agent import LifecycleAgentController
    
    print(json.dumps({
        "status": "starting", 
        "mode": "daemon", 
        "message": "Forge Organ starting Agentic Controller (Autonomous Mempool Consumer)"
    }))
    
    controller = LifecycleAgentController(repo_path)
    try:
        await controller.run_loop()
    except asyncio.CancelledError:
        print(json.dumps({"status": "shutdown", "message": "Forge Organ shutting down gracefully"}))
    except Exception as e:
        print(json.dumps({"status": "fatal", "message": f"Agent loop crashed: {e}"}))
        return 1
    return 0

def _profile_uses_bootstrap_lane(profile: str) -> bool:
    return profile == "bootstrap"


async def delegate_intent(repo_path, intent, emit_logs: bool = True, strict_lifecycle: bool = True) -> dict:
    perceiver = LocalGitPerceiver(repo_path)
    kernel_client = KernelRestClient(strict_lifecycle=strict_lifecycle)
    service = BuildDelegationService(perceiver, kernel_client)
    result = await service.delegate_task(intent, emit_logs=emit_logs)
    return {"intent": intent, "status": "ok", "result": result}

async def run_delegate(repo_path, intent, json_mode: bool = False):
    try:
        if not json_mode:
            print(f"▶ Delegating task '{intent}' to CYNIC Hub...")
        result = await delegate_intent(repo_path, intent, emit_logs=not json_mode)
        if json_mode:
            print(
                json.dumps(
                    {"status": "ok", **result},
                    separators=(",", ":"),
                )
            )
        else:
            print(f"\n▶ Task '{intent}' completed successfully.")
        return 0
    except Exception as e:
        if json_mode:
            payload = {"status": "error", "intent": intent, "error": str(e)}
            payload.update(lifecycle_error_context())
            print(json.dumps(payload, separators=(",", ":")), file=sys.stderr)
        else:
            print(f"[!] Task '{intent}' failed: {e}", file=sys.stderr)
        return 1

async def run_delegate_profile(repo_path, profile, json_mode: bool = False):
    try:
        intents = lifecycle_profile_intents(profile)
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

    results = []
    strict_lifecycle = not _profile_uses_bootstrap_lane(profile)
    for intent in intents:
        try:
            if not json_mode:
                print(f"▶ Delegating task '{intent}' to CYNIC Hub...")
            result = await delegate_intent(
                repo_path,
                intent,
                emit_logs=not json_mode,
                strict_lifecycle=strict_lifecycle,
            )
            if not json_mode:
                print(f"\n▶ Task '{intent}' completed successfully.")
        except Exception as e:
            if json_mode:
                payload = {
                    "status": "error",
                    "profile": profile,
                    "intent": intent,
                    "error": str(e),
                    "results": results,
                }
                payload.update(lifecycle_error_context())
                print(json.dumps(payload, separators=(",", ":")), file=sys.stderr)
            else:
                print(f"[!] Task '{intent}' failed: {e}", file=sys.stderr)
            return 1
        results.append(result)
    print(
        json.dumps(
            {"status": "ok", "profile": profile, "intents": intents, "results": results},
            separators=(",", ":"),
        )
    )
    return 0

def main():
    parser = argparse.ArgumentParser(description="CYNIC Forge Organ CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)
    
    # Daemon Command
    subparsers.add_parser("daemon", help="Run the organ in Event Listener Mode (Model B) listening to the CYNIC Kernel")

    # Audit Command
    subparsers.add_parser("audit", help="Audit the repository state")
    subparsers.add_parser("organ-memory-status", help="Print organ-forge local memory/outbox status")
    memory_record_parser = subparsers.add_parser(
        "organ-memory-record",
        help="Append a JSON object to organ-forge local memory journal",
    )
    memory_record_parser.add_argument("event_type", help="Local event type")
    memory_record_parser.add_argument("payload", help="JSON object payload")
    memory_record_parser.add_argument(
        "--class",
        dest="storage_class",
        default="local_journal",
        help="Organ-memory storage class, e.g. raw, ccm, local_journal",
    )
    memory_flush_parser = subparsers.add_parser(
        "organ-memory-flush",
        help="Flush organ-forge local outbox to the Kernel with idempotent local accounting",
    )
    memory_flush_parser.add_argument("--dry-run", action="store_true", help="Inspect pending outbox without delivery")
    memory_flush_parser.add_argument("--limit", type=int, default=25, help="Maximum outbox events to process")
    memory_flush_parser.add_argument("--max-attempts", type=int, default=3, help="Deadletter after this many failed attempts")
    memory_flush_parser.add_argument("--kernel-url", help="Override CYNIC_REST_ADDR for controlled tests")
    memory_flush_parser.add_argument("--api-key", help="Override CYNIC_API_KEY for controlled tests")

    subparsers.add_parser("lifecycle-check", help="Resolve registry lifecycle and verify canonical kernel bus")
    subparsers.add_parser("lifecycle-diagnose", help="Resolve registry lifecycle without enforcing readiness")
    subparsers.add_parser("lifecycle-blockers", help="Print concise lifecycle readiness blockers")
    subparsers.add_parser(
        "organism-gap-map",
        help="Print a machine-readable map of observed and structural organism gaps",
    )
    subparsers.add_parser(
        "organism-next-repair-batch",
        help="Print the next executable repair batch derived from the organism gap map",
    )
    subparsers.add_parser(
        "lifecycle-root-install-sequence",
        help="Print the minimal ordered root-install sequence for local network repair dependencies",
    )
    subparsers.add_parser(
        "lifecycle-root-install-script",
        help="Print an executable shell script for the canonical local root-install sequence",
    )
    subparsers.add_parser("lifecycle-status", help="Print concise lifecycle status for humans and agents")
    subparsers.add_parser("lifecycle-consumer-status", help="Print organ-forge lifecycle mempool consumer status")
    subparsers.add_parser("lifecycle-constellation", help="Print registry-backed lifecycle constellation summary")
    subparsers.add_parser("lifecycle-transition-contract", help="Print registry-backed lifecycle state transition contract")
    subparsers.add_parser("lifecycle-transition-debt", help="Print lifecycle transition debt by state layer")
    subparsers.add_parser("lifecycle-activation-plan", help="Print registry-derived lifecycle activation plan")
    subparsers.add_parser(
        "lifecycle-bootstrap-source-check",
        help="Clone and verify the configured bootstrap source contract",
    )
    subparsers.add_parser("lifecycle-runtime-paths", help="Print registry-derived runtime path requirements")
    subparsers.add_parser(
        "lifecycle-mcp-freshness",
        help="Check MCP launcher freshness and map it to the lifecycle mcp-reload policy",
    )
    subparsers.add_parser(
        "lifecycle-runtime-artifacts",
        help="Print registry-derived runtime artifact propagation manifest",
    )
    verify_runtime_paths_parser = subparsers.add_parser(
        "lifecycle-verify-runtime-paths",
        help="Verify registry runtime path proofs on the current machine for one node",
    )
    verify_runtime_paths_parser.add_argument("node", help="Registry node name to verify")
    service_evidence_parser = subparsers.add_parser(
        "lifecycle-service-evidence-patch",
        help="Print registry-derived service activation evidence patch for one node",
    )
    service_evidence_parser.add_argument("node", help="Registry node name to build evidence for")
    service_evidence_parser.add_argument(
        "--service-states",
        help="Optional JSON mapping of service_name -> {active, checked_at, source}",
    )
    evidence_patch_parser = subparsers.add_parser(
        "lifecycle-validate-activation-evidence-patch",
        help="Validate a runtime activation evidence patch against the registry contract",
    )
    evidence_patch_parser.add_argument("patch", help="Activation evidence patch JSON")
    apply_evidence_patch_parser = subparsers.add_parser(
        "lifecycle-apply-activation-evidence-patch",
        help="Validate and persist an activation evidence patch into the registry",
    )
    apply_evidence_patch_parser.add_argument("patch", help="Activation evidence patch JSON")
    apply_evidence_patch_parser.add_argument(
        "--registry-path",
        help="Override registry path for tests or controlled maintenance",
    )
    activate_node_parser = subparsers.add_parser(
        "lifecycle-activate-node",
        help="Validate activation evidence and persist node status=active",
    )
    activate_node_parser.add_argument("node", help="Registry node name to activate")
    activate_node_parser.add_argument(
        "--registry-path",
        help="Override registry path for tests or controlled maintenance",
    )
    subparsers.add_parser("lifecycle-lint", help="Validate registry lifecycle policy structure")
    subparsers.add_parser(
        "lifecycle-activation-requests",
        help="Build organ-generated activation requests from the lifecycle plan",
    )
    subparsers.add_parser(
        "lifecycle-activation-dispatch",
        help="Dispatch organ-generated activation requests to the kernel mempool",
    )
    payload_parser = subparsers.add_parser(
        "lifecycle-task-payload",
        help="Build a registry-derived /agent-tasks payload",
    )
    payload_parser.add_argument("intent", help="Lifecycle intent, e.g. deploy-kernel")
    payload_parser.add_argument("domain", help="Task domain to emit into the mempool")
    payload_parser.add_argument("agent_id", help="Agent id recorded on the task")
    validate_payload_parser = subparsers.add_parser(
        "lifecycle-validate-task-payload",
        help="Validate a registry-derived /agent-tasks payload before dispatch",
    )
    validate_payload_parser.add_argument("payload", help="JSON payload to validate")
    profile_payloads_parser = subparsers.add_parser(
        "lifecycle-profile-payloads",
        help="Build and validate every /agent-tasks payload for a lifecycle request profile",
    )
    profile_payloads_parser.add_argument("profile", help="Lifecycle request profile")
    profile_payloads_parser.add_argument("domain", help="Task domain to emit into the mempool")
    profile_payloads_parser.add_argument("agent_id", help="Agent id recorded on each task")
    profile_intents_parser = subparsers.add_parser(
        "lifecycle-profile-intents",
        help="Print the intents declared by a registry request profile",
    )
    profile_intents_parser.add_argument("profile", help="Lifecycle request profile")
    health_parser = subparsers.add_parser(
        "lifecycle-health-accepted",
        help="Check an HTTP status against registry kernel_bus_probe.accepted_status",
    )
    health_parser.add_argument("status_code", help="HTTP status code")
    kernel_url_parser = subparsers.add_parser(
        "kernel-url",
        help="Normalize CYNIC_REST_ADDR the same way organ-forge REST clients do",
    )
    kernel_url_parser.add_argument("raw", nargs="?", help="Optional raw kernel address")
    
    # Delegate Command
    delegate_parser = subparsers.add_parser("delegate", help="Delegate a build task to the distributed network")
    
    # Add an intent argument properly
    delegate_parser.add_argument("intent", help="The intent (e.g., build-and-test)")
    delegate_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON only")
    profile_parser = subparsers.add_parser(
        "delegate-profile",
        help="Delegate every lifecycle intent declared by a registry request profile",
    )
    profile_parser.add_argument("profile", help="Lifecycle request profile, e.g. gate-2 or deploy")
    profile_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON only")
    
    args = parser.parse_args()
    
    repo_path = PROJECT_ROOT
    state_file = PROJECT_ROOT / "organs" / "forge" / "state" / "state.json"
    
    if args.command == "daemon":
        return asyncio.run(run_daemon(repo_path, state_file))
    elif args.command == "audit":
        return run_audit(repo_path, state_file)
    elif args.command == "organ-memory-status":
        return run_organ_memory_status(repo_path)
    elif args.command == "organ-memory-record":
        return run_organ_memory_record(repo_path, args.event_type, args.payload, args.storage_class)
    elif args.command == "organ-memory-flush":
        return run_organ_memory_flush(
            repo_path,
            dry_run=args.dry_run,
            limit=args.limit,
            max_attempts=args.max_attempts,
            kernel_url=args.kernel_url,
            api_key=args.api_key,
        )
    elif args.command == "lifecycle-check":
        return run_lifecycle_check()
    elif args.command == "lifecycle-diagnose":
        return run_lifecycle_check(strict=False)
    elif args.command == "lifecycle-blockers":
        return run_lifecycle_blockers()
    elif args.command == "organism-gap-map":
        return run_organism_gap_map()
    elif args.command == "organism-next-repair-batch":
        return run_organism_next_repair_batch()
    elif args.command == "lifecycle-root-install-sequence":
        return run_lifecycle_root_install_sequence()
    elif args.command == "lifecycle-root-install-script":
        return run_lifecycle_root_install_script()
    elif args.command == "lifecycle-status":
        return run_lifecycle_status()
    elif args.command == "lifecycle-consumer-status":
        return run_lifecycle_consumer_status()
    elif args.command == "lifecycle-constellation":
        return run_lifecycle_constellation()
    elif args.command == "lifecycle-transition-contract":
        return run_lifecycle_transition_contract()
    elif args.command == "lifecycle-transition-debt":
        return run_lifecycle_transition_debt()
    elif args.command == "lifecycle-activation-plan":
        return run_lifecycle_activation_plan()
    elif args.command == "lifecycle-bootstrap-source-check":
        return run_lifecycle_bootstrap_source_check()
    elif args.command == "lifecycle-runtime-paths":
        return run_lifecycle_runtime_paths()
    elif args.command == "lifecycle-mcp-freshness":
        return run_lifecycle_mcp_freshness()
    elif args.command == "lifecycle-runtime-artifacts":
        return run_lifecycle_runtime_artifacts()
    elif args.command == "lifecycle-verify-runtime-paths":
        return run_lifecycle_verify_runtime_paths(args.node)
    elif args.command == "lifecycle-service-evidence-patch":
        try:
            parsed_service_states = parse_service_states_argument(args.service_states)
            return run_lifecycle_service_evidence_patch(
                args.node,
                service_states=parsed_service_states,
            )
        except Exception as e:
            print(
                json.dumps(
                    {"status": "error", "error": str(e)},
                    separators=(",", ":"),
                ),
                file=sys.stderr,
            )
            return 1
    elif args.command == "lifecycle-validate-activation-evidence-patch":
        return run_lifecycle_validate_activation_evidence_patch(args.patch)
    elif args.command == "lifecycle-apply-activation-evidence-patch":
        return run_lifecycle_apply_activation_evidence_patch(
            args.patch,
            registry_path=args.registry_path,
        )
    elif args.command == "lifecycle-activate-node":
        return run_lifecycle_activate_node(
            args.node,
            registry_path=args.registry_path,
        )
    elif args.command == "lifecycle-lint":
        return run_lifecycle_lint()
    elif args.command == "lifecycle-activation-requests":
        return run_lifecycle_activation_requests()
    elif args.command == "lifecycle-activation-dispatch":
        return run_lifecycle_activation_dispatch()
    elif args.command == "lifecycle-task-payload":
        return run_lifecycle_task_payload(args.intent, args.domain, args.agent_id)
    elif args.command == "lifecycle-validate-task-payload":
        return run_lifecycle_validate_task_payload(args.payload)
    elif args.command == "lifecycle-profile-payloads":
        return run_lifecycle_profile_payloads(args.profile, args.domain, args.agent_id)
    elif args.command == "lifecycle-profile-intents":
        return run_lifecycle_profile_intents(args.profile)
    elif args.command == "lifecycle-health-accepted":
        return run_lifecycle_health_accepted(args.status_code)
    elif args.command == "kernel-url":
        return run_kernel_url(args.raw)
    elif args.command == "delegate":
        return asyncio.run(run_delegate(repo_path, args.intent, json_mode=args.json))
    elif args.command == "delegate-profile":
        return asyncio.run(run_delegate_profile(repo_path, args.profile, json_mode=args.json))

if __name__ == "__main__":
    sys.exit(main())
