import json
import tempfile
import unittest
from pathlib import Path

from organs.twitter.src.adapters.context_repository import JsonlContextRepository
from organs.twitter.src.adapters.x_resolver import (
    CompositeSocialSourceResolver,
    LocalHermesDatasetResolver,
    RawCaptureResolver,
    extract_article_id,
    extract_handle,
    extract_tweet_id,
    extract_x_article_from_visible_text,
)
from organs.twitter.src.core.use_cases import IngestSocialUrl


class XResolverTest(unittest.TestCase):
    def test_extracts_tweet_identity_from_status_url(self):
        url = "https://x.com/TheAhmadOsman/status/2057183854444843202?s=20"

        self.assertEqual(extract_handle(url), "TheAhmadOsman")
        self.assertEqual(extract_tweet_id(url), "2057183854444843202")
        self.assertEqual(extract_article_id("https://x.com/i/article/2057179946351534080"), "2057179946351534080")
        self.assertEqual(extract_handle("https://x.com/i/article/2057179946351534080"), "")

    def test_ingest_url_resolves_from_local_dataset(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            hermes_dataset = tmp_path / "hermes_dataset.jsonl"
            context_dataset = tmp_path / "context_items.jsonl"
            hermes_dataset.write_text(
                json.dumps({
                    "tweet_id": "2057183854444843202",
                    "author_screen_name": "TheAhmadOsman",
                    "text": "Local dataset tweet text.",
                    "cashtags": ["SOL"],
                    "narratives": ["infrastructure"],
                }) + "\n",
                encoding="utf-8",
            )

            resolver = CompositeSocialSourceResolver([LocalHermesDatasetResolver(hermes_dataset)])
            item = IngestSocialUrl(resolver, JsonlContextRepository(context_dataset)).execute(
                "https://x.com/TheAhmadOsman/status/2057183854444843202?s=20",
                tags=["operator"],
            )

            self.assertEqual(item.author, "TheAhmadOsman")
            self.assertEqual(item.text, "Local dataset tweet text.")
            self.assertIn("SOL", item.tags)
            self.assertIn("operator", item.tags)
            self.assertEqual(JsonlContextRepository(context_dataset).count_items(), 1)

    def test_ingest_url_resolves_from_raw_capture_article(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            captures = tmp_path / "captures"
            captures.mkdir()
            context_dataset = tmp_path / "context_items.jsonl"
            (captures / "capture.json").write_text(
                json.dumps({
                    "tweet_results": {
                        "result": {
                            "rest_id": "2057183854444843202",
                            "legacy": {
                                "id_str": "2057183854444843202",
                                "full_text": "https://t.co/example",
                                "favorite_count": 884,
                            },
                            "core": {
                                "user_results": {
                                    "result": {
                                        "core": {"screen_name": "TheAhmadOsman", "name": "Ahmad"},
                                        "legacy": {"followers_count": 60120},
                                    }
                                }
                            },
                            "article": {
                                "article_results": {
                                    "result": {
                                        "rest_id": "2057179946351534080",
                                        "title": "Inference Engines for LLMs & Local AI Hardware",
                                        "preview_text": "You do not pick an inference engine first.",
                                    }
                                }
                            },
                        }
                    }
                }),
                encoding="utf-8",
            )

            resolver = CompositeSocialSourceResolver([RawCaptureResolver(captures)])
            item = IngestSocialUrl(resolver, JsonlContextRepository(context_dataset)).execute(
                "https://x.com/TheAhmadOsman/status/2057183854444843202?s=20",
            )

            self.assertEqual(item.author, "TheAhmadOsman")
            self.assertIn("Inference Engines", item.title)
            self.assertIn("Article preview", item.text)
            self.assertIn("x-article", item.tags)
            self.assertEqual(item.metadata["article_extraction_status"], "preview_only")
            self.assertFalse(item.metadata["article_full_text_available"])

    def test_extracts_full_article_from_authenticated_x_visible_text(self):
        rendered = """
To view keyboard shortcuts, press question mark
Post
Inference Engines for LLMs & Local AI Hardware (2026 Edition)
Ahmad
@TheAhmadOsman
·
May 20
Follow
You don't pick an inference engine first. You pick a hardware strategy, a workload shape, and a serving model. The engine follows.
That is the most useful way to think about LLM inference engines.
""" + "\n".join(f"Body line {index} with enough context for extraction." for index in range(80)) + """
Until next time.
-Ahmad
Want to publish your own Article?
Upgrade to Premium
"""

        article = extract_x_article_from_visible_text(
            rendered,
            source_url="https://x.com/TheAhmadOsman/article/2057183854444843202",
        )

        self.assertIsNotNone(article)
        assert article is not None
        self.assertEqual(article["title"], "Inference Engines for LLMs & Local AI Hardware (2026 Edition)")
        self.assertEqual(article["author"], "Ahmad (@TheAhmadOsman)")
        self.assertEqual(article["published"], "May 20")
        self.assertEqual(article["article_id"], "2057183854444843202")
        self.assertIn("You don't pick an inference engine first", article["text"])
        self.assertIn("-Ahmad", article["text"])
        self.assertNotIn("Upgrade to Premium", article["text"])


if __name__ == "__main__":
    unittest.main()
