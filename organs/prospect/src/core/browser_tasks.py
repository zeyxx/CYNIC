"""Browser task payloads for sovereign prospect sourcing."""

from __future__ import annotations

from datetime import datetime, timezone
import os
from urllib.parse import quote_plus


def default_search_url(category: str, city: str) -> str:
    query = quote_plus(f"{category} {city} site officiel")
    base_url = os.environ.get("CYNIC_SEARXNG_URL", "http://127.0.0.1:8080").rstrip("/")
    return f"{base_url}/search?q={query}"


def build_browser_sourcing_task(
    category: str,
    city: str,
    url: str | None = None,
    task_id: str | None = None,
) -> dict:
    resolved_id = task_id or f"prospect-{category}-{city}".lower().replace(" ", "-")
    return {
        "id": resolved_id,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "source": "organ-prospect",
        "objective": (
            f"Find official websites for local businesses matching category '{category}' in '{city}'. "
            "Extract only direct business websites and visible business names. Do not submit forms, "
            "log in, send messages, call numbers, or mutate external state."
        ),
        "url": url or default_search_url(category, city),
        "context": {
            "category": category,
            "city": city,
            "output_hint": "Return one URL per evidence item when possible; organ-prospect will audit URLs separately.",
        },
        "allowed_actions": [
            "navigate",
            "read_text",
            "extract_links",
            "screenshot",
            "scroll",
        ],
        "forbidden_actions": [
            "submit_form",
            "send_message",
            "login",
            "purchase",
            "call_phone",
            "write_external_state",
        ],
        "evidence_required": [
            "business_name",
            "official_website_url",
            "source_page_url",
        ],
        "human_gate": {
            "required": False,
            "reason": "Read-only public prospect sourcing; outreach remains human-gated.",
        },
    }
