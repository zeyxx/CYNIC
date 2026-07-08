import os
import json
import datetime
from pathlib import Path
import httpx
from typing import Dict, Any
from organs.contracts.src.surface_contracts import read_json_file_or_default
from ..core.ports import BrowserPort
from ..core.entities import NavigationIntent, PageEvidence, EvidenceItem

class BrowserHubAdapter(BrowserPort):
    def __init__(self, state_dir: str = None):
        project_root = (
            os.environ.get("CYNIC_PROJECT_ROOT")
            or os.environ.get("PROJECT_ROOT")
            or ""
        )
        self.state_dir = Path(
            state_dir
            or os.environ.get("ORGAN_BROWSER_STATE_DIR", "")
            or (str(Path(project_root) / "organs/browser/state") if project_root else "")
            or os.path.expanduser("~/.cynic/organs/browser")
        )
        self.project_root = Path(project_root) if project_root else None
        self.default_hub_url = os.environ.get("ORGAN_BROWSER_HUB_URL", "http://127.0.0.1:40770")
        self.default_cdp_url = os.environ.get("ORGAN_BROWSER_CDP_URL", "http://127.0.0.1:40769")

    def _discover_endpoints(self) -> tuple[str, str]:
        state_path = self.state_dir / "browser-state.json"
        if not state_path.exists():
            repo_state_path = (
                self.project_root / "organs/browser/state/browser-state.json"
                if self.project_root
                else None
            )
            if repo_state_path and repo_state_path.exists():
                state_path = repo_state_path
        
        if state_path.exists():
            try:
                data = read_json_file_or_default(state_path, {})
                hub_url = data.get("hub_url") or f"http://127.0.0.1:{data.get('hub_port', 40770)}"
                cdp_port = data.get("cdp_port", 40769)
                cdp_url = f"http://127.0.0.1:{cdp_port}"
                return hub_url, cdp_url
            except Exception:
                pass
        return self.default_hub_url, self.default_cdp_url

    def interact(self, intent: NavigationIntent) -> PageEvidence:
        hub_url, cdp_url = self._discover_endpoints()
        
        for action in intent.allowed_actions:
            if action in intent.forbidden_actions:
                return PageEvidence(
                    task_id=intent.id,
                    status="blocked",
                    observed_at=datetime.datetime.now(),
                    url=intent.url,
                    blockers=[f"Action {action} is explicitly forbidden"],
                    summary="Blocked due to policy constraints."
                )

        is_mutation = any(
            act in ["submit", "post", "connect wallet", "sign", "purchase", "stake", "deposit"]
            for act in intent.allowed_actions
        )
        if is_mutation and not intent.human_gate.required:
            return PageEvidence(
                task_id=intent.id,
                status="blocked",
                observed_at=datetime.datetime.now(),
                url=intent.url,
                blockers=["Mutation requested but human gate is missing"],
                summary="Blocked task execution to prevent un-gated mutations."
            )

        if "interact" not in intent.allowed_actions and "scrape" in intent.allowed_actions:
            try:
                r = httpx.get(f"{hub_url}/render", params={"url": intent.url}, timeout=30.0)
                if r.status_code == 200:
                    data = r.json()
                    markdown = data.get("markdown", "")
                    title = data.get("title", "")
                    return PageEvidence(
                        task_id=intent.id,
                        status="completed",
                        observed_at=datetime.datetime.now(),
                        url=intent.url,
                        evidence=[
                            EvidenceItem(kind="rendered_markdown", value=markdown),
                            EvidenceItem(kind="rendered_title", value=title)
                        ],
                        summary=f"Rendered page via Browser Hub: {title}"
                    )
            except Exception:
                pass

        try:
            from playwright.sync_api import sync_playwright
        except ImportError:
            raise RuntimeError("Playwright library is not installed in the current environment.")

        with sync_playwright() as p:
            try:
                browser = p.chromium.connect_over_cdp(cdp_url)
                context = browser.contexts[0] if browser.contexts else browser.new_context()
            except Exception:
                user_data_dir = self.state_dir / "playwright_profile"
                context = p.chromium.launch_persistent_context(
                    user_data_dir=str(user_data_dir),
                    headless=True,
                    args=["--disable-blink-features=AutomationControlled"]
                )
            
            page = context.new_page()
            
            try:
                page.goto(intent.url, wait_until="domcontentloaded", timeout=20000)
                page.evaluate("window.scrollBy(0, 500)")
                page.wait_for_timeout(1000)
                
                articles = page.locator('[role="article"]').count()
                content = page.content()
                
                from bs4 import BeautifulSoup
                soup = BeautifulSoup(content, "html.parser")
                for element in soup(["script", "style", "nav", "footer", "header"]):
                    element.decompose()
                markdown = soup.get_text(separator="\n")
                clean_lines = [line.strip() for line in markdown.splitlines() if line.strip()]
                clean_markdown = "\n".join(clean_lines)

                try:
                    screenshot_data = page.screenshot(type="png")
                    screenshot_hex = screenshot_data.hex()
                except Exception:
                    screenshot_hex = ""
                
                evidence = PageEvidence(
                    task_id=intent.id,
                    status="completed",
                    observed_at=datetime.datetime.now(),
                    url=intent.url,
                    evidence=[
                        EvidenceItem(kind="page_markdown", value=clean_markdown),
                        EvidenceItem(kind="screenshot_hex", value=screenshot_hex),
                        EvidenceItem(kind="article_count", value=str(articles))
                    ],
                    summary=f"Browsed page. Found {articles} articles."
                )
                return evidence
            finally:
                page.close()
                try:
                    context.close()
                except Exception:
                    pass
                if 'browser' in locals():
                    try:
                        browser.close()
                    except Exception:
                        pass
