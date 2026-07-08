import json
import subprocess
import sys
import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path, resolve_project_root
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path, resolve_project_root


MODULE_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "runtime_principals.py"
REPO_ROOT = resolve_project_root(MODULE_PATH)


def load_module():
    return load_module_from_path("runtime_principals", MODULE_PATH)


class RuntimePrincipalsReportTests(unittest.TestCase):
    def test_mcp_freshness_points_operator_to_native_reload_command(self):
        module = load_module()
        wrapper = module.CANONICAL_PROJECT_ROOT / ".cortex/mcp/run_cynic_mcp.py"
        body = wrapper.read_text(encoding="utf-8")

        self.assertIn("with: cynic mcp-reload", body)
        self.assertIn("Legacy fallback", body)
        self.assertIn("make mcp-reload-request", body)

        globals_ref = module.mcp_freshness_report.__globals__
        original_run = globals_ref["subprocess"].run

        class Completed:
            returncode = 78
            stdout = "MCP binary: /tmp/cynic-kernel-mcp\n"
            stderr = "stale\n"

        def fake_run(*args, **kwargs):
            return Completed()

        globals_ref["subprocess"].run = fake_run
        try:
            report = module.mcp_freshness_report()
        finally:
            globals_ref["subprocess"].run = original_run

        self.assertEqual("cynic mcp-reload", report["operator_fallback"])
        self.assertEqual("make mcp-reload-request", report["legacy_operator_fallback"])
        requirements = {item["check"]: item for item in report["proof_requirements"]}
        self.assertEqual("satisfied", requirements["mcp_binary_selected"]["status"])
        self.assertEqual("/tmp/cynic-kernel-mcp", requirements["mcp_binary_selected"]["observed"])
        self.assertEqual("missing", requirements["mcp_binary_fresh"]["status"])
        self.assertEqual("cynic mcp-reload", requirements["mcp_binary_fresh"]["command"])
        self.assertEqual("cynic mcp-reload --json", requirements["mcp_reload_read_back"]["command"])

    def test_organism_report_is_machine_readable_and_non_mutating(self):
        module = load_module()
        report = module.organism_report(run_probes=False)

        self.assertEqual("cynic.organism.signs_of_life.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertIn("runtime", report)
        self.assertIn("local_validation", report)
        self.assertIn("kernel", report)
        self.assertIn("storage_primary", report)
        self.assertIn("mcp_freshness", report)
        self.assertIn("lifecycle", report)
        self.assertIn("lifecycle_activation", report)
        self.assertIn("actions", report)
        self.assertIn("mcp_reload", report["lifecycle"])
        self.assertEqual("cynic.lifecycle.activation_summary.v1", report["lifecycle_activation"]["schema"])
        self.assertEqual("cynic.organism.actions.v1", report["actions"]["schema"])
        self.assertEqual("cynic.local_validation.status.v1", report["local_validation"]["schema"])

    def test_kernel_health_report_distinguishes_authenticated_path_stall(self):
        module = load_module()
        globals_ref = module.kernel_health_report.__globals__
        original_urlopen = globals_ref["urllib"].request.urlopen
        original_diag = globals_ref["diagnose_kernel_http_failure"]
        original_env = globals_ref["os"].environ.get("CYNIC_REST_ADDR")

        class Timeouting:
            def __call__(self, *args, **kwargs):
                raise TimeoutError("timed out")

        globals_ref["urllib"].request.urlopen = Timeouting()
        globals_ref["diagnose_kernel_http_failure"] = lambda **kwargs: {
            "endpoint": "http://10.x.x.x:3030",
            "path": "/health",
            "tcp_reachable": True,
            "tcp_error": None,
            "unauthenticated_http_status": 401,
            "unauthenticated_http_error": None,
            "failure_class": "authenticated_path_stalled",
        }
        globals_ref["os"].environ["CYNIC_REST_ADDR"] = "http://10.10.10.140:3030"
        try:
            report = module.kernel_health_report()
        finally:
            globals_ref["urllib"].request.urlopen = original_urlopen
            globals_ref["diagnose_kernel_http_failure"] = original_diag
            if original_env is None:
                globals_ref["os"].environ.pop("CYNIC_REST_ADDR", None)
            else:
                globals_ref["os"].environ["CYNIC_REST_ADDR"] = original_env

        self.assertEqual("unreachable", report["status"])
        self.assertEqual("authenticated_path_stalled", report["diagnosis"]["failure_class"])
        self.assertEqual(401, report["diagnosis"]["unauthenticated_http_status"])

    def test_status_exposes_native_lifecycle_commands(self):
        module = load_module()
        report = module.status()

        self.assertIn("requires_root_evidence", report)
        self.assertEqual("sudo cynic status --json", report["root_evidence_command"])
        self.assertIn(report["cynic_cli_projection_ok"], {True, False, None})
        self.assertEqual("cynic bootstrap", report["cynic_cli_projection_command"])
        requirements = {item["check"]: item for item in report["proof_requirements"]}
        self.assertIn("root_evidence_context", requirements)
        self.assertIn("operator_cli_projection_fresh", requirements)
        self.assertIn("codex_bounded_yolo_projection_fresh", requirements)
        self.assertIn("agy_bounded_yolo_projection_fresh", requirements)
        self.assertIn("hermes_bounded_yolo_projection_fresh", requirements)
        self.assertIn("codex_yolo_flags_present", requirements)
        self.assertIn("agy_yolo_flags_present", requirements)
        self.assertIn("hermes_yolo_flags_present", requirements)
        self.assertEqual("cynic bootstrap", requirements["operator_cli_projection_fresh"]["command"])
        self.assertEqual("cynic bootstrap", requirements["codex_bounded_yolo_projection_fresh"]["command"])
        self.assertEqual("cynic bootstrap", requirements["agy_bounded_yolo_projection_fresh"]["command"])
        self.assertEqual("cynic bootstrap", requirements["hermes_bounded_yolo_projection_fresh"]["command"])
        self.assertIn(report["codex_launcher_projection_ok"], {True, False, None})
        self.assertIn(report["antigravity_launcher_projection_ok"], {True, False, None})
        self.assertIn(report["hermes_launcher_projection_ok"], {True, False, None})
        self.assertIn(report["codex_yolo_flags_present"], {True, False, None})
        self.assertIn(report["agy_yolo_flags_present"], {True, False, None})
        self.assertIn(report["hermes_yolo_flags_present"], {True, False, None})
        self.assertEqual("bounded_yolo", report["execution_posture"]["name"])
        self.assertEqual("cynic", report["execution_posture"]["runtime_user"])
        self.assertEqual("canonical_checkout", report["execution_posture"]["scope"])
        self.assertEqual("passworded_or_mcp_helper_only", report["execution_posture"]["destructive_authority"])
        self.assertEqual("cynic codex", report["execution_posture"]["agents"]["codex"]["command"])
        self.assertEqual("cynic agy", report["execution_posture"]["agents"]["agy"]["command"])
        self.assertEqual("cynic hermes", report["execution_posture"]["agents"]["hermes"]["command"])
        self.assertEqual(["--dangerously-bypass-approvals-and-sandbox"], report["execution_posture"]["agents"]["codex"]["yolo_flags"])
        self.assertEqual(["--dangerously-skip-permissions"], report["execution_posture"]["agents"]["agy"]["yolo_flags"])
        self.assertEqual([], report["execution_posture"]["agents"]["hermes"]["yolo_flags"])
        self.assertIn(requirements["root_evidence_context"]["status"], {"satisfied", "missing"})
        self.assertIn(report["runtime_evidence_status"], {"ready", "not_ready", "requires_root_evidence"})
        if report["requires_root_evidence"]:
            self.assertEqual("requires_root_evidence", report["runtime_evidence_status"])
            self.assertIsNone(report["home_owner_ok"])
        self.assertEqual("cynic status", report["runtime_commands"]["status"])
        self.assertEqual("cynic status --json", report["runtime_commands"]["status_json"])
        self.assertEqual("cynic doctor", report["runtime_commands"]["doctor"])
        self.assertEqual("cynic plan", report["runtime_commands"]["plan"])
        self.assertEqual("cynic shell", report["runtime_commands"]["shell"])
        self.assertEqual("cynic actions", report["lifecycle_commands"]["actions"]["inspect"])
        self.assertEqual("cynic actions --json", report["lifecycle_commands"]["actions"]["inspect_json"])
        self.assertEqual("cynic activate", report["lifecycle_commands"]["activation"]["dispatch"])
        self.assertEqual("cynic activate --json", report["lifecycle_commands"]["activation"]["dispatch_json"])
        self.assertEqual("cynic mcp-reload --dry-run", report["lifecycle_commands"]["mcp_reload"]["inspect"])
        self.assertEqual("cynic mcp-reload", report["lifecycle_commands"]["mcp_reload"]["dispatch"])
        self.assertEqual("cynic mcp-reload --json", report["lifecycle_commands"]["mcp_reload"]["dispatch_json"])
        self.assertEqual("cynic gate-2 --dry-run", report["lifecycle_commands"]["gate_2"]["inspect"])
        self.assertEqual("cynic gate-2", report["lifecycle_commands"]["gate_2"]["dispatch"])
        self.assertEqual("cynic gate-2 --json", report["lifecycle_commands"]["gate_2"]["dispatch_json"])
        self.assertEqual("cynic deploy --dry-run", report["lifecycle_commands"]["deploy"]["inspect"])
        self.assertEqual("cynic deploy", report["lifecycle_commands"]["deploy"]["dispatch"])
        self.assertEqual("cynic deploy --json", report["lifecycle_commands"]["deploy"]["dispatch_json"])
        self.assertEqual("cynic storage plan", report["storage_commands"]["plan"])
        self.assertEqual("cynic storage bundle --output-dir <dir>", report["storage_commands"]["bundle"])
        self.assertEqual("cynic storage status", report["storage_commands"]["status"])
        self.assertEqual("cynic storage status --json", report["storage_commands"]["status_json"])
        self.assertEqual("cynic bridge plan", report["bridge_commands"]["plan"])
        self.assertEqual("cynic bridge status", report["bridge_commands"]["status"])
        self.assertEqual("cynic bridge status --json", report["bridge_commands"]["status_json"])
        self.assertEqual("cynic fleet status", report["fleet_commands"]["status"])
        self.assertEqual("cynic fleet status --json", report["fleet_commands"]["status_json"])
        self.assertEqual("cynic mesh status", report["mesh_commands"]["status"])
        self.assertEqual("cynic mesh status --json", report["mesh_commands"]["status_json"])
        self.assertEqual(
            "cynic mesh status --resolve-dns --include-records",
            report["mesh_commands"]["status_with_dns"],
        )

    def test_stale_installed_cynic_cli_projection_blocks_runtime_ready(self):
        module = load_module()
        globals_ref = module.status.__globals__
        original_cli = globals_ref["CYNIC_CLI"]
        original_geteuid = globals_ref["os"].geteuid
        original_owner = globals_ref["_owner"]
        original_exists_user = globals_ref["_exists_user"]
        original_exists_group = globals_ref["_exists_group"]
        original_is_dir = globals_ref["_is_dir"]
        original_can_execute = globals_ref["_can_execute"]
        original_user_can_access_dir = globals_ref["_user_can_access_dir"]
        original_legacy_violation = globals_ref["_legacy_pvesh_sudoers_present"]

        temp_cli = REPO_ROOT / "tmp" / "stale-cynic-cli-test"
        temp_cli.parent.mkdir(exist_ok=True)
        temp_cli.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
        temp_cli.chmod(0o755)
        try:
            globals_ref["CYNIC_CLI"] = temp_cli
            globals_ref["os"].geteuid = lambda: 0
            globals_ref["_owner"] = lambda path: (globals_ref["USER_NAME"], globals_ref["GROUP_NAME"])
            globals_ref["_exists_user"] = lambda name: True
            globals_ref["_exists_group"] = lambda name: True
            globals_ref["_is_dir"] = lambda path: True
            globals_ref["_can_execute"] = lambda path: True
            globals_ref["_user_can_access_dir"] = lambda user, path: True
            globals_ref["_legacy_pvesh_sudoers_present"] = lambda: False

            report = module.status()
        finally:
            globals_ref["CYNIC_CLI"] = original_cli
            globals_ref["os"].geteuid = original_geteuid
            globals_ref["_owner"] = original_owner
            globals_ref["_exists_user"] = original_exists_user
            globals_ref["_exists_group"] = original_exists_group
            globals_ref["_is_dir"] = original_is_dir
            globals_ref["_can_execute"] = original_can_execute
            globals_ref["_user_can_access_dir"] = original_user_can_access_dir
            globals_ref["_legacy_pvesh_sudoers_present"] = original_legacy_violation
            temp_cli.unlink(missing_ok=True)

        self.assertFalse(report["cynic_cli_projection_ok"])
        self.assertFalse(report["runtime_ready"])
        self.assertEqual("not_ready", report["runtime_evidence_status"])
        requirements = {item["check"]: item for item in report["proof_requirements"]}
        self.assertEqual("missing", requirements["operator_cli_projection_fresh"]["status"])
        self.assertEqual("satisfied", requirements["root_evidence_context"]["status"])

    def test_doctor_text_is_human_readable(self):
        module = load_module()
        body = module.doctor_text()

        self.assertIn("CYNIC runtime:", body)
        self.assertIn("Checkout:", body)
        self.assertIn("Execution posture: bounded_yolo", body)
        self.assertIn("Posture native default:", body)
        self.assertIn("Launch:", body)
        self.assertIn("Update:", body)
        self.assertIn("Operator CLI projection:", body)
        self.assertIn("Codex launcher projection:", body)
        self.assertIn("Antigravity launcher projection:", body)
        if module.status()["cynic_cli_projection_ok"] is False:
            self.assertIn("Refresh projection: cynic bootstrap", body)
        if module.status()["requires_root_evidence"]:
            self.assertIn("Proof: sudo cynic status --json", body)
        self.assertIn("inspect primary CLIs: cynic update --dry-run", body)
        self.assertIn("primary CLIs: cynic update", body)
        self.assertIn("Organism:", body)
        self.assertIn("signs of life: cynic organism", body)
        self.assertIn("next actions: cynic actions", body)
        self.assertIn("inspect lifecycle activation: cynic activate --dry-run", body)
        self.assertIn("lifecycle activation: cynic activate", body)
        self.assertIn("inspect mcp reload: cynic mcp-reload --dry-run", body)
        self.assertIn("request mcp reload: cynic mcp-reload", body)
        self.assertIn("inspect gate 2: cynic gate-2 --dry-run", body)
        self.assertIn("request gate 2: cynic gate-2", body)
        self.assertIn("inspect deploy: cynic deploy --dry-run", body)
        self.assertIn("request deploy: cynic deploy", body)
        self.assertIn("Storage:", body)
        self.assertIn("plan: cynic storage plan", body)
        self.assertIn("bundle: cynic storage bundle --output-dir <dir>", body)
        self.assertIn("status: cynic storage status", body)
        self.assertIn("Bridge:", body)
        self.assertIn("plan: cynic bridge plan", body)
        self.assertIn("status: cynic bridge status", body)
        self.assertIn("Fleet:", body)
        self.assertIn("status: cynic fleet status", body)
        self.assertIn("Mesh:", body)
        self.assertIn("status: cynic mesh status", body)
        self.assertIn("dns: cynic mesh status --resolve-dns --include-records", body)
        self.assertIn("codex: cynic codex", body)
        self.assertIn("agy: cynic agy", body)
        self.assertIn("hermes: cynic hermes", body)
        self.assertIn("shell: cynic shell", body)

    def test_primary_update_commands_are_native_cli_updates(self):
        module = load_module()

        self.assertEqual(["codex", "update"], module.PRIMARY_UPDATE_COMMANDS["codex"])
        self.assertEqual(["agy", "update"], module.PRIMARY_UPDATE_COMMANDS["agy"])
        self.assertEqual(["make", "hermes-update"], module.PRIMARY_UPDATE_COMMANDS["hermes"])


if __name__ == "__main__":
    unittest.main()
