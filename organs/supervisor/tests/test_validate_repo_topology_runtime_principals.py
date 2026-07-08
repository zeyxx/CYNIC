import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path, resolve_project_root
    from .contract_authority_fixture import validate_complete_contract_authority_fixture
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path, resolve_project_root
    from contract_authority_fixture import validate_complete_contract_authority_fixture


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"
REPO_ROOT = resolve_project_root(MODULE_PATH)


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


class RuntimePrincipalTopologyProjectionTests(unittest.TestCase):
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
    def test_validates_contract_authorities_and_mcp_paths(self):
        findings = validate_complete_contract_authority_fixture(load_module())

        self.assertEqual([], [item for item in findings if item.severity == "critical"])


    def test_rejects_hardcoded_project_path_in_mcp(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            mcp_src = root / "crates" / "cynic-mcp" / "src"
            mcp_src.mkdir(parents=True)
            (mcp_src / "proxy.rs").write_text(
                'let path = "/' + 'home/user/Bureau/example";\n',
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.MCP_SRC_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.MCP_SRC_ROOT = mcp_src
                findings = module.validate_mcp_no_absolute_project_paths()
            finally:
                module.WORKSPACE_ROOT, module.MCP_SRC_ROOT = old_values

        self.assertEqual("hardcoded_project_path_in_mcp", findings[0].kind)

    def test_rejects_draft_contract_without_governance(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            contract_path = contracts / "surface-v1.json"
            contract_path.write_text(
                json.dumps({"schema": "cynic.test_surface.v1", "status": "draft", "draft_reason": "test"}),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.GOVERNED_CONTRACT_PATHS)
            try:
                module.WORKSPACE_ROOT = root
                module.GOVERNED_CONTRACT_PATHS = [contract_path]
                findings = module.validate_draft_contract_governance()
            finally:
                module.WORKSPACE_ROOT, module.GOVERNED_CONTRACT_PATHS = old_values

        self.assertEqual(
            ["decision_authority", "promotion_condition"],
            sorted(item.detail.removeprefix("draft contracts must declare ").removesuffix(".") for item in findings),
        )
        self.assertTrue(all(item.kind == "draft_contract_governance_missing" for item in findings))

    def test_accepts_runtime_principals_projection_for_bounded_yolo(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            infra = root / "infra"
            contracts.mkdir(parents=True)
            (root / "organs" / "supervisor" / "entrypoints").mkdir(parents=True)
            infra.mkdir()
            (root / "organs" / "supervisor" / "entrypoints" / "runtime_principals.py").write_text(
                "choices=['status', 'plan', 'update', 'apply', 'install-defaults']\n"
                'PRIMARY_UPDATE_COMMANDS = {"codex": ["codex", "update"], "agy": ["agy", "update"]}\n'
                'help_text = "cynic update cynic update --dry-run"\n',
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
                        "postures": {
                            "bounded_yolo": {
                                "forbidden_nopasswd_concerns": ["infra-write", "network"],
                                "forbidden_commands": ["rm -rf /", "git reset --hard HEAD~"],
                            }
                        },
                        "cortex_cli_policy": {
                            "required_agents": ["codex", "antigravity"],
                            "required_runtime_user": "cynic",
                            "required_posture": "bounded_yolo",
                            "projection": "infra/agent-policy.toml",
                        },
                        "projection_requirements": {"agent_policy_toml": {"path": "infra/agent-policy.toml"}},
                        "activation": {
                            "script": "organs/supervisor/entrypoints/runtime_principals.py",
                            "primary_dependency_updates": {
                                "operator_command": "cynic update",
                                "allowed_update_commands": {
                                    "codex": ["codex", "update"],
                                    "agy": ["agy", "update"],
                                },
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

[concerns.infra-write]
nopasswd = false

[concerns.network]
nopasswd = false

[agents.codex]
runtime_user = "cynic"
execution_posture = "bounded_yolo"

[agents.antigravity]
runtime_user = "cynic"
execution_posture = "bounded_yolo"

[deny]
commands = ["rm -rf /", "git reset --hard HEAD~"]
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

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_runtime_principals_projection_user_mismatch(self):
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
                        "postures": {"bounded_yolo": {"forbidden_nopasswd_concerns": [], "forbidden_commands": []}},
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

[agents.codex]
runtime_user = "user"
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

        self.assertIn("agent_policy_runtime_user_mismatch", [item.kind for item in findings])

    def test_rejects_runtime_update_patterns_missing_infra_forbidden_on_cortex(self):
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
            (entrypoints / "runtime_principals.py").write_text("choices=['status', 'update']\n", encoding="utf-8")
            (runtime_src / "runtime_principals_actions.py").write_text(
                'PRIMARY_UPDATE_COMMANDS = {"codex": ["codex", "update"]}\n'
                'help_text = "cynic update --dry-run cynic update"\n',
                encoding="utf-8",
            )
            contract_path = contracts / "runtime-principals-v1.json"
            infra_contract_path = contracts / "infra-consumption-v1.json"
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
                                "forbidden_update_patterns": ["cargo build -p cynic-mcp"],
                            },
                        },
                    }
                ),
                encoding="utf-8",
            )
            infra_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.infra_consumption.v1",
                        "status": "normative",
                        "native_lifecycle": {
                            "forbidden_on_cortex": [
                                "cargo build -p cynic-mcp",
                                "cp target/release/cynic-kernel",
                            ]
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

        self.assertIn(
            "runtime_principals_update_forbidden_patterns_incomplete",
            [item.kind for item in findings],
        )

if __name__ == "__main__":
    unittest.main()
