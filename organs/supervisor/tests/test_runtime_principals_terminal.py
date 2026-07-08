import contextlib
import io
import json
import subprocess
import sys
import unittest
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "runtime_principals.py"
REPO_ROOT = resolve_repo_root(MODULE_PATH.resolve())


def load_module():
    return load_module_from_path("runtime_principals", MODULE_PATH)


class RuntimePrincipalsTerminalTests(unittest.TestCase):
    def test_organism_text_shows_next_organ_actions_when_activation_requests_exist(self):
        module = load_module()
        report = module.organism_report(run_probes=False)
        report["lifecycle_activation"] = {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "blocked",
            "activation_request_summary": [
                {
                    "action": "refresh_fleet_inventory",
                    "node": None,
                    "code": None,
                    "coord_targets": ["organs/fleet"],
                    "consumer": "organ-forge-hermes-agent",
                }
            ],
            "activation_dispatch_command": "cynic activate",
        }

        body = module.organism_text(report)

        self.assertIn("Next organ actions:", body)
        self.assertIn("refresh_fleet_inventory: organs/fleet -> organ-forge-hermes-agent", body)
        self.assertIn("Dispatch: cynic activate", body)

    def test_lifecycle_activation_report_uses_framework_dispatch_command(self):
        module = load_module()
        report = module.organism_report(run_probes=False)
        report["lifecycle_activation"] = {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "blocked",
            "activation_request_summary": [
                {
                    "action": "activate_registry_node",
                    "node": "cynic-lxc-surrealdb",
                    "consumer": "organ-forge-hermes-agent",
                }
            ],
            "activation_dispatch_command": "cynic activate",
        }

        body = module.organism_text(report)

        self.assertIn("Activation pressure:", body)
        self.assertIn("Dispatch: cynic activate", body)

    def test_activation_dispatch_text_is_human_readable_and_points_to_json(self):
        module = load_module()
        body = module.activation_dispatch_text(
            {
                "status": "ok",
                "plan_status": "blocked",
                "dispatch_summary": {
                    "requested": 3,
                    "dispatched": 2,
                    "skipped_existing": 1,
                    "task_ids": ["task-1", "task-2"],
                },
                "skipped_existing": [
                    {
                        "task_id": "task-0",
                        "action": "refresh_fleet_inventory",
                        "reason": "pending_duplicate",
                    }
                ],
                "blockers": [{"code": "fleet_inventory:degraded"}],
                "inspect_command": "cynic activate --dry-run",
                "proof_commands": ["cynic actions", "cynic organism", "cynic activate --json"],
                "done_when": ["organ-forge consumes activation tasks"],
            }
        )

        self.assertIn("CYNIC lifecycle activation dispatch", body)
        self.assertIn("Requested: 3", body)
        self.assertIn("Dispatched: 2", body)
        self.assertIn("Skipped existing: 1", body)
        self.assertIn("task-1", body)
        self.assertIn("task-0: refresh_fleet_inventory (pending_duplicate)", body)
        self.assertIn("fleet_inventory:degraded", body)
        self.assertIn("Inspect before dispatch: cynic activate --dry-run", body)
        self.assertIn("Proof commands:", body)
        self.assertIn("cynic actions", body)
        self.assertIn("Done when:", body)
        self.assertIn("Machine dispatch output: cynic activate --json", body)

    def test_organism_actions_report_is_native_and_non_mutating(self):
        module = load_module()
        globals_ref = module.organism_actions_report.__globals__
        original = globals_ref["lifecycle_activation_report"]

        def fake_activation():
            return {
                "schema": "cynic.lifecycle.activation_summary.v1",
                "status": "blocked",
                "activation_request_summary": [
                    {
                        "action": "refresh_fleet_inventory",
                        "node": None,
                        "code": None,
                        "priority": 97,
                        "coord_targets": ["organs/fleet"],
                        "consumer": "organ-forge-hermes-agent",
                    }
                ],
                "blockers": [
                    {
                        "code": "fleet_inventory:degraded",
                        "kind": "fleet_inventory",
                        "actual": "degraded",
                        "required": "ok",
                        "resolver_organ": "organ-fleet",
                        "resolver_consumer": "organ-forge-hermes-agent",
                        "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
                    },
                    {
                        "category": "fleet",
                        "code": "active_without_runtime_proof:cynic-vm-v2",
                        "kind": "fleet_inventory_evidence_gap",
                        "provider": "fleet_inventory",
                    }
                ],
                "activation_dispatch_command": "cynic activate",
            }

        globals_ref["lifecycle_activation_report"] = fake_activation
        try:
            report = module.organism_actions_report()
            body = module.organism_actions_text(report)
        finally:
            globals_ref["lifecycle_activation_report"] = original

        self.assertEqual("cynic.organism.actions.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertEqual(1, report["action_count"])
        self.assertEqual("organs/fleet", report["actions"][0]["target"])
        self.assertEqual("degraded", report["blockers"][0]["actual"])
        self.assertEqual("organ-fleet", report["blockers"][0]["resolver_organ"])
        self.assertEqual("cynic fleet status", report["blockers"][0]["native_check_command"])
        self.assertEqual("cynic-vm-v2", report["blockers"][1]["node"])
        self.assertEqual("organ-fleet", report["blockers"][1]["resolver_organ"])
        self.assertEqual("organ-forge-hermes-agent", report["blockers"][1]["resolver_consumer"])
        self.assertEqual("cynic fleet status", report["blockers"][1]["native_check_command"])
        self.assertEqual("cynic activate", report["dispatch_command"])
        self.assertTrue(report["dispatch_mutates_mempool"])
        self.assertEqual(1, report["dispatch_plan"]["request_count"])
        self.assertEqual(["organ-forge-hermes-agent"], report["dispatch_plan"]["consumers"])
        self.assertEqual(["organs/fleet"], report["dispatch_plan"]["coord_targets"])
        self.assertEqual("cynic activate --dry-run", report["dispatch_plan"]["operator_sequence"][0]["command"])
        self.assertEqual("cynic activate", report["dispatch_plan"]["operator_sequence"][1]["command"])
        self.assertIn("Kernel mempool", report["dispatch_plan"]["read_back"])
        self.assertIn("CYNIC organism actions", body)
        self.assertIn("refresh_fleet_inventory: organs/fleet p=97 -> organ-forge-hermes-agent", body)
        self.assertIn("fleet_inventory:degraded (required=ok, actual=degraded", body)
        self.assertIn("active_without_runtime_proof:cynic-vm-v2", body)
        self.assertIn("check: cynic fleet status", body)
        self.assertIn("Dispatch plan:", body)
        self.assertIn("requests: 1", body)
        self.assertIn("consumers: organ-forge-hermes-agent", body)
        self.assertIn("dispatch: cynic activate", body)
        self.assertIn("read-back:", body)
        self.assertNotIn("python3 organs/fleet/src/entrypoints/cli.py", body)

    def test_request_lifecycle_profile_wraps_delegate_profile(self):
        module = load_module()
        globals_ref = module.request_lifecycle_profile.__globals__
        original = globals_ref["_run_capture"]

        def fake_run(command, *, timeout=15, stdout_limit=1200, stderr_limit=1200):
            return {
                "status": "ok",
                "exit_code": 0,
                "stdout": '{"status":"ok","profile":"gate-2","task_ids":["task-1"]}',
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            result = module.request_lifecycle_profile("gate-2")
        finally:
            globals_ref["_run_capture"] = original

        self.assertEqual("ok", result["status"])
        self.assertEqual("cynic gate-2", result["command"])
        self.assertEqual("cynic gate-2 --dry-run", result["inspect_command"])
        self.assertEqual("cynic gate-2 --json", result["dispatch_json_command"])
        self.assertEqual("inspect", result["operator_sequence"][0]["step"])
        self.assertEqual("cynic gate-2 --dry-run", result["operator_sequence"][0]["command"])
        self.assertIn("cynic organism", result["proof_commands"])
        self.assertEqual(["task-1"], result["task_ids"])

    def test_request_lifecycle_profile_extracts_task_ids_from_delegate_results(self):
        module = load_module()
        globals_ref = module.request_lifecycle_profile.__globals__
        original = globals_ref["_run_capture"]

        def fake_run(command, *, timeout=15, stdout_limit=1200, stderr_limit=1200):
            return {
                "status": "ok",
                "exit_code": 0,
                "stdout": (
                    '{"status":"ok","profile":"gate-2","results":['
                    '{"intent":"build-and-test","status":"ok","result":"{\\\"status\\\":\\\"pending\\\",\\\"task_id\\\":\\\"agent_tasks:one\\\"}\\n"},'
                    '{"intent":"integration-test","status":"ok","result":"{\\\"status\\\":\\\"pending\\\",\\\"task_id\\\":\\\"agent_tasks:two\\\"}\\n"}'
                    "]}",
                )[0],
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            result = module.request_lifecycle_profile("gate-2")
        finally:
            globals_ref["_run_capture"] = original

        self.assertEqual(["agent_tasks:one", "agent_tasks:two"], result["task_ids"])
        self.assertEqual(
            "python3 organs/forge/src/entrypoints/cli.py delegate-profile gate-2 --json",
            result["fallback_command"],
        )
        self.assertTrue(result["mutates_mempool"])

    def test_request_lifecycle_profile_prefers_json_error_payload_from_stderr(self):
        module = load_module()
        globals_ref = module.request_lifecycle_profile.__globals__
        original = globals_ref["_run_capture"]

        def fake_run(command, *, timeout=15, stdout_limit=1200, stderr_limit=1200):
            return {
                "status": "failed",
                "exit_code": 1,
                "stdout": "",
                "stderr": (
                    '{"status":"error","error":"agent task dispatch failed: HTTP 503",'
                    '"error_kind":"dispatch_error","diagnostic_status":"ok"}'
                ),
            }

        globals_ref["_run_capture"] = fake_run
        try:
            result = module.request_lifecycle_profile("mcp-reload")
        finally:
            globals_ref["_run_capture"] = original

        self.assertEqual("error", result["status"])
        self.assertEqual("dispatch_error", result["error_kind"])
        self.assertEqual("ok", result["diagnostic_status"])
        self.assertEqual("cynic mcp-reload --json", result["dispatch_json_command"])

    def test_request_lifecycle_profile_timeout_surfaces_dispatch_timeout_context(self):
        module = load_module()
        globals_ref = module.request_lifecycle_profile.__globals__
        original = globals_ref["_run_capture"]

        def fake_run(command, *, timeout=15, stdout_limit=1200, stderr_limit=1200):
            return {
                "status": "timeout",
                "exit_code": None,
                "stdout": "",
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            result = module.request_lifecycle_profile("mcp-reload")
        finally:
            globals_ref["_run_capture"] = original

        self.assertEqual("timeout", result["status"])
        self.assertEqual("dispatch_timeout", result["error_kind"])
        self.assertEqual(30, result["timeout_seconds"])
        self.assertIn("cynic organism --json", result["operator_action"])

    def test_lifecycle_profile_request_text_is_human_readable_and_points_to_json(self):
        module = load_module()
        body = module.lifecycle_profile_request_text(
            {
                "status": "ok",
                "profile": "mcp-reload",
                "command": "cynic mcp-reload",
                "mutates_mempool": True,
                "dispatch_summary": {
                    "requested": 1,
                    "dispatched": 1,
                    "skipped_existing": 0,
                    "task_ids": ["task-mcp"],
                },
                "blockers": [{"code": "fleet_inventory:degraded"}],
                "operator_sequence": [
                    {"step": "inspect", "command": "cynic mcp-reload --dry-run"},
                    {"step": "dispatch", "command": "cynic mcp-reload"},
                    {"step": "prove", "command": "cynic mcp-reload --json"},
                ],
                "proof_commands": ["cynic mcp-reload --json", "cynic actions", "cynic organism"],
                "done_when": ["organ-forge read-back proves the request"],
            }
        )

        self.assertIn("CYNIC lifecycle profile request", body)
        self.assertIn("Profile: mcp-reload", body)
        self.assertIn("Command: cynic mcp-reload", body)
        self.assertIn("Mutates mempool: True", body)
        self.assertIn("Requested: 1", body)
        self.assertIn("Dispatched: 1", body)
        self.assertIn("task-mcp", body)
        self.assertIn("fleet_inventory:degraded", body)
        self.assertIn("Operator sequence:", body)
        self.assertIn("inspect: cynic mcp-reload --dry-run", body)
        self.assertIn("Proof commands:", body)
        self.assertIn("cynic organism", body)
        self.assertIn("Done when:", body)
        self.assertIn("Machine dispatch output: cynic mcp-reload --json", body)

    def test_activation_dry_run_is_non_mutating(self):
        module = load_module()
        globals_ref = module.activation_dry_run.__globals__
        original = globals_ref["lifecycle_activation_report"]

        def fake_activation():
            return {
                "schema": "cynic.lifecycle.activation_summary.v1",
                "status": "blocked",
                "activation_request_summary": [{"action": "refresh_fleet_inventory"}],
                "blockers": [{"code": "fleet_inventory:degraded"}],
                "mutates_mempool": False,
            }

        globals_ref["lifecycle_activation_report"] = fake_activation
        try:
            result = module.activation_dry_run()
        finally:
            globals_ref["lifecycle_activation_report"] = original

        self.assertEqual("cynic activate --dry-run", result["command"])
        self.assertEqual("cynic activate", result["dispatch_command"])
        self.assertEqual("cynic activate --json", result["dispatch_json_command"])
        self.assertEqual("cynic activate --dry-run", result["inspect_command"])
        self.assertFalse(result["mutates_mempool"])
        self.assertIn("lifecycle-blockers", result["fallback_command"])
        self.assertEqual("inspect", result["operator_sequence"][0]["step"])
        self.assertEqual("cynic activate --dry-run", result["operator_sequence"][0]["command"])
        self.assertEqual("cynic activate", result["operator_sequence"][1]["command"])
        self.assertEqual("dispatch_json", result["operator_sequence"][2]["step"])
        self.assertEqual("cynic activate --json", result["operator_sequence"][2]["command"])
        self.assertTrue(result["operator_sequence"][2]["mutates_mempool"])
        self.assertEqual("cynic organism", result["operator_sequence"][3]["command"])
        self.assertEqual(["cynic actions", "cynic organism"], result["proof_commands"])
        self.assertIn("cynic actions and cynic organism show updated activation pressure", result["done_when"][2])

    def test_command_help_is_handled_before_dispatch(self):
        module = load_module()
        handle_help = module.main.__globals__["_handle_command_help"]

        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertTrue(handle_help("activate", ["--help"]))
        self.assertIn("usage: cynic activate", stream.getvalue())
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertTrue(handle_help("update", ["--help"]))
        self.assertIn("usage: cynic update", stream.getvalue())
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertTrue(handle_help("bootstrap", ["--help"]))
        self.assertIn("usage: cynic bootstrap", stream.getvalue())
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertTrue(handle_help("activate", ["-h"]))
        self.assertFalse(handle_help("activate", []))

    def test_infra_report_text_summarizes_checks_and_json_command(self):
        module = load_module()
        body = module.infra_report_text(
            {
                "status": "blocked",
                "command": "cynic storage status",
                "mutates_mempool": False,
                "target": {
                    "node": "cynic-lxc-surrealdb",
                    "service": "surrealdb",
                    "endpoint": "http://organ-surrealdb.synapse.cynic:8000",
                },
                "missing": ["bridge_active"],
                "checks": {"bridge_active": False, "registry_declares_storage_node": True},
                "fallback_command": "python3 organs/keep/src/entrypoints/cli.py storage-primary status",
            },
            title="CYNIC storage primary status",
            json_command="cynic storage status --json",
        )

        self.assertIn("CYNIC storage primary status", body)
        self.assertIn("Status: blocked", body)
        self.assertIn("Node: cynic-lxc-surrealdb", body)
        self.assertIn("Missing:", body)
        self.assertIn("bridge_active", body)
        self.assertIn("Failed checks:", body)
        self.assertIn("Next checks:", body)
        self.assertIn("cynic bridge status", body)
        self.assertNotIn("Fallback:", body)
        self.assertNotIn("python3 organs/keep/src/entrypoints/cli.py", body)
        self.assertIn("Machine output: cynic storage status --json", body)

    def test_infra_report_text_suggests_native_next_checks(self):
        module = load_module()

        bridge_body = module.infra_report_text(
            {
                "status": "missing_activation",
                "command": "cynic bridge status",
                "missing": ["helper_exists"],
                "checks": {"helper_exists": False},
            },
            title="CYNIC bridge status",
            json_command="cynic bridge status --json",
        )
        self.assertIn("cynic bridge plan", bridge_body)
        self.assertIn("cynic activate --dry-run", bridge_body)

        fleet_body = module.infra_report_text(
            {
                "command": "cynic fleet status",
                "anomalies": [{"kind": "active_without_runtime_proof", "target": "cynic-vm-v2"}],
            },
            title="CYNIC fleet status",
            json_command="cynic fleet status --json",
        )
        self.assertIn("cynic actions", fleet_body)

        mesh_body = module.infra_report_text(
            {
                "command": "cynic mesh status",
                "dns_status": "not_checked",
                "runtime_status": "not_observed",
            },
            title="CYNIC mesh status",
            json_command="cynic mesh status --json",
        )
        self.assertIn("cynic mesh status --resolve-dns --include-records", mesh_body)
        self.assertIn("cynic activate --dry-run", mesh_body)



class RuntimePrincipalsSurfaceTests(unittest.TestCase):
    def test_organism_text_shows_next_organ_actions_when_activation_requests_exist(self):
        module = load_module()
        report = module.organism_report(run_probes=False)
        report["lifecycle_activation"] = {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "blocked",
            "activation_request_summary": [
                {
                    "action": "refresh_fleet_inventory",
                    "node": None,
                    "code": None,
                    "coord_targets": ["organs/fleet"],
                    "consumer": "organ-forge-hermes-agent",
                }
            ],
            "activation_dispatch_command": "cynic activate",
        }

        body = module.organism_text(report)

        self.assertIn("Next organ actions:", body)
        self.assertIn("refresh_fleet_inventory: organs/fleet -> organ-forge-hermes-agent", body)
        self.assertIn("Dispatch: cynic activate", body)

    def test_lifecycle_activation_report_uses_framework_dispatch_command(self):
        module = load_module()
        report = module.organism_report(run_probes=False)
        report["lifecycle_activation"] = {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "blocked",
            "activation_request_summary": [
                {
                    "action": "activate_registry_node",
                    "node": "cynic-lxc-surrealdb",
                    "consumer": "organ-forge-hermes-agent",
                }
            ],
            "activation_dispatch_command": "cynic activate",
        }

        body = module.organism_text(report)

        self.assertIn("Activation pressure:", body)
        self.assertIn("Dispatch: cynic activate", body)

    def test_activation_dispatch_text_is_human_readable_and_points_to_json(self):
        module = load_module()
        body = module.activation_dispatch_text(
            {
                "status": "ok",
                "plan_status": "blocked",
                "dispatch_summary": {
                    "requested": 3,
                    "dispatched": 2,
                    "skipped_existing": 1,
                    "task_ids": ["task-1", "task-2"],
                },
                "skipped_existing": [
                    {
                        "task_id": "task-0",
                        "action": "refresh_fleet_inventory",
                        "reason": "pending_duplicate",
                    }
                ],
                "blockers": [{"code": "fleet_inventory:degraded"}],
                "inspect_command": "cynic activate --dry-run",
                "proof_commands": ["cynic actions", "cynic organism", "cynic activate --json"],
                "done_when": ["organ-forge consumes activation tasks"],
            }
        )

        self.assertIn("CYNIC lifecycle activation dispatch", body)
        self.assertIn("Requested: 3", body)
        self.assertIn("Dispatched: 2", body)
        self.assertIn("Skipped existing: 1", body)
        self.assertIn("task-1", body)
        self.assertIn("task-0: refresh_fleet_inventory (pending_duplicate)", body)
        self.assertIn("fleet_inventory:degraded", body)
        self.assertIn("Inspect before dispatch: cynic activate --dry-run", body)
        self.assertIn("Proof commands:", body)
        self.assertIn("cynic actions", body)
        self.assertIn("Done when:", body)
        self.assertIn("Machine dispatch output: cynic activate --json", body)

    def test_organism_actions_report_is_native_and_non_mutating(self):
        module = load_module()
        globals_ref = module.organism_actions_report.__globals__
        original = globals_ref["lifecycle_activation_report"]

        def fake_activation():
            return {
                "schema": "cynic.lifecycle.activation_summary.v1",
                "status": "blocked",
                "activation_request_summary": [
                    {
                        "action": "refresh_fleet_inventory",
                        "node": None,
                        "code": None,
                        "priority": 97,
                        "coord_targets": ["organs/fleet"],
                        "consumer": "organ-forge-hermes-agent",
                    }
                ],
                "blockers": [
                    {
                        "code": "fleet_inventory:degraded",
                        "kind": "fleet_inventory",
                        "actual": "degraded",
                        "required": "ok",
                        "resolver_organ": "organ-fleet",
                        "resolver_consumer": "organ-forge-hermes-agent",
                        "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
                    },
                    {
                        "category": "fleet",
                        "code": "active_without_runtime_proof:cynic-vm-v2",
                        "kind": "fleet_inventory_evidence_gap",
                        "provider": "fleet_inventory",
                    }
                ],
                "activation_dispatch_command": "cynic activate",
            }

        globals_ref["lifecycle_activation_report"] = fake_activation
        try:
            report = module.organism_actions_report()
            body = module.organism_actions_text(report)
        finally:
            globals_ref["lifecycle_activation_report"] = original

        self.assertEqual("cynic.organism.actions.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertEqual(1, report["action_count"])
        self.assertEqual("organs/fleet", report["actions"][0]["target"])
        self.assertEqual("degraded", report["blockers"][0]["actual"])
        self.assertEqual("organ-fleet", report["blockers"][0]["resolver_organ"])
        self.assertEqual("cynic fleet status", report["blockers"][0]["native_check_command"])
        self.assertEqual("cynic-vm-v2", report["blockers"][1]["node"])
        self.assertEqual("organ-fleet", report["blockers"][1]["resolver_organ"])
        self.assertEqual("organ-forge-hermes-agent", report["blockers"][1]["resolver_consumer"])
        self.assertEqual("cynic fleet status", report["blockers"][1]["native_check_command"])
        self.assertEqual("cynic activate", report["dispatch_command"])
        self.assertTrue(report["dispatch_mutates_mempool"])
        self.assertEqual(1, report["dispatch_plan"]["request_count"])
        self.assertEqual(["organ-forge-hermes-agent"], report["dispatch_plan"]["consumers"])
        self.assertEqual(["organs/fleet"], report["dispatch_plan"]["coord_targets"])
        self.assertEqual("cynic activate --dry-run", report["dispatch_plan"]["operator_sequence"][0]["command"])
        self.assertEqual("cynic activate", report["dispatch_plan"]["operator_sequence"][1]["command"])
        self.assertIn("Kernel mempool", report["dispatch_plan"]["read_back"])
        self.assertIn("CYNIC organism actions", body)
        self.assertIn("refresh_fleet_inventory: organs/fleet p=97 -> organ-forge-hermes-agent", body)
        self.assertIn("fleet_inventory:degraded (required=ok, actual=degraded", body)
        self.assertIn("active_without_runtime_proof:cynic-vm-v2", body)
        self.assertIn("check: cynic fleet status", body)
        self.assertIn("Dispatch plan:", body)
        self.assertIn("requests: 1", body)
        self.assertIn("consumers: organ-forge-hermes-agent", body)
        self.assertIn("dispatch: cynic activate", body)
        self.assertIn("read-back:", body)
        self.assertNotIn("python3 organs/fleet/src/entrypoints/cli.py", body)

    def test_request_lifecycle_profile_wraps_delegate_profile(self):
        module = load_module()
        globals_ref = module.request_lifecycle_profile.__globals__
        original = globals_ref["_run_capture"]

        def fake_run(command, *, timeout=15, stdout_limit=1200, stderr_limit=1200):
            return {
                "status": "ok",
                "exit_code": 0,
                "stdout": '{"status":"ok","profile":"gate-2","task_ids":["task-1"]}',
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            result = module.request_lifecycle_profile("gate-2")
        finally:
            globals_ref["_run_capture"] = original

        self.assertEqual("ok", result["status"])
        self.assertEqual("cynic gate-2", result["command"])
        self.assertEqual("cynic gate-2 --dry-run", result["inspect_command"])
        self.assertEqual("cynic gate-2 --json", result["dispatch_json_command"])
        self.assertEqual("inspect", result["operator_sequence"][0]["step"])
        self.assertEqual("cynic gate-2 --dry-run", result["operator_sequence"][0]["command"])
        self.assertIn("cynic organism", result["proof_commands"])
        self.assertEqual(
            "python3 organs/forge/src/entrypoints/cli.py delegate-profile gate-2 --json",
            result["fallback_command"],
        )
        self.assertTrue(result["mutates_mempool"])

    def test_lifecycle_profile_request_text_is_human_readable_and_points_to_json(self):
        module = load_module()
        body = module.lifecycle_profile_request_text(
            {
                "status": "ok",
                "profile": "mcp-reload",
                "command": "cynic mcp-reload",
                "mutates_mempool": True,
                "dispatch_summary": {
                    "requested": 1,
                    "dispatched": 1,
                    "skipped_existing": 0,
                    "task_ids": ["task-mcp"],
                },
                "blockers": [{"code": "fleet_inventory:degraded"}],
                "operator_sequence": [
                    {"step": "inspect", "command": "cynic mcp-reload --dry-run"},
                    {"step": "dispatch", "command": "cynic mcp-reload"},
                    {"step": "prove", "command": "cynic mcp-reload --json"},
                ],
                "proof_commands": ["cynic mcp-reload --json", "cynic actions", "cynic organism"],
                "done_when": ["organ-forge read-back proves the request"],
            }
        )

        self.assertIn("CYNIC lifecycle profile request", body)
        self.assertIn("Profile: mcp-reload", body)
        self.assertIn("Command: cynic mcp-reload", body)
        self.assertIn("Mutates mempool: True", body)
        self.assertIn("Requested: 1", body)
        self.assertIn("Dispatched: 1", body)
        self.assertIn("task-mcp", body)
        self.assertIn("fleet_inventory:degraded", body)
        self.assertIn("Operator sequence:", body)
        self.assertIn("inspect: cynic mcp-reload --dry-run", body)
        self.assertIn("Proof commands:", body)
        self.assertIn("cynic organism", body)
        self.assertIn("Done when:", body)
        self.assertIn("Machine dispatch output: cynic mcp-reload --json", body)

    def test_activation_dry_run_is_non_mutating(self):
        module = load_module()
        globals_ref = module.activation_dry_run.__globals__
        original = globals_ref["lifecycle_activation_report"]

        def fake_activation():
            return {
                "schema": "cynic.lifecycle.activation_summary.v1",
                "status": "blocked",
                "activation_request_summary": [{"action": "refresh_fleet_inventory"}],
                "blockers": [{"code": "fleet_inventory:degraded"}],
                "mutates_mempool": False,
            }

        globals_ref["lifecycle_activation_report"] = fake_activation
        try:
            result = module.activation_dry_run()
        finally:
            globals_ref["lifecycle_activation_report"] = original

        self.assertEqual("cynic activate --dry-run", result["command"])
        self.assertEqual("cynic activate", result["dispatch_command"])
        self.assertEqual("cynic activate --json", result["dispatch_json_command"])
        self.assertEqual("cynic activate --dry-run", result["inspect_command"])
        self.assertFalse(result["mutates_mempool"])
        self.assertIn("lifecycle-blockers", result["fallback_command"])
        self.assertEqual("inspect", result["operator_sequence"][0]["step"])
        self.assertEqual("cynic activate --dry-run", result["operator_sequence"][0]["command"])
        self.assertEqual("cynic activate", result["operator_sequence"][1]["command"])
        self.assertEqual("dispatch_json", result["operator_sequence"][2]["step"])
        self.assertEqual("cynic activate --json", result["operator_sequence"][2]["command"])
        self.assertTrue(result["operator_sequence"][2]["mutates_mempool"])
        self.assertEqual("cynic organism", result["operator_sequence"][3]["command"])
        self.assertEqual(["cynic actions", "cynic organism"], result["proof_commands"])
        self.assertIn("cynic actions and cynic organism show updated activation pressure", result["done_when"][2])

    def test_command_help_is_handled_before_dispatch(self):
        module = load_module()
        handle_help = module.main.__globals__["_handle_command_help"]

        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertTrue(handle_help("activate", ["--help"]))
        self.assertIn("usage: cynic activate", stream.getvalue())
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertTrue(handle_help("update", ["--help"]))
        self.assertIn("usage: cynic update", stream.getvalue())
        with contextlib.redirect_stdout(io.StringIO()) as stream:
            self.assertTrue(handle_help("bootstrap", ["--help"]))
        self.assertIn("usage: cynic bootstrap", stream.getvalue())
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertTrue(handle_help("activate", ["-h"]))
        self.assertFalse(handle_help("activate", []))

    def test_infra_report_text_summarizes_checks_and_json_command(self):
        module = load_module()
        body = module.infra_report_text(
            {
                "status": "blocked",
                "command": "cynic storage status",
                "mutates_mempool": False,
                "target": {
                    "node": "cynic-lxc-surrealdb",
                    "service": "surrealdb",
                    "endpoint": "http://organ-surrealdb.synapse.cynic:8000",
                },
                "missing": ["bridge_active"],
                "checks": {"bridge_active": False, "registry_declares_storage_node": True},
                "fallback_command": "python3 organs/keep/src/entrypoints/cli.py storage-primary status",
            },
            title="CYNIC storage primary status",
            json_command="cynic storage status --json",
        )

        self.assertIn("CYNIC storage primary status", body)
        self.assertIn("Status: blocked", body)
        self.assertIn("Node: cynic-lxc-surrealdb", body)
        self.assertIn("Missing:", body)
        self.assertIn("bridge_active", body)
        self.assertIn("Failed checks:", body)
        self.assertIn("Next checks:", body)
        self.assertIn("cynic bridge status", body)
        self.assertNotIn("Fallback:", body)
        self.assertNotIn("python3 organs/keep/src/entrypoints/cli.py", body)
        self.assertIn("Machine output: cynic storage status --json", body)

    def test_print_launchers_uses_codex_launcher_name(self):
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "print-launchers"],
            check=False,
            cwd=REPO_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        self.assertEqual(0, result.returncode)
        self.assertEqual("", result.stderr)
        self.assertNotIn("codic", result.stdout)
        self.assertIn("codex-yolo", result.stdout)
        self.assertIn("antigravity-yolo", result.stdout)
        self.assertIn("hermes-yolo", result.stdout)

    def test_command_manifest_exposes_declared_command_metadata(self):
        module = load_module()
        manifest = module.command_manifest()

        self.assertEqual("cynic.runtime_principals.command_manifest.v1", manifest["schema"])
        self.assertGreaterEqual(manifest["count"], 20)
        commands = {entry["name"]: entry for entry in manifest["commands"]}
        self.assertIn("status", commands)
        self.assertIn("repos", commands)
        self.assertIn("print-launchers", commands)
        self.assertFalse(commands["status"]["mutates"])
        self.assertTrue(commands["status"]["json_output"])
        self.assertTrue(commands["repos"]["mutates"])
        self.assertTrue(commands["update"]["json_output"])
        self.assertTrue(commands["bootstrap"]["mutates"])

if __name__ == "__main__":
    unittest.main()
