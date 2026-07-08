"""Convert organ-browser/Hermes evidence into prospect audit inputs."""

from __future__ import annotations

import json
import re
from pathlib import Path
from urllib.parse import urlparse

from .prospecting import is_non_prospect_url


URL_RE = re.compile(r"https?://[^\s\"'<>),]+")


def read_browser_result(path: Path) -> dict:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("browser result must be a JSON object")
    return payload


def urls_from_browser_result(payload: dict) -> list[str]:
    """Extract direct prospect URLs from a CYNIC browser result payload."""
    evidence = payload.get("evidence")
    if not isinstance(evidence, list):
        return []

    urls: list[str] = []
    for item in evidence:
        if not isinstance(item, dict):
            continue
        kind = str(item.get("kind") or "").lower()
        value = str(item.get("value") or "").strip()
        if not value:
            continue

        if kind in {"official_website_url", "website", "url", "business_url"}:
            urls.append(value)
            continue

        if kind in {"business_listing", "prospect", "json", "structured_result"}:
            urls.extend(_urls_from_jsonish(value))
            continue

        urls.extend(URL_RE.findall(value))

    return dedupe_prospect_urls(urls)


def dedupe_prospect_urls(urls: list[str]) -> list[str]:
    seen_hosts: set[str] = set()
    out: list[str] = []
    for raw in urls:
        url = raw.strip().rstrip("/.,;")
        parsed = urlparse(url)
        if parsed.scheme not in {"http", "https"} or not parsed.netloc:
            continue
        if is_non_prospect_url(url):
            continue
        host = parsed.netloc.lower().removeprefix("www.")
        if host in seen_hosts:
            continue
        seen_hosts.add(host)
        out.append(url)
    return out


def _urls_from_jsonish(value: str) -> list[str]:
    try:
        payload = json.loads(value)
    except json.JSONDecodeError:
        return URL_RE.findall(value)
    return _walk_urls(payload)


def _walk_urls(value) -> list[str]:
    if isinstance(value, str):
        return URL_RE.findall(value)
    if isinstance(value, list):
        urls: list[str] = []
        for item in value:
            urls.extend(_walk_urls(item))
        return urls
    if isinstance(value, dict):
        urls: list[str] = []
        for key, item in value.items():
            if str(key).lower() in {"official_website_url", "website", "url", "business_url"} and isinstance(item, str):
                urls.append(item)
            else:
                urls.extend(_walk_urls(item))
        return urls
    return []
