import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path, resolve_project_root
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path, resolve_project_root


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"
REPO_ROOT = resolve_project_root(MODULE_PATH)


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


class ContractAndRuntimePrincipalTopologyTests(unittest.TestCase):
    def test_topology_validator_wrapper_bootstraps_project_root(self):
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
        self.assertIn("Validate CYNIC repo topology contract.", result.stdout)


    def test_validate_pipeline_includes_multiscale_property_map_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_multiscale_organism_property_map_contract
        sentinel = module.Finding(
            "critical",
            "multiscale_organism_property_map_pipeline_sentinel",
            "axioms/multiscale-organism-property-map-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_multiscale_organism_property_map_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_multiscale_organism_property_map_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_storage_primary_activation_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_storage_primary_activation_contract
        sentinel = module.Finding(
            "critical",
            "storage_primary_activation_pipeline_sentinel",
            "axioms/storage-primary-activation-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_storage_primary_activation_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_storage_primary_activation_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_infrastructure_mcp_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_infrastructure_mcp_contract
        sentinel = module.Finding(
            "critical",
            "infrastructure_mcp_pipeline_sentinel",
            "axioms/infrastructure-mcp-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_infrastructure_mcp_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_infrastructure_mcp_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_python_vital_function_placement_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.validate_python_vital_function_placement_contract
        sentinel = module.Finding(
            "critical",
            "python_vital_function_placement_pipeline_sentinel",
            "axioms/python-vital-function-placement-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.validate_python_vital_function_placement_contract = fake_validate
            findings = module.validate()
        finally:
            policy.validate_python_vital_function_placement_contract = original

        self.assertIn(sentinel, findings)

    def test_validate_pipeline_includes_node_network_interfaces_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.validate_node_network_interfaces_contract
        sentinel = module.Finding(
            "critical",
            "node_network_interfaces_pipeline_sentinel",
            "axioms/node-network-interfaces-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.validate_node_network_interfaces_contract = fake_validate
            findings = module.validate()
        finally:
            policy.validate_node_network_interfaces_contract = original

        self.assertIn(sentinel, findings)

    def test_infrastructure_mcp_rejects_storage_readiness_dependency_drift(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "axioms" / "infrastructure-mcp-v1.json"
            contract_path.parent.mkdir(parents=True)
            contract = {
                "schema": "cynic.infrastructure_mcp.v1",
                "status": "target_contract",
                "principles": [
                    "deny_by_default",
                    "registry_desired_state_first",
                    "synapse_dns_is_the_service_contract",
                    "mutating_operations_require_preflight_and_audit",
                    "private_keys_never_leave_the_node",
                ],
                "operation_families": {
                    "ipam": {
                        "operations": {
                            "fleet.node.exec": {
                                "mcp_tool": "fleet_node_exec",
                                "mutates": True,
                                "lxc_bridge_contract": "axioms/lxc-exec-bridge-v1.json",
                                "current_coverage": [],
                            }
                        },
                        "lxc_exec_bridge.plan": {"mcp_tool": "lxc_exec_bridge_plan", "mutates": False},
                        "lxc_exec_bridge.bundle": {"mcp_tool": "lxc_exec_bridge_bundle", "mutates": False},
                        "lxc_exec_bridge.status": {"mcp_tool": "lxc_exec_bridge_status", "mutates": False},
                    },
                    "mesh": {
                        "operations": {
                            "network.mesh.status": {
                                "mcp_tool": "network_mesh_status",
                                "mutates": False,
                                "runtime_observation_contract": {
                                    "schema": "cynic.network_mesh.runtime_observation.v1",
                                    "required_fields": [
                                        "schema",
                                        "source",
                                        "status",
                                        "severity",
                                        "blockers",
                                        "missing",
                                        "drift",
                                        "blocking",
                                    ],
                                    "shared_between": [
                                        "crates/cynic-node",
                                        "organs/fleet",
                                    ],
                                },
                            },
                            "network.mesh.dns.render": {"mcp_tool": "network_mesh_dns_render", "mutates": False},
                            "network.mesh.dns.activation_plan": {
                                "mcp_tool": "network_mesh_dns_activation_plan",
                                "mutates": False,
                            },
                            "network.mesh.dns.activation_bundle": {
                                "mcp_tool": "network_mesh_dns_activation_bundle",
                                "mutates": False,
                            },
                            "network.mesh.dns.activation_status": {
                                "mcp_tool": "network_mesh_dns_activation_status",
                                "mutates": False,
                            },
                        }
                    },
                    "storage": {
                        "operations": {
                            "storage.primary.plan": {"mcp_tool": "storage_primary_plan", "mutates": False},
                            "storage.primary.bundle": {"mcp_tool": "storage_primary_bundle", "mutates": False},
                            "storage.primary.status": {
                                "mcp_tool": "storage_primary_status",
                                "mutates": False,
                                "preconditions_reported": ["bundle_complete"],
                            },
                            "storage.primary.readiness": {
                                "mcp_tool": "storage_primary_readiness",
                                "mutates": False,
                                "composes": ["storage.primary.status"],
                            },
                        }
                    },
                },
            }
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            from organs.supervisor.src.core.infrastructure_mcp_contract import (
                InfrastructureMcpContractService,
            )

            service = InfrastructureMcpContractService(
                workspace_root=root,
                contract_path=contract_path,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        kinds = {item.kind for item in findings}
        self.assertIn("infrastructure_mcp_lxc_precondition_guard_missing", kinds)
        self.assertIn("infrastructure_mcp_readiness_dependency_missing", kinds)
        self.assertIn("infrastructure_mcp_storage_status_check_missing", kinds)
        self.assertIn("infrastructure_mcp_bundle_output_contract_missing", kinds)

    def test_validate_pipeline_includes_lxc_exec_bridge_contract(self):
        module = load_module()
        import organs.supervisor.src.core.repo_topology_policy as policy

        original = policy.host.validate_lxc_exec_bridge_contract
        sentinel = module.Finding(
            "critical",
            "lxc_exec_bridge_pipeline_sentinel",
            "axioms/lxc-exec-bridge-v1.json",
            "sentinel",
        )

        def fake_validate():
            return [sentinel]

        try:
            policy.host.validate_lxc_exec_bridge_contract = fake_validate
            findings = module.validate()
        finally:
            policy.host.validate_lxc_exec_bridge_contract = original

        self.assertIn(sentinel, findings)

    def test_lxc_exec_bridge_contract_rejects_raw_pct_sudoers_drift(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "axioms" / "lxc-exec-bridge-v1.json"
            contract_path.parent.mkdir(parents=True)
            contract = {
                "schema": "cynic.lxc_exec_bridge.v1",
                "status": "normative",
                "authority": {
                    "infra_contract": "axioms/infrastructure-mcp-v1.json",
                    "organ": "organs/fleet",
                    "policy_file": "organs/fleet/policies/lxc-exec-bridge.policy.json",
                },
                "principles": ["deny_by_default", "no_raw_shell"],
                "runtime_shape": {
                    "helper_path": "/usr/local/libexec/cynic-lxc-exec-bridge",
                    "source_path": "organs/fleet/src/entrypoints/lxc_exec_bridge_helper.py",
                    "request_transport": "stdin_json",
                    "response_transport": "stdout_json",
                    "bundle_command": "make lxc-exec-bridge-bundle out=/tmp/cynic-lxc-bridge",
                    "mcp_transport_env": {},
                    "sudoers_subject": "cynic",
                    "sudoers_rule_shape": "cynic ALL=(root) NOPASSWD: /usr/sbin/pct",
                    "forbidden_sudoers_shape": "/bin/sh",
                },
                "request": {"required_fields": ["node", "vmid", "purpose", "argv"]},
                "policy": {
                    "allowed_command_shape": "shell_string",
                    "max_timeout_seconds": 600,
                    "allowed_purposes": ["read_only"],
                },
                "mcp_integration": {
                    "operation": "fleet.node.exec",
                    "lxc_transition": "execute directly",
                    "activation_evidence": [],
                },
            }
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            from organs.supervisor.src.core.lxc_exec_bridge_contract import (
                LxcExecBridgeContractService,
            )

            service = LxcExecBridgeContractService(
                workspace_root=root,
                contract_path=contract_path,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        kinds = {item.kind for item in findings}
        self.assertIn("lxc_exec_bridge_runtime_shape_drift", kinds)
        self.assertIn("lxc_exec_bridge_forbidden_sudoers_shape_incomplete", kinds)
        self.assertIn("lxc_exec_bridge_transport_env_missing", kinds)
        self.assertIn("lxc_exec_bridge_request_field_missing", kinds)
        self.assertIn("lxc_exec_bridge_allowed_command_shape_invalid", kinds)
        self.assertIn("lxc_exec_bridge_transition_guard_missing", kinds)

    def test_storage_primary_activation_rejects_missing_network_mesh_dns_bundle(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "axioms" / "storage-primary-activation-v1.json"
            contract_path.parent.mkdir(parents=True)
            contract = {
                "schema": "cynic.storage_primary_activation.v1",
                "status": "normative",
                "authority": {
                    "registry": "infra/registry.json",
                    "storage_schema": "infra/surrealdb/schema.surql",
                    "lxc_exec_bridge": "axioms/lxc-exec-bridge-v1.json",
                    "organ": "organs/keep",
                },
                "target": {
                    "node": "cynic-lxc-surrealdb",
                    "role": "storage_primary",
                    "service": "surrealdb",
                    "canonical_endpoint": "http://organ-surrealdb.synapse.cynic:8000",
                },
                "status_contract": {
                    "active_requires": [
                        "endpoint_is_synapse_dns",
                        "bridge_active",
                        "fleet_observes_node_running",
                        "mesh_reaches_storage",
                        "bundle_complete",
                        "service_health_healthy",
                    ],
                },
                "activation_request_contract": {
                    "bundles": {
                        "lxc_exec_bridge": "/tmp/cynic-lxc-bridge",
                        "storage_primary": "/tmp/cynic-storage-primary",
                    },
                    "coord_targets": ["organs/fleet", "organs/keep", "infra/registry.json"],
                    "required_sequence": [
                        "activate lxc_exec_bridge",
                        "activate storage_primary",
                        "verify storage_primary_readiness.ready",
                        "verify kernel health has no storage_down",
                    ],
                    "no_cortex_root_execution": True,
                },
                "forbidden_patterns": [
                    "curl | sh",
                    "root/root default credentials",
                    "192.168.* service endpoint",
                    "direct pct exec exposed to agents",
                ],
            }
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            from organs.supervisor.src.core.storage_primary_activation_contract import (
                StoragePrimaryActivationContractService,
            )

            service = StoragePrimaryActivationContractService(
                workspace_root=root,
                contract_path=contract_path,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        kinds = {item.kind for item in findings}
        self.assertIn("storage_primary_activation_authority_field_missing", kinds)
        self.assertIn("storage_primary_activation_bundle_missing", kinds)
        self.assertIn("storage_primary_activation_sequence_step_missing", kinds)




if __name__ == "__main__":
    unittest.main()
