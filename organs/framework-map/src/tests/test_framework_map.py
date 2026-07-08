#!/usr/bin/env python3
import unittest

from pathlib import Path
import sys
import tempfile

SRC_ROOT = Path(__file__).resolve().parent.parent
REPO_ROOT = SRC_ROOT.parents[2]
for item in (REPO_ROOT, SRC_ROOT):
    if str(item) not in sys.path:
        sys.path.insert(0, str(item))

from adapters.contracts_dependency_inventory import ContractsDependencyInventory
from adapters.hermes_capability_discovery import HermesCapabilityDiscovery
from adapters.hermes_command_map_discovery import (
    HermesCommandMapDiscovery,
    _parse_root_help,
    _status_summary,
)
from adapters.hermes_kanban_mirror import HermesKanbanMirrorDiscovery, _observed_graph_surfaces
from adapters.hermes_skill_bridge import HermesSkillBridgeDiscovery, _split_frontmatter
from core.framework_map import FrameworkMapService, FrameworkTarget
from entrypoints.cli import map_hermes, surface_manifest


class FrameworkMapTests(unittest.TestCase):
    def test_map_hermes_reports_missing_source_as_gap(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            target = Path(tmpdir) / "missing-hermes"
            payload = FrameworkMapService(
                capability_discovery=HermesCapabilityDiscovery(),
                dependency_inventory=ContractsDependencyInventory(),
                command_map_discovery=HermesCommandMapDiscovery(),
                skill_bridge_discovery=HermesSkillBridgeDiscovery(),
                kanban_mirror_discovery=HermesKanbanMirrorDiscovery(),
                repo_root=REPO_ROOT,
            ).map_target(FrameworkTarget("hermes-agent", target, "definitely-missing-hermes-binary"))

        self.assertEqual("cynic.framework_map.report.v1", payload["schema"])
        self.assertGreaterEqual(payload["coverage_gap_count"], 1)
        self.assertEqual(0, payload["dependencies"]["dependency_count"])

    def test_map_hermes_with_source_root_composes_reports(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "README.md").write_text(
                "Hermes Agent has a learning loop, skills, Telegram gateway, cron scheduler, MCP tooling.",
                encoding="utf-8",
            )
            (root / "pyproject.toml").write_text(
                """
[project]
dependencies = ["openai==2.24.0", "croniter==6.0.0"]
[project.optional-dependencies]
mcp = ["mcp==1.26.0"]
""",
                encoding="utf-8",
            )
            (root / "agent").mkdir()
            (root / "tools").mkdir()
            (root / "tools" / "registry.py").write_text("", encoding="utf-8")
            (root / "toolsets.py").write_text(
                '_HERMES_CORE_TOOLS = ["terminal"]\nTOOLSETS = {"hermes-cli": {"description": "CLI", "tools": _HERMES_CORE_TOOLS, "includes": []}}\n',
                encoding="utf-8",
            )
            (root / "providers").mkdir()
            (root / "hermes_cli" / "subcommands").mkdir(parents=True)
            (root / "hermes_cli" / "subcommands" / "tools.py").write_text(
                'def wire(sub):\n    sub.add_parser("list")\n',
                encoding="utf-8",
            )
            payload = FrameworkMapService(
                capability_discovery=HermesCapabilityDiscovery(),
                dependency_inventory=ContractsDependencyInventory(),
                command_map_discovery=HermesCommandMapDiscovery(),
                skill_bridge_discovery=HermesSkillBridgeDiscovery(),
                kanban_mirror_discovery=HermesKanbanMirrorDiscovery(),
                repo_root=REPO_ROOT,
            ).map_target(FrameworkTarget("hermes-agent", root, "definitely-missing-hermes-binary"))

        self.assertEqual("hermes-agent", payload["framework"])
        self.assertGreater(payload["dependencies"]["dependency_count"], 0)
        self.assertGreater(payload["capabilities"]["capability_count"], 0)
        self.assertIn("architecture_patterns", payload)
        self.assertIn("cynic_integration", payload)
        self.assertIn("command_map", payload)
        self.assertIn("skill_bridge", payload)
        self.assertIn("kanban_mirror", payload)
        self.assertGreater(payload["source_inventory"]["toolset_count"], 0)
        self.assertGreater(payload["source_inventory"]["cli_command_count"], 0)
        self.assertIn("bridges", payload["cynic_integration"])
        self.assertIn("command_surface", payload["cynic_integration"])
        self.assertEqual("cynic.hermes_agent_command_map.v1", payload["command_map"]["schema"])
        self.assertEqual("cynic.hermes_skill_bridge.v1", payload["skill_bridge"]["schema"])
        self.assertEqual("cynic.hermes_kanban_kernel_mirror.v1", payload["kanban_mirror"]["schema"])

    def test_cli_surface_manifest(self):
        payload = surface_manifest()
        schemas = {item["schema"] for item in payload["registry"]["reports"]}

        self.assertIn("cynic.framework_map.report.v1", schemas)

    def test_cli_map_hermes_accepts_missing_default(self):
        payload = map_hermes("external/hermes-agent", "definitely-missing-hermes-binary", output=None)

        self.assertEqual("cynic.framework_map.report.v1", payload["schema"])

    def test_parse_root_help_merges_wrapped_descriptions(self):
        payload = _parse_root_help(
            """
positional arguments:
    chat                Interactive chat with the agent
    kanban              Multi-profile collaboration board
                        with task links and comments

options:
"""
        )

        self.assertEqual("chat", payload[0]["name"])
        self.assertEqual("Interactive chat with the agent", payload[0]["purpose"])
        self.assertEqual(
            "Multi-profile collaboration board with task links and comments",
            payload[1]["purpose"],
        )

    def test_status_summary_extracts_runtime_snapshot(self):
        summary = _status_summary(
            """
◆ Environment
  Model:        default
  Provider:     Mixture of Agents
◆ Terminal Backend
  Backend:      local
◆ Messaging Platforms
  Slack         ✓ configured
  Telegram      ✗ not configured
◆ Gateway Service
  Status:       ✓ running
◆ Scheduled Jobs
  Jobs:         2
◆ Sessions
  Active:       1
◆ API Keys
  Google / Gemini  ✓ AIza...abcd
"""
        )

        self.assertEqual("Mixture of Agents", summary["provider"])
        self.assertEqual("default", summary["model"])
        self.assertEqual("local", summary["terminal_backend"])
        self.assertEqual("✓ running", summary["gateway_status"])
        self.assertEqual(2, summary["scheduled_jobs"])
        self.assertEqual(1, summary["active_sessions"])
        self.assertEqual(["Slack"], summary["configured_messaging_platforms"])

    def test_split_frontmatter_extracts_name_and_tags(self):
        header, body = _split_frontmatter(
            """---
name: sample-skill
tags: [python, audit]
description: Example
---
# Body
"""
        )

        self.assertEqual("sample-skill", header["name"])
        self.assertEqual(["python", "audit"], header["tags"])
        self.assertIn("# Body", body)

    def test_observed_graph_surfaces_maps_events_to_kernel_graph_surface_names(self):
        observed = _observed_graph_surfaces(
            [
                "task_received",
                "kanban_created",
                "kanban_status",
                "kanban_completed",
                "kanban_terminal_non_success",
                "task_finished",
            ]
        )

        self.assertIn("created", observed)
        self.assertIn("status", observed)
        self.assertIn("completed", observed)
        self.assertIn("runs", observed)
        self.assertNotIn("terminal_non_success", observed)


if __name__ == "__main__":
    unittest.main()
