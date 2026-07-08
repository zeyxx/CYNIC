#!/usr/bin/env python3
import unittest

from pathlib import Path
import tempfile

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


CORE_PATH = Path(__file__).resolve().parent.parent / "core" / "agent_framework_capabilities.py"
ADAPTER_PATH = Path(__file__).resolve().parent.parent / "adapters" / "hermes_agent_capabilities.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


class AgentFrameworkCapabilitiesTests(unittest.TestCase):
    def test_capability_registry_round_trip(self):
        module = load_module_from_path("agent_framework_capabilities_core", CORE_PATH)
        capability = module.AgentFrameworkCapability(
            id="repo_review",
            input_schemas=("cynic.git_matter_review_request.v1",),
            output_schemas=("cynic.git_matter_review.v1",),
            side_effects=("repo_read", "repo_write_requires_claim"),
            confidence="observed",
        )
        registry = module.AgentFrameworkCapabilities(
            framework="hermes-agent",
            version="hermes 0.1.0",
            observed_at="2026-07-01T00:00:00Z",
            capabilities=(capability,),
            evidence=({"kind": "command", "status": "observed"},),
        )

        payload = registry.to_dict()
        parsed = module.AgentFrameworkCapabilities.from_dict(payload)

        self.assertEqual("cynic.agent_framework_capabilities.v1", payload["schema"])
        self.assertEqual("hermes-agent", parsed.framework)
        self.assertEqual("repo_review", parsed.capabilities[0].id)
        self.assertEqual(1, payload["capability_count"])

    def test_rejects_unknown_side_effect(self):
        module = load_module_from_path("agent_framework_capabilities_core_invalid", CORE_PATH)
        with self.assertRaises(RuntimeError):
            module.AgentFrameworkCapability(
                id="bad",
                input_schemas=("cynic.input.v1",),
                output_schemas=("cynic.output.v1",),
                side_effects=("repo_write",),
                confidence="observed",
            )

    def test_hermes_scanner_infers_repo_review_from_help(self):
        adapter = load_module_from_path("hermes_agent_capabilities_adapter", ADAPTER_PATH)

        def runner(argv):
            if argv[-1] == "--version":
                return adapter.CommandResult(argv=argv, returncode=0, stdout="hermes 1.2.3\n", stderr="")
            return adapter.CommandResult(
                argv=argv,
                returncode=0,
                stdout="Commands: chat, review git repository diffs\n",
                stderr="",
            )

        payload = adapter.HermesAgentCapabilityScanner(
            runner=runner,
            skill_roots=(),
        ).scan("2026-07-01T00:00:00Z").to_dict()
        capabilities = {item["id"]: item for item in payload["capabilities"]}

        self.assertEqual("hermes 1.2.3", payload["version"])
        self.assertIn("agent_task_execution", capabilities)
        self.assertIn("repo_review", capabilities)
        self.assertEqual("inferred", capabilities["repo_review"]["confidence"])

    def test_hermes_scanner_reads_skill_evidence(self):
        adapter = load_module_from_path("hermes_agent_capabilities_adapter_skill", ADAPTER_PATH)

        def runner(argv):
            return adapter.CommandResult(argv=argv, returncode=127, stdout="", stderr="missing")

        with tempfile.TemporaryDirectory() as tmpdir:
            skill = Path(tmpdir) / "repo-review" / "SKILL.md"
            skill.parent.mkdir()
            skill.write_text("Review git repository changes and produce an audit.", encoding="utf-8")
            payload = adapter.HermesAgentCapabilityScanner(
                runner=runner,
                skill_roots=(Path(tmpdir),),
            ).scan("2026-07-01T00:00:00Z").to_dict()

        self.assertEqual(["repo_review"], [item["id"] for item in payload["capabilities"]])
        self.assertTrue(any(item["kind"] == "skill" for item in payload["evidence"]))

    def test_hermes_scanner_derives_source_capabilities(self):
        adapter = load_module_from_path("hermes_agent_capabilities_adapter_source", ADAPTER_PATH)

        def runner(argv):
            return adapter.CommandResult(argv=argv, returncode=127, stdout="", stderr="missing")

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "README.md").write_text(
                "Learning loop with skills self-improve, Telegram gateway, cron scheduler, "
                "subagents, Modal and Daytona terminal backends, MCP tooling, dashboard, "
                "browser computer-use, voice TTS, and batch trajectory generation.",
                encoding="utf-8",
            )
            (root / "pyproject.toml").write_text(
                """
[project]
dependencies = ["openai==2.24.0", "croniter==6.0.0"]
[project.optional-dependencies]
mcp = ["mcp==1.26.0"]
acp = ["agent-client-protocol==0.9.0"]
voice = ["faster-whisper==1.2.1"]
""",
                encoding="utf-8",
            )
            (root / "gateway").mkdir()
            (root / "skills").mkdir()
            payload = adapter.HermesAgentCapabilityScanner(
                runner=runner,
                skill_roots=(),
                source_root=root,
            ).scan("2026-07-01T00:00:00Z").to_dict()

        capabilities = {item["id"] for item in payload["capabilities"]}
        self.assertIn("closed_learning_loop", capabilities)
        self.assertIn("messaging_gateway", capabilities)
        self.assertIn("scheduled_automation", capabilities)
        self.assertIn("mcp_tooling", capabilities)
        self.assertEqual([], payload.get("coverage_gaps", []))
        self.assertEqual(2, payload["source_summary"]["source_files_observed"])

    def test_cli_manifest_exposes_capability_report(self):
        module = load_module_from_path("contracts_cli_capabilities", CLI_PATH)
        manifest = module.surface_manifest()
        schemas = {item["schema"] for item in manifest["registry"]["reports"]}

        self.assertIn("cynic.agent_framework_capabilities.v1", schemas)


if __name__ == "__main__":
    unittest.main()
