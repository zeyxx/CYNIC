import json
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "supervise_body.py"


def load_module():
    return load_module_from_path("supervise_body", MODULE_PATH)


class SuperviseBodyTests(unittest.TestCase):
    def test_ssot_freshness_helper_distinguishes_fresh_and_stale_state(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            ssot = root / "state" / "SSOT.json"
            ssot.parent.mkdir(parents=True, exist_ok=True)
            ssot.write_text(json.dumps({"status": "active"}), encoding="utf-8")

            age, status, fresh = module.ssot_freshness(ssot)
            self.assertEqual(status, "active")
            self.assertTrue(fresh)
            self.assertIsNotNone(age)

            old_ts = datetime.now(timezone.utc).timestamp() - (module.FRESH_SECONDS + 60)
            Path(ssot).touch()
            import os

            os.utime(ssot, (old_ts, old_ts))
            age, status, fresh = module.ssot_freshness(ssot)
            self.assertEqual(status, "active")
            self.assertFalse(fresh)
            self.assertGreaterEqual(age, module.FRESH_SECONDS)

    def test_inventory_classifies_declared_planned_and_runtime_wired_organs(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organs = root / "organs"
            infra_systemd = root / "infra" / "systemd"
            organs.mkdir()
            infra_systemd.mkdir(parents=True)

            registry = {
                "organs": [
                    {
                        "name": "organ-active-missing",
                        "instance": "organ-active-missing-daemon",
                        "status": "active",
                    },
                    {
                        "name": "organ-planned-cli",
                        "instance": "organ-planned-cli",
                        "status": "planned",
                        "cron": "on demand",
                    },
                    {
                        "name": "organ-reflection",
                        "instance": "organ-reflection-cron",
                        "status": "active",
                    },
                    {
                        "name": "organ-lifecycle",
                        "instance": "organ-lifecycle",
                        "status": "planned",
                    },
                ]
            }
            registry_path = root / "infra" / "registry.json"
            registry_path.parent.mkdir(exist_ok=True)
            registry_path.write_text(json.dumps(registry), encoding="utf-8")

            planned = organs / "planned-cli"
            (planned / "src" / "entrypoints").mkdir(parents=True)
            (planned / "MANIFEST.yaml").write_text("status: planned\n", encoding="utf-8")
            (planned / "src" / "entrypoints" / "cli.py").write_text("# cli\n", encoding="utf-8")

            reflection = organs / "reflection"
            (reflection / "systemd").mkdir(parents=True)
            (reflection / "MANIFEST.yaml").write_text("status: active\n", encoding="utf-8")
            (reflection / "systemd" / "organ-reflection-cron.service").write_text(
                "[Service]\nExecStart=/bin/true\n",
                encoding="utf-8",
            )

            lifecycle = organs / "lifecycle"
            lifecycle.mkdir(parents=True)
            (lifecycle / "MANIFEST.yaml").write_text("status: planned\n", encoding="utf-8")
            (lifecycle / "state").mkdir(parents=True)
            (lifecycle / "state" / "SSOT.json").write_text(
                json.dumps({"status": "active", "last_synced": "2026-06-29T20:00:00Z"}),
                encoding="utf-8",
            )
            (infra_systemd / "organ-lifecycle.service").write_text("[Service]\nExecStart=/bin/true\n", encoding="utf-8")
            (infra_systemd / "organ-lifecycle.timer").write_text("[Timer]\nUnit=organ-lifecycle.service\n", encoding="utf-8")

            old_values = (
                module.WORKSPACE_ROOT,
                module.ORGANS_ROOT,
                module.REGISTRY,
                module.INFRA_SYSTEMD,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = organs
                module.REGISTRY = registry_path
                module.INFRA_SYSTEMD = infra_systemd

                records, incoherences, repairs = module.build_inventory()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.ORGANS_ROOT,
                    module.REGISTRY,
                    module.INFRA_SYSTEMD,
                ) = old_values

        by_name = {record.name: record for record in records}
        self.assertEqual(by_name["organ-active-missing"].classification, "declared_without_filesystem")
        self.assertIn("registry_active_without_filesystem", by_name["organ-active-missing"].incoherences)
        self.assertEqual(by_name["organ-planned-cli"].classification, "declared_implemented")
        self.assertTrue(by_name["organ-planned-cli"].runtime_proven)
        self.assertEqual(by_name["organ-reflection"].classification, "declared_and_runtime_wired")
        self.assertTrue(by_name["organ-reflection"].runtime_proven)
        self.assertEqual(by_name["organ-lifecycle"].classification, "declared_implemented")
        self.assertIn("organ-lifecycle.service", by_name["organ-lifecycle"].infra_systemd_units)
        self.assertIn("organ-lifecycle.timer", by_name["organ-lifecycle"].infra_systemd_units)
        self.assertTrue(by_name["organ-lifecycle"].ssot_fresh)
        self.assertEqual(by_name["organ-lifecycle"].ssot_status, "active")
        self.assertEqual([repair.organ for repair in repairs], ["organ-active-missing"])
        self.assertTrue(any(item.kind == "registry_active_without_filesystem" for item in incoherences))

    def test_inventory_treats_legacy_blocked_manifest_without_registry_as_legacy_compat(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organs = root / "organs"
            infra_systemd = root / "infra" / "systemd"
            organs.mkdir()
            infra_systemd.mkdir(parents=True)

            registry_path = root / "infra" / "registry.json"
            registry_path.parent.mkdir(exist_ok=True)
            registry_path.write_text(json.dumps({"organs": []}), encoding="utf-8")

            legacy = organs / "wireguard"
            legacy.mkdir(parents=True)
            (legacy / "MANIFEST.yaml").write_text("status: legacy_blocked\n", encoding="utf-8")

            old_values = (
                module.WORKSPACE_ROOT,
                module.ORGANS_ROOT,
                module.REGISTRY,
                module.INFRA_SYSTEMD,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = organs
                module.REGISTRY = registry_path
                module.INFRA_SYSTEMD = infra_systemd

                records, incoherences, repairs = module.build_inventory()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.ORGANS_ROOT,
                    module.REGISTRY,
                    module.INFRA_SYSTEMD,
                ) = old_values

        self.assertEqual(1, len(records))
        self.assertEqual("legacy_compatibility_implementation", records[0].classification)
        self.assertEqual([], records[0].incoherences)
        self.assertEqual([], incoherences)
        self.assertEqual([], repairs)


if __name__ == "__main__":
    unittest.main()
