"""
Tier 1 EXPERIMENTAL: FTC Do Not Call complaint data collector for CallShield.

Research question: Can real FTC complaint data calibrate the phone-number deterministic dog?
Success condition: 200+ unique phone numbers with 3+ complaints each, mapped to PhoneData JSONL.
Timeline: 7 days (collect, validate, archive or promote to Tier 2).
Will this become Tier 2 or die? Promote if accuracy measurement succeeds; die if FTC data
  doesn't map meaningfully to PhoneData (e.g., too few repeat-reported numbers).

Note: If not promoted to Tier 2 by 2026-06-20, delete.
"""

from __future__ import annotations

import json
import logging
import os
import sys
import time
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError

__version__ = "0.1.0"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
)
log = logging.getLogger("ftc_dnc_collector")

# ── Config ──────────────────────────────────────────────

FTC_API_URL = "https://api.ftc.gov/v0/dnc-complaints"
API_KEY = os.environ.get("FTC_API_KEY", "DEMO_KEY")
ITEMS_PER_PAGE = 50  # FTC API cap per request
MAX_PAGES = 7  # DEMO_KEY rate-limits after ~6 pages; register key for more
REQUEST_DELAY_S = 1.0  # Be polite to the API

# Subject -> ground truth label mapping
SUBJECT_TO_LABEL: dict[str, str] = {
    "Calls pretending to be government, businesses, or family and friends": "scam",
    "Reducing your debt (credit cards, mortgage, student loans)": "scam",
    "Warranties & protection plans": "scam",
    "Medical  & prescriptions": "nuisance",
    "Home improvement  & cleaning": "nuisance",
    "Vacation & timeshares": "nuisance",
    "Energy": "nuisance",
    "Grants": "scam",
    "Lotteries, prizes, & sweepstakes": "scam",
    "Business opportunities & work from home": "scam",
    "Insurance": "nuisance",
    "Other": "unknown",
}

OUTPUT_DIR = Path(__file__).parent.parent / "data" / "phone_numbers"


def fetch_page(page: int, max_retries: int = 3) -> list[dict[str, Any]]:
    """Fetch one page of FTC DNC complaints."""
    url = f"{FTC_API_URL}?api_key={API_KEY}&items_per_page={ITEMS_PER_PAGE}&page={page}"
    req = Request(url, headers={"Accept": "application/json"})
    for attempt in range(max_retries):
        try:
            with urlopen(req, timeout=30) as resp:
                data = json.loads(resp.read().decode())
                return data.get("data", [])
        except HTTPError as e:
            if e.code == 429 and attempt < max_retries - 1:
                wait = 10 * (attempt + 1)  # P18: backoff on transient
                log.warning("Rate limited (429) on page %d — retry %d/%d in %ds", page, attempt + 1, max_retries, wait)
                time.sleep(wait)
                continue
            if e.code in (400, 404, 422):  # P18: permanent errors — don't retry
                log.error("Permanent HTTP %d on page %d: %s", e.code, page, e.reason)
                return []
            log.error("HTTP %d on page %d: %s", e.code, page, e.reason)
            return []
        except URLError as e:
            log.error("Network error on page %d: %s", page, e.reason)
            return []
    log.error("Exhausted retries for page %d", page)
    return []


def collect_complaints(max_pages: int = MAX_PAGES) -> list[dict[str, Any]]:
    """Paginate through FTC API and collect all complaints."""
    all_complaints: list[dict[str, Any]] = []
    for page in range(max_pages):
        entries = fetch_page(page)
        if not entries:
            log.info("No more entries at page %d — stopping", page)
            break
        all_complaints.extend(entries)
        log.info("Page %d: %d entries (total: %d)", page, len(entries), len(all_complaints))
        if len(entries) < ITEMS_PER_PAGE:
            break
        time.sleep(REQUEST_DELAY_S)
    return all_complaints


def aggregate_by_number(
    complaints: list[dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    """Aggregate complaints by phone number into PhoneData-compatible records."""
    by_number: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for entry in complaints:
        attrs = entry.get("attributes", {})
        phone = attrs.get("company-phone-number", "")
        if not phone or len(phone) < 7:
            continue
        by_number[phone].append(attrs)

    aggregated: dict[str, dict[str, Any]] = {}
    for phone, records in by_number.items():
        subjects = [r.get("subject", "Other") for r in records]
        robocall_count = sum(1 for r in records if r.get("recorded-message-or-robocall") == "Y")

        # Parse dates for age calculation
        dates = []
        for r in records:
            created = r.get("created-date", "")
            if created:
                try:
                    dates.append(datetime.strptime(created[:10], "%Y-%m-%d"))
                except ValueError:
                    pass

        age_days = 0
        days_since_last = 0
        if dates:
            now = datetime.now(timezone.utc).replace(tzinfo=None)
            oldest = min(dates)
            newest = max(dates)
            age_days = max(1, (now - oldest).days)
            days_since_last = max(0, (now - newest).days)

        # Derive labels from subjects
        labels: dict[str, int] = {"legitimate": 0, "nuisance": 0, "scam": 0, "unknown": 0}
        for subj in subjects:
            label = SUBJECT_TO_LABEL.get(subj, "unknown")
            labels[label] += 1

        total = sum(labels.values())
        spam_score = (labels["nuisance"] * 0.75 + labels["scam"] * 1.0 + labels["unknown"] * 0.5) / max(total, 1)

        # Determine ground truth from majority label
        majority_label = max(labels, key=lambda k: labels[k])
        if majority_label == "legitimate" and labels["legitimate"] == 0:
            majority_label = "unknown"

        # Build PhoneData-compatible record
        aggregated[phone] = {
            "schema_version": 1,
            "number": f"+1{phone}",
            "country_code": "US",
            "total_events": len(records),
            "label_distribution": {
                "legitimate": labels["legitimate"],
                "nuisance": labels["nuisance"],
                "scam": labels["scam"],
                "unknown": labels["unknown"],
            },
            "reporter_count": len(records),  # Each complaint = 1 reporter
            "mean_reporter_trust": 0.60,  # FTC complaints are unverified — moderate trust
            "age_days": age_days,
            "days_since_last_report": days_since_last,
            "challenge_pass_rate": None,
            "contestation_count": 0,
            "owner_verified": False,
            "ground_truth": majority_label,
            "source": f"ftc-dnc-api:{datetime.now(timezone.utc).strftime('%Y-%m-%d')}",
            "notes": f"FTC DNC complaints. Subjects: {dict(zip(set(subjects), [subjects.count(s) for s in set(subjects)]))}. Robocall: {robocall_count}/{len(records)}.",
        }

    return aggregated


def write_dataset(aggregated: dict[str, dict[str, Any]], min_complaints: int = 1) -> Path:
    """Write aggregated phone data to JSONL."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    date_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    output_path = OUTPUT_DIR / f"ftc_dnc_{date_str}.jsonl"

    written = 0
    with open(output_path, "w", encoding="utf-8") as f:
        for phone, record in sorted(aggregated.items(), key=lambda x: -x[1]["total_events"]):
            if record["total_events"] < min_complaints:
                continue
            f.write(json.dumps(record, ensure_ascii=False) + "\n")
            written += 1

    log.info("Wrote %d records to %s", written, output_path)
    return output_path


def print_summary(aggregated: dict[str, dict[str, Any]]) -> None:
    """Print collection summary statistics."""
    from collections import Counter

    total = len(aggregated)
    by_truth = Counter(r["ground_truth"] for r in aggregated.values())
    by_complaints = Counter()
    for r in aggregated.values():
        n = r["total_events"]
        if n >= 5:
            by_complaints["5+"] += 1
        elif n >= 3:
            by_complaints["3-4"] += 1
        elif n >= 2:
            by_complaints["2"] += 1
        else:
            by_complaints["1"] += 1

    log.info("=" * 50)
    log.info("FTC DNC Collection Summary")
    log.info("Total unique numbers: %d", total)
    log.info("")
    log.info("By ground truth:")
    for label, count in by_truth.most_common():
        log.info("  %-12s %d", label, count)
    log.info("")
    log.info("By complaint count:")
    for bucket in ["5+", "3-4", "2", "1"]:
        log.info("  %-12s %d", f"{bucket} complaints:", by_complaints.get(bucket, 0))
    log.info("=" * 50)


def main() -> None:
    """Collect FTC DNC complaints and output PhoneData JSONL."""
    log.info("ftc_dnc_collector v%s starting", __version__)
    log.info("API: %s (max %d pages)", FTC_API_URL, MAX_PAGES)

    complaints = collect_complaints()
    if not complaints:
        log.error("No complaints collected — check API access")
        sys.exit(1)

    log.info("Collected %d raw complaints", len(complaints))

    aggregated = aggregate_by_number(complaints)
    log.info("Aggregated into %d unique phone numbers", len(aggregated))

    print_summary(aggregated)
    output_path = write_dataset(aggregated, min_complaints=1)

    log.info("Done. Output: %s", output_path)


if __name__ == "__main__":
    main()
