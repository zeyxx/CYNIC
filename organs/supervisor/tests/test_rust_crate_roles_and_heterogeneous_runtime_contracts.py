import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from organs.supervisor.src.core import heterogeneous_runtime_matrix_contract as hetero
from organs.supervisor.src.core import rust_crate_roles_contract as rust_roles


def runtime_role(
    *,
    role: str,
    role_class: str,
    activation_mode: str,
    agents: list[str],
    proofs: list[str],
) -> dict[str, object]:
    return {
        "role": role,
        "class": role_class,
        "organism_role": f"{role} organism role",
        "bootstrap_target_stage": "b5_runtime_actionable",
        "required_local_agents": agents,
        "authorized_local_actuators": ["service_restart"],
        "activation_mode": activation_mode,
        "required_network_planes": ["synapse"],
        "required_dns_contracts": ["kernel_service_resolution"],
        "required_proof_surfaces": proofs,
        "capabilities": {"build": False, "deploy": False, "recover": True},
        "why": f"{role} host",
    }


class RustCrateRolesContractTests(unittest.TestCase):
    def test_rejects_workspace_member_missing_from_role_contract(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "crates" / "known").mkdir(parents=True)
            (root / "crates" / "known" / "Cargo.toml").write_text(
                "[package]\nname='known'\nversion='0.1.0'\nedition='2021'\n",
                encoding="utf-8",
            )
            (root / "crates" / "missing").mkdir(parents=True)
            (root / "crates" / "missing" / "Cargo.toml").write_text(
                "[package]\nname='missing'\nversion='0.1.0'\nedition='2021'\n",
                encoding="utf-8",
            )
            (root / "Cargo.toml").write_text(
                "[workspace]\nmembers = ['crates/known', 'crates/missing']\n",
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "rust-crate-roles-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.rust_crate_roles.v1",
                        "crates": [
                            {
                                "path": "crates/known",
                                "package": "known",
                                "family": "nervous_system",
                                "class": "kernel_runtime",
                                "workspace_member": True,
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(rust_roles.host, "WORKSPACE_ROOT", root), mock.patch.object(
                rust_roles.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(rust_roles, "CONTRACT_PATH", contract_path), mock.patch.object(
                rust_roles, "WORKSPACE_TOML_PATH", root / "Cargo.toml"
            ):
                findings = rust_roles.validate_rust_crate_roles_contract()
        self.assertIn("rust_crate_visible_unclassified", {item.kind for item in findings})

    def test_rejects_workspace_member_flag_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "crates" / "gateway").mkdir(parents=True)
            (root / "crates" / "gateway" / "Cargo.toml").write_text(
                "[package]\nname='gateway'\nversion='0.1.0'\nedition='2021'\n",
                encoding="utf-8",
            )
            (root / "Cargo.toml").write_text(
                "[workspace]\nmembers = ['crates/gateway']\n",
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "rust-crate-roles-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.rust_crate_roles.v1",
                        "crates": [
                            {
                                "path": "crates/gateway",
                                "package": "gateway",
                                "family": "quarantine",
                                "class": "transitional_visible_crate",
                                "workspace_member": False,
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(rust_roles.host, "WORKSPACE_ROOT", root), mock.patch.object(
                rust_roles.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(rust_roles, "CONTRACT_PATH", contract_path), mock.patch.object(
                rust_roles, "WORKSPACE_TOML_PATH", root / "Cargo.toml"
            ):
                findings = rust_roles.validate_rust_crate_roles_contract()
        self.assertIn("rust_crate_roles_workspace_member_mismatch", {item.kind for item in findings})


class HeterogeneousRuntimeMatrixContractTests(unittest.TestCase):
    def test_rejects_registry_role_missing_from_matrix(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "infra").mkdir()
            (root / "axioms" / "node-bootstrap-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "runtime-principals-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "rust-crate-roles-v1.json").write_text("{}", encoding="utf-8")
            (root / "infra" / "registry.json").write_text(
                json.dumps({"nodes": [{"name": "a", "role": "kernel_primary"}, {"name": "b", "role": "mystery_role"}]}),
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "heterogeneous-runtime-matrix-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.heterogeneous_runtime_matrix.v1",
                        "authorities": {
                            "runtime_registry": "infra/registry.json",
                            "node_bootstrap": "axioms/node-bootstrap-v1.json",
                            "runtime_principals": "axioms/runtime-principals-v1.json",
                            "rust_crate_roles": "axioms/rust-crate-roles-v1.json"
                        },
                        "allowed_classes": ["kernel_node"],
                        "allowed_machine_types": ["vm_runtime_node"],
                        "allowed_activation_modes": ["kernel_primary"],
                        "allowed_local_agents": ["cynic-node", "cynic-kernel"],
                        "allowed_local_actuators": ["service_restart"],
                        "allowed_network_planes": ["synapse"],
                        "allowed_dns_contracts": ["kernel_service_resolution"],
                        "allowed_proof_surfaces": ["service.health"],
                        "roles": [
                            {
                                **runtime_role(
                                    role="kernel_primary",
                                    role_class="kernel_node",
                                    activation_mode="kernel_primary",
                                    agents=["cynic-node", "cynic-kernel"],
                                    proofs=["service.health"],
                                ),
                                "capabilities": {"build": False, "deploy": True, "recover": True},
                            }
                        ],
                        "node_bindings": [
                            {"node": "a", "role": "kernel_primary", "machine_type": "vm_runtime_node"}
                        ],
                        "totals": {"roles": 1, "node_bindings": 1}
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(hetero.host, "WORKSPACE_ROOT", root), mock.patch.object(
                hetero.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(hetero, "CONTRACT_PATH", contract_path), mock.patch.object(
                hetero, "REGISTRY_PATH", root / "infra" / "registry.json"
            ):
                findings = hetero.validate_heterogeneous_runtime_matrix_contract()
        self.assertIn("heterogeneous_runtime_registry_role_unmapped", {item.kind for item in findings})

    def test_accepts_complete_runtime_matrix(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "infra").mkdir()
            (root / "axioms" / "node-bootstrap-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "runtime-principals-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "rust-crate-roles-v1.json").write_text("{}", encoding="utf-8")
            (root / "infra" / "registry.json").write_text(
                json.dumps({"nodes": [{"name": "a", "role": "kernel_primary"}, {"name": "b", "role": "cortex_host"}]}),
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "heterogeneous-runtime-matrix-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.heterogeneous_runtime_matrix.v1",
                        "authorities": {
                            "runtime_registry": "infra/registry.json",
                            "node_bootstrap": "axioms/node-bootstrap-v1.json",
                            "runtime_principals": "axioms/runtime-principals-v1.json",
                            "rust_crate_roles": "axioms/rust-crate-roles-v1.json"
                        },
                        "allowed_classes": ["kernel_node", "cortex_surface"],
                        "allowed_machine_types": ["vm_runtime_node", "operator_workstation"],
                        "allowed_activation_modes": ["kernel_primary", "cortex_interactive"],
                        "allowed_local_agents": ["cynic-node", "cynic-kernel", "cynic-mcp"],
                        "allowed_local_actuators": ["service_restart", "manual_operator"],
                        "allowed_network_planes": ["synapse"],
                        "allowed_dns_contracts": ["kernel_service_resolution"],
                        "allowed_proof_surfaces": ["service.health", "fleet.node.status"],
                        "roles": [
                            {
                                **runtime_role(
                                    role="kernel_primary",
                                    role_class="kernel_node",
                                    activation_mode="kernel_primary",
                                    agents=["cynic-node", "cynic-kernel"],
                                    proofs=["service.health"],
                                ),
                                "capabilities": {"build": False, "deploy": True, "recover": True},
                            },
                            {
                                **runtime_role(
                                    role="cortex_host",
                                    role_class="cortex_surface",
                                    activation_mode="cortex_interactive",
                                    agents=["cynic-node", "cynic-mcp"],
                                    proofs=["fleet.node.status"],
                                ),
                                "authorized_local_actuators": ["manual_operator"],
                                "bootstrap_target_stage": "b2_kernel_reachable",
                            }
                        ],
                        "node_bindings": [
                            {"node": "a", "role": "kernel_primary", "machine_type": "vm_runtime_node"},
                            {"node": "b", "role": "cortex_host", "machine_type": "operator_workstation"}
                        ],
                        "totals": {"roles": 2, "node_bindings": 2}
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(hetero.host, "WORKSPACE_ROOT", root), mock.patch.object(
                hetero.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(hetero, "CONTRACT_PATH", contract_path), mock.patch.object(
                hetero, "REGISTRY_PATH", root / "infra" / "registry.json"
            ):
                findings = hetero.validate_heterogeneous_runtime_matrix_contract()
        self.assertEqual([], findings)


if __name__ == "__main__":
    unittest.main()
