import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from organs.supervisor.src.core import lifecycle_heterogeneous_runtime_contract as module


class LifecycleHeterogeneousRuntimeContractTests(unittest.TestCase):
    def test_rejects_unmapped_activation_requirement_role(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "infra").mkdir()
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps({"crates": [{"path": "crates/cynic-node"}]}),
                encoding="utf-8",
            )
            (root / "axioms" / "heterogeneous-runtime-matrix-v1.json").write_text(
                json.dumps(
                    {
                        "roles": [
                            {
                                "role": "build_runner",
                                "class": "build_node",
                                "activation_mode": "build_and_publish",
                                "required_local_agents": ["cynic-node"],
                            },
                            {
                                "role": "kernel_primary",
                                "class": "kernel_node",
                                "activation_mode": "kernel_primary",
                                "required_local_agents": ["cynic-node", "cynic-kernel"],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "infra" / "registry.json").write_text(
                json.dumps(
                    {
                        "lifecycle": {
                            "kernel_release": {
                                "build_runner_role": "build_runner",
                                "deploy_target_role": "kernel_primary",
                                "activation_requirements": {
                                    "build_runner": {},
                                    "storage_primary": {},
                                },
                                "runtime_artifacts": [
                                    {
                                        "name": "cynic-node",
                                        "runtime_targets": ["all_runtime_nodes"],
                                        "build_targets": ["crates/cynic-node"],
                                    }
                                ],
                                "build_policy": {
                                    "artifact_targets": ["crates/cynic-node"]
                                },
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(module, "REGISTRY_PATH", root / "infra" / "registry.json"), mock.patch.object(
                module, "MATRIX_PATH", root / "axioms" / "heterogeneous-runtime-matrix-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_lifecycle_heterogeneous_runtime_contract()
        self.assertIn(
            "lifecycle_activation_requirement_role_unmapped",
            {item.kind for item in findings},
        )

    def test_rejects_build_runner_role_with_wrong_class(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "infra").mkdir()
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps({"crates": [{"path": "crates/cynic-node"}]}),
                encoding="utf-8",
            )
            (root / "axioms" / "heterogeneous-runtime-matrix-v1.json").write_text(
                json.dumps(
                    {
                        "roles": [
                            {
                                "role": "build_runner",
                                "class": "kernel_node",
                                "activation_mode": "kernel_primary",
                                "required_local_agents": ["cynic-node"],
                            },
                            {
                                "role": "kernel_primary",
                                "class": "kernel_node",
                                "activation_mode": "kernel_primary",
                                "required_local_agents": ["cynic-node", "cynic-kernel"],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "infra" / "registry.json").write_text(
                json.dumps(
                    {
                        "lifecycle": {
                            "kernel_release": {
                                "build_runner_role": "build_runner",
                                "deploy_target_role": "kernel_primary",
                                "activation_requirements": {
                                    "build_runner": {},
                                    "kernel_primary": {},
                                },
                                "runtime_artifacts": [
                                    {
                                        "name": "cynic-node",
                                        "runtime_targets": ["all_runtime_nodes"],
                                        "build_targets": ["crates/cynic-node"],
                                    }
                                ],
                                "build_policy": {
                                    "artifact_targets": ["crates/cynic-node"]
                                },
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(module, "REGISTRY_PATH", root / "infra" / "registry.json"), mock.patch.object(
                module, "MATRIX_PATH", root / "axioms" / "heterogeneous-runtime-matrix-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_lifecycle_heterogeneous_runtime_contract()
        kinds = {item.kind for item in findings}
        self.assertIn("lifecycle_primary_role_class_mismatch", kinds)
        self.assertIn("lifecycle_primary_role_activation_mode_mismatch", kinds)

    def test_accepts_consistent_lifecycle_and_runtime_matrix(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "infra").mkdir()
            (root / "axioms" / "rust-crate-roles-v1.json").write_text(
                json.dumps(
                    {
                        "crates": [
                            {"path": "crates/cynic-node"},
                            {"path": "crates/cynic-mcp"},
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "axioms" / "heterogeneous-runtime-matrix-v1.json").write_text(
                json.dumps(
                    {
                        "roles": [
                            {
                                "role": "build_runner",
                                "class": "build_node",
                                "activation_mode": "build_and_publish",
                                "required_local_agents": ["cynic-node", "organ-forge"],
                                "capabilities": {"build": True, "deploy": False, "recover": True},
                            },
                            {
                                "role": "kernel_primary",
                                "class": "kernel_node",
                                "activation_mode": "kernel_primary",
                                "required_local_agents": ["cynic-node", "cynic-kernel"],
                                "capabilities": {"build": False, "deploy": True, "recover": True},
                            },
                            {
                                "role": "storage_primary",
                                "class": "storage_node",
                                "activation_mode": "storage_primary",
                                "required_local_agents": ["cynic-node"],
                                "capabilities": {"build": False, "deploy": False, "recover": True},
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (root / "infra" / "registry.json").write_text(
                json.dumps(
                    {
                        "lifecycle": {
                            "kernel_release": {
                                "build_runner_role": "build_runner",
                                "deploy_target_role": "kernel_primary",
                                "activation_requirements": {
                                    "build_runner": {},
                                    "kernel_primary": {},
                                    "storage_primary": {},
                                },
                                "runtime_artifacts": [
                                    {
                                        "name": "cynic-node",
                                        "runtime_targets": ["all_runtime_nodes"],
                                        "build_targets": ["crates/cynic-node"],
                                    },
                                    {
                                        "name": "cynic-mcp",
                                        "runtime_targets": ["cortex"],
                                        "build_targets": ["crates/cynic-mcp"],
                                    },
                                ],
                                "build_policy": {
                                    "artifact_targets": ["crates/cynic-node", "crates/cynic-mcp"]
                                },
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(module.host, "WORKSPACE_ROOT", root), mock.patch.object(
                module.host, "CONTRACTS_ROOT", root / "axioms"
            ), mock.patch.object(module, "REGISTRY_PATH", root / "infra" / "registry.json"), mock.patch.object(
                module, "MATRIX_PATH", root / "axioms" / "heterogeneous-runtime-matrix-v1.json"
            ), mock.patch.object(
                module, "RUST_CRATE_ROLES_PATH", root / "axioms" / "rust-crate-roles-v1.json"
            ):
                findings = module.validate_lifecycle_heterogeneous_runtime_contract()
        self.assertEqual([], findings)


if __name__ == "__main__":
    unittest.main()
