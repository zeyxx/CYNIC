import unittest

from organs.forge.src.adapters.kernel_adapter import (
    KernelRestClient,
    normalize_kernel_base_url,
    validate_lifecycle_policy,
)
from organs.forge.src.core.entities import FleetProbe
from organs.forge.src.core.services import BuildDelegationService
from organs.forge.src.tests.fixtures import (
    PROJECT_ROOT,
    registry_fixture,
)


class LifecyclePolicyTests(unittest.TestCase):
    def test_makefile_canonical_targets_use_lifecycle_cli_contracts(self):
        makefile = (PROJECT_ROOT / "Makefile").read_text()
        cortex_surface = (PROJECT_ROOT / "make/surfaces/cortex.mk").read_text()
        lifecycle_surface = (PROJECT_ROOT / "make/surfaces/lifecycle.mk").read_text()
        make_surfaces = makefile + cortex_surface + lifecycle_surface
        gate_2_request = cortex_surface.split("gate-2-request:", 1)[1].split(".PHONY: mcp-reload-request", 1)[0]
        mcp_reload_request = cortex_surface.split("mcp-reload-request:", 1)[1].split(".PHONY: gate-1", 1)[0]
        deploy_request = lifecycle_surface.split("deploy-request:", 1)[1]
        e2e = makefile.split("e2e:", 1)[1].split("# ── Standalone: System status", 1)[0]

        self.assertIn(
            'python3 organs/forge/src/entrypoints/cli.py delegate-profile "gate-2" --json',
            make_surfaces,
        )
        self.assertIn(
            'python3 organs/forge/src/entrypoints/cli.py delegate-profile "deploy" --json',
            make_surfaces,
        )
        self.assertIn(
            'python3 organs/forge/src/entrypoints/cli.py delegate-profile "mcp-reload" --json',
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py kernel-url",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-task-payload",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-validate-task-payload",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-plan",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-requests",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-dispatch",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-runtime-paths",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-verify-runtime-paths",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-service-evidence-patch",
            make_surfaces,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-validate-activation-evidence-patch",
            make_surfaces,
        )
        self.assertLess(
            gate_2_request.index('lifecycle-profile-payloads "gate-2"'),
            gate_2_request.index("lifecycle-check"),
        )
        self.assertLess(
            gate_2_request.index('lifecycle-profile-payloads "gate-2"'),
            gate_2_request.index('delegate-profile "gate-2" --json'),
        )
        self.assertLess(
            mcp_reload_request.index('lifecycle-profile-payloads "mcp-reload"'),
            mcp_reload_request.index("lifecycle-check"),
        )
        self.assertLess(
            mcp_reload_request.index('lifecycle-profile-payloads "mcp-reload"'),
            mcp_reload_request.index('delegate-profile "mcp-reload" --json'),
        )
        self.assertLess(
            deploy_request.index('lifecycle-profile-payloads "deploy"'),
            deploy_request.index("lifecycle-check"),
        )
        self.assertLess(
            deploy_request.index('lifecycle-profile-payloads "deploy"'),
            deploy_request.index('delegate-profile "deploy" --json'),
        )
        self.assertIn("observations?limit=1", e2e)
        self.assertIn("audit?limit=1", e2e)
        self.assertIn("crystals?limit=1", e2e)
        self.assertIn('TASK_ID="$$TASK"', e2e)
        self.assertIn("pending_visible=$$PENDING", e2e)
        self.assertIn("completed_visible=$$COMPLETED", e2e)
        self.assertIn("any(t.get('id') == tid", e2e)
        self.assertNotIn('"$${PENDING:-0}" -gt 0', e2e)
        self.assertNotIn('"$${COMPLETED:-0}" -gt 0', e2e)
        self.assertEqual(e2e.count("2>/dev/null || true"), 7)
        self.assertNotIn('ADDR="http://$${CYNIC_REST_ADDR}"', makefile)

    def test_agent_task_target_contract_does_not_drift_across_boundaries(self):
        expected_keys = '["coord_targets", "targets", "repo_targets", "files"]'
        expected_error = (
            "repo-affecting hermes tasks must declare coord_targets, targets, "
            "repo_targets, or files"
        )
        expected_lifecycle_error = (
            "repo-affecting forge tasks must match registry {field}={expected}"
        )
        expected_lifecycle_fields = [
            "validate_forge_lifecycle_contract",
            "load_registry()",
            "required_non_empty_string",
            "require_registry_json_match",
            "node_name_for_role",
            "build_target",
            "deploy_target",
            "runtime_paths",
            "runtime_path_proofs",
            "activation_evidence_contract",
        ]
        surfaces = [
            PROJECT_ROOT / "crates/cynic-kernel/src/api/rest/agent_tasks.rs",
            PROJECT_ROOT / "crates/cynic-kernel/src/api/mcp/mod.rs",
            PROJECT_ROOT / "crates/cynic-mcp/src/types.rs",
        ]

        for surface in surfaces:
            source = surface.read_text()
            self.assertIn(expected_keys, source, str(surface))
            self.assertIn(expected_error, source, str(surface))
            self.assertIn(expected_lifecycle_error, source, str(surface))
            for expected_field in expected_lifecycle_fields:
                self.assertIn(expected_field, source, str(surface))

    def test_normalize_kernel_base_url_accepts_plain_or_scheme_address(self):
        self.assertEqual(
            normalize_kernel_base_url("100.64.0.1:3030/"),
            "http://100.64.0.1:3030",
        )
        self.assertEqual(
            normalize_kernel_base_url("http://100.64.0.1:3030/"),
            "http://100.64.0.1:3030",
        )
        self.assertEqual(
            normalize_kernel_base_url("https://kernel.example:3030/"),
            "https://kernel.example:3030",
        )

    def test_activation_evidence_gap_details_are_structured(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.build_target = client.build_node["name"]
        client.deploy_target = client.deploy_node["name"]
        client.kernel_probe = FleetProbe.not_run()
        client.base_url = "<unresolved>"
        client.kernel_endpoint_source = "test"
        client.kernel_bus_canonical = False

        details = client._activation_evidence_gap_details()

        self.assertEqual(
            [gap["code"] for gap in details],
            [
                "cynic-vm-anvil:synapse_env_SYNAPSE_ANVIL_missing",
                "cynic-vm-anvil:service_organ-forge_inactive",
                "cynic-vm-anvil:service_cargo-build_inactive",
                "cynic-vm-anvil:service_gate-2_inactive",
                "cynic-vm-anvil:runtime_path_repo_root_unverified",
                "cynic-vm-core:synapse_env_SYNAPSE_CORE_missing",
                "cynic-vm-core:service_cynic-kernel_inactive",
                "cynic-vm-core:service_cynic-portal_inactive",
                "cynic-vm-core:runtime_path_repo_root_unverified",
                "cynic-lxc-surrealdb:synapse_env_SYNAPSE_STORAGE_missing",
                "cynic-lxc-surrealdb:service_surrealdb_inactive",
            ],
        )
        self.assertEqual(details[0]["kind"], "network")
        self.assertEqual(details[0]["role"], "build_runner")
        self.assertEqual(details[0]["reason"], "missing_env")
        self.assertEqual(details[0]["env"], "SYNAPSE_ANVIL")
        self.assertEqual(details[1]["kind"], "service")
        self.assertEqual(details[1]["role"], "build_runner")
        self.assertEqual(details[1]["reason"], "missing_service_evidence")
        self.assertEqual(details[1]["requirement"], "organ-forge")
        self.assertEqual(details[4]["kind"], "filesystem")
        self.assertEqual(details[4]["role"], "build_runner")
        self.assertEqual(details[4]["reason"], "missing_runtime_path_proof")
        self.assertEqual(details[4]["path"], "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}")
        self.assertEqual(details[4]["proof"]["kind"], "git_worktree")

    def test_activation_evidence_resolves_additional_storage_primary_role(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        lifecycle["activation_requirements"]["storage_primary"] = {
            "network": ["synapse"],
            "filesystem": [],
            "services": ["surrealdb"],
        }
        registry["nodes"].append(
            {
                "name": "cynic-lxc-surrealdb",
                "role": "storage_primary",
                "status": "planned",
                "network": {"synapse": "10.20.0.3"},
                "services": [{"name": "surrealdb"}],
            }
        )
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.build_target = client.build_node["name"]
        client.deploy_target = client.deploy_node["name"]
        client.kernel_probe = FleetProbe.not_run()
        client.base_url = "<unresolved>"
        client.kernel_endpoint_source = "test"
        client.kernel_bus_canonical = False

        details = client._activation_evidence_gap_details()
        codes = [gap["code"] for gap in details]

        self.assertEqual(validate_lifecycle_policy(registry)["status"], "ok")
        self.assertNotIn("storage_primary:node_not_resolved", codes)
        self.assertIn("cynic-lxc-surrealdb:service_surrealdb_inactive", codes)

        client.fleet_probe = {"status": "ok", "missing": [], "matches": {}}
        client.fleet_inventory_probe = {"enabled": True, "status": "ok"}
        context = client.lifecycle_context()
        blocker_codes = [blocker["code"] for blocker in context["lifecycle_blockers"]]

        self.assertFalse(context["status_nodes_ready"])
        self.assertIn("cynic-lxc-surrealdb:status_planned_not_active", blocker_codes)

    def test_activation_evidence_gap_details_accepts_runtime_path_proof(self):
        registry = registry_fixture()
        registry["nodes"][0]["activation_evidence"] = {
            "runtime_paths": {
                "repo_root": {
                    "verified": True,
                    "verified_at": "2026-06-15T00:00:00Z",
                }
            }
        }
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]

        details = client._activation_evidence_gap_details()

        self.assertNotIn(
            "cynic-vm-anvil:runtime_path_repo_root_unverified",
            [gap["code"] for gap in details],
        )
        self.assertIn(
            "cynic-vm-core:runtime_path_repo_root_unverified",
            [gap["code"] for gap in details],
        )

    def test_build_delegation_service_can_capture_without_emitting_logs(self):
        class GitPerceiver:
            def extract_patch(self):
                return "patch"

        class KernelClient:
            async def dispatch_task_and_stream(self, intent, patch):
                self.intent = intent
                self.patch = patch
                yield '{"task_id":"agent_tasks:test"}\n'

        kernel = KernelClient()
        service = BuildDelegationService(GitPerceiver(), kernel)

        result = __import__("asyncio").run(
            service.delegate_task("deploy-kernel", emit_logs=False)
        )

        self.assertEqual(result, '{"task_id":"agent_tasks:test"}\n')
        self.assertEqual(kernel.intent, "deploy-kernel")
        self.assertEqual(kernel.patch, "patch")

    def test_observation_storage_migrates_legacy_optional_schema(self):
        storage_source = (
            PROJECT_ROOT / "crates/cynic-kernel/src/storage/mod.rs"
        ).read_text()

        for field in ["confidence", "consumer", "action"]:
            self.assertIn(
                f"DEFINE FIELD OVERWRITE {field} ON observation TYPE string | null",
                storage_source,
            )
        for field in ["maturity", "consensus_score"]:
            self.assertIn(
                f"DEFINE FIELD OVERWRITE {field} ON observation TYPE float | null",
                storage_source,
            )

    def test_observation_storage_uses_explicit_projection(self):
        activity_source = (
            PROJECT_ROOT / "crates/cynic-kernel/src/storage/surreal/activity.rs"
        ).read_text()

        self.assertIn("OBSERVATION_SELECT_FIELDS", activity_source)
        self.assertNotIn("SELECT * FROM observation", activity_source)

    def test_forge_cron_health_task_does_not_use_release_lifecycle_domain(self):
        dispatch_source = (PROJECT_ROOT / "organs/forge/src/dispatch.py").read_text()

        self.assertIn('"domain": "forge"', dispatch_source)
        self.assertNotIn('"domain": "organ-forge"', dispatch_source)

    def test_forge_systemd_units_wire_cron_and_health_domain(self):
        cron_unit = (
            PROJECT_ROOT / "organs/forge/systemd/organ-forge-cron.service"
        ).read_text()
        hermes_unit = (
            PROJECT_ROOT / "organs/forge/systemd/organ-forge-hermes-agent.service"
        ).read_text()

        self.assertIn("entrypoints/cli.py audit", cron_unit)
        self.assertIn("HERMES_AGENT_DOMAIN_ALLOW=forge,organ-forge,repo-lifecycle", hermes_unit)
        self.assertIn("organs/forge/src/entrypoints/cli.py daemon", hermes_unit)
        self.assertNotIn("-m organs.core.daemon", hermes_unit)

    def test_lifecycle_critical_systemd_units_use_registry_repo_root(self):
        unit_paths = [
            PROJECT_ROOT / "organs/forge/systemd/organ-forge-cron.service",
            PROJECT_ROOT / "organs/forge/systemd/organ-forge-hermes-agent.service",
            PROJECT_ROOT / "infra/systemd/cynic-kernel.service",
            PROJECT_ROOT / "infra/systemd/cynic-portal.service",
        ]

        for unit_path in unit_paths:
            source = unit_path.read_text()
            self.assertIn("@CYNIC_PROJECT_ROOT@", source, str(unit_path))
            self.assertNotIn("/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC", source, str(unit_path))
            self.assertNotIn("/" + "home/${CYNIC_RUNTIME_USER}", source, str(unit_path))

    def test_rust_min_stack_is_consistent_across_lifecycle_surfaces(self):
        expected_stack = "67108864"
        cargo_config = (PROJECT_ROOT / ".cargo/config.toml").read_text()
        makefile = (PROJECT_ROOT / "Makefile").read_text()
        local_tools = (PROJECT_ROOT / "crates/cynic-mcp/src/local_tools.rs").read_text()

        self.assertIn(f'RUST_MIN_STACK = "{expected_stack}"', cargo_config)
        self.assertIn(f"export RUST_MIN_STACK := {expected_stack}", makefile)
        self.assertIn(f"export RUST_MIN_STACK={expected_stack}", makefile)
        self.assertIn(".NOTPARALLEL:", makefile)
        self.assertIn(f'.env("RUST_MIN_STACK", "{expected_stack}")', local_tools)
        self.assertNotIn("2147483648", cargo_config + local_tools)
        self.assertNotIn("33554432", cargo_config + local_tools)
        self.assertNotIn("4294967296", cargo_config + makefile + local_tools)
        self.assertNotIn("8589934592", cargo_config + makefile + local_tools)
        self.assertNotIn("17179869184", cargo_config + makefile + local_tools)

    def test_codex_mcp_wrapper_refuses_stale_binary_contract(self):
        wrapper = (PROJECT_ROOT / ".cortex/mcp/run_cynic_mcp.py").read_text()
        makefile = (PROJECT_ROOT / "Makefile").read_text()

        self.assertIn("from organs.contracts.src.project_root import resolve_repo_root", wrapper)
        self.assertIn("CYNIC_PROJECT_ROOT", wrapper)
        self.assertIn('os.environ.pop("CYNIC_PROJECT_ROOT", None)', wrapper)
        self.assertIn('os.environ["CYNIC_PROJECT_ROOT"] = str(project_root)', wrapper)
        self.assertIn(
            "project_root = resolve_repo_root(PROJECT_ROOT_CANDIDATE, prefer_runtime_mirror=not repair_local)",
            wrapper,
        )
        self.assertIn("select_mcp_binary,", wrapper)
        self.assertIn("selection = select_mcp_binary(project_root)", wrapper)
        self.assertIn("binary = selection.binary", wrapper)
        self.assertIn("selection.stale_source", wrapper)
        self.assertIn("Refusing to start an MCP contract that can drift from repo source", wrapper)
        self.assertIn("make mcp-reload-request", wrapper)
        self.assertIn('"status": "missing"', wrapper)
        self.assertIn('"fallback_available": True', wrapper)
        self.assertNotIn("Operator-only fallback", wrapper)
        forbidden_build = "cargo build -p " + "cynic-mcp"
        self.assertNotIn(forbidden_build, wrapper)
        self.assertIn("return 78", wrapper)
        self.assertIn("CYNIC_MCP_CHECK_ONLY", wrapper)
        self.assertIn("os.execv(str(binary), [str(binary)])", wrapper)
        self.assertNotIn('exec "$HOME/bin/cynic-kernel-mcp"', wrapper)
        self.assertIn(".PHONY: mcp-freshness", makefile)
        self.assertIn("mcp-freshness:", makefile)
        self.assertIn("CYNIC_MCP_CHECK_ONLY=1 python3 .cortex/mcp/run_cynic_mcp.py", makefile)
        self.assertIn("make mcp-reload-request", makefile)

if __name__ == "__main__":
    unittest.main()
