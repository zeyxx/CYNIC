"""Adapters package for the browser organ.

The package initializer must stay dependency-light. Importing one small adapter
such as ``hub_status`` is a topology read and must not transitively require the
full browser client stack.
"""

from __future__ import annotations

from typing import Any

_LAZY_EXPORTS = {
    "BrowserHubAdapter": ".browser_hub_adapter",
    "CynicKernelObservationAdapter": ".kernel_observation_adapter",
    "FilesystemEvidenceStore": ".filesystem_evidence_store",
    "FirecrawlAdapter": ".firecrawl_adapter",
    "HermesAgentBrowserAdapter": ".hermes_agent_browser_adapter",
    "RegexSecretScannerAdapter": ".secret_scanner_adapter",
    "SearXNGAdapter": ".searxng_adapter",
}

__all__ = [
    "BrowserHubAdapter",
    "CynicKernelObservationAdapter",
    "FilesystemEvidenceStore",
    "FirecrawlAdapter",
    "HermesAgentBrowserAdapter",
    "RegexSecretScannerAdapter",
    "SearXNGAdapter",
]


def __getattr__(name: str) -> Any:
    if name not in _LAZY_EXPORTS:
        raise AttributeError(name)
    from importlib import import_module

    module = import_module(_LAZY_EXPORTS[name], __name__)
    value = getattr(module, name)
    globals()[name] = value
    return value
