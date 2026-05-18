#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Render web pages, return markdown.

4-tier fallback chain (sovereign, zero paid APIs):
  L0: curl_cffi     — TLS-spoofed HTTP, no browser (~30ms, static pages)
  L1: camoufox      — Firefox anti-detect, JS rendering (~8-15s)
  L2: cloakbrowser  — Stealth Chromium, Cloudflare bypass (~15-22s)
  L3: playwright    — Vanilla Chromium fallback (~6-13s)

K15 Consumer: agents (read external content for enrichment/judgment)
              /observe (optional: store rendered content as observation)

Usage:
    python3 web_render.py <url>                       # stdout markdown
    python3 web_render.py <url> --json                 # stdout JSON
    python3 web_render.py <url> --selector '#main'     # extract specific element
    python3 web_render.py <url> --engine cloakbrowser  # force engine

As library:
    from web_render import render_url
    result = render_url("https://www.idontbelieve.link/")
    print(result["markdown"])
"""

__version__ = "0.3.0"

import argparse
import json
import logging
import time
from typing import Optional

logger = logging.getLogger("web-render")

_ENGINES = ["curl_cffi", "camoufox", "cloakbrowser", "playwright"]

# Content selectors in priority order (most specific → broadest)
_CONTENT_SELECTORS = [
    "[role='main']",
    "main",
    "#notion-app .notion-page-content",
    ".notion-page-content",
    "article",
    "body",
]

# Markers that indicate the page needs JS rendering
_JS_MARKERS = [
    "enable javascript",
    "noscript",
    "just a moment",
    "checking your browser",
    "please turn javascript on",
    # SPA shells — HTML exists but content is JS-rendered
    '"__next_data__"',
    '<div id="root"></div>',
    '<div id="app"></div>',
    '<div id="__nuxt">',
    "window.__remix",
]

# Post-render markers that indicate the browser got a challenge page, not real content
_CHALLENGE_MARKERS = [
    "just a moment",
    "checking your browser",
    "verify you are human",
    "attention required",
    "please wait",
    "ray id",
]


def _extract_text(page, selector: Optional[str]) -> str:
    """Extract text from a rendered page. Shared across browser engines."""
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
    """Wait for page content to render. Shared across browser engines."""
    page.wait_for_timeout(wait_ms)
    for content_sel in _CONTENT_SELECTORS[:3]:
        try:
            page.wait_for_selector(content_sel, timeout=3_000)
            return
        except Exception:
            continue


def _needs_js(html: str) -> bool:
    """Detect if the HTML response requires JS rendering."""
    lower = html[:10_000].lower()
    return any(marker in lower for marker in _JS_MARKERS)


def _is_challenge_page(title: str, text: str) -> bool:
    """Detect if rendered content is a bot challenge, not real content."""
    combined = (title + " " + text[:500]).lower()
    return any(marker in combined for marker in _CHALLENGE_MARKERS)


# ---------------------------------------------------------------------------
# L0: curl_cffi — TLS-spoofed static fetch, no browser
# ---------------------------------------------------------------------------

def _render_curl(url: str, selector: Optional[str], timeout_ms: int) -> dict:
    """Fast static fetch with Chrome TLS fingerprint. No JS rendering."""
    from curl_cffi import requests as curl_requests

    t0 = time.monotonic()
    r = curl_requests.get(url, impersonate="chrome", timeout=timeout_ms / 1000)

    if _needs_js(r.text):
        raise ValueError("Page requires JS rendering — escalating")

    # Extract text from HTML
    try:
        from lxml import html as lxml_html
        doc = lxml_html.fromstring(r.text)
        # Try content selectors
        for xpath in [
            "//main",
            "//*[@role='main']",
            "//article",
            "//body",
        ]:
            els = doc.xpath(xpath)
            if els:
                text = els[0].text_content()
                break
        else:
            text = doc.text_content()
        title_els = doc.xpath("//title")
        title = title_els[0].text_content().strip() if title_els else ""
    except ImportError:
        # Fallback: crude text extraction without lxml
        import re
        text = re.sub(r"<[^>]+>", " ", r.text)
        text = re.sub(r"\s+", " ", text).strip()
        title_match = re.search(r"<title>(.*?)</title>", r.text, re.IGNORECASE | re.DOTALL)
        title = title_match.group(1).strip() if title_match else ""

    elapsed = int((time.monotonic() - t0) * 1000)

    return {
        "url": url,
        "title": title,
        "markdown": _clean_text(text),
        "engine": "curl_cffi",
        "rendered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "elapsed_ms": elapsed,
    }


# ---------------------------------------------------------------------------
# L1: Camoufox — Firefox anti-detect
# ---------------------------------------------------------------------------

def _render_camoufox(
    url: str, selector: Optional[str], timeout_ms: int, wait_ms: int,
) -> dict:
    from camoufox.sync_api import Camoufox

    t0 = time.monotonic()
    with Camoufox(headless=True) as browser:
        page = browser.new_page()
        page.goto(url, timeout=timeout_ms)
        _wait_for_content(page, wait_ms)
        title = page.title()
        text = _extract_text(page, selector)
        elapsed = int((time.monotonic() - t0) * 1000)

    markdown = _clean_text(text)
    if _is_challenge_page(title, markdown):
        raise ValueError(f"Challenge page detected (title='{title[:30]}') — escalating")

    return {
        "url": url, "title": title, "markdown": markdown,
        "engine": "camoufox",
        "rendered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "elapsed_ms": elapsed,
    }


# ---------------------------------------------------------------------------
# L2: CloakBrowser — Stealth Chromium (Cloudflare bypass)
# ---------------------------------------------------------------------------

def _render_cloakbrowser(
    url: str, selector: Optional[str], timeout_ms: int, wait_ms: int,
) -> dict:
    from cloakbrowser import launch

    t0 = time.monotonic()
    browser = launch(headless=True)
    page = browser.new_page()
    page.set_viewport_size({"width": 1280, "height": 900})
    page.goto(url, timeout=timeout_ms, wait_until="domcontentloaded")
    _wait_for_content(page, wait_ms)
    title = page.title()
    text = _extract_text(page, selector)
    elapsed = int((time.monotonic() - t0) * 1000)
    browser.close()

    markdown = _clean_text(text)
    if _is_challenge_page(title, markdown):
        raise ValueError(f"Challenge page detected (title='{title[:30]}') — escalating")

    return {
        "url": url, "title": title, "markdown": markdown,
        "engine": "cloakbrowser",
        "rendered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "elapsed_ms": elapsed,
    }


# ---------------------------------------------------------------------------
# L3: Playwright — Vanilla Chromium fallback
# ---------------------------------------------------------------------------

def _render_playwright(
    url: str, selector: Optional[str], timeout_ms: int, wait_ms: int,
) -> dict:
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
        "url": url, "title": title, "markdown": _clean_text(text),
        "engine": "playwright",
        "rendered_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "elapsed_ms": elapsed,
    }


# ---------------------------------------------------------------------------
# Orchestrator
# ---------------------------------------------------------------------------

_ENGINE_FNS = {
    "curl_cffi": lambda url, sel, t, w: _render_curl(url, sel, t),
    "camoufox": _render_camoufox,
    "cloakbrowser": _render_cloakbrowser,
    "playwright": _render_playwright,
}


def render_url(
    url: str,
    selector: Optional[str] = None,
    timeout_ms: int = 45_000,
    wait_ms: int = 5_000,
    engine: Optional[str] = None,
) -> dict:
    """Render a URL and extract content as markdown.

    Fallback chain: curl_cffi → camoufox → cloakbrowser → playwright.
    Force a specific engine with the engine parameter.
    """
    chain = [engine] if engine else _ENGINES

    last_error = None
    for eng in chain:
        fn = _ENGINE_FNS.get(eng)
        if not fn:
            raise ValueError(f"Unknown engine: {eng}. Available: {_ENGINES}")
        try:
            return fn(url, selector, timeout_ms, wait_ms)
        except Exception as e:
            last_error = e
            logger.warning("Engine %s failed for %s: %s", eng, url, e)
            continue

    raise RuntimeError(f"All engines failed for {url}: {last_error}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Render web pages to markdown")
    parser.add_argument("url", help="URL to render")
    parser.add_argument("--json", action="store_true", dest="as_json", help="Output as JSON")
    parser.add_argument("--selector", "-s", help="CSS selector to extract")
    parser.add_argument("--timeout", type=int, default=45000, help="Navigation timeout (ms)")
    parser.add_argument("--wait", type=int, default=5000, help="Extra wait after load (ms)")
    parser.add_argument("--engine", choices=_ENGINES, help="Force specific engine")
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
