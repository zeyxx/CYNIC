import contextlib
import io
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

from organs.contracts.src.project_root import resolve_repo_root

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "runtime_principals.py"
RUNTIME_ACTIONS_MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "runtime_principals_actions.py"
REPO_ACCESS_MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "runtime_principals_repo_access.py"
LIFECYCLE_ACTIONS_MODULE_PATH = (
    Path(__file__).resolve().parent.parent / "src" / "controllers" / "actions" / "lifecycle_activation.py"
)
REPO_ROOT = resolve_repo_root(MODULE_PATH.resolve())


def load_module():
    return load_module_from_path("runtime_principals", MODULE_PATH)


def load_runtime_actions_module():
    return load_module_from_path("runtime_principals_actions", RUNTIME_ACTIONS_MODULE_PATH)


def load_repo_access_module():
    return load_module_from_path("runtime_principals_repo_access", REPO_ACCESS_MODULE_PATH)


def load_lifecycle_actions_module():
    return load_module_from_path("runtime_principals_lifecycle_activation", LIFECYCLE_ACTIONS_MODULE_PATH)


class RuntimePrincipalsTests(unittest.TestCase):
    def test_runtime_principals_wrapper_bootstraps_project_root(self):
        result = subprocess.run(
            [sys.executable, str(MODULE_PATH), "--help"],
            check=False,
            cwd=REPO_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        self.assertEqual("", result.stderr)
        self.assertEqual(0, result.returncode)
        self.assertIn("CYNIC runtime principal activation.", result.stdout)

    def test_plan_is_machine_readable_and_non_mutating(self):
        module = load_module()
        plan = module.plan()

        self.assertEqual("cynic.runtime_principals.plan.v1", plan["schema"])
        self.assertFalse(plan["mutates"])
        self.assertIn(
            f"grant cynic traverse access to every parent of {module.CANONICAL_PROJECT_ROOT}",
            plan["actions"],
        )
        self.assertIn("grant raw pvesh, pct, shell, disk, or network NOPASSWD permissions", plan["not_done_by_apply"])

    def test_launcher_body_uses_runtime_repo_and_clear_errors(self):
        module = load_module()
        body = module.launcher_body("codex")

        self.assertIn("CYNIC_PROJECT_ROOT", body)
        self.assertNotIn("/" + "home/user", body)
        self.assertIn("required CLI not found in cynic PATH: codex", body)
        self.assertIn("GSETTINGS_BACKEND", body)

    def test_profile_routes_interactive_commands_through_cynic_framework(self):
        module = load_module()
        body = module.profile_body()

        self.assertIn("alias codex='cynic codex'", body)
        self.assertIn("alias agy='cynic agy'", body)
        self.assertIn("alias hermes='cynic hermes'", body)
        self.assertIn("alias antigravity='cynic agy'", body)

    def test_repo_access_profile_load_deduplicates_repository_entries(self):
        module = load_repo_access_module()
        temp_dir = REPO_ROOT / "tmp" / "runtime-principals-repo-access"
        temp_dir.mkdir(parents=True, exist_ok=True)
        profile_path = temp_dir / "repo-access-profile.json"
        repo_root = REPO_ROOT
        duplicate = temp_dir / "duplicate-repo"
        duplicate.mkdir(parents=True, exist_ok=True)

        profile_path.write_text(
            json.dumps(
                {
                    "schema": module.REPO_ACCESS_PROFILE_SCHEMA,
                    "repositories": [
                        {
                            "path": str(repo_root),
                            "owner": "first",
                            "added_by": "manual",
                            "discovered": False,
                            "added_at": "2026-01-01T00:00:00+00:00",
                            "source": "manual",
                        },
                        {
                            "path": str(duplicate),
                            "owner": "second",
                            "added_by": "manual",
                            "discovered": False,
                            "added_at": "2026-01-01T00:00:01+00:00",
                            "source": "manual",
                        },
                        {
                            "path": str(repo_root),
                            "owner": "third",
                            "added_by": "auto",
                            "discovered": True,
                            "added_at": "2026-01-01T00:00:02+00:00",
                            "source": "auto",
                        },
                    ],
                },
                indent=2,
            ),
            encoding="utf-8",
        )

        entries = module.RepoAccessProfile(profile_path).load()
        self.assertEqual(2, len(entries))
        self.assertEqual(str(repo_root.resolve()), entries[0].path)
        self.assertEqual(str(duplicate.resolve()), entries[1].path)

    def test_repo_access_manager_add_update_and_remove_without_grant(self):
        module = load_repo_access_module()
        temp_dir = REPO_ROOT / "tmp" / "runtime-principals-repo-access"
        temp_dir.mkdir(parents=True, exist_ok=True)
        profile_path = temp_dir / "manager-repo-access-profile.json"
        tracked = temp_dir / "tracked"
        tracked.mkdir(parents=True, exist_ok=True)
        manager = module.RepoAccessManager(
            dedicated_user="cynic",
            dedicated_group="cynic",
            profile=profile_path,
            operator_user=None,
            canonical_project_root=None,
        )

        first = manager.ensure_repos([tracked], added_by="unit-test", source="manual", discovered=True, grant=False)
        self.assertEqual("ok", first["status"])
        self.assertEqual(1, first["count"])
        self.assertEqual("added", first["entries"][0]["status"])

        second = manager.ensure_repos([tracked], added_by="unit-test", source="manual", discovered=False, grant=False)
        self.assertEqual("ok", second["status"])
        self.assertEqual(1, second["count"])
        self.assertEqual("present", second["entries"][0]["status"])

        removed = manager.remove_path(tracked)
        self.assertEqual("removed", removed["status"])
        self.assertTrue(removed["mutates"])

        missing = manager.remove_path(tracked)
        self.assertEqual("missing", missing["status"])
        self.assertFalse(missing["mutates"])

    def test_cynic_cli_exposes_clear_cortex_commands(self):
        module = load_module()
        body = module.cynic_cli_body()

        self.assertIn("usage: cynic <command>", body)
        self.assertIn('PROJECT_ROOT="$(resolve_project_root)"', body)
        self.assertIn("set -a", body)
        self.assertIn('source "${HOME}/.cynic-env"', body)
        self.assertIn("set +a", body)
        self.assertIn('CYNIC_HOME="$(getent passwd cynic | cut -d: -f6)"', body)
        self.assertIn('env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/codex-yolo"', body)
        self.assertIn('env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/antigravity-yolo"', body)
        self.assertIn('env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1 "$CYNIC_HOME/bin/hermes-yolo"', body)
        self.assertIn("runtime_principal_python()", body)
        self.assertIn('exec sudo -u cynic -H env', body)
        self.assertIn('HOME="$CYNIC_HOME" USER="cynic" LOGNAME="cynic"', body)
        self.assertIn('CYNIC_PROJECT_ROOT="$PROJECT_ROOT" GSETTINGS_BACKEND=memory NO_AT_BRIDGE=1', body)
        self.assertIn('update)', body)
        self.assertIn('organism)', body)
        self.assertIn('actions)', body)
        self.assertIn('activate)', body)
        self.assertIn('mcp-reload)', body)
        self.assertIn('gate-2)', body)
        self.assertIn('deploy)', body)
        self.assertIn('storage)', body)
        self.assertIn('bridge)', body)
        self.assertIn('fleet)', body)
        self.assertIn('mesh)', body)
        self.assertIn('python3 "$ENTRYPOINT" update', body)
        self.assertIn('runtime_principal_python organism "$@"', body)
        self.assertIn('runtime_principal_python actions "$@"', body)
        self.assertIn('runtime_principal_python activate "$@"', body)
        self.assertIn('runtime_principal_python bootstrap-request "$@"', body)
        self.assertIn('runtime_principal_python mcp-reload "$@"', body)
        self.assertIn('runtime_principal_python gate-2 "$@"', body)
        self.assertIn('runtime_principal_python deploy "$@"', body)
        self.assertIn('python3 "$ENTRYPOINT" storage', body)
        self.assertIn('python3 "$ENTRYPOINT" bridge', body)
        self.assertIn('python3 "$ENTRYPOINT" fleet', body)
        self.assertIn('python3 "$ENTRYPOINT" mesh', body)
        self.assertIn('bootstrap)', body)
        self.assertIn('python3 "$ENTRYPOINT" bootstrap', body)
        self.assertIn('exec sudo -iu cynic env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" bash -lc', body)
        self.assertIn("repos         Manage dedicated cynic repository access metadata. Usage: cynic repos <status|add|remove|sync>.", body)
        self.assertIn("register_repo_for_cynic()", body)
        self.assertIn("register_repo_args_for_cynic()", body)
        self.assertIn("python3 \"$ENTRYPOINT\" repos add --no-grant --discovered --source=auto \"$repo_path\"", body)
        self.assertIn("register_repo_args_for_cynic \"$@\"", body)
        self.assertIn("update        Update primary cortex CLI dependencies and refresh runtime bridges. Use --dry-run", body)
        self.assertIn("activate      Dispatch registry-derived lifecycle activation requests through organ-forge. Use --dry-run", body)
        self.assertIn("Use status --json for machine output", body)
        self.assertIn("Use --json for machine output", body)
        self.assertIn("canonical command is 'cynic agy'", body)
        self.assertIn('if [ "${1:-}" = "--json" ]; then', body)
        self.assertIn('exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" status', body)
        self.assertIn('exec sudo env CYNIC_PROJECT_ROOT="$PROJECT_ROOT" python3 "$ENTRYPOINT" doctor', body)
        self.assertIn("status)", body)
        self.assertIn("doctor)", body)
        self.assertIn("bootstrap     Refresh the full runtime-principal projection (update, apply, install-defaults).", body)
        self.assertNotIn("/" + "home/user", body)

    def test_yolo_flags_are_applied_by_launchers(self):
        module = load_module()

        self.assertIn("set -a", module.launcher_body("codex"))
        self.assertIn("set +a", module.launcher_body("codex"))
        self.assertIn("--dangerously-bypass-approvals-and-sandbox", module.launcher_body("codex"))
        self.assertIn("--dangerously-skip-permissions", module.launcher_body("agy"))
        self.assertIn("exec hermes \"$@\"", module.launcher_body("hermes"))

    def test_primary_update_commands_are_native_cli_updates(self):
        module = load_module()

        self.assertEqual(["codex", "update"], module.PRIMARY_UPDATE_COMMANDS["codex"])
        self.assertEqual(["agy", "update"], module.PRIMARY_UPDATE_COMMANDS["agy"])
        self.assertEqual(["make", "hermes-update"], module.PRIMARY_UPDATE_COMMANDS["hermes"])

    def test_update_dry_run_is_non_mutating_and_native(self):
        module = load_module()
        result = module.update_dry_run()

        self.assertEqual("cynic.runtime_principals.update_plan.v1", result["schema"])
        self.assertEqual("cynic update --dry-run", result["command"])
        self.assertEqual("cynic update", result["dispatch_command"])
        self.assertFalse(result["mutates"])
        self.assertEqual(["codex", "update"], result["updates"]["codex"]["command"])
        self.assertEqual(["agy", "update"], result["updates"]["agy"]["command"])
        self.assertIn("curl | sh", result["forbidden_patterns"])
        self.assertIn("cargo build -p cynic-mcp", result["forbidden_patterns"])
        self.assertIn("install -m 0755 target/release/cynic-mcp", result["forbidden_patterns"])
        self.assertIn("cp target/release/cynic-kernel", result["forbidden_patterns"])
        self.assertIn("sudo cynic status --json", result["proof_commands"])
        sequence = result["operator_sequence"]
        self.assertEqual("inspect", sequence[0]["step"])
        self.assertEqual("cynic update --dry-run", sequence[0]["command"])
        self.assertEqual("update_dependency", sequence[1]["step"])
        self.assertEqual("codex update", sequence[1]["command"])
        self.assertTrue(sequence[1]["mutates_process"])
        self.assertEqual("update_dependency", sequence[2]["step"])
        self.assertEqual("agy update", sequence[2]["command"])
        self.assertTrue(sequence[2]["mutates_process"])
        self.assertEqual("update_dependency", sequence[3]["step"])
        self.assertEqual("make hermes-update", sequence[3]["command"])
        self.assertTrue(sequence[3]["mutates_process"])
        self.assertEqual("apply", sequence[4]["step"])
        self.assertEqual("cynic bootstrap", sequence[4]["command"])
        self.assertTrue(sequence[4]["mutates_host"])
        self.assertEqual("sudo cynic status --json", sequence[5]["command"])
        self.assertIn("execution_posture.native_default=true", result["read_back"])
        self.assertIn("execution_posture.native_default=true", result["done_when"][3])

    def test_update_bootstrap_flag_delegates_to_bootstrap(self):
        module = load_runtime_actions_module()
        globals_ref = module._handle_update.__globals__
        original_bootstrap = globals_ref["_handle_bootstrap"]
        captured: dict[str, list[str]] = {}

        def fake_bootstrap(extra: list[str]) -> int:
            captured["extra"] = list(extra)
            return 7

        globals_ref["_handle_bootstrap"] = fake_bootstrap
        try:
            result = module._handle_update(["--bootstrap", "--json"])
        finally:
            globals_ref["_handle_bootstrap"] = original_bootstrap

        self.assertEqual(7, result)
        self.assertEqual(["--json"], captured["extra"])

    def test_bootstrap_dry_run_is_composed(self):
        module = load_runtime_actions_module()
        result = module._bootstrap_dry_run()

        self.assertEqual("cynic.runtime_principals.bootstrap_plan.v1", result["schema"])
        self.assertEqual("cynic bootstrap --dry-run", result["command"])
        self.assertEqual("cynic bootstrap", result["dispatch_command"])
        self.assertFalse(result["mutates"])
        sequence = result["operator_sequence"]
        self.assertEqual("inspect", sequence[0]["step"])
        self.assertEqual("cynic update --dry-run", sequence[0]["command"])
        self.assertEqual("update", sequence[1]["step"])
        self.assertEqual("cynic update", sequence[1]["command"])
        self.assertEqual("apply", sequence[2]["step"])
        self.assertEqual("cynic apply", sequence[2]["command"])
        self.assertEqual("install_defaults", sequence[3]["step"])
        self.assertEqual("cynic install-defaults", sequence[3]["command"])

    def test_update_forbidden_patterns_cover_infra_cortex_forbidden_rules(self):
        module = load_module()
        result = module.update_dry_run()
        infra = json.loads((REPO_ROOT / "axioms" / "infra-consumption-v1.json").read_text(encoding="utf-8"))
        forbidden_on_cortex = set(infra["native_lifecycle"]["forbidden_on_cortex"])

        self.assertTrue(forbidden_on_cortex.issubset(set(result["forbidden_patterns"])))

    def test_runtime_principals_contract_declares_native_operator_commands(self):
        module = load_module()
        contract = json.loads(module.CONTRACT.read_text(encoding="utf-8"))
        commands = set(contract["activation"]["operator_commands"])

        expected = {
            "cynic organism --json",
            "cynic repos status",
            "cynic repos add",
            "cynic repos remove",
            "cynic repos sync",
            "cynic activate --dry-run",
            "cynic activate --json",
            "cynic mcp-reload --json",
            "cynic gate-2 --json",
            "cynic deploy --json",
            "cynic shell",
        }
        self.assertTrue(expected.issubset(commands), sorted(expected - commands))
        sequence = contract["activation"]["lifecycle_operator_sequence"]
        json_steps = [
            item
            for item in sequence["steps"]
            if item["command_template"] == "cynic {profile} --json"
        ]
        self.assertEqual(1, len(json_steps))
        self.assertEqual("dispatch_json", json_steps[0]["name"])
        self.assertTrue(json_steps[0]["mutates_mempool"])

    def test_agent_docs_present_cynic_lifecycle_as_native_operator_path(self):
        agents = (REPO_ROOT / "AGENTS.md").read_text(encoding="utf-8")
        codex = (REPO_ROOT / "CODEX.md").read_text(encoding="utf-8")

        self.assertIn("cynic gate-2 --dry-run", agents)
        self.assertIn("cynic mcp-reload", agents)
        self.assertIn("cynic deploy", agents)
        self.assertIn("cynic activate --dry-run", agents)
        self.assertNotIn("make gate-2-request  # Canonical", agents)
        self.assertNotIn("make mcp-reload-request  # Canonical", agents)
        self.assertNotIn("make deploy-request  # Canonical", agents)
        self.assertIn("legacy fallback", codex)
        self.assertIn("cynic mcp-reload", codex)

    def test_local_validation_report_reads_gate_markers_without_mutating(self):
        module = load_module()
        temp_root = REPO_ROOT / "tmp" / "local-validation-test"
        temp_root.mkdir(parents=True, exist_ok=True)
        gate_0 = temp_root / ".gate-0"
        gate_1 = temp_root / ".gate-1"
        gate_0.unlink(missing_ok=True)
        gate_1.unlink(missing_ok=True)
        try:
            blocked = module.local_validation_report(temp_root)
            self.assertEqual("blocked", blocked["status"])
            self.assertEqual(["gate_0", "gate_1"], blocked["missing"])
            self.assertFalse(blocked["mutates"])
            gate_0.write_text("", encoding="utf-8")
            gate_1.write_text("", encoding="utf-8")
            os.utime(gate_0, (10, 10))
            os.utime(gate_1, (10, 10))
            ready = module.local_validation_report(temp_root)
            self.assertEqual("ready", ready["status"])
            self.assertEqual([], ready["missing"])
            self.assertEqual("make check", ready["commands"]["check"])
            rust_dir = temp_root / "crates" / "demo" / "src"
            rust_dir.mkdir(parents=True)
            rust_source = rust_dir / "lib.rs"
            rust_source.write_text("pub fn demo() {}\n", encoding="utf-8")
            os.utime(rust_source, (20, 20))
            stale = module.local_validation_report(temp_root)
            self.assertEqual("blocked", stale["status"])
            self.assertIn("gate_1_fresh", stale["missing"])
        finally:
            gate_0.unlink(missing_ok=True)
            gate_1.unlink(missing_ok=True)
            rust_source.unlink(missing_ok=True) if "rust_source" in locals() else None
            rust_dir.rmdir() if "rust_dir" in locals() and rust_dir.exists() else None
            (temp_root / "crates" / "demo").rmdir() if (temp_root / "crates" / "demo").exists() else None
            (temp_root / "crates").rmdir() if (temp_root / "crates").exists() else None
            temp_root.rmdir()

    def test_organism_report_surfaces_stale_cynic_cli_projection_as_next_check(self):
        module = load_module()
        globals_ref = module.organism_report.__globals__
        original_status = globals_ref["status"]
        original_mcp = globals_ref["mcp_freshness_report"]
        original_storage = globals_ref["storage_primary_status"]
        original_kernel = globals_ref["kernel_health_report"]
        original_lifecycle = globals_ref["lifecycle_profile_report"]
        original_activation = globals_ref["lifecycle_activation_report"]
        original_local_validation = globals_ref["local_validation_report"]
        try:
            globals_ref["status"] = lambda: {
                "schema": "cynic.runtime_principals.status.v1",
                "runtime_ready": False,
                "runtime_evidence_status": "not_ready",
                "requires_root_evidence": False,
                "root_evidence_command": "sudo cynic status --json",
                "cynic_cli_projection_ok": False,
                "cynic_cli_projection_command": "cynic bootstrap",
                "proof_requirements": [
                    {
                        "check": "operator_cli_projection_fresh",
                        "status": "missing",
                        "command": "cynic bootstrap",
                        "expected": "operator CLI matches generated wrapper",
                    }
                ],
            }
            globals_ref["mcp_freshness_report"] = lambda: {"status": "fresh"}
            globals_ref["storage_primary_status"] = lambda args=None: {"status": "active", "next_checks": []}
            globals_ref["kernel_health_report"] = lambda: {"status": "unknown"}
            globals_ref["lifecycle_profile_report"] = lambda profile: {"status": "ok"}
            globals_ref["lifecycle_activation_report"] = lambda: {
                "schema": "cynic.lifecycle.activation_summary.v1",
                "status": "ready",
                "activation_request_summary": [],
                "activation_request_count": 0,
            }
            globals_ref["local_validation_report"] = lambda project_root: {"status": "ready", "proof_requirements": []}

            report = module.organism_report()
        finally:
            globals_ref["status"] = original_status
            globals_ref["mcp_freshness_report"] = original_mcp
            globals_ref["storage_primary_status"] = original_storage
            globals_ref["kernel_health_report"] = original_kernel
            globals_ref["lifecycle_profile_report"] = original_lifecycle
            globals_ref["lifecycle_activation_report"] = original_activation
            globals_ref["local_validation_report"] = original_local_validation

        blocker = next(item for item in report["blockers"] if item["surface"] == "cortex_runtime")
        self.assertEqual(["cynic bootstrap"], blocker["next_checks"])
        self.assertEqual("operator_cli_projection_fresh", blocker["proof_requirements"][0]["check"])
        self.assertIn("cynic bootstrap", report["next_checks"])
        self.assertLess(
            report["next_checks"].index("cynic bootstrap"),
            report["next_checks"].index("sudo cynic status --json"),
        )
        commands = [item["command"] for item in report["operator_sequence"]]
        self.assertEqual("cynic bootstrap", commands[0])
        apply_step = next(item for item in report["operator_sequence"] if item["command"] == "cynic bootstrap")
        self.assertEqual("apply", apply_step["step"])
        self.assertTrue(apply_step["mutates_host"])

    def test_organism_report_carries_native_next_checks_from_infra_surfaces(self):
        module = load_module()
        globals_ref = module.organism_report.__globals__
        originals = {
            "status": globals_ref["status"],
            "mcp_freshness_report": globals_ref["mcp_freshness_report"],
            "storage_primary_status": globals_ref["storage_primary_status"],
            "lxc_exec_bridge_status": globals_ref["lxc_exec_bridge_status"],
            "fleet_status": globals_ref["fleet_status"],
            "mesh_status": globals_ref["mesh_status"],
            "kernel_health_report": globals_ref["kernel_health_report"],
            "lifecycle_profile_report": globals_ref["lifecycle_profile_report"],
            "lifecycle_activation_report": globals_ref["lifecycle_activation_report"],
        }

        globals_ref["status"] = lambda: {
            "runtime_ready": True,
            "runtime_evidence_status": "ready",
            "canonical_project_root": str(REPO_ROOT),
        }
        globals_ref["mcp_freshness_report"] = lambda: {
            "status": "stale",
            "canonical_action": "cynic mcp-reload",
            "mcp_dispatch": "cynic_lifecycle_dispatch(profile=\"mcp-reload\")",
            "proof_requirements": [
                {
                    "check": "mcp_binary_fresh",
                    "status": "missing",
                    "command": "cynic mcp-reload",
                    "expected": "selected MCP binary is newer than repo MCP sources",
                }
            ],
        }
        globals_ref["storage_primary_status"] = lambda: {
            "status": "blocked",
            "missing": ["bridge_active"],
            "next_checks": ["cynic bridge status", "cynic organism"],
            "proof_requirements": [
                {
                    "check": "bridge_active",
                    "status": "missing",
                    "command": "cynic bridge status --json",
                    "expected": "status=active",
                }
            ],
        }
        globals_ref["lxc_exec_bridge_status"] = lambda: {
            "status": "missing_activation",
            "proof_requirements": [
                {
                    "check": "helper_exists",
                    "status": "missing",
                    "command": "cynic bridge status --json",
                    "expected": "bridge activation check helper_exists=true",
                }
            ],
        }
        globals_ref["fleet_status"] = lambda: {
            "status": "degraded",
            "proof_requirements": [
                {
                    "check": "fleet_inventory_observed",
                    "status": "missing",
                    "command": "cynic fleet status --json",
                    "expected": "provider or fleet evidence observes declared runtime nodes",
                }
            ],
        }
        globals_ref["mesh_status"] = lambda args=None: {
            "status": "blocked",
            "proof_requirements": [
                {
                    "check": "mesh_dns_ok",
                    "status": "missing",
                    "command": "cynic mesh status --resolve-dns --include-records --json",
                    "expected": "all rendered Synapse DNS names resolve to registry IPAM addresses",
                }
            ],
        }
        globals_ref["kernel_health_report"] = lambda: {"status": "unknown"}
        globals_ref["lifecycle_profile_report"] = lambda profile: {"profile": profile, "status": "ok"}
        globals_ref["lifecycle_activation_report"] = lambda: {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "blocked",
            "activation_request_count": 1,
            "activation_request_summary": [],
            "activation_dispatch_command": "cynic activate",
        }
        try:
            report = module.organism_report()
        finally:
            globals_ref.update(originals)

        storage_blocker = next(item for item in report["blockers"] if item["surface"] == "storage_primary")
        activation_blocker = next(item for item in report["blockers"] if item["surface"] == "lifecycle_activation")
        mcp_blocker = next(item for item in report["blockers"] if item["surface"] == "mcp_freshness")
        self.assertEqual("cynic mcp-reload", mcp_blocker["action"])
        self.assertEqual("cynic_lifecycle_dispatch(profile=\"mcp-reload\")", mcp_blocker["mcp_dispatch"])
        self.assertEqual("mcp_binary_fresh", mcp_blocker["proof_requirements"][0]["check"])
        self.assertEqual(["cynic bridge status"], storage_blocker["next_checks"])
        self.assertEqual("bridge_active", storage_blocker["proof_requirements"][0]["check"])
        self.assertEqual("helper_exists", storage_blocker["dependency_proofs"]["bridge"][0]["check"])
        self.assertEqual("fleet_inventory_observed", storage_blocker["dependency_proofs"]["fleet"][0]["check"])
        self.assertEqual("mesh_dns_ok", storage_blocker["dependency_proofs"]["mesh"][0]["check"])
        self.assertEqual(["cynic actions", "cynic activate --dry-run"], activation_blocker["next_checks"])
        self.assertEqual("activation_request_inventory", activation_blocker["proof_requirements"][0]["check"])
        self.assertEqual("cynic actions --json", activation_blocker["proof_requirements"][0]["command"])
        self.assertEqual("cynic activate", activation_blocker["dispatch_plan"]["command"])
        self.assertIn("cynic bridge status", report["next_checks"])
        self.assertIn("cynic activate --dry-run", report["next_checks"])
        self.assertNotIn("cynic organism", report["next_checks"])
        sequence_by_command = {item["command"]: item for item in report["operator_sequence"]}
        self.assertTrue(sequence_by_command["cynic mcp-reload"]["mutates_mempool"])
        self.assertFalse(sequence_by_command["cynic bridge status"]["mutates_mempool"])
        self.assertTrue(sequence_by_command["cynic activate"]["mutates_mempool"])

        body = module.organism_text(report)
        self.assertIn("Next checks:", body)
        self.assertIn("proof:", body)
        self.assertIn("bridge_active: cynic bridge status --json -> status=active", body)
        self.assertIn("bridge proof gaps: 1", body)
        self.assertIn("fleet proof gaps: 1", body)
        self.assertIn("mesh proof gaps: 1", body)
        self.assertIn("activation_dispatch_read_back: cynic activate --json [mutates mempool]", body)
        self.assertIn("Operator sequence:", body)
        self.assertIn("dispatch: cynic mcp-reload [mutates mempool]", body)
        self.assertIn("cynic bridge status", body)
        self.assertIn("cynic activate --dry-run", body)

    def test_actions_report_is_non_mutating_dispatch_inventory(self):
        module = load_module()
        globals_ref = module.organism_actions_report.__globals__
        original_activation = globals_ref["lifecycle_activation_report"]
        globals_ref["lifecycle_activation_report"] = lambda: {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "blocked",
            "activation_dispatch_command": "cynic activate",
            "activation_request_summary": [
                {
                    "action": "refresh_fleet_inventory",
                    "node": None,
                    "code": "fleet_inventory:degraded",
                    "priority": 97,
                    "coord_targets": ["organs/fleet"],
                    "consumer": "organ-forge-hermes-agent",
                }
            ],
            "blockers": [],
        }
        try:
            report = module.organism_actions_report()
        finally:
            globals_ref["lifecycle_activation_report"] = original_activation

        self.assertEqual("cynic.organism.actions.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertFalse(report["mutates_mempool"])
        self.assertEqual(1, report["activation_request_count"])
        self.assertEqual(["organ-forge-hermes-agent"], report["consumers"])
        self.assertEqual(["organs/fleet"], report["coord_targets"])
        self.assertEqual("cynic activate", report["dispatch_plan"]["command"])
        self.assertTrue(report["dispatch_plan"]["mutates_mempool"])
        self.assertEqual("cynic activate --dry-run", report["dispatch_plan"]["operator_sequence"][0]["command"])
        self.assertEqual("cynic activate", report["dispatch_plan"]["operator_sequence"][1]["command"])
        requirements = {item["check"]: item for item in report["proof_requirements"]}
        self.assertEqual("satisfied", requirements["activation_request_inventory"]["status"])
        self.assertEqual("satisfied", requirements["activation_request_contract"]["status"])
        self.assertEqual("missing", requirements["activation_dispatch_read_back"]["status"])
        self.assertTrue(requirements["activation_dispatch_read_back"]["mutates_mempool"])

    def test_bridge_status_exposes_activation_proof_requirements(self):
        module = load_module()
        globals_ref = module.lxc_exec_bridge_status.__globals__
        original_run = globals_ref["_run_capture"]

        def fake_run(*args, **kwargs):
            return {
                "status": "failed",
                "exit_code": 1,
                "stdout": json.dumps(
                    {
                        "schema": "cynic.lxc_exec_bridge.activation_status.v1",
                        "status": "missing_activation",
                        "checks": {"helper_exists": False, "sudoers_no_shell": True},
                    }
                ),
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            report = module.lxc_exec_bridge_status()
        finally:
            globals_ref["_run_capture"] = original_run
        by_check = {item["check"]: item for item in report["proof_requirements"]}

        self.assertEqual("missing", by_check["helper_exists"]["status"])
        self.assertEqual("satisfied", by_check["sudoers_no_shell"]["status"])
        self.assertEqual("organ-fleet", by_check["helper_exists"]["resolver_organ"])
        body = module.infra_report_text(
            report,
            title="CYNIC bridge status",
            json_command="cynic bridge status --json",
        )
        self.assertIn("Proof:", body)
        self.assertIn("helper_exists: cynic bridge status --json", body)

    def test_fleet_status_exposes_runtime_observation_proof_requirements(self):
        module = load_module()
        globals_ref = module.fleet_status.__globals__
        original_run = globals_ref["_run_capture"]

        def fake_run(*args, **kwargs):
            return {
                "status": "ok",
                "exit_code": 0,
                "stdout": json.dumps(
                    {
                        "schema": "cynic.fleet.status.v1",
                        "summary": {"observed_resources": 0},
                        "anomalies": [
                            {
                                "severity": "critical",
                                "kind": "active_without_runtime_proof",
                                "target": "cynic-vm-v2",
                                "message": "Node is declared active but no provider observation matched.",
                            }
                        ],
                    }
                ),
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            report = module.fleet_status()
        finally:
            globals_ref["_run_capture"] = original_run
        by_check = {item["check"]: item for item in report["proof_requirements"]}

        self.assertEqual("missing", by_check["fleet_inventory_observed"]["status"])
        self.assertEqual(0, by_check["fleet_inventory_observed"]["observed"])
        self.assertEqual("missing", by_check["active_without_runtime_proof"]["status"])
        self.assertEqual("cynic activate --dry-run", by_check["active_without_runtime_proof"]["command"])
        self.assertEqual("organ-fleet", by_check["active_without_runtime_proof"]["resolver_organ"])

    def test_mesh_status_exposes_dns_and_runtime_proof_requirements(self):
        module = load_module()
        globals_ref = module.mesh_status.__globals__
        original_run = globals_ref["_run_capture"]

        def fake_run(*args, **kwargs):
            return {
                "status": "failed",
                "exit_code": 1,
                "stdout": json.dumps(
                    {
                        "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
                        "policy_status": "ok",
                        "dns_status": "violation",
                        "runtime_status": "not_observed",
                    }
                ),
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            report = module.mesh_status(["--resolve-dns", "--include-records"])
        finally:
            globals_ref["_run_capture"] = original_run
        by_check = {item["check"]: item for item in report["proof_requirements"]}

        self.assertEqual("satisfied", by_check["mesh_policy_ok"]["status"])
        self.assertEqual("missing", by_check["mesh_dns_ok"]["status"])
        self.assertEqual("cynic mesh status --resolve-dns --include-records --json", by_check["mesh_dns_ok"]["command"])
        self.assertEqual("missing", by_check["wireguard_runtime_observed"]["status"])
        self.assertEqual("cynic activate --dry-run", by_check["wireguard_runtime_observed"]["command"])
        self.assertEqual("cynic-node", by_check["mesh_dns_ok"]["resolver_organ"])

    def test_mesh_status_projects_cynic_node_runtime_surface(self):
        module = load_module()
        globals_ref = module.mesh_status.__globals__
        original_run = globals_ref["_run_capture"]

        def fake_run(*args, **kwargs):
            return {
                "status": "ok",
                "exit_code": 0,
                "stdout": json.dumps(
                    {
                        "schema": "cynic.node.network_runtime_status.v1",
                        "status": "blocked",
                        "hostname": "ubuntu",
                        "observation": {
                            "network_mesh_status": {
                                "policy_status": "ok",
                                "dns_status": "violation",
                                "runtime_status": "mesh_active",
                            }
                        },
                        "dns_activation": {"missing": ["synapse_dns_resolves"]},
                        "blockers": [{"kind": "runtime_blocker"}],
                    }
                ),
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            report = module.mesh_status(["--resolve-dns", "--include-records"])
        finally:
            globals_ref["_run_capture"] = original_run

        self.assertEqual("cynic.infrastructure_mcp.network_mesh_status.v1", report["schema"])
        self.assertEqual("ok", report["policy_status"])
        self.assertEqual("violation", report["dns_status"])
        self.assertEqual("mesh_active", report["runtime_status"])
        self.assertEqual(["synapse_dns_resolves"], report["missing"])

    def test_lifecycle_subprocesses_run_from_repo_root_with_pythonpath(self):
        module = load_lifecycle_actions_module()

        class Completed:
            stdout = '{"status":"ok"}\n'
            stderr = ""

        calls = []

        def fake_run(cmd, **kwargs):
            calls.append((cmd, kwargs))
            return Completed()

        with mock.patch.object(module.subprocess, "run", side_effect=fake_run):
            with mock.patch.dict(module.os.environ, {"PYTHONPATH": "existing-path"}, clear=True):
                self.assertEqual("ok", module.activation_dry_run()["status"])
                self.assertEqual("ok", module.activate_lifecycle()["status"])
                self.assertEqual("ok", module.lifecycle_profile_dry_run("gate-2")["status"])
                self.assertEqual("ok", module.request_lifecycle_profile("mcp-reload")["status"])

        self.assertEqual(4, len(calls))
        self.assertEqual("lifecycle-blockers", calls[0][0][-1])
        self.assertEqual("lifecycle-activation-dispatch", calls[1][0][-1])
        self.assertIn("lifecycle-profile-payloads", calls[2][0])
        self.assertIn("delegate-profile", calls[3][0])
        for cmd, kwargs in calls:
            self.assertEqual(REPO_ROOT, kwargs["cwd"])
            self.assertEqual(str(REPO_ROOT), kwargs["env"]["CYNIC_PROJECT_ROOT"])
            self.assertEqual(str(REPO_ROOT), kwargs["env"]["PYTHONPATH"].split(os.pathsep)[0])
            self.assertEqual("existing-path", kwargs["env"]["PYTHONPATH"].split(os.pathsep)[1])
            self.assertEqual(sys.executable, cmd[0])
            self.assertFalse(kwargs["check"])
            self.assertTrue(kwargs["capture_output"])
            self.assertTrue(kwargs["text"])

    def test_organism_summary_omits_deep_runtime_payloads(self):
        module = load_runtime_actions_module()
        report = {
            "status": "blocked",
            "actions": {
                "actions": [
                    {
                        "action": "materialize_runtime_path",
                        "node": "cynic-vm-anvil",
                        "coord_targets": ["infra/registry.json"],
                    }
                ]
            },
            "blockers": [
                {
                    "surface": "storage_primary",
                    "status": "blocked",
                    "missing": ["bridge_active", "mesh_reaches_storage"],
                    "evidence": {"very": "large"},
                }
            ],
            "fleet": {"declared": ["large"]},
            "mesh": {"dns_checks": ["large"]},
            "stack": {
                "actions": [{"entity": "cynic-mcp", "next_action": "cynic mcp-reload"}],
                "blockers": [{"entity": "cynic-mcp", "health": "blocked", "next_action": "cynic mcp-reload"}],
                "entities": ["large"],
            },
            "next_checks": ["make check", "None", None, "cynic mcp-reload"],
        }

        summary = module._organism_summary(report)

        self.assertEqual("cynic.organism.summary.v1", summary["schema"])
        self.assertEqual("blocked", summary["status"])
        self.assertEqual(2, summary["action_count"])
        self.assertEqual(2, summary["blocker_count"])
        self.assertEqual(["make check", "cynic mcp-reload"], summary["next_checks"])
        self.assertNotIn("fleet", summary)
        self.assertNotIn("mesh", summary)
        self.assertNotIn("stack", summary)
        self.assertNotIn("evidence", summary["blockers"][0])



if __name__ == "__main__":
    unittest.main()
