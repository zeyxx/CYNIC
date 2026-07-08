import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


SOURCE_ROOT = resolve_repo_root(Path(__file__).resolve()) / "organs" / "exposure" / "src"
MODULE_PATH = SOURCE_ROOT / "core" / "services.py"


def load_services_module():
    package_root = SOURCE_ROOT
    if str(package_root) not in sys.path:
        sys.path.insert(0, str(package_root))
    spec = importlib.util.spec_from_file_location("core.services", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules["core.services"] = module
    spec.loader.exec_module(module)
    return module


class StaticExposureSource:
    def __init__(self, graph):
        self._graph = graph

    def load(self):
        return self._graph


class ExposureValidationTests(unittest.TestCase):
    def exposure_graph(self):
        services = load_services_module()
        return services.AgentExposureGraph.from_dict(
            {
                "version": 1,
                "projection_targets": {
                    "codex": ".codex/config.toml",
                    "claude": ".mcp.json",
                    "antigravity": ".antigravity/settings.json",
                },
                "mcp_servers": [
                    {
                        "name": "cynic-coord",
                        "command": "python3",
                        "args": ["./.cortex/mcp/run_cynic_mcp.py"],
                        "agents": ["codex", "claude", "antigravity"],
                        "tools": [{"name": "service_health", "approval_mode": "auto"}],
                    },
                    {
                        "name": "cynic-browser",
                        "command": "python3",
                        "args": ["./.cortex/mcp/run_browser_mcp.py"],
                        "agents": ["codex", "claude", "antigravity"],
                        "tools": [{"name": "browser_status", "approval_mode": "auto"}],
                    },
                ],
                "mcp_tool_domains": {
                    "organism_status": {
                        "tools": ["service_health"],
                        "fallback_tools": [],
                    }
                },
            }
        )

    def test_render_claude_mcp_json_uses_json_client_aliases(self):
        services = load_services_module()
        rendered = services.ClaudeMcpJsonProjection().render(self.exposure_graph(), "claude")
        payload = json.loads(rendered)

        self.assertEqual(
            {
                "cynic": {"command": "python3", "args": ["./.cortex/mcp/run_cynic_mcp.py"]},
                "browser": {"command": "python3", "args": ["./.cortex/mcp/run_browser_mcp.py"]},
            },
            payload["mcpServers"],
        )

    def test_render_antigravity_settings_uses_json_client_aliases_and_agents(self):
        services = load_services_module()
        rendered = services.AntigravitySettingsProjection().render(
            self.exposure_graph(),
            "antigravity",
        )
        payload = json.loads(rendered)

        self.assertEqual(
            {
                "cynic": {"command": "python3", "args": ["./.cortex/mcp/run_cynic_mcp.py"]},
                "browser": {"command": "python3", "args": ["./.cortex/mcp/run_browser_mcp.py"]},
            },
            payload["mcpServers"],
        )
        self.assertTrue(payload["agents"]["generalist"]["enabled"])

    def test_validate_contract_alignment_rejects_declared_agent_without_projection(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text('#[tool(name = "service_health", description = "x")]\n', encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            graph = self.exposure_graph()
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts)

        self.assertFalse(report.ok)
        self.assertEqual("declared_agent_without_projection", report.findings[0].kind)

    def test_validate_projection_files_rejects_drift(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            target = root / ".mcp.json"
            target.write_text("{}\n", encoding="utf-8")
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "projection_targets": {"claude": ".mcp.json"},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["claude"],
                            "tools": [],
                        }
                    ],
                }
            )
            findings = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"claude": services.ClaudeMcpJsonProjection()},
            ).validate_projection_files(root)

        self.assertEqual("projection_target_drift", findings[0].kind)

    def test_validate_projection_files_accepts_matching_target(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "projection_targets": {"claude": ".mcp.json"},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["claude"],
                            "tools": [],
                        }
                    ],
                }
            )
            projection = services.ClaudeMcpJsonProjection()
            (root / ".mcp.json").write_text(projection.render(graph, "claude"), encoding="utf-8")
            findings = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"claude": projection},
            ).validate_projection_files(root)

        self.assertEqual([], findings)

    def test_apply_projection_files_check_reports_drift_without_writing(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "projection_targets": {"claude": ".mcp.json"},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["claude"],
                            "tools": [],
                        }
                    ],
                }
            )
            target = root / ".mcp.json"
            target.write_text("{}\n", encoding="utf-8")
            result = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"claude": services.ClaudeMcpJsonProjection()},
            ).apply_projection_files(root, check=True)
            target_content = target.read_text(encoding="utf-8")

        self.assertFalse(result["ok"])
        self.assertTrue(result["changed"])
        self.assertEqual("{}\n", target_content)

    def test_apply_projection_files_writes_changed_target(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "projection_targets": {"claude": ".mcp.json"},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["claude"],
                            "tools": [],
                        }
                    ],
                }
            )
            projection = services.ClaudeMcpJsonProjection()
            service = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"claude": projection},
            )
            result = service.apply_projection_files(root)

            self.assertTrue(result["ok"])
            self.assertTrue(result["changed"])
            self.assertEqual(projection.render(graph, "claude"), (root / ".mcp.json").read_text(encoding="utf-8"))

    def test_validate_contract_alignment_accepts_exposed_implemented_tool(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text('#[tool(name = "service_health", description = "x")]\n', encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            (contracts / "infrastructure.json").write_text(
                json.dumps(
                    {
                        "operation": {
                            "mcp_tool": "service_health",
                            "implementation_status": "implemented_local",
                        }
                    }
                ),
                encoding="utf-8",
            )
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "mcp_tool_domains": {"organism_status": {"tools": ["service_health"]}},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["codex"],
                            "tools": [{"name": "service_health", "approval_mode": "auto"}],
                        }
                    ],
                }
            )
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts)

        self.assertTrue(report.ok)
        self.assertEqual([], report.findings)

    def test_validate_contract_alignment_rejects_exposed_tool_missing_in_rust(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text("", encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "mcp_tool_domains": {"organism_status": {"tools": ["ghost_tool"]}},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["codex"],
                            "tools": [{"name": "ghost_tool", "approval_mode": "auto"}],
                        }
                    ],
                }
            )
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts)

        self.assertFalse(report.ok)
        self.assertEqual("exposed_tool_missing_in_rust", report.findings[0].kind)

    def test_validate_contract_alignment_rejects_implemented_contract_tool_not_exposed(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text('#[tool(name = "service_health", description = "x")]\n', encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            (contracts / "infrastructure.json").write_text(
                json.dumps(
                    {
                        "operation": {
                            "mcp_tool": "service_health",
                            "implementation_status": "implemented_local",
                        }
                    }
                ),
                encoding="utf-8",
            )
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "mcp_tool_domains": {"organism_status": {"tools": []}},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["codex"],
                            "tools": [],
                        }
                    ],
                }
            )
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts)

        self.assertFalse(report.ok)
        self.assertIn("implemented_contract_tool_not_exposed", {item.kind for item in report.findings})

    def test_validate_contract_alignment_rejects_exposed_tool_without_domain(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text('#[tool(name = "service_health", description = "x")]\n', encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "mcp_tool_domains": {},
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["codex"],
                            "tools": [{"name": "service_health", "approval_mode": "auto"}],
                        }
                    ],
                }
            )
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts)

        self.assertFalse(report.ok)
        self.assertEqual("exposed_tool_missing_domain", report.findings[0].kind)

    def test_validate_contract_alignment_rejects_fallback_tool_missing_in_python(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text('#[tool(name = "cynic_health", description = "x")]\n', encoding="utf-8")
            fallback = root / "python_rest_mcp.py"
            fallback.write_text('TOOLS = {}\n', encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "mcp_tool_domains": {
                        "organism_status": {
                            "tools": ["cynic_health"],
                            "fallback_tools": ["cynic_health"],
                        }
                    },
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["codex"],
                            "tools": [{"name": "cynic_health", "approval_mode": "auto"}],
                        }
                    ],
                }
            )
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts, python_fallback_path=fallback)

        self.assertFalse(report.ok)
        self.assertEqual("fallback_tool_missing_in_python", report.findings[0].kind)

    def test_validate_contract_alignment_accepts_fallback_tool_in_python(self):
        services = load_services_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proxy = root / "proxy.rs"
            proxy.write_text('#[tool(name = "cynic_health", description = "x")]\n', encoding="utf-8")
            fallback = root / "python_rest_mcp.py"
            fallback.write_text('"cynic_health": _tool("cynic_health", "x", {}, handler)\n', encoding="utf-8")
            contracts = root / "contracts"
            contracts.mkdir()
            graph = services.AgentExposureGraph.from_dict(
                {
                    "version": 1,
                    "mcp_tool_domains": {
                        "organism_status": {
                            "tools": ["cynic_health"],
                            "fallback_tools": ["cynic_health"],
                        }
                    },
                    "mcp_servers": [
                        {
                            "name": "cynic-coord",
                            "command": "./run.sh",
                            "agents": ["codex"],
                            "tools": [{"name": "cynic_health", "approval_mode": "auto"}],
                        }
                    ],
                }
            )
            report = services.ExposureProjectionService(
                source=StaticExposureSource(graph),
                projections={"codex": services.CodexTomlProjection()},
            ).validate_contract_alignment(proxy, contracts, python_fallback_path=fallback)

        self.assertTrue(report.ok)


if __name__ == "__main__":
    unittest.main()
