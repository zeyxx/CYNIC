import unittest

from organs.forge.src.core.lifecycle_runtime_artifacts import (
    REQUIRED_STAGES,
    build_runtime_artifact_manifest,
)
from organs.forge.src.adapters.registry_adapter import validate_lifecycle_policy
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleRuntimeArtifactTests(unittest.TestCase):
    def test_manifest_normalizes_runtime_artifacts_under_one_lifecycle(self):
        manifest = build_runtime_artifact_manifest(
            registry_fixture(),
            now="2026-07-07T00:00:00+00:00",
        )

        self.assertEqual("cynic.lifecycle.runtime_artifact_manifest.v1", manifest["schema"])
        self.assertEqual("drift", manifest["status"])
        self.assertEqual(6, manifest["artifact_count"])
        by_name = {artifact["name"]: artifact for artifact in manifest["artifacts"]}
        self.assertEqual("rust_binary", by_name["cynic-mcp"]["kind"])
        self.assertEqual("rust_binary", by_name["cynic-cognitive-sensor"]["kind"])
        self.assertEqual("rust_binary", by_name["cynic-node"]["kind"])
        self.assertEqual(["pkg-config", "libxi-dev"], by_name["cynic-cognitive-sensor"]["system_packages"])
        self.assertEqual("python_organ", by_name["organ-forge"]["kind"])
        self.assertEqual("agent_projection", by_name["agent-exposure-projections"]["kind"])
        self.assertEqual("systemd_unit", by_name["lifecycle-systemd-units"]["kind"])
        self.assertEqual(REQUIRED_STAGES, by_name["cynic-mcp"]["lifecycle_stages"])
        self.assertIn("proof_drift", {item["kind"] for item in manifest["drift"]})

    def test_manifest_accepts_observed_proofs(self):
        observed = {
            "artifacts": {
                "cynic-mcp": {
                    "proof": {
                        "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py reports fresh": True,
                        "MCP initialize responds": True,
                        "MCP tools/list returns cynic_coord_claim": True,
                    }
                }
            }
        }

        manifest = build_runtime_artifact_manifest(
            registry_fixture(),
            observed=observed,
            now="2026-07-07T00:00:00+00:00",
        )

        mcp = next(artifact for artifact in manifest["artifacts"] if artifact["name"] == "cynic-mcp")
        self.assertEqual([], mcp["drift"])

    def test_registry_policy_requires_runtime_artifacts(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"].pop("runtime_artifacts")

        with self.assertRaisesRegex(RuntimeError, "runtime_artifacts missing cynic-mcp"):
            validate_lifecycle_policy(registry)

    def test_registry_policy_requires_mcp_artifact_to_cover_trigger_sources(self):
        registry = registry_fixture()
        mcp = registry["lifecycle"]["kernel_release"]["runtime_artifacts"][0]
        mcp["source_targets"].remove("infra/agent-exposure.json")

        with self.assertRaisesRegex(RuntimeError, "runtime_artifacts.cynic-mcp.source_targets"):
            validate_lifecycle_policy(registry)


if __name__ == "__main__":
    unittest.main()
