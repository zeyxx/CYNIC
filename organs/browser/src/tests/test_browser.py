import unittest
import datetime
import tempfile
import shutil

from organs.browser.src.adapters import (
    BrowserHubAdapter,
    CynicKernelObservationAdapter,
    FilesystemEvidenceStore,
    RegexSecretScannerAdapter,
    SearXNGAdapter,
    FirecrawlAdapter,
)
from organs.browser.src.core.entities import EvidenceItem, HumanGate, NavigationIntent, PageEvidence
from organs.browser.src.core.services import BrowserOrganService

class TestSecretScanner(unittest.TestCase):
    def setUp(self):
        self.scanner = RegexSecretScannerAdapter()

    def test_sanitize_solana_key(self):
        key = "5rW7qBvG9H8sP6wT3qA2z1x4y5u7t8m9qA2z1x4y5u7t"
        text = f"My key is {key}"
        sanitized = self.scanner.sanitize(text)
        self.assertNotIn(key, sanitized)
        self.assertIn("[REDACTED_BASE58_KEY]", sanitized)

    def test_sanitize_openai_key(self):
        key = "sk-" + "abcdefghijklmnopqrstuvwxyz0123456789ABCDEF"
        text = f"My key is {key}"
        sanitized = self.scanner.sanitize(text)
        self.assertNotIn(key, sanitized)
        self.assertIn("[REDACTED_OPENAI_KEY]", sanitized)

class TestBrowserService(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.evidence_store = FilesystemEvidenceStore(base_dir=self.temp_dir)
        self.scanner = RegexSecretScannerAdapter()
        self.kernel_obs = CynicKernelObservationAdapter(fallback_dir=self.temp_dir)
        
        class MockSearch(SearXNGAdapter):
            def search(self, query, limit=5):
                return [{"title": "Mock", "url": "http://mock.com", "content": "Mock search Content"}]
                
        class MockScrape(FirecrawlAdapter):
            def scrape(self, url):
                return {"markdown": "Mock Markdown Content", "metadata": {"title": "Mock page"}}
                
        class MockBrowser(BrowserHubAdapter):
            def interact(self, intent):
                return PageEvidence(
                    task_id=intent.id,
                    status="completed",
                    observed_at=datetime.datetime.now(),
                    url=intent.url,
                    evidence=[EvidenceItem(kind="rendered_text", value="Mock Interactive Content")],
                    summary="Interacted successfully"
                )

        self.service = BrowserOrganService(
            search_port=MockSearch(),
            scrape_port=MockScrape(),
            browser_port=MockBrowser(),
            evidence_store=self.evidence_store,
            secret_scanner=self.scanner,
            kernel_obs=self.kernel_obs
        )

    def tearDown(self):
        shutil.rmtree(self.temp_dir)

    def test_search_routing(self):
        intent = NavigationIntent(
            id="task-1",
            created_at=datetime.datetime.now(),
            source="test",
            objective="Query test",
            url="search://test",
            allowed_actions=["query"],
            forbidden_actions=["submit"],
            evidence_required=["search_result"],
            human_gate=HumanGate(required=False, reason="")
        )
        evidence = self.service.execute_task(intent)
        self.assertEqual(evidence.status, "completed")
        self.assertEqual(len(evidence.evidence), 1)
        self.assertEqual(evidence.evidence[0].kind, "search_result")
        self.assertIn("Mock search Content", evidence.evidence[0].value)

    def test_mutation_protection(self):
        intent = NavigationIntent(
            id="task-2",
            created_at=datetime.datetime.now(),
            source="test",
            objective="Submit wallet",
            url="https://mutate-endpoint.com",
            allowed_actions=["submit"],
            forbidden_actions=[],
            evidence_required=["submit_confirmation"],
            human_gate=HumanGate(required=False, reason="")
        )
        evidence = self.service.execute_task(intent)
        self.assertEqual(evidence.status, "blocked")
        self.assertEqual(len(evidence.evidence), 0)
        self.assertIn("Mutation requested but no explicit human gate", evidence.blockers[0])
