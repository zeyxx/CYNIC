import json
import unittest
from unittest import mock

from organs.forge.src.adapters.kernel_adapter import (
    KernelRestClient,
    build_lifecycle_activation_plan,
    build_lifecycle_activation_requests,
    dispatch_lifecycle_activation_requests,
    validate_lifecycle_activation_request_payload,
)
from organs.forge.src.core.lifecycle_activation import LIFECYCLE_ACTIVATION_PLAN_SURFACE
from organs.forge.src.core.lifecycle_activation_requests import (
    LIFECYCLE_SURFACE_REGISTRY,
    _activation_request_targets,
)
from organs.forge.src.core.lifecycle_bootstrap import BUILD_RUNNER_BOOTSTRAP_SCHEMA
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleActivationDispatchTests(unittest.TestCase):
    def test_lifecycle_activation_plan_is_registry_derived(self):
        plan = build_lifecycle_activation_plan(registry_fixture())

        self.assertEqual(plan["status"], "blocked")
        self.assertEqual("cynic.lifecycle.activation_summary.v1", plan["surface_contract"]["schema"])
        self.assertEqual(plan["build_target"], "cynic-vm-anvil")
        self.assertEqual(plan["deploy_target"], "cynic-vm-core")
        self.assertEqual(
            [node["name"] for node in plan["nodes"]],
            ["cynic-vm-anvil", "cynic-vm-core", "cynic-lxc-surrealdb"],
        )
        self.assertEqual(
            plan["nodes"][0]["required_services"],
            ["organ-forge", "cargo-build", "gate-2"],
        )
        self.assertEqual(
            plan["nodes"][0]["required_filesystem"],
            {"repo_root": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"},
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
            "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
        )
        self.assertEqual(
            [request["make_target"] for request in plan["next_requests"]],
            ["bootstrap-request", "deploy-request", "gate-2-request", "mcp-reload-request"],
        )
        self.assertEqual(
            plan["activation_requirements"],
            registry_fixture()["lifecycle"]["kernel_release"]["activation_requirements"],
        )
        self.assertEqual(
            plan["fleet_requirements"]["provider"],
            "synapse",
        )
        self.assertEqual(LIFECYCLE_ACTIVATION_PLAN_SURFACE.as_manifest(), plan["surface_contract"])

    def test_lifecycle_activation_plan_maps_blockers_to_actions(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_bootstrap_source",
            return_value={"status": "ok", "reason": "bootstrap_source_ready"},
        ):
            plan = build_lifecycle_activation_plan(registry_fixture())
        actions = plan["actions"]

        materialize = next(
            action
            for action in actions
            if action["action"] == "materialize_runtime_path"
            and action["node"] == "cynic-vm-anvil"
        )
        self.assertEqual(materialize["role"], "build_runner")
        self.assertEqual(materialize["path"], "repo_root")
        self.assertEqual(materialize["value"], "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}")
        self.assertEqual(
            materialize["proof"],
            {
                "kind": "git_worktree",
                "marker": ".git",
                "sentinels": ["AGENTS.md", "infra/registry.json"],
            },
        )
        self.assertEqual(
            materialize["source"],
            "infra/registry.json:lifecycle.kernel_release.runtime_paths",
        )
        bootstrap = materialize["bootstrap_contract"]
        self.assertEqual(bootstrap["schema"], BUILD_RUNNER_BOOTSTRAP_SCHEMA)
        self.assertEqual(bootstrap["node"], "cynic-vm-anvil")
        self.assertEqual(bootstrap["constellation"], "forge")
        self.assertEqual(bootstrap["access"]["preferred_transport"], "fleet_node_exec")
        self.assertEqual(
            bootstrap["access"]["transport_command_env"],
            "CYNIC_FLEET_NODE_EXEC_COMMAND",
        )
        self.assertEqual(
            bootstrap["access"]["transport_command_default"],
            "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
        )
        self.assertEqual(bootstrap["repo"]["path"], "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}")
        self.assertEqual(bootstrap["toolchain"]["rust_toolchain"], "1.88.0")
        self.assertEqual(
            bootstrap["toolchain"]["rust_release_target"],
            "x86_64-unknown-linux-musl",
        )
        self.assertIn("musl-tools", bootstrap["toolchain"]["host_packages"])
        self.assertIn(
            "cargo +1.88.0 build -p cynic-node --release --target x86_64-unknown-linux-musl",
            bootstrap["build"]["commands"],
        )
        self.assertEqual(bootstrap["watchdog"]["mode"], "l0_watchdog")
        self.assertFalse(bootstrap["watchdog"]["requires_dog_backend"])
        self.assertIn("watchdog_observation_received", bootstrap["proofs"])
        self.assertIn(
            {
                "action": "provide_placeholder_env_evidence",
                "node": "cynic-vm-anvil",
                "role": "build_runner",
                "env": "SYNAPSE_ANVIL",
                "requirement": "synapse",
            },
            actions,
        )
        self.assertIn(
            {
                "action": "verify_runtime_path_proof",
                "node": "cynic-vm-anvil",
                "role": "build_runner",
                "path": "repo_root",
                "value": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
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

    def test_lifecycle_activation_plan_maps_fleet_blockers_to_actions(self):
        registry = registry_fixture()
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_bootstrap_source",
            return_value={"status": "ok", "reason": "bootstrap_source_ready"},
        ):
            plan = build_lifecycle_activation_plan(registry)
        actions = plan["actions"]

        self.assertIn(
            {
                "action": "restore_fleet_probe",
                "provider": "synapse",
                "required": "ok",
                "actual": "not_run",
                "error": None,
                "resolver_organ": "organ-fleet",
                "resolver_consumer": "organ-forge-hermes-agent",
            },
            actions,
        )
        self.assertIn(
            {
                "action": "refresh_fleet_inventory",
                "provider": "organ-fleet",
                "required": "ok",
                "actual": "error",
                "error": "organ_fleet_report_failed",
                "resolver_organ": "organ-fleet",
                "resolver_consumer": "organ-forge-hermes-agent",
                "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
            },
            actions,
        )

    def test_lifecycle_activation_plan_maps_network_blockers_to_actions(self):
        registry = registry_fixture()
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "hostname": "cynic-host-cortex",
            "blockers": [
                {
                    "category": "network",
                    "kind": "runtime_blocker",
                    "hostname": "cynic-host-cortex",
                    "severity": "warning",
                    "runtime_status": "provider_fallback_active",
                    "code": "network_runtime:canonical_dns_projection_drift",
                    "blocker": "canonical_dns_projection_drift",
                    "resolver_organ": "cynic-node",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "cargo run -q -p cynic-node -- observe --json",
                },
                {
                    "category": "network",
                    "kind": "actuator_activation",
                    "hostname": "cynic-host-cortex",
                    "actual": "missing_activation",
                    "required": "active",
                    "code": "network_actuator:missing_activation",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "cargo run -q -p cynic-node -- network actuator-status --json",
                    "next_actions": ["install_network_actuator_bundle_on_target_node"],
                    "root_authority": {"sudo_noninteractive_available": False, "command": "sudo -n true"},
                },
                {
                    "category": "network",
                    "kind": "root_authority",
                    "hostname": "cynic-host-cortex",
                    "actual": "sudo_prompt_required",
                    "required": "sudo_noninteractive_available",
                    "code": "network_root_authority:sudo_noninteractive_unavailable",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "sudo -n true",
                },
                {
                    "category": "network",
                    "kind": "remote_exec_transport",
                    "hostname": "cynic-host-cortex",
                    "actual": "mcp_initialize_failed",
                    "required": "ok",
                    "code": "network_remote_exec_transport:mcp_initialize_failed",
                    "resolver_organ": "organ-forge",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                    "transport_probe": {
                        "status": "blocked",
                        "error": "mcp_initialize_failed",
                        "repair_local_debug_binary": True,
                        "canonical_binary": "/repo/target/debug/cynic-mcp",
                        "stale_source": "/repo/crates/cynic-mcp/src/helpers_split/misc.rs",
                    },
                },
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_bootstrap_source",
            return_value={"status": "ok", "reason": "bootstrap_source_ready"},
        ), mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            plan = build_lifecycle_activation_plan(registry, include_network_probe=True)

        self.assertEqual(plan["network_probe"]["status"], "blocked")
        self.assertIn(network_probe["blockers"][0], plan["blockers"])
        self.assertIn(network_probe["blockers"][1], plan["blockers"])
        self.assertIn(
            {
                "action": "restore_canonical_dns_projection",
                "hostname": "cynic-host-cortex",
                "code": "network_runtime:canonical_dns_projection_drift",
                "resolver_organ": "cynic-node",
                "resolver_consumer": "organ-forge-hermes-agent",
                "command": "cargo run -q -p cynic-node -- observe --json",
                "operator_action": "render and activate the canonical Synapse DNS projection, then rerun cynic-node observe",
                "bundle_command": "python3 organs/fleet/src/entrypoints/cli.py dns bundle --registry infra/registry.json --output-dir /tmp/cynic-synapse-dns",
                "activation_status_command": "python3 organs/fleet/src/entrypoints/cli.py dns activation-status --resolve-dns",
                "validation_command": "cargo run -q -p cynic-node -- observe --json",
            },
            plan["actions"],
        )
        self.assertIn(
            {
                "action": "obtain_root_install_channel",
                "hostname": "cynic-host-cortex",
                "code": "network_root_authority:sudo_noninteractive_unavailable",
                "resolver_organ": "organ-fleet",
                "resolver_consumer": "organ-forge-hermes-agent",
                "command": "sudo -n true",
                "required": "sudo_noninteractive_available",
                "actual": "sudo_prompt_required",
                "operator_action": "use the registry-approved root or remote execution channel instead of local sudo-prompted mutation",
                "preferred_transport": "fleet_node_exec",
                "fallback_transport": "parent_provider_exec",
                "transport_command_env": "CYNIC_FLEET_NODE_EXEC_COMMAND",
                "transport_command_default": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                "remote_exec_request": {
                    "tool": "fleet_node_exec",
                    "node": "cynic-host-cortex",
                    "command": "sudo -n true",
                    "timeout_seconds": 30,
                    "purpose": "verify remote noninteractive root channel for network actuator installation",
                    "agent_id": "organ-forge-network-root-channel",
                },
            },
            plan["actions"],
        )
        self.assertIn(
            {
                "action": "refresh_remote_exec_transport",
                "hostname": "cynic-host-cortex",
                "code": "network_remote_exec_transport:mcp_initialize_failed",
                "resolver_organ": "organ-forge",
                "resolver_consumer": "organ-forge-hermes-agent",
                "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                "required": "ok",
                "actual": "mcp_initialize_failed",
                "transport_probe": {
                    "status": "blocked",
                    "error": "mcp_initialize_failed",
                    "repair_local_debug_binary": True,
                    "canonical_binary": "/repo/target/debug/cynic-mcp",
                    "stale_source": "/repo/crates/cynic-mcp/src/helpers_split/misc.rs",
                },
                "operator_action": "refresh the MCP-backed fleet_node_exec transport before attempting remote root installation",
                "mcp_reload_command": "cynic mcp-reload",
                "legacy_mcp_reload_command": "make mcp-reload-request",
                "emergency_repair_command": "make mcp-repair-local",
                "dispatch_profile": "mcp-reload",
                "dispatch_tool": "cynic_lifecycle_dispatch",
                "request_intents": ["build-and-reload-mcp"],
                "request_make_target": "mcp-reload-request",
                "proof_requirements": [
                    "mcp-reload dispatch returns a task id or duplicate pending read-back",
                    "completed task is visible through Kernel agent task read-back",
                    "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py reports fresh",
                    "organ-forge emits an observation for refresh decision and refresh result",
                ],
                "debounce_seconds": 900,
                "repair_local_debug_binary": True,
                "canonical_binary": "/repo/target/debug/cynic-mcp",
                "stale_source": "/repo/crates/cynic-mcp/src/helpers_split/misc.rs",
                "recommended_build_profile": "debug_repair",
                "transport_failure_mode": "mcp_initialize_failed",
                "transport_timeout_seconds": None,
                "response_error_message": None,
                "response_error_code": None,
                "required_bridge": None,
                "transport_reason": None,
                "transport_missing": None,
                "declared_role": None,
                "declared_transport": None,
            },
            plan["actions"],
        )
        network_install = next(
            action for action in plan["actions"] if action["action"] == "install_network_actuator_bundle"
        )
        self.assertEqual(network_install["hostname"], "cynic-host-cortex")
        self.assertEqual(network_install["code"], "network_actuator:missing_activation")
        self.assertEqual(network_install["resolver_organ"], "organ-fleet")
        self.assertEqual(
            network_install["activation_plan_command"],
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator",
        )
        self.assertEqual(
            network_install["install_commands"][0],
            "install -d -o root -g root -m 0755 /usr/local/libexec",
        )
        self.assertIn(
            "cargo run -q -p cynic-node -- observe --json",
            network_install["postflight_commands"],
        )
        self.assertEqual(network_install["preferred_transport"], "fleet_node_exec")
        self.assertEqual(
            network_install["remote_exec_request"],
            {
                "tool": "fleet_node_exec",
                "node": "cynic-host-cortex",
                "command": "install -d -o root -g root -m 0755 /usr/local/libexec && install -d -o root -g root -m 0755 /etc/cynic && install -d -o root -g root -m 0755 /etc/sudoers.d && install -o root -g root -m 0755 /tmp/cynic-network-actuator/cynic-network-actuator /usr/local/libexec/cynic-network-actuator && install -o root -g root -m 0644 /tmp/cynic-network-actuator/network-actuator.policy.json /etc/cynic/network-actuator.policy.json && install -o root -g root -m 0440 /tmp/cynic-network-actuator/cynic-network-actuator.sudoers /etc/sudoers.d/cynic-network-actuator && visudo -cf /etc/sudoers.d/cynic-network-actuator",
                "timeout_seconds": 120,
                "purpose": "install network actuator bundle through registry-approved remote execution",
                "agent_id": "organ-forge-network-root-channel",
            },
        )
        self.assertEqual(
            [step["action"] for step in plan["root_install_sequence"]],
            ["install_network_actuator_bundle"],
        )
        self.assertEqual(
            plan["root_install_sequence"][0]["dependency"],
            "requires_host_exec_transport",
        )

    def test_lifecycle_activation_plan_surfaces_bootstrap_source_blocker(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_bootstrap_source",
            return_value={
                "status": "blocked",
                "reason": "bootstrap_source_contract_stale",
                "selected_remote": "http://localhost:18300/jeanterre13/CYNIC.git",
                "selected_ref": "main",
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_consumer_runtime",
            return_value={"status": "ok"},
        ):
            plan = build_lifecycle_activation_plan(registry_fixture())

        self.assertEqual(plan["status"], "blocked")
        self.assertEqual(
            plan["bootstrap_source"]["reason"],
            "bootstrap_source_contract_stale",
        )
        self.assertIn(
            {
                "category": "bootstrap_source",
                "code": "bootstrap_source_contract_stale",
                "detail": {
                    "selected_remote": "http://localhost:18300/jeanterre13/CYNIC.git",
                    "selected_ref": "main",
                },
            },
            plan["blockers"],
        )
        self.assertIn(
            {
                "action": "inspect_bootstrap_source",
                "code": "bootstrap_source_contract_stale",
                "selected_remote": "http://localhost:18300/jeanterre13/CYNIC.git",
                "selected_ref": "main",
                "command": "python3 organs/forge/src/entrypoints/cli.py lifecycle-bootstrap-source-check",
                "repair_goal": "publish a bootstrap source ref whose lifecycle contract matches current DNA",
            },
            plan["actions"],
        )

    def test_lifecycle_activation_plan_maps_missing_fallback_env_to_runtime_action(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_bootstrap_source",
            return_value={
                "status": "blocked",
                "reason": "bootstrap_source_fallback_env_missing",
                "selected_remote": None,
                "selected_ref": "main",
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_consumer_runtime",
            return_value={"status": "ok"},
        ):
            plan = build_lifecycle_activation_plan(registry_fixture())

        self.assertIn(
            {
                "action": "inspect_bootstrap_source",
                "code": "bootstrap_source_fallback_env_missing",
                "selected_remote": None,
                "selected_ref": "main",
                "command": "python3 organs/forge/src/entrypoints/cli.py lifecycle-bootstrap-source-check",
                "repair_goal": "inject the fallback Forgejo bootstrap remote into the organ-forge runtime environment",
                "operator_action": "set CYNIC_FORGEJO_BOOTSTRAP_REMOTE from `git remote get-url forgejo`, regenerate ~/.config/cynic/env, then restart organ-forge-hermes-agent.service",
            },
            plan["actions"],
        )

    def test_lifecycle_activation_plan_surfaces_runtime_consumer_blocker(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_bootstrap_source",
            return_value={"status": "ok", "reason": "bootstrap_source_ready"},
        ), mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_consumer_runtime",
            return_value={
                "status": "blocked",
                "runtime_user": "cynic",
                "runtime_user_active": False,
                "cargo_available": False,
                "problems": ["runtime_user_not_active", "cargo_missing"],
                "operator_action": "restore runtime consumer",
            },
        ):
            plan = build_lifecycle_activation_plan(registry_fixture())

        self.assertEqual(plan["runtime_probe"]["problems"], ["runtime_user_not_active", "cargo_missing"])
        self.assertNotIn("repair_runtime_consumer", {action["action"] for action in plan["actions"]})

    def test_lifecycle_activation_requests_are_registry_derived(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value={"enabled": True, "status": "ok", "blockers": []},
        ):
            payload = build_lifecycle_activation_requests(registry_fixture())

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["activation_schema_version"], "activation.v1")
        self.assertEqual(payload["build_target"], "cynic-vm-anvil")
        self.assertEqual(payload["deploy_target"], "cynic-vm-core")
        self.assertEqual(len(payload["requests"]), len(payload["actions"]))
        self.assertEqual(len(payload["request_summary"]), len(payload["requests"]))
        request = payload["requests"][0]
        self.assertEqual(request["kind"], "hermes")
        self.assertEqual(request["domain"], "organ-forge")
        self.assertEqual(request["consumer"], "organ-forge-hermes-agent")
        self.assertEqual(request["priority"], 99)
        content = json.loads(request["content"])
        self.assertEqual(content["lifecycle"], "kernel_release")
        self.assertEqual(content["activation_schema_version"], "activation.v1")
        self.assertEqual(content["build_target"], "cynic-vm-anvil")
        self.assertEqual(content["deploy_target"], "cynic-vm-core")
        self.assertEqual(content["action"]["action"], "provide_placeholder_env_evidence")
        self.assertEqual(
            content["targets"],
            ["infra/registry.json"],
        )
        self.assertEqual(content["coord_targets"], ["infra/registry.json"])
        self.assertIn("action", content)
        self.assertIn("objective", content)
        self.assertRegex(content["correlation_id"], r"^activation:[0-9a-f]{16}$")
        proposal = content["operator_interface_proposal"]
        self.assertEqual(proposal["schema"], "cynic.agent_proposal.v1")
        self.assertEqual(proposal["visibility"], "operator_private")
        self.assertEqual(proposal["operator_review"]["state"], "pending")
        self.assertFalse(proposal["public_projection"]["eligible"])
        self.assertEqual(proposal["public_projection"]["redaction_status"], "needs_review")
        self.assertEqual(proposal["targets"], content["targets"])
        self.assertEqual(proposal["coord_targets"], content["coord_targets"])
        first_summary = payload["request_summary"][0]
        self.assertRegex(first_summary["correlation_id"], r"^activation:[0-9a-f]{16}$")
        self.assertEqual(first_summary["correlation_id"], content["correlation_id"])
        self.assertEqual(first_summary["action"], "provide_placeholder_env_evidence")
        self.assertEqual(first_summary["node"], "cynic-vm-anvil")
        self.assertEqual(first_summary["role"], "build_runner")
        self.assertIsNone(first_summary["code"])
        self.assertEqual(first_summary["targets"], ["infra/registry.json"])
        self.assertEqual(first_summary["coord_targets"], ["infra/registry.json"])
        self.assertEqual(first_summary["consumer"], "organ-forge-hermes-agent")
        self.assertEqual(first_summary["priority"], 99)

        targets_by_action = {
            json.loads(item["content"])["action"]["action"]: json.loads(item["content"])[
                "targets"
            ]
            for item in payload["requests"]
        }
        priorities_by_action = {
            json.loads(item["content"])["action"]["action"]: item["priority"]
            for item in payload["requests"]
        }
        self.assertEqual(priorities_by_action["restore_fleet_probe"], 98)
        self.assertEqual(
            targets_by_action["restore_fleet_probe"],
            ["organs/fleet"],
        )

    def test_lifecycle_activation_requests_route_network_actions_to_network_surfaces(self):
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "runtime_blocker",
                    "hostname": "cynic-host-cortex",
                    "severity": "warning",
                    "runtime_status": "provider_fallback_active",
                    "code": "network_runtime:wireguard_mesh_absent",
                    "blocker": "wireguard_mesh_absent",
                    "resolver_organ": "cynic-node",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "cargo run -q -p cynic-node -- observe --json",
                }
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = build_lifecycle_activation_requests(
                registry_fixture(),
                include_network_probe=True,
            )

        request = next(
            item
            for item in payload["requests"]
            if json.loads(item["content"])["action"]["action"] == "activate_wireguard_mesh"
        )
        content = json.loads(request["content"])
        summary = next(
            item
            for item in payload["request_summary"]
            if item["action"] == "activate_wireguard_mesh"
        )
        self.assertEqual(request["priority"], 97)
        self.assertEqual(
            content["targets"],
            ["organs/fleet", "crates/cynic-node"],
        )
        self.assertEqual(content["coord_targets"], content["targets"])
        self.assertEqual(summary["hostname"], "cynic-host-cortex")
        self.assertEqual(summary["resolver_organ"], "cynic-node")
        self.assertEqual(summary["resolver_consumer"], "organ-forge-hermes-agent")
        self.assertEqual(
            summary["activation_plan_command"],
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator",
        )
        self.assertEqual(
            summary["bundle_command"],
            "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator",
        )
        self.assertEqual(
            summary["validation_command"],
            "cargo run -q -p cynic-node -- network repair --dry-run",
        )
        self.assertEqual(summary["preferred_transport"], "fleet_node_exec")
        self.assertEqual(summary["transport_command_env"], "CYNIC_FLEET_NODE_EXEC_COMMAND")

    def test_lifecycle_activation_requests_route_runtime_observation_to_fleet_and_node(self):
        action = {
            "action": "provide_runtime_observation",
            "provider": "synapse",
            "code": "fleet_inventory_evidence_gap",
        }
        scoped = _activation_request_targets(action)

        self.assertEqual(scoped["targets"], ["organs/fleet", "crates/cynic-node"])
        self.assertEqual(scoped["coord_targets"], ["organs/fleet", "crates/cynic-node"])

    def test_lifecycle_activation_requests_skip_verified_runtime_paths(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        build_role = lifecycle["build_runner_role"]
        deploy_role = lifecycle["deploy_target_role"]
        for node in registry["nodes"]:
            if node.get("role") in {build_role, deploy_role}:
                node.setdefault("activation_evidence", {}).setdefault(
                    "runtime_paths",
                    {},
                )["repo_root"] = {"verified": True}

        payload = build_lifecycle_activation_requests(registry)

        self.assertNotIn(
            "materialize_runtime_path",
            {action["action"] for action in payload["actions"]},
        )

    def test_validate_lifecycle_activation_request_payload_rejects_fake_global_scope(self):
        registry = registry_fixture()
        payload = build_lifecycle_activation_requests(registry)["requests"][0]
        content = json.loads(payload["content"])
        content["targets"] = registry["lifecycle"]["kernel_release"]["repo_targets"]
        content["coord_targets"] = registry["lifecycle"]["kernel_release"]["repo_targets"]
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "activation action scope"):
            validate_lifecycle_activation_request_payload(payload, registry)

    def test_validate_lifecycle_activation_request_payload_rejects_missing_consumer(self):
        registry = registry_fixture()
        payload = build_lifecycle_activation_requests(registry)["requests"][0]
        del payload["consumer"]

        with self.assertRaisesRegex(RuntimeError, "payload.consumer"):
            validate_lifecycle_activation_request_payload(payload, registry)

    def test_validate_lifecycle_activation_request_payload_rejects_missing_operator_proposal(self):
        registry = registry_fixture()
        payload = build_lifecycle_activation_requests(registry)["requests"][0]
        content = json.loads(payload["content"])
        del content["operator_interface_proposal"]
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "operator_interface_proposal"):
            validate_lifecycle_activation_request_payload(payload, registry)

    def test_validate_lifecycle_activation_request_payload_rejects_priority_drift(self):
        registry = registry_fixture()
        payload = build_lifecycle_activation_requests(registry)["requests"][0]
        payload["priority"] = 1

        with self.assertRaisesRegex(RuntimeError, "payload.priority"):
            validate_lifecycle_activation_request_payload(payload, registry)

    def test_validate_lifecycle_activation_request_payload_rejects_correlation_drift(self):
        registry = registry_fixture()
        payload = build_lifecycle_activation_requests(registry)["requests"][0]
        content = json.loads(payload["content"])
        content["correlation_id"] = "activation:0000000000000000"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.correlation_id"):
            validate_lifecycle_activation_request_payload(payload, registry)

    def test_validate_lifecycle_activation_request_payload_rejects_schema_drift(self):
        registry = registry_fixture()
        payload = build_lifecycle_activation_requests(registry)["requests"][0]
        content = json.loads(payload["content"])
        content["activation_schema_version"] = "activation.v0"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.activation_schema_version"):
            validate_lifecycle_activation_request_payload(payload, registry)

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
        workflow_state = client.lifecycle_context()["workflow_state"]

        self.assertEqual(workflow["name"], "kernel_release")
        self.assertEqual(workflow_state["declared"]["required_node_status"], "active")
        self.assertEqual(
            workflow_state["observed"]["activation_evidence_gaps"],
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
        self.assertFalse(workflow_state["effective"]["lifecycle_nodes_ready"])
        self.assertEqual(len(workflow["canonical_stages"]), 7)
        self.assertEqual(
            workflow["planned_policy"]["critical_targets"],
            [
                "infra/registry.json",
                "Makefile",
                "crates/cynic-kernel",
                "crates/cynic-mcp",
                "organs/forge",
                "organs/forge/src/adapters/kernel_adapter.py",
                "organs/forge/src/core/lifecycle_workflow.py",
                "organs/forge/src/core/lifecycle_payloads.py",
                "organs/forge/src/core/lifecycle_runtime.py",
                "organs/forge/src/core/lifecycle_evidence.py",
                "organs/forge/src/core/lifecycle_activation.py",
                "organs/forge/src/core/lifecycle_readiness.py",
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
                return {"task_id": f"task-{len(posted)}", "status": "queued"}

            def _list_agent_tasks(self, **kwargs):
                return {"tasks": [], "count": 0}

            def _assert_agent_task_visible(self, task_id, *, kind, domain):
                self.visible = getattr(self, "visible", [])
                self.visible.append(
                    {"task_id": task_id, "kind": kind, "domain": domain}
                )

        payload = dispatch_lifecycle_activation_requests(
            registry=registry,
            inspect_fleet=False,
            kernel_client_factory=FakeClient,
        )

        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["activation_schema_version"], "activation.v1")
        self.assertEqual(payload["plan_status"], "blocked")
        self.assertEqual("cynic.lifecycle.activation_dispatch.v1", payload["surface_contract"]["schema"])
        self.assertEqual(payload["dispatched_count"], len(payload["requests"]))
        self.assertEqual(len(posted), len(payload["requests"]))
        self.assertEqual(payload["responses"][0]["status"], "queued")
        self.assertEqual(payload["dispatched_tasks"][0]["task_id"], "task-1")
        self.assertEqual(payload["dispatched_tasks"][0]["kind"], "hermes")
        self.assertEqual(payload["dispatched_tasks"][0]["domain"], "organ-forge")
        self.assertEqual(
            payload["dispatched_tasks"][0]["consumer"],
            "organ-forge-hermes-agent",
        )
        self.assertEqual(payload["dispatched_tasks"][0]["priority"], 99)
        self.assertRegex(
            payload["dispatched_tasks"][0]["correlation_id"],
            r"^activation:[0-9a-f]{16}$",
        )
        self.assertEqual(
            payload["dispatched_tasks"][0]["correlation_id"],
            json.loads(posted[0]["content"])["correlation_id"],
        )
        self.assertEqual(
            payload["dispatched_tasks"][0]["targets"],
            ["infra/registry.json"],
        )
        self.assertEqual(posted[0]["consumer"], "organ-forge-hermes-agent")
        self.assertEqual(posted[0]["priority"], 99)
        self.assertEqual(payload["dispatch_summary"]["requested"], len(payload["requests"]))
        self.assertEqual(payload["dispatch_summary"]["dispatched"], len(payload["requests"]))
        self.assertEqual(payload["dispatch_summary"]["skipped_existing"], 0)
        self.assertEqual(payload["dispatch_summary"]["consumer"], "organ-forge-hermes-agent")
        self.assertEqual(payload["dispatch_summary"]["task_ids"][0], "task-1")
        self.assertEqual(
            len(payload["dispatch_summary"]["correlation_ids"]),
            len(payload["requests"]),
        )
        self.assertRegex(
            payload["dispatch_summary"]["correlation_ids"][0],
            r"^activation:[0-9a-f]{16}$",
        )
        first = json.loads(payload["requests"][0]["content"])
        self.assertEqual(first["targets"], ["infra/registry.json"])
        self.assertEqual("cynic.lifecycle.activation_requests.v1", payload["requests"][0]["surface_contract"]["schema"])
        self.assertEqual("cynic.surface_registry.v1", LIFECYCLE_SURFACE_REGISTRY.manifest()["schema"])
        self.assertEqual(3, LIFECYCLE_SURFACE_REGISTRY.manifest()["report_count"])

    def test_lifecycle_activation_dispatch_carries_network_action_details(self):
        registry = registry_fixture()
        posted = []
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "runtime_blocker",
                    "hostname": "cynic-host-cortex",
                    "severity": "warning",
                    "runtime_status": "provider_fallback_active",
                    "code": "network_runtime:canonical_dns_projection_drift",
                    "blocker": "canonical_dns_projection_drift",
                    "resolver_organ": "cynic-node",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "cargo run -q -p cynic-node -- observe --json",
                }
            ],
        }

        class FakeClient:
            def __init__(self, *args, **kwargs):
                self.api_key = "token"

            def _list_agent_tasks(self, **kwargs):
                return {"tasks": [], "count": 0}

            def _post_agent_task(self, payload):
                posted.append(payload)
                return {"task_id": f"task-{len(posted)}", "status": "queued"}

            def _assert_agent_task_visible(self, task_id, *, kind, domain):
                return None

        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = dispatch_lifecycle_activation_requests(
                registry=registry,
                inspect_fleet=False,
                include_network_probe=True,
                kernel_client_factory=FakeClient,
            )

        dispatched = next(
            item
            for item in payload["dispatched_tasks"]
            if item["action"] == "restore_canonical_dns_projection"
        )
        self.assertEqual(dispatched["hostname"], "cynic-host-cortex")
        self.assertEqual(dispatched["resolver_organ"], "cynic-node")
        self.assertEqual(dispatched["resolver_consumer"], "organ-forge-hermes-agent")
        self.assertEqual(dispatched["command"], "cargo run -q -p cynic-node -- observe --json")
        self.assertEqual(
            dispatched["bundle_command"],
            "python3 organs/fleet/src/entrypoints/cli.py dns bundle --registry infra/registry.json --output-dir /tmp/cynic-synapse-dns",
        )
        self.assertEqual(
            dispatched["activation_status_command"],
            "python3 organs/fleet/src/entrypoints/cli.py dns activation-status --resolve-dns",
        )
        self.assertEqual(
            dispatched["validation_command"],
            "cargo run -q -p cynic-node -- observe --json",
        )

    def test_lifecycle_activation_requests_expose_remote_exec_contract_for_root_actions(self):
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "actuator_activation",
                    "hostname": "cynic-host-cortex",
                    "actual": "missing_activation",
                    "required": "active",
                    "code": "network_actuator:missing_activation",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "cargo run -q -p cynic-node -- network actuator-status --json",
                    "next_actions": ["install_network_actuator_bundle_on_target_node"],
                    "root_authority": {"sudo_noninteractive_available": False, "command": "sudo -n true"},
                },
                {
                    "category": "network",
                    "kind": "root_authority",
                    "hostname": "cynic-host-cortex",
                    "actual": "sudo_prompt_required",
                    "required": "sudo_noninteractive_available",
                    "code": "network_root_authority:sudo_noninteractive_unavailable",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "sudo -n true",
                },
                {
                    "category": "network",
                    "kind": "remote_exec_transport",
                    "hostname": "cynic-host-cortex",
                    "actual": "mcp_initialize_failed",
                    "required": "ok",
                    "code": "network_remote_exec_transport:mcp_initialize_failed",
                    "resolver_organ": "organ-forge",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                    "transport_probe": {
                        "status": "blocked",
                        "error": "mcp_initialize_failed",
                        "repair_local_debug_binary": True,
                        "canonical_binary": "/repo/target/debug/cynic-mcp",
                        "stale_source": "/repo/crates/cynic-mcp/src/helpers_split/misc.rs",
                    },
                },
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = build_lifecycle_activation_requests(
                registry_fixture(),
                include_network_probe=True,
            )

        install_summary = next(
            item for item in payload["request_summary"] if item["action"] == "install_network_actuator_bundle"
        )
        self.assertEqual(install_summary["preferred_transport"], "fleet_node_exec")
        self.assertEqual(
            install_summary["transport_command_default"],
            "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
        )
        self.assertEqual(
            install_summary["install_commands"][0],
            "install -d -o root -g root -m 0755 /usr/local/libexec",
        )
        self.assertIn(
            "cargo run -q -p cynic-node -- observe --json",
            install_summary["postflight_commands"],
        )
        self.assertEqual(install_summary["remote_exec_request"]["node"], "cynic-host-cortex")
        self.assertIn("visudo -cf", install_summary["remote_exec_request"]["command"])

        channel_summary = next(
            item for item in payload["request_summary"] if item["action"] == "obtain_root_install_channel"
        )
        self.assertEqual(channel_summary["remote_exec_request"]["command"], "sudo -n true")

        transport_summary = next(
            item for item in payload["request_summary"] if item["action"] == "refresh_remote_exec_transport"
        )
        self.assertEqual(transport_summary["dispatch_profile"], "mcp-reload")
        self.assertEqual(transport_summary["dispatch_tool"], "cynic_lifecycle_dispatch")
        self.assertEqual(transport_summary["request_intents"], ["build-and-reload-mcp"])
        self.assertEqual(transport_summary["request_make_target"], "mcp-reload-request")
        self.assertEqual(transport_summary["mcp_reload_command"], "cynic mcp-reload")
        self.assertTrue(transport_summary["repair_local_debug_binary"])
        self.assertEqual(transport_summary["recommended_build_profile"], "debug_repair")
        self.assertEqual(transport_summary["canonical_binary"], "/repo/target/debug/cynic-mcp")
        self.assertEqual(transport_summary["transport_failure_mode"], "mcp_initialize_failed")
        self.assertEqual(
            transport_summary["proof_requirements"][0],
            "mcp-reload dispatch returns a task id or duplicate pending read-back",
        )

    def test_lifecycle_activation_requests_surface_registry_contract_error_for_transport(self):
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "remote_exec_transport",
                    "hostname": "cynic-host-cortex",
                    "actual": "mcp_tool_error",
                    "required": "ok",
                    "code": "network_remote_exec_transport:mcp_tool_error",
                    "resolver_organ": "organ-forge",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                    "transport_probe": {
                        "status": "blocked",
                        "error": "mcp_tool_error",
                        "response_error_code": -32602,
                        "response_error_message": "registry node has no Proxmox resource id: cynic-host-cortex",
                    },
                },
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = build_lifecycle_activation_requests(
                registry_fixture(),
                include_network_probe=True,
            )

        transport_summary = next(
            item for item in payload["request_summary"] if item["action"] == "refresh_remote_exec_transport"
        )
        self.assertEqual(transport_summary["transport_failure_mode"], "mcp_tool_error")
        self.assertEqual(transport_summary["response_error_code"], -32602)
        self.assertEqual(
            transport_summary["response_error_message"],
            "registry node has no Proxmox resource id: cynic-host-cortex",
        )
        self.assertIn("registry node has no Proxmox resource id", transport_summary["operator_action"])

    def test_lifecycle_activation_requests_surface_host_exec_transport_gap(self):
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "remote_exec_transport",
                    "hostname": "cynic-host-cortex",
                    "actual": "mcp_tool_non_object_payload",
                    "required": "ok",
                    "code": "network_remote_exec_transport:mcp_tool_non_object_payload",
                    "resolver_organ": "organ-forge",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                    "transport_probe": {
                        "status": "precondition_missing",
                        "error": "mcp_tool_non_object_payload",
                        "response_error_message": "declare a canonical host exec transport for cortex/manual hosts before using fleet_node_exec on this node",
                    },
                },
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = build_lifecycle_activation_requests(
                registry_fixture(),
                include_network_probe=True,
            )

        transport_summary = next(
            item for item in payload["request_summary"] if item["action"] == "refresh_remote_exec_transport"
        )
        self.assertEqual(
            transport_summary["response_error_message"],
            "declare a canonical host exec transport for cortex/manual hosts before using fleet_node_exec on this node",
        )
        self.assertEqual(
            transport_summary["bundle_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec bundle --output-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            transport_summary["install_plan_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec install-plan",
        )
        self.assertEqual(
            transport_summary["activation_plan_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-plan --bundle-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            transport_summary["activation_status_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            transport_summary["install_commands"][0],
            "install -d -o root -g root -m 0755 /usr/local/libexec",
        )
        self.assertIn(
            "visudo -cf /etc/sudoers.d/cynic-host-exec",
            transport_summary["manual_root_install_command"],
        )
        self.assertIn("canonical host-exec helper bundle", transport_summary["operator_action"])

    def test_lifecycle_activation_requests_surface_host_exec_helper_install_for_transport_failed(self):
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "remote_exec_transport",
                    "hostname": "cynic-host-cortex",
                    "actual": "transport_failed",
                    "required": "ok",
                    "code": "network_remote_exec_transport:transport_failed",
                    "resolver_organ": "organ-forge",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                    "transport_probe": {
                        "status": "transport_failed",
                        "declared_transport": {
                            "kind": "ssh_sudo_helper",
                            "transport_command_default": "sudo -n /usr/local/libexec/cynic-host-exec",
                        },
                    },
                },
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = build_lifecycle_activation_requests(
                registry_fixture(),
                include_network_probe=True,
            )

        transport_summary = next(
            item for item in payload["request_summary"] if item["action"] == "refresh_remote_exec_transport"
        )
        self.assertEqual(
            transport_summary["bundle_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec bundle --output-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            transport_summary["activation_plan_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-plan --bundle-dir /tmp/cynic-host-exec",
        )
        self.assertIn("host-exec helper bundle", transport_summary["operator_action"])

    def test_lifecycle_activation_requests_surface_host_exec_activation_blocker(self):
        network_probe = {
            "enabled": True,
            "status": "blocked",
            "blockers": [
                {
                    "category": "network",
                    "kind": "host_exec_activation",
                    "hostname": "cynic-host-cortex",
                    "actual": "missing_activation",
                    "required": "active",
                    "code": "network_host_exec:missing_activation",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec",
                    "missing": ["helper_exists", "sudoers_exists"],
                    "helper_path": "/usr/local/libexec/cynic-host-exec",
                    "sudoers_rule": "cynic ALL=(root) NOPASSWD: /usr/local/libexec/cynic-host-exec",
                    "bundle_complete": False,
                    "next_actions": ["build_host_exec_bundle", "install_host_exec_bundle_on_target_node"],
                    "proof_requirements": [{"check": "bundle_complete"}],
                },
            ],
        }
        with mock.patch(
            "organs.forge.src.core.lifecycle_activation.probe_local_network_runtime",
            return_value=network_probe,
        ):
            payload = build_lifecycle_activation_requests(
                registry_fixture(),
                include_network_probe=True,
            )

        summary = next(
            item for item in payload["request_summary"] if item["action"] == "install_host_exec_bundle"
        )
        self.assertEqual(
            summary["bundle_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec bundle --output-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            summary["activation_plan_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-plan --bundle-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            summary["activation_status_command"],
            "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec",
        )
        self.assertEqual(
            summary["install_commands"][0],
            "install -d -o root -g root -m 0755 /usr/local/libexec",
        )
        self.assertIn(
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers",
            summary["postflight_commands"],
        )
        self.assertEqual(summary["helper_path"], "/usr/local/libexec/cynic-host-exec")
        self.assertIn("sudoers_exists", summary["missing"])
        self.assertFalse(summary["bundle_complete"])
        self.assertIn("build_host_exec_bundle", summary["next_actions"])
        self.assertEqual(
            [step["action"] for step in payload["root_install_sequence"]],
            ["install_host_exec_bundle"],
        )
        self.assertEqual(
            payload["root_install_sequence"][0]["manual_root_install_command"],
            summary["manual_root_install_command"],
        )

    def test_lifecycle_activation_dispatch_requires_read_back_visibility(self):
        registry = registry_fixture()
        posted = []

        class FakeClient:
            def __init__(self, *args, **kwargs):
                self.api_key = "token"

            def _post_agent_task(self, payload):
                posted.append(payload)
                return {"task_id": "task-invisible", "status": "queued"}

            def _list_agent_tasks(self, **kwargs):
                return {"tasks": [], "count": 0}

            def _assert_agent_task_visible(self, task_id, *, kind, domain):
                raise RuntimeError(
                    f"agent task dispatch read-back incomplete: task_id={task_id}"
                )

        with self.assertRaisesRegex(RuntimeError, "read-back incomplete"):
            dispatch_lifecycle_activation_requests(
                registry=registry,
                inspect_fleet=False,
                kernel_client_factory=FakeClient,
            )

        self.assertEqual(len(posted), 1)

    def test_lifecycle_activation_dispatch_skips_existing_pending_duplicate(self):
        registry = registry_fixture()
        first_request = build_lifecycle_activation_requests(registry)["requests"][0]
        posted = []

        class FakeClient:
            def __init__(self, *args, **kwargs):
                self.api_key = "token"

            def _list_agent_tasks(self, **kwargs):
                return {
                    "tasks": [
                        {
                            "id": "agent_tasks:existing",
                            "content": first_request["content"],
                        }
                    ],
                    "count": 1,
                }

            def _post_agent_task(self, payload):
                posted.append(payload)
                return {"task_id": f"task-{len(posted)}", "status": "queued"}

            def _assert_agent_task_visible(self, task_id, *, kind, domain):
                return None

        payload = dispatch_lifecycle_activation_requests(
            registry=registry,
            inspect_fleet=False,
            kernel_client_factory=FakeClient,
        )

        self.assertEqual(payload["skipped_existing_count"], 1)
        self.assertEqual(payload["dispatch_summary"]["skipped_existing"], 1)
        self.assertEqual(
            payload["skipped_existing"][0]["task_id"],
            "agent_tasks:existing",
        )
        self.assertRegex(
            payload["skipped_existing"][0]["correlation_id"],
            r"^activation:[0-9a-f]{16}$",
        )
        self.assertEqual(len(posted), len(payload["requests"]) - 1)
