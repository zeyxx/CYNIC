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


MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "organism_physics_contract.py"


def load_module():
    return load_module_from_path("organism_physics_contract", MODULE_PATH)


def minimal_contract() -> dict:
    return {
        "schema": "cynic.organism_physics.v1",
        "authorities": {
            "human_readable_policy": "docs/foundation/CYNIC_ORGANISM_PHYSICS_POLICY.md",
            "runtime_roles": "docs/foundation/CYNIC_AGENTIC_RUNTIME_ROLES.md",
            "anti_polling_contract": "axioms/anti-polling-transition-v1.json",
            "api_topology_contract": "axioms/api-topology-v1.json",
            "registry": "infra/registry.json",
            "agent_exposure": "infra/agent-exposure.json",
        },
        "required_lifecycle_layers": [
            "desired",
            "declared",
            "observed",
            "effective",
            "applied",
            "verified",
        ],
        "required_transition_evidence": {
            "effective_to_applied": ["claimed_agent_task", "runner_result"],
            "applied_to_verified": ["completed_agent_task", "service_health_or_activation_evidence"],
        },
        "watch_propagation_rule": {
            "kernel_surface": "GET /agent-tasks/watch",
            "consumer_surface_group": "memory_tasks",
            "expected_mcp_tool": "cynic_watch_pending_agent_tasks",
        },
        "runtime_coherence_surfaces": {
            "daemon_default_capabilities_path": "services/ouroboros-runtime/src/ouroboros_runtime/core/daemon.py",
            "lifecycle_default_capabilities_path": "services/ouroboros-runtime/src/ouroboros_runtime/core/ouroboros.py",
            "kernel_watch_adapter_path": "services/coordination-runtime/src/cynic_coord_runtime/adapters/kernel_rest.py",
            "executor_status_path": "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_kanban_executor.py",
        },
        "inertia_surfaces": ["infra/systemd/hermes-agent-executor.service"],
    }


class OrganismPhysicsContractTests(unittest.TestCase):
    def test_accepts_minimal_valid_physics_contract_without_runtime_gaps(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "infra").mkdir(parents=True)
            (root / "crates" / "cynic-mcp" / "src" / "routers").mkdir(parents=True)
            (root / "services" / "coordination-runtime" / "src" / "cynic_coord_runtime" / "adapters").mkdir(parents=True)
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "adapters").mkdir(parents=True)
            (root / "infra" / "systemd").mkdir(parents=True)

            (root / "docs" / "foundation" / "CYNIC_ORGANISM_PHYSICS_POLICY.md").write_text("policy\n", encoding="utf-8")
            (root / "docs" / "foundation" / "CYNIC_AGENTIC_RUNTIME_ROLES.md").write_text("roles\n", encoding="utf-8")
            (root / "axioms" / "anti-polling-transition-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "api-topology-v1.json").write_text(
                json.dumps({"routes": {"GET /agent-tasks/watch": {"status": "canonical"}}}),
                encoding="utf-8",
            )
            (root / "infra" / "agent-exposure.json").write_text(
                json.dumps({"mcp_tool_domains": {"memory_tasks": {"tools": ["cynic_watch_pending_agent_tasks"]}}}),
                encoding="utf-8",
            )
            (root / "infra" / "registry.json").write_text(
                json.dumps(
                    {
                        "lifecycle": {
                            "kernel_release": {
                                "workflow": {
                                    "transition_contract": {
                                        "layers": minimal_contract()["required_lifecycle_layers"],
                                        "transitions": [
                                            {"from": "effective", "to": "applied", "evidence": ["claimed_agent_task", "runner_result"], "read_back_required": True},
                                            {"from": "applied", "to": "verified", "evidence": ["completed_agent_task", "service_health_or_activation_evidence"], "read_back_required": True},
                                        ],
                                    }
                                },
                                "mcp_freshness_controller": {"status": "active"},
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            (root / "crates" / "cynic-mcp" / "src" / "routers" / "forward.rs").write_text(
                'name = "cynic_watch_pending_agent_tasks"\n',
                encoding="utf-8",
            )
            (root / "services" / "coordination-runtime" / "src" / "cynic_coord_runtime" / "adapters" / "kernel_rest.py").write_text(
                "def wait_for_tasks():\n    return []\n",
                encoding="utf-8",
            )
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "daemon.py").write_text(
                'caps = "observe,claim,wait_for_tasks"\n',
                encoding="utf-8",
            )
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "ouroboros.py").write_text(
                'caps = ["observe", "claim", "wait_for_tasks"]\n',
                encoding="utf-8",
            )
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "adapters" / "hermes_kanban_executor.py").write_text(
                "def status_wait():\n    return None\n",
                encoding="utf-8",
            )
            (root / "infra" / "systemd" / "hermes-agent-executor.service").write_text(
                "[Service]\nEnvironment=HERMES_AGENT_TASK_TIMEOUT_SECS=120\n",
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "organism-physics-v1.json"
            contract_path.write_text(json.dumps(minimal_contract()), encoding="utf-8")

            old_root = module.host.WORKSPACE_ROOT
            old_contract = module.CONTRACT_PATH
            try:
                module.host.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_organism_physics_contract()
            finally:
                module.host.WORKSPACE_ROOT = old_root
                module.CONTRACT_PATH = old_contract

        self.assertEqual([], findings)

    def test_detects_watch_propagation_and_runtime_inertia_gaps(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "infra").mkdir(parents=True)
            (root / "crates" / "cynic-mcp" / "src" / "routers").mkdir(parents=True)
            (root / "services" / "coordination-runtime" / "src" / "cynic_coord_runtime" / "adapters").mkdir(parents=True)
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "adapters").mkdir(parents=True)
            (root / "infra" / "systemd").mkdir(parents=True)

            (root / "docs" / "foundation" / "CYNIC_ORGANISM_PHYSICS_POLICY.md").write_text("policy\n", encoding="utf-8")
            (root / "docs" / "foundation" / "CYNIC_AGENTIC_RUNTIME_ROLES.md").write_text("roles\n", encoding="utf-8")
            (root / "axioms" / "anti-polling-transition-v1.json").write_text("{}", encoding="utf-8")
            (root / "axioms" / "api-topology-v1.json").write_text(
                json.dumps({"routes": {"GET /agent-tasks/watch": {"status": "canonical"}}}),
                encoding="utf-8",
            )
            (root / "infra" / "agent-exposure.json").write_text(
                json.dumps({"mcp_tool_domains": {"memory_tasks": {"tools": ["cynic_list_pending_agent_tasks"]}}}),
                encoding="utf-8",
            )
            (root / "infra" / "registry.json").write_text(
                json.dumps(
                    {
                        "lifecycle": {
                            "kernel_release": {
                                "workflow": {
                                    "transition_contract": {
                                        "layers": minimal_contract()["required_lifecycle_layers"],
                                        "transitions": [
                                            {"from": "effective", "to": "applied", "evidence": ["claimed_agent_task", "runner_result"], "read_back_required": True},
                                            {"from": "applied", "to": "verified", "evidence": ["completed_agent_task", "service_health_or_activation_evidence"], "read_back_required": True},
                                        ],
                                    }
                                },
                                "mcp_freshness_controller": {"status": "planned"},
                            }
                        }
                    }
                ),
                encoding="utf-8",
            )
            (root / "crates" / "cynic-mcp" / "src" / "routers" / "forward.rs").write_text(
                'name = "cynic_list_pending_agent_tasks"\n',
                encoding="utf-8",
            )
            (root / "services" / "coordination-runtime" / "src" / "cynic_coord_runtime" / "adapters" / "kernel_rest.py").write_text(
                'logger.warning("kernel /agent-tasks/watch unavailable: %s", e)\n',
                encoding="utf-8",
            )
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "daemon.py").write_text(
                'caps = "observe,claim,wait_for_tasks"\n',
                encoding="utf-8",
            )
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "ouroboros.py").write_text(
                'caps = ["observe", "claim", "poll_tasks"]\n',
                encoding="utf-8",
            )
            (root / "services" / "ouroboros-runtime" / "src" / "ouroboros_runtime" / "core" / "adapters" / "hermes_kanban_executor.py").write_text(
                'show = "hermes kanban show"\ndef _sleep_poll():\n    return None\n',
                encoding="utf-8",
            )
            (root / "infra" / "systemd" / "hermes-agent-executor.service").write_text(
                "[Service]\nEnvironment=HERMES_AGENT_STATUS_INTERVAL_SECS=5\n",
                encoding="utf-8",
            )
            contract_path = root / "axioms" / "organism-physics-v1.json"
            contract_path.write_text(json.dumps(minimal_contract()), encoding="utf-8")

            old_root = module.host.WORKSPACE_ROOT
            old_contract = module.CONTRACT_PATH
            try:
                module.host.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_organism_physics_contract()
            finally:
                module.host.WORKSPACE_ROOT = old_root
                module.CONTRACT_PATH = old_contract

        kinds = {item.kind for item in findings}
        self.assertIn("physics_watch_not_propagated_to_mcp_surface", kinds)
        self.assertIn("physics_watch_not_propagated_to_mcp_router", kinds)
        self.assertIn("physics_runtime_capability_vocabulary_split", kinds)
        self.assertIn("physics_executor_status_open_loop", kinds)
        self.assertIn("physics_mcp_freshness_loop_open", kinds)
