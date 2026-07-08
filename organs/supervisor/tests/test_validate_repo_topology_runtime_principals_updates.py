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


class RuntimePrincipalTopologyProjectionUpdateTests(unittest.TestCase):
    def test_rejects_runtime_principals_json_dispatch_marked_non_mutating(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            infra = root / "infra"
            runtime_src = root / "organs" / "supervisor" / "src" / "entrypoints"
            entrypoints = root / "organs" / "supervisor" / "entrypoints"
            contracts.mkdir(parents=True)
            infra.mkdir()
            runtime_src.mkdir(parents=True)
            entrypoints.mkdir(parents=True)
            (entrypoints / "runtime_principals.py").write_text(
                "choices=['mcp-reload']\n",
                encoding="utf-8",
            )
            (runtime_src / "runtime_principals_views_runtime.py").write_text(
                "activation_operator_contract = True\n"
                "operator_sequence = []\n"
                "proof_commands = []\n"
                "done_when = []\n"
                "cynic mcp-reload --dry-run\n"
                "cynic mcp-reload\n"
                "cynic mcp-reload --json\n"
                "cynic organism\n",
                encoding="utf-8",
            )
            contract_path = contracts / "runtime-principals-v1.json"
            policy_path = infra / "agent-policy.toml"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.runtime_principals.v1",
                        "status": "normative",
                        "dedicated_user": {"name": "cynic"},
                        "postures": {"bounded_yolo": {"forbidden_nopasswd_concerns": [], "forbidden_commands": []}},
                        "cortex_cli_policy": {
                            "required_agents": ["codex"],
                            "required_runtime_user": "cynic",
                            "required_posture": "bounded_yolo",
                            "projection": "infra/agent-policy.toml",
                        },
                        "projection_requirements": {"agent_policy_toml": {"path": "infra/agent-policy.toml"}},
                        "activation": {
                            "script": "organs/supervisor/entrypoints/runtime_principals.py",
                            "lifecycle_operator_sequence": {
                                "profiles": ["mcp-reload"],
                                "steps": [
                                    {
                                        "name": "inspect",
                                        "command_template": "cynic {profile} --dry-run",
                                        "mutates_mempool": False,
                                    },
                                    {
                                        "name": "dispatch",
                                        "command_template": "cynic {profile}",
                                        "mutates_mempool": True,
                                    },
                                    {
                                        "name": "prove",
                                        "command_template": "cynic {profile} --json",
                                        "mutates_mempool": False,
                                    },
                                ],
                                "required_projection_fields": ["operator_sequence", "proof_commands", "done_when"],
                            },
                        },
                    }
                ),
                encoding="utf-8",
            )
            policy_path.write_text(
                """
[runtime]
authority = "axioms/runtime-principals-v1.json"
dedicated_user = "cynic"

[agents.codex]
runtime_user = "cynic"
execution_posture = "bounded_yolo"
""",
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.RUNTIME_PRINCIPALS_CONTRACT_PATH = contract_path
                module.AGENT_POLICY_PATH = policy_path
                findings = module.validate_runtime_principals_contract()
            finally:
                module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH = old_values

        self.assertIn("runtime_principals_lifecycle_json_dispatch_misclassified", [item.kind for item in findings])

    def test_rejects_runtime_principals_non_native_update_command(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            infra = root / "infra"
            runtime_src = root / "organs" / "supervisor" / "src" / "entrypoints"
            entrypoints = root / "organs" / "supervisor" / "entrypoints"
            contracts.mkdir(parents=True)
            infra.mkdir()
            runtime_src.mkdir(parents=True)
            entrypoints.mkdir(parents=True)
            (entrypoints / "runtime_principals.py").write_text(
                "choices=['status', 'update']\n",
                encoding="utf-8",
            )
            (runtime_src / "runtime_principals_actions.py").write_text(
                'PRIMARY_UPDATE_COMMANDS = {"codex": ["codex", "update"]}\n'
                'help_text = "cynic update --dry-run cynic update"\n',
                encoding="utf-8",
            )
            contract_path = contracts / "runtime-principals-v1.json"
            policy_path = infra / "agent-policy.toml"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.runtime_principals.v1",
                        "status": "normative",
                        "dedicated_user": {"name": "cynic"},
                        "postures": {"bounded_yolo": {"forbidden_nopasswd_concerns": [], "forbidden_commands": []}},
                        "cortex_cli_policy": {
                            "required_agents": ["codex"],
                            "required_runtime_user": "cynic",
                            "required_posture": "bounded_yolo",
                            "projection": "infra/agent-policy.toml",
                        },
                        "projection_requirements": {"agent_policy_toml": {"path": "infra/agent-policy.toml"}},
                        "activation": {
                            "script": "organs/supervisor/entrypoints/runtime_principals.py",
                            "operator_commands": ["cynic update", "cynic update --dry-run"],
                            "primary_dependency_updates": {
                                "operator_command": "cynic update",
                                "allowed_update_commands": {"codex": ["npm", "install", "-g", "codex"]},
                                "forbidden_update_patterns": ["curl | sh"],
                            },
                        },
                    }
                ),
                encoding="utf-8",
            )
            policy_path.write_text(
                """
[runtime]
authority = "axioms/runtime-principals-v1.json"
dedicated_user = "cynic"

[agents.codex]
runtime_user = "cynic"
execution_posture = "bounded_yolo"
""",
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.RUNTIME_PRINCIPALS_CONTRACT_PATH = contract_path
                module.AGENT_POLICY_PATH = policy_path
                findings = module.validate_runtime_principals_contract()
            finally:
                module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH = old_values

        kinds = [item.kind for item in findings]
        self.assertIn("runtime_principals_allowed_update_command_not_native", kinds)
        self.assertIn("runtime_principals_allowed_update_command_unimplemented", kinds)

    def test_rejects_runtime_principals_update_projection_field_unimplemented(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            infra = root / "infra"
            runtime_src = root / "organs" / "supervisor" / "src" / "entrypoints"
            entrypoints = root / "organs" / "supervisor" / "entrypoints"
            contracts.mkdir(parents=True)
            infra.mkdir()
            runtime_src.mkdir(parents=True)
            entrypoints.mkdir(parents=True)
            (entrypoints / "runtime_principals.py").write_text(
                "choices=['status', 'update']\n",
                encoding="utf-8",
            )
            (runtime_src / "runtime_principals_actions.py").write_text(
                'PRIMARY_UPDATE_COMMANDS = {"codex": ["codex", "update"]}\n'
                'help_text = "cynic update --dry-run cynic update operator_sequence proof_commands done_when"\n',
                encoding="utf-8",
            )
            contract_path = contracts / "runtime-principals-v1.json"
            policy_path = infra / "agent-policy.toml"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.runtime_principals.v1",
                        "status": "normative",
                        "dedicated_user": {"name": "cynic"},
                        "postures": {"bounded_yolo": {"forbidden_nopasswd_concerns": [], "forbidden_commands": []}},
                        "cortex_cli_policy": {
                            "required_agents": ["codex"],
                            "required_runtime_user": "cynic",
                            "required_posture": "bounded_yolo",
                            "projection": "infra/agent-policy.toml",
                        },
                        "projection_requirements": {"agent_policy_toml": {"path": "infra/agent-policy.toml"}},
                        "activation": {
                            "script": "organs/supervisor/entrypoints/runtime_principals.py",
                            "operator_commands": ["cynic update", "cynic update --dry-run"],
                            "primary_dependency_updates": {
                                "operator_command": "cynic update",
                                "allowed_update_commands": {"codex": ["codex", "update"]},
                                "forbidden_update_patterns": ["curl | sh"],
                                "required_projection_fields": ["operator_sequence", "proof_commands", "read_back", "done_when"],
                            },
                        },
                    }
                ),
                encoding="utf-8",
            )
            policy_path.write_text(
                """
[runtime]
authority = "axioms/runtime-principals-v1.json"
dedicated_user = "cynic"

[agents.codex]
runtime_user = "cynic"
execution_posture = "bounded_yolo"
""",
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.RUNTIME_PRINCIPALS_CONTRACT_PATH = contract_path
                module.AGENT_POLICY_PATH = policy_path
                findings = module.validate_runtime_principals_contract()
            finally:
                module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH = old_values

        self.assertIn("runtime_principals_update_projection_field_unprojected", [item.kind for item in findings])

    def test_rejects_runtime_principals_destructive_nopasswd(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            infra = root / "infra"
            contracts.mkdir(parents=True)
            (root / "organs" / "supervisor" / "entrypoints").mkdir(parents=True)
            infra.mkdir()
            (root / "organs" / "supervisor" / "entrypoints" / "runtime_principals.py").write_text(
                "choices=['status', 'plan', 'apply', 'install-defaults']\n",
                encoding="utf-8",
            )
            contract_path = contracts / "runtime-principals-v1.json"
            policy_path = infra / "agent-policy.toml"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.runtime_principals.v1",
                        "status": "normative",
                        "dedicated_user": {"name": "cynic"},
                        "postures": {"bounded_yolo": {"forbidden_nopasswd_concerns": ["infra-write"], "forbidden_commands": []}},
                        "cortex_cli_policy": {
                            "required_agents": ["codex"],
                            "required_runtime_user": "cynic",
                            "required_posture": "bounded_yolo",
                            "projection": "infra/agent-policy.toml",
                        },
                        "projection_requirements": {"agent_policy_toml": {"path": "infra/agent-policy.toml"}},
                        "activation": {"script": "organs/supervisor/entrypoints/runtime_principals.py"},
                    }
                ),
                encoding="utf-8",
            )
            policy_path.write_text(
                """
[runtime]
authority = "axioms/runtime-principals-v1.json"
dedicated_user = "cynic"

[concerns.infra-write]
nopasswd = true

[agents.codex]
runtime_user = "cynic"
execution_posture = "bounded_yolo"
""",
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.RUNTIME_PRINCIPALS_CONTRACT_PATH = contract_path
                module.AGENT_POLICY_PATH = policy_path
                findings = module.validate_runtime_principals_contract()
            finally:
                module.WORKSPACE_ROOT, module.RUNTIME_PRINCIPALS_CONTRACT_PATH, module.AGENT_POLICY_PATH = old_values

        self.assertIn("agent_policy_destructive_concern_nopasswd", [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
