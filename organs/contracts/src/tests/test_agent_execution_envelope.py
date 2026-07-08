#!/usr/bin/env python3
import unittest

from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


CORE_PATH = Path(__file__).resolve().parent.parent / "core" / "agent_execution_envelope.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


class AgentExecutionEnvelopeTests(unittest.TestCase):
    def test_example_round_trips_and_preserves_n_routes(self):
        module = load_module_from_path("agent_execution_envelope_core", CORE_PATH)

        envelope = module.example_agent_execution_envelope()
        payload = envelope.to_dict()
        parsed = module.AgentExecutionEnvelope.from_dict(payload)

        self.assertEqual("cynic.agent_execution_envelope.v1", payload["schema"])
        self.assertEqual("example-cynic-task-1", parsed.task_id)
        self.assertGreaterEqual(len(parsed.use_cases[0].paths), 3)
        self.assertGreaterEqual(len(parsed.route_candidates), 2)
        self.assertIn("human_cortex", parsed.use_cases[0].fallback_routes)
        self.assertIn("Could this be deterministic?", parsed.use_cases[0].what_else)

    def test_rejects_route_without_evidence_contract(self):
        module = load_module_from_path("agent_execution_envelope_core_invalid", CORE_PATH)

        with self.assertRaises(RuntimeError):
            module.AgentExecutionRouteCandidate(
                id="unsafe",
                provider="hermes-agent",
                transport="cli",
                why=("no evidence",),
                required_capabilities=("agent_task_execution",),
                allowed_toolsets=("terminal",),
                forbidden_toolsets=(),
                side_effect_policy=("external_process",),
                risk="medium",
                evidence_required=(),
            )

    def test_rejects_unknown_transport(self):
        module = load_module_from_path("agent_execution_envelope_core_transport", CORE_PATH)

        payload = module.example_agent_execution_envelope().to_dict()
        payload["route_candidates"][0]["transport"] = "telepathy"

        with self.assertRaises(RuntimeError):
            module.AgentExecutionEnvelope.from_dict(payload)

    def test_cli_manifest_exposes_execution_envelope_example(self):
        module = load_module_from_path("contracts_cli_execution_envelope", CLI_PATH)
        manifest = module.surface_manifest()
        schemas = {item["schema"] for item in manifest["registry"]["reports"]}

        self.assertIn("cynic.agent_execution_envelope.v1", schemas)

    def test_cli_example_outputs_round_trippable_envelope(self):
        cli = load_module_from_path("contracts_cli_execution_envelope_example", CLI_PATH)
        core = load_module_from_path("agent_execution_envelope_core_cli", CORE_PATH)

        payload = cli.agent_execution_envelope_example([])
        parsed = core.AgentExecutionEnvelope.from_dict(payload)

        self.assertEqual("framework-interop", parsed.domain)
        self.assertEqual("hermes-agent", parsed.route_candidates[0].provider)


if __name__ == "__main__":
    unittest.main()
