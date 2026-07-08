from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from organs.supervisor.src.core import node_network_interfaces_contract as module


class NodeNetworkInterfacesContractTests(unittest.TestCase):
    def test_accepts_valid_contract(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "crates" / "cynic-node" / "src" / "ports").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "core").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "adapters").mkdir(parents=True)
            (root / "docs" / "foundation" / "CYNIC_META_LAYER_CONTRACTS.md").write_text("meta\n", encoding="utf-8")
            (root / "axioms" / "repo-topology-v1.json").write_text("{}", encoding="utf-8")
            (root / "crates" / "cynic-node" / "src" / "ports" / "network_interfaces_port.rs").write_text(
                "pub trait RegistryTopologyPort {}\n",
                encoding="utf-8",
            )
            (root / "organs" / "fleet" / "src" / "core" / "network_surfaces.py").write_text("", encoding="utf-8")
            (root / "organs" / "fleet" / "src" / "adapters" / "registry_adapter.py").write_text("", encoding="utf-8")
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps(
                    {
                        "crates": [
                            {
                                "path": "crates/cynic-node",
                                "family": "soma",
                                "class": "node_runtime",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "axioms" / "python-vital-function-placement-v1.json").write_text(
                json.dumps(
                    {
                        "function_groups": [
                            {"id": "fleet_network_contracts"},
                            {"id": "wireguard_legacy_network_contracts"},
                        ]
                    }
                ),
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "node-network-interfaces-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.node_network_interfaces.v1",
                        "authorities": {
                            "meta": "docs/foundation/CYNIC_META_LAYER_CONTRACTS.md",
                            "topology": "axioms/repo-topology-v1.json",
                        },
                        "target_crate": {
                            "path": "crates/cynic-node",
                            "family": "soma",
                            "class": "node_runtime",
                        },
                        "ports": [
                            {
                                "id": "registry_topology_port",
                                "responsibility": "read topology",
                                "rust_trait": "crates/cynic-node/src/ports/network_interfaces_port.rs::RegistryTopologyPort",
                                "current_python_authority": [
                                    "organs/fleet/src/adapters/registry_adapter.py"
                                ],
                                "absorbs_function_groups": [
                                    "fleet_network_contracts"
                                ],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(module, "CONTRACT_PATH", contract_path), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ), mock.patch.object(
                module, "PYTHON_VITAL_FUNCTION_PATH", root / "axioms" / "python-vital-function-placement-v1.json"
            ):
                findings = module.validate_node_network_interfaces_contract()

        self.assertEqual([], findings)

    def test_rejects_unknown_group_and_missing_trait_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "docs" / "foundation" / "CYNIC_META_LAYER_CONTRACTS.md").write_text("meta\n", encoding="utf-8")
            (root / "axioms" / "repo-topology-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps(
                    {
                        "crates": [
                            {
                                "path": "crates/cynic-node",
                                "family": "soma",
                                "class": "node_runtime",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "axioms" / "python-vital-function-placement-v1.json").write_text(
                json.dumps({"function_groups": [{"id": "fleet_network_contracts"}]}),
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "node-network-interfaces-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.node_network_interfaces.v1",
                        "authorities": {
                            "meta": "docs/foundation/CYNIC_META_LAYER_CONTRACTS.md",
                            "topology": "axioms/repo-topology-v1.json",
                        },
                        "target_crate": {
                            "path": "crates/cynic-node",
                            "family": "soma",
                            "class": "node_runtime",
                        },
                        "ports": [
                            {
                                "id": "mesh_runtime_observation_port",
                                "responsibility": "observe runtime",
                                "rust_trait": "crates/cynic-node/src/ports/missing.rs::MeshRuntimeObservationPort",
                                "current_python_authority": [
                                    "organs/fleet/src/core/missing.py"
                                ],
                                "absorbs_function_groups": [
                                    "unknown_group"
                                ],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(module, "CONTRACT_PATH", contract_path), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ), mock.patch.object(
                module, "PYTHON_VITAL_FUNCTION_PATH", root / "axioms" / "python-vital-function-placement-v1.json"
            ):
                findings = module.validate_node_network_interfaces_contract()

        kinds = {item.kind for item in findings}
        self.assertIn("node_network_port_trait_path_missing", kinds)
        self.assertIn("node_network_port_unknown_function_group", kinds)
        self.assertIn("node_network_port_python_authority_missing", kinds)


if __name__ == "__main__":
    unittest.main()
