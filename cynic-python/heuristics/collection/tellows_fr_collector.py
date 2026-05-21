"""
Tier 1 EXPERIMENTAL: Tellows.fr public data collector for CallShield.

Research question: Can publicly listed spam numbers from tellows.fr seed the phone-number Dog?
Success condition: 30+ unique FR phone numbers with score + category, mapped to PhoneData JSONL.
Timeline: 7 days (collect, validate, archive or promote to Tier 2).
Will this become Tier 2 or die? Promote if numbers align with known FR spam patterns; die if
  tellows public pages are too rate-limited or data is too shallow.

Note: If not promoted to Tier 2 by 2026-06-21, delete.
"""

from __future__ import annotations

import json
import logging
import re
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError
from html.parser import HTMLParser

__version__ = "0.1.0"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
)
log = logging.getLogger("tellows_fr_collector")

# ── Config ──────────────────────────────────────────────

STATS_URL = "https://www.tellows.fr/stats"
NUM_URL_TEMPLATE = "https://www.tellows.fr/num/{number}"
USER_AGENT = "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36"
REQUEST_DELAY_S = 2.0  # Polite delay between requests
MAX_DETAIL_PAGES = 50  # Cap on individual number lookups

OUTPUT_DIR = Path(__file__).parent.parent / "data" / "phone_numbers"

# Tellows score → ground truth mapping
# Score 1-3 = likely legit, 4-5 = ambiguous, 6-7 = nuisance, 8-9 = scam
SCORE_TO_LABEL: dict[int, str] = {
    1: "legitimate", 2: "legitimate", 3: "legitimate",
    4: "unknown", 5: "unknown",
    6: "nuisance", 7: "nuisance",
    8: "scam", 9: "scam",
}

# ── HTML parsers ──────────────────────────────────────────

FR_PHONE_RE = re.compile(r"(?:\+33|0)([1-9]\d{8})")


class StatsPageParser(HTMLParser):
    """Extract phone numbers from tellows.fr/stats page."""

    def __init__(self) -> None:
        super().__init__()
        self.numbers: list[str] = []
        self._in_link = False
        self._current_href = ""

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag == "a":
            href = dict(attrs).get("href", "") or ""
            # Links like /num/0948008689
            if "/num/" in href:
                self._in_link = True
                self._current_href = href

    def handle_endtag(self, tag: str) -> None:
        if tag == "a":
            self._in_link = False

    def handle_data(self, data: str) -> None:
        if self._in_link and self._current_href:
            # Extract number from href /num/XXXXXXXXXX or /num/+33XXXXXXXXX
            parts = self._current_href.split("/num/")
            if len(parts) == 2:
                raw = parts[1].strip("/").replace("+33", "0").replace("%2B33", "0")
                match = FR_PHONE_RE.match(raw if raw.startswith("0") else f"0{raw}")
                if match:
                    e164 = f"+33{match.group(1)}"
                    if e164 not in self.numbers:
                        self.numbers.append(e164)
            self._current_href = ""


class NumberPageParser(HTMLParser):
    """Extract score, reports, and caller type from individual number page."""

    def __init__(self) -> None:
        super().__init__()
        self.score: int | None = None
        self.reports: int = 0
        self.searches: int = 0
        self.caller_type: str = "unknown"
        self._capture_next: str | None = None
        self._text_buffer: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attr_dict = dict(attrs)
        # Score from image: /images/score/s7.jpg
        if tag == "img":
            src = attr_dict.get("src", "") or ""
            score_match = re.search(r"/score/s(\d)", src)
            if score_match:
                self.score = int(score_match.group(1))

    def handle_data(self, data: str) -> None:
        text = data.strip()
        if not text:
            return
        self._text_buffer.append(text)

        # Look for report count patterns
        count_match = re.search(r"(\d+)\s*(?:évaluation|evaluation|comment|avis)", text, re.IGNORECASE)
        if count_match:
            self.reports = max(self.reports, int(count_match.group(1)))

        search_match = re.search(r"(\d+)\s*(?:recherche|search|requête)", text, re.IGNORECASE)
        if search_match:
            self.searches = max(self.searches, int(search_match.group(1)))

        # Caller type — only match strong scam indicators (avoid false positives
        # from page template words like "commercial", "publicité")
        scam_pattern = r"arnaque|escroquerie|fraud|phishing|usurp|faux|fake"
        if re.search(scam_pattern, text, re.IGNORECASE):
            self.caller_type = "scam"


def fetch_url(url: str, max_retries: int = 3) -> str | None:
    """Fetch URL content with retry logic."""
    req = Request(url, headers={
        "Accept": "text/html,application/xhtml+xml",
        "Accept-Language": "fr-FR,fr;q=0.9",
        "User-Agent": USER_AGENT,
    })
    for attempt in range(max_retries):
        try:
            with urlopen(req, timeout=30) as resp:
                return resp.read().decode("utf-8", errors="replace")
        except HTTPError as e:
            if e.code == 429 and attempt < max_retries - 1:
                wait = 10 * (attempt + 1)
                log.warning("Rate limited (429) — retry %d/%d in %ds", attempt + 1, max_retries, wait)
                time.sleep(wait)
                continue
            if e.code in (400, 404, 422):
                log.error("Permanent HTTP %d for %s", e.code, url)
                return None
            log.error("HTTP %d for %s: %s", e.code, url, e.reason)
            return None
        except URLError as e:
            log.error("Network error for %s: %s", url, e.reason)
            return None
    return None


def collect_stats_numbers() -> list[str]:
    """Scrape tellows.fr/stats for listed phone numbers."""
    html = fetch_url(STATS_URL)
    if not html:
        log.error("Failed to fetch stats page")
        return []

    parser = StatsPageParser()
    parser.feed(html)
    log.info("Stats page: found %d unique numbers", len(parser.numbers))
    return parser.numbers


def enrich_number(number: str) -> dict[str, Any] | None:
    """Fetch detail page for a single number and extract metadata."""
    # Convert +33XXXXXXXXX to 0XXXXXXXXX for URL
    local = "0" + number[3:]
    url = NUM_URL_TEMPLATE.format(number=local)

    html = fetch_url(url)
    if not html:
        return None

    parser = NumberPageParser()
    parser.feed(html)

    score = parser.score or 5  # default neutral
    label = SCORE_TO_LABEL.get(score, "unknown")
    # Score is the only reliable signal — page text contains template noise

    reports = max(parser.reports, 1)

    return {
        "schema_version": 1,
        "number": number,
        "country_code": "FR",
        "total_events": reports + parser.searches,
        "label_distribution": {
            "legitimate": reports if label == "legitimate" else 0,
            "nuisance": reports if label == "nuisance" else 0,
            "scam": reports if label == "scam" else 0,
            "unknown": reports if label == "unknown" else 0,
        },
        "reporter_count": reports,
        "mean_reporter_trust": 0.55,  # Community reports — moderate trust
        "age_days": 0,  # Unknown from page
        "days_since_last_report": 0,
        "challenge_pass_rate": None,
        "contestation_count": 0,
        "owner_verified": False,
        "ground_truth": label,
        "tellows_score": score,
        "source": f"tellows-fr:{datetime.now(timezone.utc).strftime('%Y-%m-%d')}",
        "notes": f"Tellows FR score {score}/9, {reports} reports, {parser.searches} searches. Type: {parser.caller_type}.",
    }


def write_dataset(records: list[dict[str, Any]]) -> Path:
    """Write collected records to JSONL."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    date_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    output_path = OUTPUT_DIR / f"tellows_fr_{date_str}.jsonl"

    with open(output_path, "w", encoding="utf-8") as f:
        for record in sorted(records, key=lambda r: -r["reporter_count"]):
            f.write(json.dumps(record, ensure_ascii=False) + "\n")

    log.info("Wrote %d records to %s", len(records), output_path)
    return output_path


def print_summary(records: list[dict[str, Any]]) -> None:
    """Print collection summary."""
    from collections import Counter

    total = len(records)
    by_truth = Counter(r["ground_truth"] for r in records)
    by_score = Counter(r.get("tellows_score", 5) for r in records)

    log.info("=" * 50)
    log.info("Tellows FR Collection Summary")
    log.info("Total unique numbers: %d", total)
    log.info("")
    log.info("By ground truth:")
    for label, count in by_truth.most_common():
        log.info("  %-12s %d", label, count)
    log.info("")
    log.info("By tellows score:")
    for score in sorted(by_score.keys()):
        log.info("  Score %-2d: %d", score, by_score[score])
    log.info("=" * 50)


def main() -> None:
    """Collect French spam numbers from tellows.fr and output PhoneData JSONL."""
    log.info("tellows_fr_collector v%s starting", __version__)

    # Step 1: Get numbers from stats page
    numbers = collect_stats_numbers()
    if not numbers:
        log.error("No numbers found on stats page")
        sys.exit(1)

    # Step 2: Enrich each number (capped)
    to_fetch = numbers[:MAX_DETAIL_PAGES]
    log.info("Enriching %d numbers (cap: %d)...", len(to_fetch), MAX_DETAIL_PAGES)

    records: list[dict[str, Any]] = []
    for i, number in enumerate(to_fetch):
        log.info("[%d/%d] Enriching %s...", i + 1, len(to_fetch), number)
        record = enrich_number(number)
        if record:
            records.append(record)
        else:
            # Minimal record if detail page failed
            records.append({
                "schema_version": 1,
                "number": number,
                "country_code": "FR",
                "total_events": 1,
                "label_distribution": {"legitimate": 0, "nuisance": 0, "scam": 0, "unknown": 1},
                "reporter_count": 1,
                "mean_reporter_trust": 0.55,
                "age_days": 0,
                "days_since_last_report": 0,
                "challenge_pass_rate": None,
                "contestation_count": 0,
                "owner_verified": False,
                "ground_truth": "unknown",
                "tellows_score": 5,
                "source": f"tellows-fr:{datetime.now(timezone.utc).strftime('%Y-%m-%d')}",
                "notes": "Tellows FR — detail page unavailable, minimal record.",
            })
        time.sleep(REQUEST_DELAY_S)

    # Step 3: Output
    print_summary(records)
    output_path = write_dataset(records)
    log.info("Done. Output: %s", output_path)


if __name__ == "__main__":
    main()
