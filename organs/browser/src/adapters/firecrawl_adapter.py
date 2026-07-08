import os
import httpx
from bs4 import BeautifulSoup
from typing import Dict, Any
from ..core.ports import ScrapePort

class FirecrawlAdapter(ScrapePort):
    def __init__(self, api_key: str = None, base_url: str = None):
        self.api_key = api_key or os.environ.get("FIRECRAWL_API_KEY")
        self.base_url = base_url or os.environ.get("FIRECRAWL_URL", "https://api.firecrawl.dev")

    def scrape(self, url: str) -> Dict[str, Any]:
        if self.api_key:
            try:
                headers = {
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json"
                }
                payload = {"url": url, "pageOptions": {"onlyMainContent": True}}
                response = httpx.post(f"{self.base_url}/v0/scrape", json=payload, headers=headers, timeout=20.0)
                if response.status_code == 200:
                    data = response.json()
                    if data.get("success"):
                        page_data = data.get("data", {})
                        return {
                            "markdown": page_data.get("markdown", ""),
                            "metadata": page_data.get("metadata", {})
                        }
            except Exception:
                pass
        
        return self._local_scrape(url)

    def _local_scrape(self, url: str) -> Dict[str, Any]:
        try:
            headers = {"User-Agent": "Mozilla/5.0 (X11; Linux x86_64) CYNIC Organ-Browser/0.1"}
            response = httpx.get(url, headers=headers, follow_redirects=True, timeout=15.0)
            if response.status_code == 200:
                soup = BeautifulSoup(response.text, "html.parser")
                for element in soup(["script", "style", "nav", "footer", "header"]):
                    element.decompose()
                
                text = soup.get_text(separator="\n")
                lines = [line.strip() for line in text.splitlines() if line.strip()]
                clean_markdown = "\n".join(lines)
                title = soup.title.string.strip() if soup.title else ""
                
                return {
                    "markdown": clean_markdown,
                    "metadata": {
                        "title": title,
                        "status_code": response.status_code,
                        "source": "local_scraper_fallback"
                    }
                }
            else:
                return {
                    "markdown": f"Error: Received status code {response.status_code} from host.",
                    "metadata": {"status_code": response.status_code}
                }
        except Exception as e:
            return {
                "markdown": f"Error: Local scrape failed. Details: {str(e)}",
                "metadata": {"error": str(e)}
            }
