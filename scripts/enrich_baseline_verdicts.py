#!/usr/bin/env python3
"""
Tier 1 EXPERIMENTAL: Backfill missing on-chain fields in baseline verdict JSONL.

Research question: Do baseline_concentration, baseline_mint_authority_active,
baseline_freeze_authority_active materially change outcome_measurement_t7 results?

Success condition: All 3 fields populated for ≥40/45 mints (88%+ coverage).
Timeline: One-shot, run once before 2026-06-02 measurement date.
Death condition: Delete after outcome_measurement_t7.sh has been run successfully.

What it does:
  1. Reads cynic-python/heuristics/data/verdicts/verdicts_2026-05-*.jsonl
  2. For each mint, fetches on-chain state via HeliusTokenProfiler
  3. Backfills: baseline_concentration (top10_pct/100), baseline_mint_authority_active,
     baseline_freeze_authority_active, plus bonus fields (age_hours, market_cap_usd,
     is_pump_fun, top1_pct_supply) for future use
  4. Writes enriched JSONL in-place (original backed up to .bak)

Rate: 0.5s between tokens → ~23s for 45 tokens (safe for Helius free tier).
Cost: ~31 Helius credits/token × 45 = ~1395 credits (well within 1M/month).

Falsify: Run twice. If values differ significantly (>5% on top10_pct),
  Helius data is non-deterministic or token supply changed — flag for investigation.
"""

import json
import os
import shutil
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent
VERDICTS_DIR = REPO_ROOT / "cynic-python/heuristics/data/verdicts"
COLLECTION_DIR = REPO_ROOT / "cynic-python/heuristics/collection"

sys.path.insert(0, str(COLLECTION_DIR))

from helius_token_profiler import HeliusTokenProfiler, load_env  # noqa: E402


RATE_LIMIT_SLEEP = 0.5  # seconds between tokens
MIN_COVERAGE = 40       # warn if fewer than this many mints enriched


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def write_jsonl(path: Path, rows: list[dict]) -> None:
    with open(path, "w") as f:
        for row in rows:
            f.write(json.dumps(row, default=str) + "\n")


def enrich_row(row: dict, profiler: HeliusTokenProfiler) -> tuple[dict, bool]:
    """Fetch on-chain data and backfill missing fields. Returns (row, success)."""
    mint = row.get("mint", "")
    if not mint:
        return row, False

    # Pass existing holder_count so profiler doesn't overwrite it
    existing_holders = row.get("baseline_holder_count")

    try:
        profile = profiler.profile(mint, holder_count=existing_holders)
    except Exception as e:
        print(f"  ERROR fetching {mint[:16]}...: {e}", file=sys.stderr)
        return row, False

    if not profile.asset_found:
        print(f"  WARN  {mint[:16]}... not found on Helius (token may be dead)")
        return row, False

    # Backfill the 3 critical missing fields
    row["baseline_concentration"] = round(profile.top10_pct_supply / 100.0, 6)
    row["baseline_mint_authority_active"] = profile.mint_authority_active
    row["baseline_freeze_authority_active"] = profile.freeze_authority_active

    # Bonus fields for Phase 2.1 calibration
    row["enriched_top1_pct"] = round(profile.top1_pct_supply, 4)
    row["enriched_top10_pct"] = round(profile.top10_pct_supply, 4)
    row["enriched_market_cap_usd"] = profile.market_cap_usd
    row["enriched_age_hours"] = round(profile.age_hours, 2) if profile.age_hours else None
    row["enriched_is_pump_fun"] = profile.is_pump_fun if hasattr(profile, "is_pump_fun") else profile.origin_pump_fun
    row["enriched_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

    return row, True


def process_file(path: Path, profiler: HeliusTokenProfiler, dry_run: bool) -> dict:
    """Enrich one JSONL file. Returns stats dict."""
    print(f"\n[enrich] Processing {path.name} ...")
    rows = load_jsonl(path)
    print(f"[enrich]   {len(rows)} rows loaded")

    enriched = 0
    skipped = 0
    errors = 0

    for i, row in enumerate(rows):
        mint = row.get("mint", "?")[:20]

        # Skip if already enriched (idempotent)
        if row.get("baseline_concentration") is not None:
            print(f"  [{i+1:2d}/{len(rows)}] {mint}... already enriched, skipping")
            skipped += 1
            continue

        print(f"  [{i+1:2d}/{len(rows)}] {mint}...", end=" ", flush=True)
        row, ok = enrich_row(row, profiler)
        rows[i] = row

        if ok:
            enriched += 1
            conc = row.get("baseline_concentration", 0)
            mint_auth = row.get("baseline_mint_authority_active", False)
            print(f"top10={conc*100:.1f}% mint_auth={mint_auth}")
        else:
            errors += 1
            print("FAILED")

        if i < len(rows) - 1:
            time.sleep(RATE_LIMIT_SLEEP)

    stats = {
        "file": path.name,
        "total": len(rows),
        "enriched": enriched,
        "skipped": skipped,
        "errors": errors,
    }

    if not dry_run:
        # Backup original before overwriting
        backup = path.with_suffix(".jsonl.bak")
        shutil.copy2(path, backup)
        write_jsonl(path, rows)
        print(f"[enrich]   Written to {path.name} (backup: {backup.name})")
    else:
        print(f"[enrich]   DRY RUN — no files written")

    coverage = enriched + skipped
    if coverage < MIN_COVERAGE and len(rows) >= MIN_COVERAGE:
        print(f"[enrich]   WARN: only {coverage}/{len(rows)} mints enriched (threshold: {MIN_COVERAGE})")

    return stats


def main() -> int:
    dry_run = "--dry-run" in sys.argv
    target_file = next((a for a in sys.argv[1:] if not a.startswith("--")), None)

    load_env()

    auth_key = os.environ.get("HELIUS_API_KEY")
    if not auth_key:
        print("ERROR: HELIUS_API_KEY not set. Source ~/.cynic-env first.", file=sys.stderr)
        return 1

    print(f"[enrich] Baseline verdict enrichment — Helius backfill")
    print(f"[enrich] Verdicts dir: {VERDICTS_DIR}")
    print(f"[enrich] Dry run: {dry_run}")

    if not VERDICTS_DIR.exists():
        print(f"ERROR: {VERDICTS_DIR} not found", file=sys.stderr)
        return 1

    profiler = HeliusTokenProfiler(auth_key=auth_key)

    # Target specific file or all baseline files
    if target_file:
        files = [VERDICTS_DIR / target_file]
        if not files[0].exists():
            print(f"ERROR: {files[0]} not found", file=sys.stderr)
            return 1
    else:
        files = sorted(VERDICTS_DIR.glob("verdicts_2026-05-*.jsonl"))

    if not files:
        print("ERROR: no verdict JSONL files found", file=sys.stderr)
        return 1

    print(f"[enrich] Files to process: {[f.name for f in files]}")

    all_stats = []
    for path in files:
        stats = process_file(path, profiler, dry_run)
        all_stats.append(stats)

    # Summary
    print("\n[enrich] ── Summary ──────────────────────────────")
    total_enriched = sum(s["enriched"] for s in all_stats)
    total_rows = sum(s["total"] for s in all_stats)
    total_errors = sum(s["errors"] for s in all_stats)
    for s in all_stats:
        print(f"  {s['file']}: {s['enriched']} enriched, {s['skipped']} skipped, {s['errors']} errors / {s['total']} total")
    print(f"  Total: {total_enriched}/{total_rows} enriched, {total_errors} errors")

    if total_errors > 5:
        print(f"[enrich] WARN: {total_errors} errors — check Helius rate limits or dead tokens")

    if dry_run:
        print("[enrich] DRY RUN complete — re-run without --dry-run to write files")
    else:
        print("[enrich] Enrichment complete. Run outcome_measurement_t7.sh to measure divergence.")

    return 0 if total_errors < total_rows else 1


if __name__ == "__main__":
    sys.exit(main())
