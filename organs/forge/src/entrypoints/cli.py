#!/usr/bin/env python3
"""
Entrypoint for organ-forge.
Implements the hexagonal architecture.
"""
import sys
import argparse
import asyncio
import json
from pathlib import Path

# Add CYNIC root to path for local imports if needed
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from organs.forge.src.adapters.git_adapter import LocalGitPerceiver
from organs.forge.src.adapters.storage_adapter import JsonStateStorage
from organs.forge.src.adapters.kernel_adapter import (
    KernelRestClient,
    apply_lifecycle_activation_evidence_patch,
    build_lifecycle_activation_plan,
    build_lifecycle_activation_requests,
    build_lifecycle_profile_payloads,
    build_lifecycle_runtime_paths,
    build_lifecycle_service_evidence_patch,
    build_lifecycle_task_payload,
    dispatch_lifecycle_activation_requests,
    lifecycle_health_status_result,
    lifecycle_profile_intents,
    normalize_kernel_base_url,
    persist_lifecycle_registry_node_activation,
    persist_lifecycle_activation_evidence_patch,
    validate_lifecycle_task_payload,
    validate_lifecycle_policy,
    verify_lifecycle_runtime_paths,
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

async def delegate_intent(repo_path, intent, emit_logs: bool = True) -> dict:
    perceiver = LocalGitPerceiver(repo_path)
    kernel_client = KernelRestClient()
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
    for intent in intents:
        try:
            if not json_mode:
                print(f"▶ Delegating task '{intent}' to CYNIC Hub...")
            result = await delegate_intent(repo_path, intent, emit_logs=not json_mode)
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

def run_lifecycle_check(strict: bool = True) -> int:
    try:
        kernel_client = KernelRestClient(require_api_key=False, strict_lifecycle=strict)
        if not strict:
            kernel_client.inspect_fleet()
        print(json.dumps(kernel_client.lifecycle_context(), separators=(",", ":")))
        return 0
    except Exception as e:
        payload = {"status": "error", "error": str(e)}
        if strict:
            payload.update(lifecycle_error_context())
        print(
            json.dumps(
                payload,
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def lifecycle_error_context() -> dict:
    try:
        diagnostic_client = KernelRestClient(
            require_api_key=False,
            strict_lifecycle=False,
        )
        diagnostic_client.inspect_fleet()
        context = diagnostic_client.lifecycle_context()
    except Exception as diagnostic_error:
        return {
            "diagnostic_status": "error",
            "diagnostic_error": str(diagnostic_error),
        }
    return {
        "diagnostic_status": "ok",
        "lifecycle_blockers": context.get("lifecycle_blockers", []),
        "activation_evidence_gap_details": context.get(
            "activation_evidence_gap_details",
            [],
        ),
        "status_nodes_ready": context.get("status_nodes_ready"),
        "fleet_nodes_ready": context.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": context.get("lifecycle_nodes_ready"),
    }

def lifecycle_blockers_payload() -> dict:
    diagnostic_client = KernelRestClient(
        require_api_key=False,
        strict_lifecycle=False,
    )
    diagnostic_client.inspect_fleet()
    context = diagnostic_client.lifecycle_context()
    return {
        "status": "ok" if context.get("lifecycle_nodes_ready") else "blocked",
        "lifecycle": context.get("lifecycle"),
        "build_target": context.get("build_target"),
        "deploy_target": context.get("deploy_target"),
        "status_nodes_ready": context.get("status_nodes_ready"),
        "fleet_nodes_ready": context.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": context.get("lifecycle_nodes_ready"),
        "blockers": context.get("lifecycle_blockers", []),
    }

def run_lifecycle_blockers() -> int:
    try:
        payload = lifecycle_blockers_payload()
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_activation_plan() -> int:
    try:
        payload = build_lifecycle_activation_plan(inspect_fleet=True)
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_activation_requests() -> int:
    try:
        payload = build_lifecycle_activation_requests(inspect_fleet=True)
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_activation_dispatch() -> int:
    try:
        payload = dispatch_lifecycle_activation_requests(inspect_fleet=False)
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_runtime_paths() -> int:
    try:
        payload = build_lifecycle_runtime_paths()
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_verify_runtime_paths(node_name: str) -> int:
    try:
        payload = verify_lifecycle_runtime_paths(node_name)
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_service_evidence_patch(node_name: str) -> int:
    try:
        payload = build_lifecycle_service_evidence_patch(node_name)
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_validate_activation_evidence_patch(raw_patch: str) -> int:
    try:
        patch = json.loads(raw_patch)
        result = apply_lifecycle_activation_evidence_patch(patch)
        print(
            json.dumps(
                {
                    "status": result["status"],
                    "node": result["node"],
                    "role": result["role"],
                    "accepted_runtime_paths": result["accepted_runtime_paths"],
                    "accepted_services": result["accepted_services"],
                },
                separators=(",", ":"),
            )
        )
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_apply_activation_evidence_patch(
    raw_patch: str,
    registry_path: str | None = None,
) -> int:
    try:
        patch = json.loads(raw_patch)
        result = persist_lifecycle_activation_evidence_patch(
            patch,
            registry_path=registry_path,
        )
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_activate_node(
    node_name: str,
    registry_path: str | None = None,
) -> int:
    try:
        result = persist_lifecycle_registry_node_activation(
            node_name,
            registry_path=registry_path,
        )
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_lint() -> int:
    try:
        print(json.dumps(validate_lifecycle_policy(), separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_task_payload(intent: str, domain: str, agent_id: str) -> int:
    try:
        payload = build_lifecycle_task_payload(
            intent=intent,
            domain=domain,
            agent_id=agent_id,
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_validate_task_payload(raw_payload: str) -> int:
    try:
        payload = json.loads(raw_payload)
        result = validate_lifecycle_task_payload(payload)
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_profile_payloads(profile: str, domain: str, agent_id: str) -> int:
    try:
        result = build_lifecycle_profile_payloads(
            profile=profile,
            domain=domain,
            agent_id=agent_id,
        )
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_health_accepted(status_code: str) -> int:
    try:
        result = lifecycle_health_status_result(int(status_code))
        print(json.dumps(result, separators=(",", ":")))
        return 0 if result["accepted"] else 1
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_lifecycle_profile_intents(profile: str) -> int:
    try:
        intents = lifecycle_profile_intents(profile)
        print(
            json.dumps(
                {"status": "ok", "profile": profile, "intents": intents},
                separators=(",", ":"),
            )
        )
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def run_kernel_url(raw: str | None = None) -> int:
    try:
        value = raw or __import__("os").environ.get("CYNIC_REST_ADDR", "")
        print(normalize_kernel_base_url(value))
        return 0
    except Exception as e:
        print(
            json.dumps(
                {"status": "error", "error": str(e)},
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1

def main():
    parser = argparse.ArgumentParser(description="CYNIC Forge Organ CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)
    
    # Audit Command
    subparsers.add_parser("audit", help="Audit the repository state")

    subparsers.add_parser("lifecycle-check", help="Resolve registry lifecycle and verify canonical kernel bus")
    subparsers.add_parser("lifecycle-diagnose", help="Resolve registry lifecycle without enforcing readiness")
    subparsers.add_parser("lifecycle-blockers", help="Print concise lifecycle readiness blockers")
    subparsers.add_parser("lifecycle-activation-plan", help="Print registry-derived lifecycle activation plan")
    subparsers.add_parser("lifecycle-runtime-paths", help="Print registry-derived runtime path requirements")
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
    
    if args.command == "audit":
        return run_audit(repo_path, state_file)
    elif args.command == "lifecycle-check":
        return run_lifecycle_check()
    elif args.command == "lifecycle-diagnose":
        return run_lifecycle_check(strict=False)
    elif args.command == "lifecycle-blockers":
        return run_lifecycle_blockers()
    elif args.command == "lifecycle-activation-plan":
        return run_lifecycle_activation_plan()
    elif args.command == "lifecycle-runtime-paths":
        return run_lifecycle_runtime_paths()
    elif args.command == "lifecycle-verify-runtime-paths":
        return run_lifecycle_verify_runtime_paths(args.node)
    elif args.command == "lifecycle-service-evidence-patch":
        return run_lifecycle_service_evidence_patch(args.node)
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
