from abc import ABC, abstractmethod
from typing import List, Dict, Any
from .entities import (
    BrowserBackendCapability,
    BrowserToolCall,
    BrowserToolResult,
    NavigationIntent,
    PageEvidence,
)

class SearchPort(ABC):
    @abstractmethod
    def search(self, query: str, limit: int = 10) -> List[Dict[str, Any]]:
        """Query search engine and return candidate search results."""
        pass

class ScrapePort(ABC):
    @abstractmethod
    def scrape(self, url: str) -> Dict[str, Any]:
        """Scrape known URL and return markdown content and metadata."""
        pass

class BrowserPort(ABC):
    @abstractmethod
    def interact(self, intent: NavigationIntent) -> PageEvidence:
        """Perform interactive navigation or CDP automation."""
        pass

class HermesBrowserBackendPort(ABC):
    @abstractmethod
    def capabilities(self) -> List[BrowserBackendCapability]:
        """Return Hermes-compatible browser tools exposed by this backend."""
        pass

    @abstractmethod
    def call_tool(self, call: BrowserToolCall) -> BrowserToolResult:
        """Execute one Hermes-compatible browser tool call after CYNIC policy approval."""
        pass

class EvidenceStorePort(ABC):
    @abstractmethod
    def save_evidence(self, task_id: str, evidence: PageEvidence) -> None:
        """Persist evidence details (text, screenshots, redacted logs)."""
        pass

    @abstractmethod
    def log_action(self, action: Dict[str, Any]) -> None:
        """Append entry to the action ledger."""
        pass

class SecretScannerPort(ABC):
    @abstractmethod
    def sanitize(self, text: str) -> str:
        """Sanitize text and replace sensitive variables with placeholders."""
        pass

class KernelObservationPort(ABC):
    @abstractmethod
    def observe(self, agent_id: str, title: str, content: Dict[str, Any], domain: str = "browser") -> None:
        """Record structured discovery or decisions in kernel memory."""
        pass
