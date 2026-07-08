from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from organs.supervisor.src.core import python_vital_function_placement_contract as module


class PythonVitalFunctionPlacementContractTests(unittest.TestCase):
    def test_accepts_declared_target_crates_and_existing_sources(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "core").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "adapters").mkdir(parents=True)
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
                                "python_absorption_authority": {"allowed_domains": ["network_contracts"]},
                            },
                            {
                                "path": "crates/cynic-mcp",
                                "family": "nervous_system",
                                "class": "portable_mcp_surface",
                                "python_absorption_authority": {"allowed_domains": ["mcp_surface_contracts"]},
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "axioms" / "python-vital-function-placement-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.python_vital_function_placement.v1",
                        "target_crate_policy": [
                            {
                                "target_crate": "crates/cynic-node",
                                "expected_family": "soma",
                                "expected_class": "node_runtime",
                                "why": "node runtime target",
                            }
                        ],
                        "function_groups": [
                            {
                                "id": "fleet_network_contracts",
                                "status": "planned_migration",
                                "organ": "organ-fleet",
                                "criticality": "high",
                                "target_crate": "crates/cynic-node",
                                "target_component": "network_contracts",
                                "source_paths": [
                                    "organs/fleet/src/core/network_surfaces.py",
                                    "organs/fleet/src/adapters/registry_adapter.py",
                                ],
                                "functions": ["mesh_status_payload"],
                                "reason": "deterministic network logic",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module, "CONTRACT_PATH", root / "axioms" / "python-vital-function-placement-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_python_vital_function_placement_contract()

        self.assertEqual([], findings)

    def test_rejects_unknown_target_crate_and_missing_source(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps(
                    {
                        "crates": [
                            {
                                "path": "crates/cynic-node",
                                "family": "soma",
                                "class": "node_runtime",
                                "python_absorption_authority": {"allowed_domains": ["network_contracts"]},
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "axioms" / "python-vital-function-placement-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.python_vital_function_placement.v1",
                        "target_crate_policy": [
                            {
                                "target_crate": "crates/cynic-node",
                                "expected_family": "soma",
                                "expected_class": "node_runtime",
                                "why": "node runtime target",
                            }
                        ],
                        "function_groups": [
                            {
                                "id": "bad_group",
                                "status": "planned_migration",
                                "organ": "organ-fleet",
                                "criticality": "high",
                                "target_crate": "crates/unknown",
                                "target_component": "network_contracts",
                                "source_paths": ["organs/fleet/src/core/missing.py"],
                                "functions": ["mesh_status_payload"],
                                "reason": "deterministic network logic",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module, "CONTRACT_PATH", root / "axioms" / "python-vital-function-placement-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_python_vital_function_placement_contract()

        kinds = {item.kind for item in findings}
        self.assertIn("python_vital_function_target_crate_unknown", kinds)
        self.assertIn("python_vital_function_source_path_missing_on_disk", kinds)

    def test_rejects_target_crate_without_absorption_authority(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "core").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "core" / "network_surfaces.py").write_text("", encoding="utf-8")
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
                        "schema": "cynic.python_vital_function_placement.v1",
                        "target_crate_policy": [
                            {
                                "target_crate": "crates/cynic-node",
                                "expected_family": "soma",
                                "expected_class": "node_runtime",
                                "why": "node runtime target",
                            }
                        ],
                        "function_groups": [
                            {
                                "id": "fleet_network_contracts",
                                "status": "planned_migration",
                                "organ": "organ-fleet",
                                "criticality": "high",
                                "target_crate": "crates/cynic-node",
                                "target_component": "network_contracts",
                                "source_paths": ["organs/fleet/src/core/network_surfaces.py"],
                                "functions": ["mesh_status_payload"],
                                "reason": "deterministic network logic",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module, "CONTRACT_PATH", root / "axioms" / "python-vital-function-placement-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_python_vital_function_placement_contract()

        kinds = {item.kind for item in findings}
        self.assertIn("python_vital_function_target_crate_not_absorption_authority", kinds)

    def test_rejects_target_policy_family_or_class_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "core").mkdir(parents=True)
            (root / "organs" / "fleet" / "src" / "core" / "network_surfaces.py").write_text("", encoding="utf-8")
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps(
                    {
                        "crates": [
                            {
                                "path": "crates/cynic-node",
                                "family": "soma",
                                "class": "node_runtime",
                                "python_absorption_authority": {"allowed_domains": ["network_contracts"]},
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "axioms" / "python-vital-function-placement-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.python_vital_function_placement.v1",
                        "target_crate_policy": [
                            {
                                "target_crate": "crates/cynic-node",
                                "expected_family": "nervous_system",
                                "expected_class": "kernel_runtime",
                                "why": "wrong on purpose",
                            }
                        ],
                        "function_groups": [
                            {
                                "id": "fleet_network_contracts",
                                "status": "planned_migration",
                                "organ": "organ-fleet",
                                "criticality": "high",
                                "target_crate": "crates/cynic-node",
                                "target_component": "network_contracts",
                                "source_paths": ["organs/fleet/src/core/network_surfaces.py"],
                                "functions": ["mesh_status_payload"],
                                "reason": "deterministic network logic",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module, "CONTRACT_PATH", root / "axioms" / "python-vital-function-placement-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_python_vital_function_placement_contract()

        kinds = {item.kind for item in findings}
        self.assertIn("python_vital_function_target_family_mismatch", kinds)
        self.assertIn("python_vital_function_target_class_mismatch", kinds)


if __name__ == "__main__":
    unittest.main()
