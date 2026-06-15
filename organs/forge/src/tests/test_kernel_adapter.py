import json
from pathlib import Path
import tempfile
import unittest
from io import StringIO
from unittest import mock

from organs.forge.src.entrypoints import cli
from organs.forge.src.adapters.kernel_adapter import (
    KernelRestClient,
    activate_lifecycle_registry_node,
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
from organs.forge.src.core.services import BuildDelegationService


PROJECT_ROOT = Path(__file__).resolve().parents[4]


def registry_fixture():
    return {
        "lifecycle": {
            "kernel_release": {
                "orchestrator_organ": "organ-forge",
                "task_kind": "hermes",
                "task_domain": "organ-forge",
                "build_runner_role": "build_runner",
                "deploy_target_role": "kernel_primary",
                "deploy_target_flag": "deploy_target",
                "required_node_status": "active",
                "activation_requirements": {
                    "build_runner": {
                        "network": ["tailscale"],
                        "filesystem": ["repo_root"],
                        "services": ["organ-forge", "cargo-build", "gate-2"],
                    },
                    "kernel_primary": {
                        "network": ["tailscale"],
                        "filesystem": ["repo_root"],
                        "services": ["cynic-kernel", "surrealdb", "cynic-portal"],
                    },
                },
                "fleet_requirements": {
                    "provider": "tailscale",
                    "required": True,
                    "require_online": True,
                },
                "kernel_bus_probe": {
                    "path": "/health",
                    "accepted_status": [200, 503],
                    "requires_bearer_auth": True,
                    "timeout_seconds": 10,
                },
                "requires_e2e": True,
                "runtime_paths": {
                    "repo_root": "%h/.cynic/repos/CYNIC",
                },
                "runtime_path_proofs": {
                    "repo_root": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    },
                },
                "activation_evidence_contract": {
                    "patch_field": "activation_evidence_patch",
                    "node_field": "node",
                    "runtime_paths_field": "activation_evidence.runtime_paths",
                    "services_field": "activation_evidence.services",
                    "verified_requires": ["marker_exists", "sentinels_verified"],
                    "service_active_requires": ["declared", "active"],
                    "ingestion_mode": "validated_merge",
                },
                "repo_targets": [
                    "crates/cynic-kernel",
                    "crates/cynic-mcp",
                    "infra/registry.json",
                    "Makefile",
                ],
                "request_profiles": {
                    "gate-2": ["build-and-test", "integration-test"],
                    "deploy": ["deploy-kernel"],
                },
                "workflow": {
                    "name": "kernel_release",
                    "owner": "organ-forge",
                    "canonical_stages": [
                        {
                            "name": "local_validation",
                            "scope": "cortex-local",
                            "make_target": "check",
                            "allowed_when_lifecycle_planned": True,
                        },
                        {
                            "name": "activation_request_generation",
                            "scope": "organ-forge",
                            "make_target": "lifecycle-activation-requests",
                            "allowed_when_lifecycle_planned": True,
                        },
                        {
                            "name": "activation_request_dispatch",
                            "scope": "organ-forge",
                            "make_target": "lifecycle-activation-dispatch",
                            "allowed_when_lifecycle_planned": False,
                        },
                        {
                            "name": "gate_2_request",
                            "scope": "organ-forge",
                            "make_target": "gate-2-request",
                            "request_profile": "gate-2",
                            "allowed_when_lifecycle_planned": False,
                        },
                        {
                            "name": "deploy_request",
                            "scope": "organ-forge",
                            "make_target": "deploy-request",
                            "request_profile": "deploy",
                            "allowed_when_lifecycle_planned": False,
                        },
                    ],
                    "planned_policy": {
                        "blocks_workflow_critical_edits": True,
                        "critical_targets": [
                            "infra/registry.json",
                            "Makefile",
                            "crates/cynic-kernel",
                            "crates/cynic-mcp",
                            "organs/forge",
                        ],
                    },
                },
                "intents": {
                    "build-and-test": {
                        "objective": "run gate-2 validation on registry build runner",
                        "deploy": False,
                    },
                    "integration-test": {
                        "objective": "run integration tests from registry build runner against kernel primary",
                        "deploy": False,
                    },
                    "deploy-kernel": {
                        "objective": "build kernel release on registry build runner and deploy to kernel primary",
                        "deploy": True,
                    },
                },
            }
        },
        "nodes": [
            {
                "name": "cynic-vm-anvil",
                "role": "build_runner",
                "network": {"tailscale": "<TAILSCALE_ANVIL>"},
                "services": [
                    {"name": "organ-forge"},
                    {"name": "cargo-build"},
                    {"name": "gate-2"},
                ],
                "status": "planned",
            },
            {
                "name": "cynic-vm-core",
                "role": "kernel_primary",
                "deploy_target": True,
                "network": {"tailscale": "<TAILSCALE_CORE>"},
                "services": [
                    {"name": "cynic-kernel"},
                    {"name": "surrealdb"},
                    {"name": "cynic-portal"},
                ],
                "status": "planned",
            },
        ],
        "organs": [{"name": "organ-forge"}],
    }


def registry_with_ready_anvil():
    registry = registry_fixture()
    node = registry["nodes"][0]
    node["network"]["tailscale"] = "100.64.0.10"
    node["activation_evidence"] = {
        "runtime_paths": {
            "repo_root": {
                "verified": True,
                "verified_at": "2026-06-15T00:00:00Z",
                "value": "%h/.cynic/repos/CYNIC",
                "expanded": "/home/cynic/.cynic/repos/CYNIC",
                "proof_kind": "git_worktree",
                "marker_exists": True,
                "sentinels_verified": True,
            }
        },
        "services": {
            "organ-forge": {
                "declared": True,
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
            },
            "cargo-build": {
                "declared": True,
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
            },
            "gate-2": {
                "declared": True,
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
            },
        },
    }
    return registry


class LifecyclePolicyTests(unittest.TestCase):
    def test_validate_lifecycle_policy_accepts_registry_contract(self):
        result = validate_lifecycle_policy(registry_fixture())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["request_profiles"], ["deploy", "gate-2"])
        self.assertEqual(result["repo_root"], "%h/.cynic/repos/CYNIC")
        self.assertEqual(result["workflow"]["name"], "kernel_release")

    def test_validate_lifecycle_policy_rejects_operator_checkout_runtime_root(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["runtime_paths"][
            "repo_root"
        ] = "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC"

        with self.assertRaises(RuntimeError) as ctx:
            validate_lifecycle_policy(registry)

        self.assertIn("runtime_paths.repo_root", str(ctx.exception))

    def test_validate_lifecycle_policy_rejects_missing_runtime_path_proof(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["runtime_path_proofs"].pop("repo_root")

        with self.assertRaises(RuntimeError) as ctx:
            validate_lifecycle_policy(registry)

        self.assertIn("runtime_path_proofs.repo_root", str(ctx.exception))

    def test_makefile_canonical_targets_use_lifecycle_cli_contracts(self):
        makefile = (PROJECT_ROOT / "Makefile").read_text()
        gate_2_request = makefile.split("gate-2-request:", 1)[1].split("# Local cortex validation", 1)[0]
        deploy_request = makefile.split("deploy-request:", 1)[1].split(".PHONY: require-local-deploy-opt-in", 1)[0]
        e2e = makefile.split("e2e:", 1)[1].split("# ── Standalone: System status", 1)[0]

        self.assertIn(
            'python3 organs/forge/src/entrypoints/cli.py delegate-profile "gate-2" --json',
            makefile,
        )
        self.assertIn(
            'python3 organs/forge/src/entrypoints/cli.py delegate-profile "deploy" --json',
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py kernel-url",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-task-payload",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-validate-task-payload",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-plan",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-requests",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-dispatch",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-runtime-paths",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-verify-runtime-paths",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-service-evidence-patch",
            makefile,
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-validate-activation-evidence-patch",
            makefile,
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

    def test_lifecycle_profile_intents_come_from_registry(self):
        registry = registry_fixture()

        self.assertEqual(
            lifecycle_profile_intents("gate-2", registry),
            ["build-and-test", "integration-test"],
        )
        self.assertEqual(lifecycle_profile_intents("deploy", registry), ["deploy-kernel"])

    def test_lifecycle_profile_payloads_builds_and_validates_each_intent(self):
        result = build_lifecycle_profile_payloads(
            profile="gate-2",
            domain="profile-dry-run",
            agent_id="cortex-profile-dry-run",
            registry=registry_fixture(),
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["intents"], ["build-and-test", "integration-test"])
        self.assertEqual(len(result["payloads"]), 2)
        self.assertEqual(len(result["validations"]), 2)
        self.assertEqual(
            [validation["intent"] for validation in result["validations"]],
            ["build-and-test", "integration-test"],
        )
        self.assertTrue(all(payload["domain"] == "profile-dry-run" for payload in result["payloads"]))

    def test_build_lifecycle_task_payload_resolves_dynamic_targets(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )

        self.assertEqual(payload["kind"], "hermes")
        self.assertEqual(payload["domain"], "supply-chain-e2e-test")
        self.assertEqual(payload["agent_id"], "cortex-e2e-supply-chain")
        content = json.loads(payload["content"])
        self.assertEqual(content["lifecycle"], "kernel_release")
        self.assertEqual(content["intent"], "deploy-kernel")
        self.assertEqual(content["orchestrator_organ"], "organ-forge")
        self.assertEqual(content["build_target"], "cynic-vm-anvil")
        self.assertEqual(content["build_runner_role"], "build_runner")
        self.assertEqual(content["deploy_target"], "cynic-vm-core")
        self.assertEqual(content["deploy_target_role"], "kernel_primary")
        self.assertEqual(content["required_node_status"], "active")
        self.assertEqual(content["fleet_requirements"]["provider"], "tailscale")
        self.assertEqual(
            content["activation_requirements"]["build_runner"]["services"],
            ["organ-forge", "cargo-build", "gate-2"],
        )
        self.assertEqual(content["runtime_paths"]["repo_root"], "%h/.cynic/repos/CYNIC")
        self.assertEqual(content["runtime_path_proofs"]["repo_root"]["kind"], "git_worktree")
        self.assertEqual(
            content["activation_evidence_contract"]["patch_field"],
            "activation_evidence_patch",
        )
        self.assertTrue(content["deploy"])
        self.assertIn("infra/registry.json", content["targets"])
        self.assertEqual(content["coord_targets"], content["targets"])
        self.assertEqual(content["coord_targets"], registry_fixture()["lifecycle"]["kernel_release"]["repo_targets"])

    def test_validate_lifecycle_task_payload_accepts_registry_payload(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )

        result = validate_lifecycle_task_payload(payload, registry_fixture())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["intent"], "deploy-kernel")
        self.assertEqual(
            result["coord_targets"],
            registry_fixture()["lifecycle"]["kernel_release"]["repo_targets"],
        )

    def test_validate_lifecycle_task_payload_rejects_missing_coord_targets(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        del content["coord_targets"]
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.coord_targets"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_runtime_path_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["runtime_paths"]["repo_root"] = "%h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.runtime_paths"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_runtime_path_proof_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["runtime_path_proofs"]["repo_root"]["marker"] = "README.md"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.runtime_path_proofs"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_activation_evidence_contract_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["activation_evidence_contract"]["ingestion_mode"] = "blind_merge"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.activation_evidence_contract"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_dispatch_validates_payload_before_posting_agent_task(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle

        invalid_content = {
            "lifecycle": "kernel_release",
            "intent": "deploy-kernel",
            "objective": "build kernel release on registry build runner and deploy to kernel primary",
            "orchestrator_organ": "organ-forge",
            "domain": "organ-forge",
            "build_target": "cynic-vm-anvil",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-core",
            "deploy_target_role": "kernel_primary",
            "required_node_status": "active",
            "activation_requirements": lifecycle["activation_requirements"],
            "fleet_requirements": lifecycle["fleet_requirements"],
            "requires_e2e": True,
            "targets": lifecycle["repo_targets"],
            "deploy": True,
        }

        with mock.patch.object(
            client,
            "_content_for_intent",
            return_value=invalid_content,
        ), mock.patch.object(client, "_post_agent_task") as post_agent_task:
            with self.assertRaisesRegex(RuntimeError, "content.coord_targets"):
                __import__("asyncio").run(
                    client.dispatch_task_and_stream("deploy-kernel", "").__anext__()
                )

        post_agent_task.assert_not_called()

    def test_lifecycle_health_status_uses_registry_accepted_status(self):
        self.assertTrue(lifecycle_health_status_result(503, registry_fixture())["accepted"])
        self.assertFalse(lifecycle_health_status_result(500, registry_fixture())["accepted"])

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

    def test_kernel_url_cli_uses_shared_normalizer(self):
        with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_kernel_url("http://100.64.0.1:3030/")

        self.assertEqual(status, 0)
        self.assertEqual(stdout.getvalue().strip(), "http://100.64.0.1:3030")

    def test_validate_lifecycle_policy_rejects_unknown_profile_intent(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["request_profiles"]["gate-2"].append(
            "unknown-intent"
        )

        with self.assertRaisesRegex(RuntimeError, "unknown intent unknown-intent"):
            validate_lifecycle_policy(registry)

    def test_activation_evidence_gap_details_are_structured(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]

        details = client._activation_evidence_gap_details()

        self.assertEqual(
            [gap["code"] for gap in details],
            [
                "cynic-vm-anvil:tailscale_env_TAILSCALE_ANVIL_missing",
                "cynic-vm-anvil:service_organ-forge_inactive",
                "cynic-vm-anvil:service_cargo-build_inactive",
                "cynic-vm-anvil:service_gate-2_inactive",
                "cynic-vm-anvil:runtime_path_repo_root_unverified",
                "cynic-vm-core:tailscale_env_TAILSCALE_CORE_missing",
                "cynic-vm-core:service_cynic-kernel_inactive",
                "cynic-vm-core:service_surrealdb_inactive",
                "cynic-vm-core:service_cynic-portal_inactive",
                "cynic-vm-core:runtime_path_repo_root_unverified",
            ],
        )
        self.assertEqual(details[0]["kind"], "network")
        self.assertEqual(details[0]["role"], "build_runner")
        self.assertEqual(details[0]["reason"], "missing_env")
        self.assertEqual(details[0]["env"], "TAILSCALE_ANVIL")
        self.assertEqual(details[1]["kind"], "service")
        self.assertEqual(details[1]["role"], "build_runner")
        self.assertEqual(details[1]["reason"], "missing_service_evidence")
        self.assertEqual(details[1]["requirement"], "organ-forge")
        self.assertEqual(details[4]["kind"], "filesystem")
        self.assertEqual(details[4]["role"], "build_runner")
        self.assertEqual(details[4]["reason"], "missing_runtime_path_proof")
        self.assertEqual(details[4]["path"], "%h/.cynic/repos/CYNIC")
        self.assertEqual(details[4]["proof"]["kind"], "git_worktree")

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

    def test_lifecycle_blockers_combine_activation_status_and_fleet(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.fleet_probe = {
            "status": "ok",
            "missing": ["cynic-vm-anvil", "cynic-vm-core"],
            "matches": {},
        }

        blockers = client._lifecycle_blockers(
            activation_gap_details=client._activation_evidence_gap_details(),
            required_status="active",
        )
        categories = [blocker["category"] for blocker in blockers]

        self.assertEqual(categories.count("activation"), 10)
        self.assertEqual(categories.count("status"), 2)
        self.assertEqual(categories.count("fleet"), 2)
        self.assertIn("cynic-vm-anvil:status_planned_not_active", [b["code"] for b in blockers])
        self.assertIn("cynic-vm-core:fleet_missing", [b["code"] for b in blockers])

    def test_lifecycle_blockers_include_fleet_probe_failure(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.fleet_probe = {"status": "error", "error": "tailscale_status_failed"}

        blockers = client._lifecycle_blockers(
            activation_gap_details=[],
            required_status="active",
        )

        self.assertIn("fleet_probe:error", [blocker["code"] for blocker in blockers])
        fleet_probe = next(blocker for blocker in blockers if blocker["code"] == "fleet_probe:error")
        self.assertEqual(fleet_probe["kind"], "tailscale_probe")
        self.assertEqual(fleet_probe["error"], "tailscale_status_failed")

    def test_lifecycle_error_context_includes_blockers(self):
        diagnostic = mock.Mock()
        diagnostic.lifecycle_context.return_value = {
            "lifecycle_blockers": [{"category": "fleet", "code": "node:fleet_missing"}],
            "activation_evidence_gap_details": [],
            "status_nodes_ready": False,
            "fleet_nodes_ready": False,
            "lifecycle_nodes_ready": False,
        }

        with mock.patch.object(cli, "KernelRestClient", return_value=diagnostic):
            context = cli.lifecycle_error_context()

        self.assertEqual(context["diagnostic_status"], "ok")
        self.assertEqual(context["lifecycle_blockers"][0]["category"], "fleet")
        self.assertFalse(context["lifecycle_nodes_ready"])
        diagnostic.inspect_fleet.assert_called_once_with()

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

    def test_delegate_profile_json_emits_single_aggregate_object(self):
        async def fake_delegate_intent(repo_path, intent, emit_logs=True):
            return {"intent": intent, "status": "ok", "result": f"{intent}:queued"}

        with mock.patch.object(
            cli,
            "lifecycle_profile_intents",
            return_value=["build-and-test", "integration-test"],
        ), mock.patch.object(cli, "delegate_intent", side_effect=fake_delegate_intent), mock.patch(
            "sys.stdout",
            new_callable=StringIO,
        ) as stdout:
            status = __import__("asyncio").run(
                cli.run_delegate_profile("/repo", "gate-2", json_mode=True)
            )

        lines = [line for line in stdout.getvalue().splitlines() if line]
        self.assertEqual(status, 0)
        self.assertEqual(len(lines), 1)
        payload = json.loads(lines[0])
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["profile"], "gate-2")
        self.assertEqual(len(payload["results"]), 2)

    def test_lifecycle_blockers_payload_is_concise(self):
        diagnostic = mock.Mock()
        diagnostic.lifecycle_context.return_value = {
            "lifecycle": "kernel_release",
            "build_target": "cynic-vm-anvil",
            "deploy_target": "cynic-vm-core",
            "status_nodes_ready": False,
            "fleet_nodes_ready": False,
            "lifecycle_nodes_ready": False,
            "lifecycle_blockers": [{"category": "fleet", "code": "node:fleet_missing"}],
        }

        with mock.patch.object(cli, "KernelRestClient", return_value=diagnostic):
            payload = cli.lifecycle_blockers_payload()

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["lifecycle"], "kernel_release")
        self.assertEqual(payload["build_target"], "cynic-vm-anvil")
        self.assertEqual(payload["blockers"][0]["category"], "fleet")
        diagnostic.inspect_fleet.assert_called_once_with()

    def test_lifecycle_activation_plan_is_registry_derived(self):
        plan = build_lifecycle_activation_plan(registry_fixture())

        self.assertEqual(plan["status"], "blocked")
        self.assertEqual(plan["build_target"], "cynic-vm-anvil")
        self.assertEqual(plan["deploy_target"], "cynic-vm-core")
        self.assertEqual(
            [node["name"] for node in plan["nodes"]],
            ["cynic-vm-anvil", "cynic-vm-core"],
        )
        self.assertEqual(
            plan["nodes"][0]["required_services"],
            ["organ-forge", "cargo-build", "gate-2"],
        )
        self.assertEqual(
            plan["nodes"][0]["required_filesystem"],
            {"repo_root": "%h/.cynic/repos/CYNIC"},
        )
        self.assertEqual(
            plan["nodes"][0]["filesystem_proofs"]["repo_root"]["kind"],
            "git_worktree",
        )
        self.assertFalse(plan["local_execution_policy"]["builds_on_operator_machine"])
        self.assertEqual(
            plan["local_execution_policy"]["source"],
            "infra/registry.json:lifecycle.kernel_release",
        )
        self.assertEqual(
            plan["local_execution_policy"]["repo_root"],
            "%h/.cynic/repos/CYNIC",
        )
        self.assertEqual(
            [request["make_target"] for request in plan["next_requests"]],
            ["deploy-request", "gate-2-request"],
        )

    def test_lifecycle_activation_plan_maps_blockers_to_actions(self):
        plan = build_lifecycle_activation_plan(registry_fixture())
        actions = plan["actions"]

        self.assertIn(
            {
                "action": "materialize_runtime_path",
                "node": "cynic-vm-anvil",
                "role": "build_runner",
                "path": "repo_root",
                "value": "%h/.cynic/repos/CYNIC",
                "proof": {
                    "kind": "git_worktree",
                    "marker": ".git",
                    "sentinels": ["AGENTS.md", "infra/registry.json"],
                },
                "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
            },
            actions,
        )
        self.assertIn(
            {
                "action": "provide_placeholder_env_evidence",
                "node": "cynic-vm-anvil",
                "role": "build_runner",
                "env": "TAILSCALE_ANVIL",
                "requirement": "tailscale",
            },
            actions,
        )
        self.assertIn(
            {
                "action": "verify_runtime_path_proof",
                "node": "cynic-vm-anvil",
                "role": "build_runner",
                "path": "repo_root",
                "value": "%h/.cynic/repos/CYNIC",
                "proof": {
                    "kind": "git_worktree",
                    "marker": ".git",
                    "sentinels": ["AGENTS.md", "infra/registry.json"],
                },
                "command": "make lifecycle-verify-runtime-paths node=cynic-vm-anvil",
                "ingestion_command": "make lifecycle-apply-activation-evidence-patch patch='<activation_evidence_patch_json>'",
            },
            actions,
        )
        self.assertIn(
            {
                "action": "record_service_activation_evidence",
                "node": "cynic-vm-anvil",
                "role": "build_runner",
                "service": "organ-forge",
                "contract": "infra/registry.json:lifecycle.kernel_release.activation_evidence_contract",
                "command": "make lifecycle-service-evidence-patch node=cynic-vm-anvil",
                "ingestion_command": "make lifecycle-apply-activation-evidence-patch patch='<activation_evidence_patch_json>'",
            },
            actions,
        )
        self.assertIn(
            {
                "action": "activate_registry_node",
                "node": "cynic-vm-core",
                "role": "kernel_primary",
                "from_status": "planned",
                "to_status": "active",
                "command": "make lifecycle-activate-node node=cynic-vm-core",
            },
            actions,
        )

    def test_lifecycle_activation_requests_are_registry_derived(self):
        payload = build_lifecycle_activation_requests(registry_fixture())

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["build_target"], "cynic-vm-anvil")
        self.assertEqual(payload["deploy_target"], "cynic-vm-core")
        self.assertEqual(len(payload["requests"]), len(payload["actions"]))
        request = payload["requests"][0]
        self.assertEqual(request["kind"], "hermes")
        self.assertEqual(request["domain"], "organ-forge")
        content = json.loads(request["content"])
        self.assertEqual(content["lifecycle"], "kernel_release")
        self.assertEqual(content["build_target"], "cynic-vm-anvil")
        self.assertEqual(content["deploy_target"], "cynic-vm-core")
        self.assertEqual(
            content["targets"],
            ["crates/cynic-kernel", "crates/cynic-mcp", "infra/registry.json", "Makefile"],
        )
        self.assertIn("action", content)
        self.assertIn("objective", content)

    def test_lifecycle_context_exposes_registry_workflow(self):
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry_fixture()
        client.lifecycle = client.registry["lifecycle"]["kernel_release"]
        client.build_node = client.registry["nodes"][0]
        client.deploy_node = client.registry["nodes"][1]
        client.build_target = client.build_node["name"]
        client.deploy_target = client.deploy_node["name"]
        client.base_url = "http://127.0.0.1:3030"
        client.kernel_endpoint_source = "not_resolved"
        client.kernel_bus_canonical = False
        client.kernel_probe = {"enabled": False, "status": "not_run"}
        client.fleet_probe = {"enabled": False, "status": "not_run"}

        workflow = client.lifecycle_context()["workflow"]

        self.assertEqual(workflow["name"], "kernel_release")
        self.assertEqual(len(workflow["canonical_stages"]), 5)
        self.assertEqual(
            workflow["planned_policy"]["critical_targets"],
            [
                "infra/registry.json",
                "Makefile",
                "crates/cynic-kernel",
                "crates/cynic-mcp",
                "organs/forge",
            ],
        )

    def test_lifecycle_activation_requests_dispatch_via_kernel_client(self):
        registry = registry_fixture()
        posted = []

        class FakeClient:
            def __init__(self, *args, **kwargs):
                self.api_key = "token"

            def _post_agent_task(self, payload):
                posted.append(payload)
                return {"id": f"task-{len(posted)}", "status": "queued"}

        payload = dispatch_lifecycle_activation_requests(
            registry=registry,
            inspect_fleet=False,
            kernel_client_factory=FakeClient,
        )

        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["plan_status"], "blocked")
        self.assertEqual(payload["dispatched_count"], len(payload["requests"]))
        self.assertEqual(len(posted), len(payload["requests"]))
        self.assertEqual(payload["responses"][0]["status"], "queued")
        first = json.loads(payload["requests"][0]["content"])
        self.assertEqual(first["targets"], ["crates/cynic-kernel", "crates/cynic-mcp", "infra/registry.json", "Makefile"])

    def test_lifecycle_runtime_paths_are_registry_derived(self):
        payload = build_lifecycle_runtime_paths(registry_fixture())

        self.assertEqual(payload["status"], "blocked")
        self.assertTrue(payload["blocked"])
        self.assertEqual(
            payload["paths"],
            [
                {
                    "node": "cynic-vm-anvil",
                    "role": "build_runner",
                    "path": "repo_root",
                    "value": "%h/.cynic/repos/CYNIC",
                    "proof": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    },
                    "required_status": "active",
                    "current_status": "planned",
                    "materialized_by": "lifecycle_activation",
                    "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
                },
                {
                    "node": "cynic-vm-core",
                    "role": "kernel_primary",
                    "path": "repo_root",
                    "value": "%h/.cynic/repos/CYNIC",
                    "proof": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    },
                    "required_status": "active",
                    "current_status": "planned",
                    "materialized_by": "lifecycle_activation",
                    "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
                },
            ],
        )

    def test_lifecycle_runtime_paths_cli_returns_blocked_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.cli.build_lifecycle_runtime_paths",
            return_value={
                "status": "blocked",
                "lifecycle": "kernel_release",
                "paths": [{"path": "repo_root"}],
                "blocked": True,
                "blockers": [],
            },
        ) as runtime_paths, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_runtime_paths()

        self.assertEqual(status, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["paths"][0]["path"], "repo_root")
        runtime_paths.assert_called_once_with()

    def test_verify_lifecycle_runtime_paths_accepts_materialized_git_worktree(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            (repo_root / ".git").mkdir(parents=True)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            payload = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )

        self.assertEqual(payload["status"], "ok")
        self.assertTrue(payload["checks"][0]["ok"])
        self.assertTrue(payload["checks"][0]["marker_exists"])
        evidence = payload["activation_evidence_patch"]["activation_evidence"]
        self.assertEqual(
            payload["activation_evidence_patch"]["node"],
            "cynic-vm-anvil",
        )
        self.assertTrue(evidence["runtime_paths"]["repo_root"]["verified"])
        self.assertEqual(
            evidence["runtime_paths"]["repo_root"]["value"],
            "%h/.cynic/repos/CYNIC",
        )
        self.assertTrue(evidence["runtime_paths"]["repo_root"]["sentinels_verified"])

    def test_verify_lifecycle_runtime_paths_blocks_missing_sentinel(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            (repo_root / ".git").mkdir(parents=True)
            (repo_root / "AGENTS.md").write_text("# agents\n")

            payload = verify_lifecycle_runtime_paths(
                "cynic-vm-core",
                registry_fixture(),
                home=home,
            )

        self.assertEqual(payload["status"], "blocked")
        self.assertFalse(payload["checks"][0]["ok"])
        missing = [
            item for item in payload["checks"][0]["sentinels"] if not item["exists"]
        ]
        self.assertEqual(missing[0]["path"], "infra/registry.json")
        evidence = payload["activation_evidence_patch"]["activation_evidence"]
        self.assertFalse(evidence["runtime_paths"]["repo_root"]["verified"])
        self.assertFalse(evidence["runtime_paths"]["repo_root"]["sentinels_verified"])

    def test_apply_lifecycle_activation_evidence_patch_merges_runtime_proof(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            (repo_root / ".git").mkdir(parents=True)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            verification = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )

        result = apply_lifecycle_activation_evidence_patch(
            verification["activation_evidence_patch"],
            registry_fixture(),
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["node"], "cynic-vm-anvil")
        self.assertEqual(result["accepted_runtime_paths"], ["repo_root"])
        build_node = next(
            node
            for node in result["registry"]["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["runtime_paths"]["repo_root"]["verified"]
        )

    def test_apply_lifecycle_activation_evidence_patch_merges_service_proof(self):
        patch = {
            "node": "cynic-vm-anvil",
            "activation_evidence": {
                "services": {
                    "organ-forge": {
                        "declared": True,
                        "active": True,
                        "checked_at": "2026-06-15T00:00:00Z",
                    }
                }
            },
        }

        result = apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["accepted_runtime_paths"], [])
        self.assertEqual(result["accepted_services"], ["organ-forge"])
        build_node = next(
            node
            for node in result["registry"]["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["services"]["organ-forge"]["active"]
        )

    def test_persist_lifecycle_activation_evidence_patch_writes_validated_merge(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_fixture()) + "\n")
            patch = {
                "node": "cynic-vm-anvil",
                "activation_evidence": {
                    "services": {
                        "organ-forge": {
                            "declared": True,
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                        }
                    }
                },
            }

            result = persist_lifecycle_activation_evidence_patch(
                patch,
                registry_path=registry_path,
            )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["accepted_services"], ["organ-forge"])
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["services"]["organ-forge"]["active"]
        )

    def test_activate_lifecycle_registry_node_rejects_incomplete_evidence(self):
        with self.assertRaisesRegex(RuntimeError, "activation evidence is incomplete"):
            activate_lifecycle_registry_node("cynic-vm-anvil", registry_fixture())

    def test_activate_lifecycle_registry_node_requires_lifecycle_node(self):
        registry = registry_with_ready_anvil()
        registry["nodes"].append(
            {
                "name": "cynic-vm-gpu",
                "role": "dog_runner",
                "status": "planned",
            }
        )

        with self.assertRaisesRegex(RuntimeError, "not part of lifecycle"):
            activate_lifecycle_registry_node("cynic-vm-gpu", registry)

    def test_activate_lifecycle_registry_node_marks_ready_node_active(self):
        result = activate_lifecycle_registry_node(
            "cynic-vm-anvil",
            registry_with_ready_anvil(),
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["from_status"], "planned")
        self.assertEqual(result["to_status"], "active")
        build_node = next(
            node
            for node in result["registry"]["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertEqual(build_node["status"], "active")
        self.assertEqual(build_node["status_source"], "lifecycle_node_activation")
        self.assertIn("status_updated_at", build_node)

    def test_persist_lifecycle_registry_node_activation_writes_status(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_with_ready_anvil()) + "\n")

            result = persist_lifecycle_registry_node_activation(
                "cynic-vm-anvil",
                registry_path=registry_path,
            )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["to_status"], "active")
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertEqual(build_node["status"], "active")

    def test_lifecycle_service_evidence_patch_is_registry_derived(self):
        payload = build_lifecycle_service_evidence_patch(
            "cynic-vm-anvil",
            registry_fixture(),
        )

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        self.assertEqual(
            [check["service"] for check in payload["checks"]],
            ["organ-forge", "cargo-build", "gate-2"],
        )
        self.assertTrue(all(check["declared"] for check in payload["checks"]))
        self.assertFalse(any(check["active"] for check in payload["checks"]))
        services = payload["activation_evidence_patch"]["activation_evidence"]["services"]
        self.assertEqual(sorted(services), ["cargo-build", "gate-2", "organ-forge"])
        self.assertFalse(services["organ-forge"]["active"])

    def test_lifecycle_service_evidence_patch_accepts_external_service_states(self):
        payload = build_lifecycle_service_evidence_patch(
            "cynic-vm-anvil",
            registry_fixture(),
            service_states={
                "organ-forge": {
                    "active": True,
                    "checked_at": "2026-06-15T00:00:00Z",
                    "source": "unit-test",
                },
                "cargo-build": {
                    "active": True,
                    "checked_at": "2026-06-15T00:00:00Z",
                    "source": "unit-test",
                },
                "gate-2": {
                    "active": True,
                    "checked_at": "2026-06-15T00:00:00Z",
                    "source": "unit-test",
                },
            },
        )

        self.assertEqual(payload["status"], "ok")
        services = payload["activation_evidence_patch"]["activation_evidence"]["services"]
        self.assertTrue(all(item["active"] for item in services.values()))

    def test_lifecycle_service_evidence_patch_cli_returns_blocked_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.cli.build_lifecycle_service_evidence_patch",
            return_value={
                "status": "blocked",
                "node": "cynic-vm-anvil",
                "checks": [{"service": "organ-forge", "active": False}],
                "activation_evidence_patch": {
                    "node": "cynic-vm-anvil",
                    "activation_evidence": {"services": {}},
                },
            },
        ) as service_patch, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_service_evidence_patch("cynic-vm-anvil")

        self.assertEqual(status, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        service_patch.assert_called_once_with("cynic-vm-anvil")

    def test_apply_lifecycle_activation_evidence_patch_rejects_unknown_service(self):
        patch = {
            "node": "cynic-vm-anvil",
            "activation_evidence": {
                "services": {
                    "cynic-kernel": {
                        "declared": True,
                        "active": True,
                        "checked_at": "2026-06-15T00:00:00Z",
                    }
                }
            },
        }

        with self.assertRaisesRegex(RuntimeError, "not required by node role"):
            apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

    def test_apply_lifecycle_activation_evidence_patch_rejects_runtime_path_drift(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            (repo_root / ".git").mkdir(parents=True)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            verification = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )
        patch = verification["activation_evidence_patch"]
        patch["activation_evidence"]["runtime_paths"]["repo_root"]["value"] = "/tmp/CYNIC"

        with self.assertRaisesRegex(RuntimeError, "value does not match registry"):
            apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

    def test_apply_lifecycle_activation_evidence_patch_rejects_incoherent_verified_flag(self):
        verification = verify_lifecycle_runtime_paths(
            "cynic-vm-core",
            registry_fixture(),
            home="/tmp/nonexistent-cynic-home",
        )
        patch = verification["activation_evidence_patch"]
        patch["activation_evidence"]["runtime_paths"]["repo_root"]["verified"] = True

        with self.assertRaisesRegex(RuntimeError, "cannot be verified without marker"):
            apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

    def test_lifecycle_validate_activation_evidence_patch_cli_accepts_valid_patch(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            (repo_root / ".git").mkdir(parents=True)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            verification = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )

        raw_patch = json.dumps(verification["activation_evidence_patch"])
        with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_validate_activation_evidence_patch(raw_patch)

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["accepted_runtime_paths"], ["repo_root"])
        self.assertEqual(payload["accepted_services"], [])

    def test_lifecycle_apply_activation_evidence_patch_cli_persists_patch(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_fixture()) + "\n")
            patch = {
                "node": "cynic-vm-anvil",
                "activation_evidence": {
                    "services": {
                        "organ-forge": {
                            "declared": True,
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                        }
                    }
                },
            }

            with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
                status = cli.run_lifecycle_apply_activation_evidence_patch(
                    json.dumps(patch),
                    registry_path=str(registry_path),
                )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["accepted_services"], ["organ-forge"])
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["services"]["organ-forge"]["active"]
        )

    def test_lifecycle_activate_node_cli_persists_status(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_with_ready_anvil()) + "\n")

            with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
                status = cli.run_lifecycle_activate_node(
                    "cynic-vm-anvil",
                    registry_path=str(registry_path),
                )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertEqual(build_node["status"], "active")

    def test_lifecycle_activation_plan_cli_returns_blocked_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.cli.build_lifecycle_activation_plan",
            return_value={
                "status": "blocked",
                "lifecycle": "kernel_release",
                "actions": [{"action": "activate_registry_node"}],
            },
        ) as activation_plan, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_activation_plan()

        self.assertEqual(status, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["actions"][0]["action"], "activate_registry_node")
        activation_plan.assert_called_once_with(inspect_fleet=True)

    def test_lifecycle_activation_dispatch_cli_returns_ok_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.cli.dispatch_lifecycle_activation_requests",
            return_value={
                "status": "ok",
                "plan_status": "blocked",
                "requests": [{"kind": "hermes"}],
                "responses": [{"id": "task-1"}],
                "dispatched_count": 1,
            },
        ) as dispatch_requests, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_activation_dispatch()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["dispatched_count"], 1)
        dispatch_requests.assert_called_once_with(inspect_fleet=False)

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

    def test_lifecycle_critical_systemd_units_use_registry_repo_root(self):
        unit_paths = [
            PROJECT_ROOT / "organs/forge/systemd/organ-forge-cron.service",
            PROJECT_ROOT / "organs/forge/systemd/organ-forge-hermes-agent.service",
            PROJECT_ROOT / "infra/systemd/cynic-kernel.service",
            PROJECT_ROOT / "infra/systemd/cynic-portal.service",
        ]

        for unit_path in unit_paths:
            source = unit_path.read_text()
            self.assertIn("%h/.cynic/repos/CYNIC", source, str(unit_path))
            self.assertNotIn("/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC", source, str(unit_path))
            self.assertNotIn("/home/user", source, str(unit_path))

    def test_rust_min_stack_is_consistent_across_lifecycle_surfaces(self):
        expected_stack = '17179869184'
        cargo_config = (PROJECT_ROOT / ".cargo/config.toml").read_text()
        makefile = (PROJECT_ROOT / "Makefile").read_text()
        local_tools = (PROJECT_ROOT / "crates/cynic-mcp/src/local_tools.rs").read_text()

        self.assertIn(f'RUST_MIN_STACK = "{expected_stack}"', cargo_config)
        self.assertIn(f"export RUST_MIN_STACK := {expected_stack}", makefile)
        self.assertIn(f"export RUST_MIN_STACK={expected_stack}", makefile)
        self.assertIn(".NOTPARALLEL:", makefile)
        self.assertIn(f'.env("RUST_MIN_STACK", "{expected_stack}")', local_tools)
        self.assertNotIn("2147483648", cargo_config + makefile + local_tools)
        self.assertNotIn("67108864", cargo_config + makefile + local_tools)
        self.assertNotIn("4294967296", cargo_config + makefile + local_tools)
        self.assertNotIn("8589934592", cargo_config + makefile + local_tools)


if __name__ == "__main__":
    unittest.main()
