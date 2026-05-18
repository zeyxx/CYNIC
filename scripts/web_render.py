#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Render JS-heavy pages, return markdown.

Fallback chain: Camoufox (Firefox, anti-detect) → Playwright (Chromium) → error.

K15 Consumer: agents (read external content for enrichment/judgment)
              /observe (optional: store rendered content as observation)

Usage:
    python3 web_render.py <url>                     # stdout markdown
    python3 web_render.py <url> --json               # stdout JSON
    python3 web_render.py <url> --selector '#main'   # extract specific element
    python3 web_render.py <url> --engine camoufox    # force engine
    python3 web_render.py <url> --engine playwright  # force engine

As library:
    from web_render import render_url
    result = render_url("https://www.idontbelieve.link/")
    print(result["markdown"])
"""

__version__ = "0.2.0"

import argparse
import json
import logging
import time
from typing import Optional

logger = logging.getLogger("web-render")

# Content selectors in priority order (most specific → broadest)
_CONTENT_SELECTORS = [
    "[role='main']",
    "main",
    "#notion-app .notion-page-content",
    ".notion-page-content",
    "article",
    "body",
]


def _extract_text(page, selector: Optional[str]) -> str:
    """Extract text from a rendered page. Shared across engines."""
    if selector:
        element = page.query_selector(selector)
        return element.inner_text() if element else f"[selector '{selector}' not found]"

    for sel in _CONTENT_SELECTORS:
        el = page.query_selector(sel)
        if el:
            return el.inner_text()
    return page.inner_text("body")


def _clean_text(text: str) -> str:
    """Collapse blank lines, strip whitespace."""
    lines = text.strip().split("\n")
    cleaned: list[str] = []
    prev_blank = False
    for line in lines:
        stripped = line.strip()
        if not stripped:
            if not prev_blank:
                cleaned.append("")
            prev_blank = True
        else:
            cleaned.append(stripped)
            prev_blank = False
    return "\n".join(cleaned).strip()


def _wait_for_content(page, wait_ms: int) -> None:
    """Wait for page content to render. Shared across engines.

    Strategy: wait_ms base wait, then try ONE fast selector probe.
    Avoids the 4×10s timeout cascade that dominated benchmarks.
    """
    page.wait_for_timeout(wait_ms)
    # Single fast probe — if main content exists, great; if not, we already waited
    for content_sel in _CONTENT_SELECTORS[:3]:
        try:
            page.wait_for_selector(content_sel, timeout=3_000)
            return
        except Exception:
            continue


def _render_camoufox(
    url: str,
    selector: Optional[str],
    timeout_ms: int,
    wait_ms: int,
) -> dict:
    """Render using Camoufox (Firefox with anti-detect fingerprinting)."""
    from camoufox.sync_api import Camoufox

    t0 = time.monotonic()
    with Camoufox(headless=True) as browser:
        page = browser.new_page()
        page.goto(url, timeout=timeout_ms)
        _wait_for_content(page, wait_ms)

        title = page.title()
        text = _extract_text(page, selector)
        elapsed = int((time.monotonic() - t0) * 1000)

    return {
        "url": url,
        "title": title,
        "markdown": _clean_text(text),
        "engine": "camoufox",
        "rendered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "elapsed_ms": elapsed,
    }


def _render_playwright(
    url: str,
    selector: Optional[str],
    timeout_ms: int,
    wait_ms: int,
) -> dict:
    """Render using Playwright Chromium (fallback)."""
    from playwright.sync_api import sync_playwright

    t0 = time.monotonic()
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        page.set_viewport_size({"width": 1280, "height": 900})

        page.goto(url, timeout=timeout_ms, wait_until="domcontentloaded")
        _wait_for_content(page, wait_ms)

        title = page.title()
        text = _extract_text(page, selector)
        elapsed = int((time.monotonic() - t0) * 1000)
        browser.close()

    return {
        "url": url,
        "title": title,
        "markdown": _clean_text(text),
        "engine": "playwright",
        "rendered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "elapsed_ms": elapsed,
    }


def render_url(
    url: str,
    selector: Optional[str] = None,
    timeout_ms: int = 45_000,
    wait_ms: int = 5_000,
    engine: Optional[str] = None,
) -> dict:
    """Render a URL and extract content as markdown.

    Fallback chain: camoufox → playwright.
    Force a specific engine with the engine parameter.

    Returns:
        dict with keys: url, title, markdown, engine, rendered_at, elapsed_ms
    """
    engines = []
    if engine:
        engines = [engine]
    else:
        # Prefer camoufox (faster, anti-detect), fall back to playwright
        engines = ["camoufox", "playwright"]

    last_error = None
    for eng in engines:
        try:
            if eng == "camoufox":
                return _render_camoufox(url, selector, timeout_ms, wait_ms)
            elif eng == "playwright":
                return _render_playwright(url, selector, timeout_ms, wait_ms)
            else:
                raise ValueError(f"Unknown engine: {eng}")
        except Exception as e:
            last_error = e
            logger.warning("Engine %s failed for %s: %s", eng, url, e)
            continue

    raise RuntimeError(f"All engines failed for {url}: {last_error}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Render JS-heavy pages to markdown")
    parser.add_argument("url", help="URL to render")
    parser.add_argument("--json", action="store_true", dest="as_json", help="Output as JSON")
    parser.add_argument("--selector", "-s", help="CSS selector to extract")
    parser.add_argument("--timeout", type=int, default=45000, help="Navigation timeout (ms)")
    parser.add_argument("--wait", type=int, default=5000, help="Extra wait after load (ms)")
    parser.add_argument("--engine", choices=["camoufox", "playwright"], help="Force specific engine")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")
    result = render_url(
        args.url,
        selector=args.selector,
        timeout_ms=args.timeout,
        wait_ms=args.wait,
        engine=args.engine,
    )

    if args.as_json:
        print(json.dumps(result, indent=2))
    else:
        print(result["markdown"])


if __name__ == "__main__":
    main()
