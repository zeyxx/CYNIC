import json
import tempfile
import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


def surface_status_transition_policy(*statuses):
    return {
        status: {
            "owner": "organ-supervisor",
            "rule": f"test {status} rule",
            "exit_condition": f"test {status} exit",
        }
        for status in statuses
    }


class RepoTopologySurfaceValidationPart1Tests(unittest.TestCase):
    def test_rejects_missing_service_surface_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "services" / "ignis").mkdir(parents=True)
            inventory_path = contracts / "service-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.service_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["protocol_service_embryo"],
                        "required_evidence_for_open_membrane": "untracked service body files",
                        "surfaces": [],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.SERVICES_ROOT,
                module.SERVICE_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.SERVICES_ROOT = root / "services"
                module.SERVICE_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_service_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.SERVICES_ROOT,
                    module.SERVICE_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("service_surface_missing_inventory", [item.kind for item in findings])

    def test_rejects_untracked_service_without_inventory_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "services" / "ignis").mkdir(parents=True)
            inventory_path = contracts / "service-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.service_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["protocol_service_embryo"],
                        "status_transition_policy": surface_status_transition_policy("protocol_service_embryo"),
                        "required_evidence_for_open_membrane": "untracked service body files",
                        "surfaces": [
                            {
                                "path": "services/ignis/",
                                "class": "protocol_service",
                                "owner": "ignis-protocol",
                                "status": "protocol_service_embryo",
                                "evidence": ["program/Cargo.toml"],
                                "next_action": "close membrane",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.SERVICES_ROOT,
                module.SERVICE_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.SERVICES_ROOT = root / "services"
                module.SERVICE_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["services/ignis/program/src/entrypoint.rs"]
                findings = module.validate_service_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.SERVICES_ROOT,
                    module.SERVICE_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "service_surface_untracked_body_missing_evidence",
            [item.kind for item in findings],
        )

    def test_rejects_surface_status_without_transition_policy(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "services" / "ignis").mkdir(parents=True)
            inventory_path = contracts / "service-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.service_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["protocol_service_embryo"],
                        "required_evidence_for_open_membrane": "untracked service body files",
                        "surfaces": [
                            {
                                "path": "services/ignis/",
                                "class": "protocol_service",
                                "owner": "ignis-protocol",
                                "status": "protocol_service_embryo",
                                "evidence": ["program/Cargo.toml"],
                                "next_action": "close membrane",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.SERVICES_ROOT,
                module.SERVICE_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.SERVICES_ROOT = root / "services"
                module.SERVICE_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_service_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.SERVICES_ROOT,
                    module.SERVICE_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "service_surface_inventory_status_transition_policy_missing",
            [item.kind for item in findings],
        )

    def test_allows_untracked_service_with_inventory_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "services" / "ignis").mkdir(parents=True)
            inventory_path = contracts / "service-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.service_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["protocol_service_embryo"],
                        "status_transition_policy": surface_status_transition_policy("protocol_service_embryo"),
                        "required_evidence_for_open_membrane": "untracked service body files",
                        "surfaces": [
                            {
                                "path": "services/ignis/",
                                "class": "protocol_service",
                                "owner": "ignis-protocol",
                                "status": "protocol_service_embryo",
                                "evidence": ["program/Cargo.toml", "untracked service body files"],
                                "next_action": "close membrane",
                            }
                        ],
                        "totals": {
                            "surfaces": 1,
                            "by_status": {"protocol_service_embryo": 1},
                        },
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.SERVICES_ROOT,
                module.SERVICE_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.SERVICES_ROOT = root / "services"
                module.SERVICE_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["services/ignis/program/src/entrypoint.rs"]
                findings = module.validate_service_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.SERVICES_ROOT,
                    module.SERVICE_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_untracked_implementation_surface_without_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            surface = root / "crates" / "cynic-kernel"
            surface.mkdir(parents=True)
            inventory_path = contracts / "implementation-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.implementation_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["active_partial"],
                        "status_transition_policy": surface_status_transition_policy("active_partial"),
                        "required_evidence_for_open_membrane": "untracked implementation source files",
                        "surfaces": [
                            {
                                "path": "crates/cynic-kernel/",
                                "class": "kernel_runtime",
                                "owner": "cynic-kernel",
                                "status": "active_partial",
                                "evidence": ["Cargo.toml"],
                                "next_action": "close membrane",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.IMPLEMENTATION_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["crates/cynic-kernel/src/api/rest/mesh.rs"]
                findings = module.validate_implementation_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "implementation_surface_untracked_body_missing_evidence",
            [item.kind for item in findings],
        )

    def test_rejects_unknown_implementation_surface_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            surface = root / "crates" / "cynic-kernel"
            surface.mkdir(parents=True)
            inventory_path = contracts / "implementation-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.implementation_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["active_partial"],
                        "status_transition_policy": surface_status_transition_policy("active_partial"),
                        "required_evidence_for_open_membrane": "untracked implementation source files",
                        "surfaces": [
                            {
                                "path": "crates/cynic-kernel/",
                                "class": "kernel_runtime",
                                "owner": "cynic-kernel",
                                "status": "improvised",
                                "evidence": ["Cargo.toml", "untracked implementation source files"],
                                "next_action": "close membrane",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.IMPLEMENTATION_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_implementation_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("implementation_surface_inventory_status_unknown", [item.kind for item in findings])

    def test_allows_untracked_implementation_surface_with_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            surface = root / "crates" / "cynic-kernel"
            surface.mkdir(parents=True)
            inventory_path = contracts / "implementation-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.implementation_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["active_partial"],
                        "status_transition_policy": surface_status_transition_policy("active_partial"),
                        "required_evidence_for_open_membrane": "untracked implementation source files",
                        "surfaces": [
                            {
                                "path": "crates/cynic-kernel/",
                                "class": "kernel_runtime",
                                "owner": "cynic-kernel",
                                "status": "active_partial",
                                "evidence": ["Cargo.toml", "untracked implementation source files"],
                                "next_action": "close membrane",
                            }
                        ],
                        "totals": {
                            "surfaces": 1,
                            "by_status": {"active_partial": 1},
                        },
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.IMPLEMENTATION_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["crates/cynic-kernel/src/api/rest/mesh.rs"]
                findings = module.validate_implementation_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.IMPLEMENTATION_SURFACE_INVENTORY_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_immediate_infra_file_missing_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "infra").mkdir()
            (root / "infra" / "agent-exposure.json").write_text(
                '{"version": 1}',
                encoding="utf-8",
            )
            inventory_path = contracts / "infra-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.infra_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["infra_active"],
                        "required_evidence_for_open_membrane": "infra file without inventory",
                        "surfaces": [],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.INFRA_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.INFRA_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["infra/agent-exposure.json"]
                findings = module.validate_infra_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.INFRA_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("infra_surface_missing_inventory", [item.kind for item in findings])

    def test_rejects_untracked_infra_file_without_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "infra").mkdir()
            (root / "infra" / "agent-exposure.json").write_text(
                '{"version": 1}',
                encoding="utf-8",
            )
            inventory_path = contracts / "infra-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.infra_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["infra_active"],
                        "status_transition_policy": surface_status_transition_policy("infra_active"),
                        "required_evidence_for_open_membrane": "infra file without inventory",
                        "surfaces": [
                            {
                                "path": "infra/agent-exposure.json",
                                "status": "infra_active",
                                "owner": "organ-supervisor",
                                "class": "systemd_service",
                                "evidence": ["other-note"],
                                "next_action": "inventory it",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.INFRA_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.INFRA_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["infra/agent-exposure.json"]
                findings = module.validate_infra_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.INFRA_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "infra_surface_untracked_file_missing_evidence",
            [item.kind for item in findings],
        )

    def test_rejects_unknown_infra_surface_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "infra").mkdir()
            (root / "infra" / "agent-exposure.json").write_text(
                '{"version": 1}',
                encoding="utf-8",
            )
            inventory_path = contracts / "infra-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.infra_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["infra_active"],
                        "status_transition_policy": surface_status_transition_policy("infra_active"),
                        "required_evidence_for_open_membrane": "infra file without inventory",
                        "surfaces": [
                            {
                                "path": "infra/agent-exposure.json",
                                "status": "mysterious",
                                "owner": "organ-supervisor",
                                "class": "systemd_service",
                                "evidence": ["infra/agent-exposure.json"],
                                "next_action": "inventory it",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.INFRA_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.INFRA_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["infra/agent-exposure.json"]
                findings = module.validate_infra_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.INFRA_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("infra_surface_inventory_status_unknown", [item.kind for item in findings])



