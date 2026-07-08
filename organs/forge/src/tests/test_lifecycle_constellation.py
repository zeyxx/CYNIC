import unittest

from organs.forge.src.core.lifecycle_constellation import build_lifecycle_constellation_report
from organs.forge.src.tests.fixtures import registry_fixture


def ready_registry():
    registry = registry_fixture()
    for node in registry["nodes"]:
        node["status"] = "active"
        node["constellation"] = {
            "build_runner": "forge",
            "kernel_primary": "kernel",
            "storage_primary": "storage",
        }.get(node["role"], "unclassified")
        for key, value in list(node.get("network", {}).items()):
            if isinstance(value, str) and value.startswith("<"):
                node["network"][key] = f"10.20.0.{len(node['name'])}"
        node["activation_evidence"] = {"runtime_paths": {}, "services": {}}
        if node["role"] in {"build_runner", "kernel_primary"}:
            node["activation_evidence"]["runtime_paths"]["repo_root"] = {"verified": True}
        for service in node.get("services", []):
            node["activation_evidence"]["services"][service["name"]] = {
                "declared": True,
                "active": True,
            }
    return registry


class LifecycleConstellationTests(unittest.TestCase):
    def test_constellation_report_groups_lifecycle_nodes(self):
        report = build_lifecycle_constellation_report(ready_registry())

        self.assertEqual("cynic.lifecycle_constellation.v1", report["schema"])
        self.assertEqual("ok", report["status"])
        lifecycle_names = {node["name"] for node in report["lifecycle_nodes"]}
        self.assertIn("cynic-vm-anvil", lifecycle_names)
        self.assertIn("cynic-vm-core", lifecycle_names)
        self.assertIn("cynic-lxc-surrealdb", lifecycle_names)

    def test_constellation_report_blocks_when_required_service_evidence_is_missing(self):
        registry = ready_registry()
        build_node = next(node for node in registry["nodes"] if node["role"] == "build_runner")
        build_node["activation_evidence"]["services"]["gate-2"]["active"] = False

        report = build_lifecycle_constellation_report(registry)

        self.assertEqual("blocked", report["status"])
        build_report = next(node for node in report["lifecycle_nodes"] if node["role"] == "build_runner")
        self.assertFalse(build_report["activation"]["services"]["gate-2"])

    def test_constellation_report_preserves_forge_constellation_members(self):
        registry = ready_registry()
        registry["nodes"].append(
            {
                "name": "cynic-lxc-git",
                "role": "forge_primary",
                "constellation": "forge",
                "status": "active",
                "services": [{"name": "forgejo"}],
            }
        )

        report = build_lifecycle_constellation_report(registry)
        forge = next(item for item in report["constellations"] if item["name"] == "forge")

        self.assertEqual(2, forge["node_count"])
        self.assertIn("forge_primary", forge["roles"])


if __name__ == "__main__":
    unittest.main()
