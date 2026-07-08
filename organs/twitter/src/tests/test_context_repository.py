import json
import tempfile
import unittest
from pathlib import Path

from organs.twitter.src.adapters.context_exporter import ContextBundleExporter
from organs.twitter.src.adapters.context_repository import JsonlContextRepository
from organs.twitter.src.core.entities import ContextBundle, SocialContextItem


class ContextRepositoryTest(unittest.TestCase):
    def test_add_item_deduplicates_by_stable_id(self):
        with tempfile.TemporaryDirectory() as tmp:
            dataset = Path(tmp) / "context_items.jsonl"
            repo = JsonlContextRepository(dataset)
            item = SocialContextItem(
                item_id="",
                source="manual",
                source_type="tweet",
                text="Solana local fee markets are changing validator behavior.",
                url="https://x.com/example/status/1",
                author="example",
                tags=["solana", "fees"],
            )

            first = repo.add_item(item)
            second = repo.add_item(item)

            self.assertEqual(first.item_id, second.item_id)
            self.assertEqual(repo.count_items(), 1)

    def test_query_requires_all_terms(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = JsonlContextRepository(Path(tmp) / "context_items.jsonl")
            repo.add_item(SocialContextItem("", "manual", "article", "Solana governance proposal", tags=["governance"]))
            repo.add_item(SocialContextItem("", "manual", "tweet", "Ethereum staking thread", tags=["staking"]))

            items = repo.list_items(query="solana governance", limit=10)

            self.assertEqual(len(items), 1)
            self.assertEqual(items[0].source_type, "article")


class ContextBundleExporterTest(unittest.TestCase):
    def test_export_writes_latest_json_and_markdown(self):
        with tempfile.TemporaryDirectory() as tmp:
            item = SocialContextItem(
                item_id="ctx1",
                source="manual",
                source_type="tweet",
                text="Useful market structure observation.",
                title="Market structure",
            )
            bundle = ContextBundle("bundle1", "2026-06-25T00:00:00+00:00", "market", 5, [item])

            paths = ContextBundleExporter(Path(tmp)).export_bundle(bundle)

            self.assertTrue(Path(paths["latest_markdown"]).exists())
            self.assertTrue(Path(paths["latest_json"]).exists())
            self.assertIn("Useful market structure", Path(paths["latest_markdown"]).read_text())
            payload = json.loads(Path(paths["latest_json"]).read_text())
            self.assertEqual(payload["items"][0]["item_id"], "ctx1")


if __name__ == "__main__":
    unittest.main()
