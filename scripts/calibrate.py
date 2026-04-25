#!/usr/bin/env python3
"""
CYNIC Calibration Runner — token-analysis domain.

Measures per-Dog FIDELITY (ordering accuracy) and VERIFY (test-retest σ_intra).
Protocol: 9 stimuli × 3 runs = 27 kernel calls via POST /judge (crystals=false).

Usage:
    python3 scripts/calibrate.py              # full run (27 calls)
    python3 scripts/calibrate.py --runs 1     # quick check (9 calls)
    python3 scripts/calibrate.py --feed-ccm   # SolRPDS → /judge (crystals=true)
    python3 scripts/calibrate.py --feed-ccm --max 200  # feed N tokens

Output: table + JSON to stdout (--json flag)

References:
    docs/domains/calibration-token-analysis.md
"""

import json
import os
import sys
import time
import math
import csv
import argparse
from pathlib import Path

# ── Env load ──────────────────────────────────────────────────
def load_env():
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, _, val = line.partition("=")
                val = val.strip().strip('"').strip("'")
                os.environ.setdefault(key.strip(), val)

load_env()

KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
API_KEY = os.environ.get("CYNIC_API_KEY", "")

# ── 9 Calibration stimuli (docs/domains/calibration-token-analysis.md) ──
# Format: (id, format_group, expected_tier, content)
# expected_tier: "howl" > "ambiguous" > "bark" — ordering must hold within group

STIMULI = [
    # ── FORMAT 1: Metadata brute (on-chain numbers only) ──
    {
        "id": "M-HOWL",
        "group": "format1",
        "tier": "howl",
        "content": (
            "Token: JUP (Jupiter). Type: FungibleToken. Program: SPL Token.\n"
            "Supply: 6,863,982,190. Decimals: 6. Price: $0.177 USDC.\n"
            "Mint authority: EXISTS (61aq585V8cR2sZBeawJFt2NPqmN7zDi1sws4KLs5xHXV).\n"
            "Metadata: Mutable. Description: \"Official governance token for Jupiter.\"\n"
            "Top holder concentration: ~4.2% (286 JUP in top account of sampled set).\n"
            "Age: 14+ months. DEX aggregator with $2B+ TVL across Solana."
        ),
    },
    {
        "id": "M-AMBIGUOUS",
        "group": "format1",
        "tier": "ambiguous",
        "content": (
            "Token: Bonk. Type: FungibleToken. Program: SPL Token.\n"
            "Supply: 87,994,739,245,442. Decimals: 5. Price: $0.000006 USDC.\n"
            "Mint authority: EXISTS (9AhKqLR67hwapvG8SA2JFXaCshXc9nALJjpKaHZrsbkw).\n"
            "Metadata: Mutable. Description: \"The Official Bonk Inu token.\"\n"
            "Top holder: 34M BONK (~0.00004% of supply). Distribution: wide.\n"
            "Age: 2+ years. Community meme token. Major exchange listings."
        ),
    },
    {
        "id": "M-BARK",
        "group": "format1",
        "tier": "bark",
        "content": (
            "Token: MOONAI. Type: FungibleToken. Program: SPL Token.\n"
            "Supply: 1,000,000,000. Decimals: 9. Price: $0.00000034 USDC.\n"
            "Mint authority: ACTIVE (deployer wallet, created 2 hours ago).\n"
            "Freeze authority: ACTIVE. Metadata: Mutable.\n"
            "Holders: 3 wallets. Top holder: 94.7% of supply. LP: None detected.\n"
            "Age: 2 hours. No website, no social links. Deployer funded by mixer."
        ),
    },
    # ── FORMAT 2: Narrative enrichie (project + context) ──
    {
        "id": "N-HOWL",
        "group": "format2",
        "tier": "howl",
        "content": (
            "Pyth Network is a first-party oracle that publishes financial market data on-chain. "
            "Over 500 price feeds across crypto, equities, FX, and commodities. "
            "Used by major Solana protocols (Jupiter, Drift, Marginfi, Kamino). "
            "Backed by Jump Trading. Governance token PYTH launched Nov 2023 with 85,000+ unique claimers. "
            "Cross-chain deployment via Wormhole to 50+ chains. "
            "Revenue model: data licensing fees from integrators. "
            "Team: public, experienced (ex-Jump, ex-FTX Research). "
            "Authorities: governed by Pyth DAO multisig, not single deployer."
        ),
    },
    {
        "id": "N-AMBIGUOUS",
        "group": "format2",
        "tier": "ambiguous",
        "content": (
            "Kamino Finance is a Solana DeFi protocol offering automated liquidity vaults, "
            "lending/borrowing, and leveraged yield strategies. $1.2B TVL. "
            "Team is pseudonymous but consistently active since 2023. "
            "Has been audited by OtterSec and Sec3. Token KMNO launched Apr 2024. "
            "Supply distribution: 30% to community via points program, 20% team (2-year vest). "
            "Criticism: some users report liquidation issues during high volatility. "
            "Protocol has survived 3 major Solana outages without fund loss. "
            "Revenue: protocol fees from lending spreads, currently $2M/month. "
            "Risk: concentrated TVL in a few vaults, smart contract upgrade authority held by 3/7 multisig."
        ),
    },
    {
        "id": "N-BARK",
        "group": "format2",
        "tier": "bark",
        "content": (
            "SolanaGPT-X claims to be \"the first AI-powered gaming metaverse on Solana.\" "
            "Website launched 3 days ago, copied template from a known rugpull. "
            "Whitepaper is 2 pages, mostly buzzwords (\"quantum neural blockchain AI synergy\"). "
            "Team: anonymous, no LinkedIn, no GitHub history. "
            "Token launched on pump.fun 6 hours ago. "
            "Telegram group has 12,000 members (growth from 0 to 12K in 4 hours — likely botted). "
            "Smart contract not verified. No audit. LP locked for 7 days only (minimum pump.fun default)."
        ),
    },
    # ── FORMAT 3: Red flag patterns (behavioral signals) ──
    {
        "id": "R-HOWL",
        "group": "format3",
        "tier": "howl",
        "content": (
            "\"We've completed our Q1 audit with OtterSec — full report published at docs.example.com/audit. "
            "Three medium-severity findings were identified and patched before mainnet deployment. "
            "Our treasury diversification proposal (PROP-47) passed governance with 73% approval. "
            "Next milestone: v2 lending module, estimated June. "
            "We're hiring a senior Rust engineer — see our careers page. "
            "Current TVL: $340M, down 12% from ATH due to broader market conditions.\""
        ),
    },
    {
        "id": "R-AMBIGUOUS",
        "group": "format3",
        "tier": "ambiguous",
        "content": (
            "\"Our zero-knowledge proof verification module is now live on devnet. "
            "Benchmarks show 340ms verification time, 60% faster than competitors using Groth16. "
            "We've open-sourced the prover at github.com/example/zkprover (47 stars, 3 contributors). "
            "However, we should note that our mainnet launch has been delayed twice — "
            "originally planned for Q4 2025, now targeting Q3 2026. "
            "Our lead cryptographer left the project in January for personal reasons. "
            "We've hired a replacement from Trail of Bits. "
            "Current funding: $1.2M remaining from seed round, runway ~8 months at current burn rate.\""
        ),
    },
    {
        "id": "R-BARK",
        "group": "format3",
        "tier": "bark",
        "content": (
            "\"LAUNCHING IN 30 MINUTES! This is NOT a drill! "
            "The devs are DOXXED (trust me bro). "
            "Already 50x from presale and we haven't even listed yet! "
            "Get in NOW before the CEX listing announcement tomorrow. "
            "NFA but this is the easiest 1000x of your life. "
            "Telegram link in bio. Whitelist spots almost GONE. "
            "Don't be the one who missed $PEPE. "
            "Burn mechanism activated — supply going to ZERO.\""
        ),
    },
]

TIER_RANK = {"howl": 3, "ambiguous": 2, "bark": 1}

# ── Kernel call ────────────────────────────────────────────────

def get_healthy_dogs() -> list[str] | None:
    """Query /health and return Dogs with circuit=closed. Returns None on error."""
    import urllib.request, urllib.error
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    req = urllib.request.Request(
        f"{addr}/health",
        headers={"Authorization": f"Bearer {API_KEY}"},
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        # 503 = degraded kernel — body still contains full health JSON for auth'd callers
        if e.code in (503, 200):
            data = json.loads(e.read())
        else:
            return None
    except Exception:
        return None
    healthy = [d["id"] for d in data.get("dogs", []) if d.get("circuit") == "closed"]
    return healthy or None


def judge(
    content: str,
    domain: str = "token-analysis",
    crystals: bool = False,
    dogs: list[str] | None = None,
    max_retries: int = 3,
) -> dict | None:
    """Call /judge with retry on 429 (rate limit). dogs= filters jury to specific Dogs."""
    import urllib.request, urllib.error
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    payload: dict = {"content": content, "domain": domain, "crystals": crystals}
    if dogs:
        payload["dogs"] = dogs
    body = json.dumps(payload).encode()

    for attempt in range(max_retries + 1):
        req = urllib.request.Request(
            f"{addr}/judge",
            data=body,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {API_KEY}",
            },
            method="POST",
        )
        t0 = time.monotonic()
        try:
            with urllib.request.urlopen(req, timeout=180) as resp:
                data = json.loads(resp.read())
            data["_latency_ms"] = int((time.monotonic() - t0) * 1000)
            return data
        except urllib.error.HTTPError as e:
            if e.code == 429 and attempt < max_retries:
                print(f"  [429 rate limit — sleeping 7s, retry {attempt+1}/{max_retries}]", file=sys.stderr)
                time.sleep(7)
                continue
            print(f"  [HTTP {e.code}] {e.read()[:200]}", file=sys.stderr)
            return None
        except Exception as e:
            print(f"  [ERR] {e}", file=sys.stderr)
            return None
    return None


# ── Calibration run ────────────────────────────────────────────

def run_calibration(n_runs: int, verbose: bool = False) -> list[dict]:
    """Run 9 stimuli × n_runs via /judge (crystals=false). Returns all records."""
    healthy_dogs = get_healthy_dogs()
    if healthy_dogs:
        print(f"  Dogs filter: {healthy_dogs}", file=sys.stderr)
    else:
        print("  Dogs filter: none (using all)", file=sys.stderr)
    records = []
    total = len(STIMULI) * n_runs
    done = 0
    for s in STIMULI:
        for run in range(1, n_runs + 1):
            done += 1
            label = f"[{done:2d}/{total}] {s['id']} run{run}"
            print(f"  {label} ...", end="", flush=True, file=sys.stderr)
            verdict = judge(s["content"], domain="token-analysis", crystals=False, dogs=healthy_dogs)
            if verdict is None:
                print(" FAIL", file=sys.stderr)
                records.append({**s, "run": run, "error": True, "q_total": None, "dog_scores": []})
                continue
            q_total = verdict.get("q_score", {}).get("total", 0.0)
            lat = verdict.get("_latency_ms", 0)
            vtype = verdict.get("verdict", "?")
            print(f" {vtype:<5} Q={q_total:.3f} lat={lat}ms", file=sys.stderr)
            records.append({
                **s,
                "run": run,
                "error": False,
                "q_total": q_total,
                "verdict": vtype,
                "latency_ms": lat,
                "dog_scores": verdict.get("dog_scores", []),
            })
    return records


# ── Analysis ───────────────────────────────────────────────────

def stddev(vals: list[float]) -> float:
    if len(vals) < 2:
        return 0.0
    mean = sum(vals) / len(vals)
    return math.sqrt(sum((v - mean) ** 2 for v in vals) / (len(vals) - 1))


def analyze(records: list[dict]) -> dict:
    # σ_intra per stimulus
    by_id: dict[str, list[float]] = {}
    for r in records:
        if not r["error"] and r["q_total"] is not None:
            by_id.setdefault(r["id"], []).append(r["q_total"])

    sigma_intra = {sid: stddev(scores) for sid, scores in by_id.items()}
    mean_scores = {sid: sum(scores) / len(scores) for sid, scores in by_id.items()}

    # Ordering accuracy per format group
    ordering = {}
    for group in ["format1", "format2", "format3"]:
        group_stim = [s for s in STIMULI if s["group"] == group]
        by_tier = {s["tier"]: mean_scores.get(s["id"]) for s in group_stim}
        if all(v is not None for v in by_tier.values()):
            correct = (
                by_tier["howl"] > by_tier["ambiguous"] and
                by_tier["ambiguous"] > by_tier["bark"]
            )
            ordering[group] = {
                "howl_q": by_tier["howl"],
                "ambiguous_q": by_tier["ambiguous"],
                "bark_q": by_tier["bark"],
                "delta_h_b": by_tier["howl"] - by_tier["bark"],
                "correct_order": correct,
            }

    # Per-Dog aggregation
    dog_data: dict[str, dict] = {}
    for r in records:
        if r["error"]:
            continue
        for ds in r.get("dog_scores", []):
            name = ds.get("dog_id", ds.get("dog", "?"))
            if name not in dog_data:
                dog_data[name] = {"q_scores": [], "json_valid": 0, "total": 0}
            dog_data[name]["total"] += 1
            q = ds.get("q_score", {}).get("total", 0) if isinstance(ds.get("q_score"), dict) else ds.get("q_score", 0)
            dog_data[name]["q_scores"].append(q)

    return {
        "sigma_intra": sigma_intra,
        "mean_scores": mean_scores,
        "ordering": ordering,
        "dog_data": dog_data,
    }


def print_report(records: list[dict], analysis: dict):
    print()
    print("══ CALIBRATION RESULTS ══════════════════════════════════════════════")
    print(f"{'Stimulus':<14} {'Tier':<10} {'Mean Q':>7} {'σ_intra':>8} {'VERIFY':>7}")
    print("─" * 55)
    for s in STIMULI:
        sid = s["id"]
        mean = analysis["mean_scores"].get(sid)
        sig = analysis["sigma_intra"].get(sid, 0.0)
        verify_ok = sig < 0.10
        if mean is not None:
            print(f"  {sid:<12} {s['tier']:<10} {mean:>7.3f} {sig:>8.3f} {'✓' if verify_ok else '✗':>7}")
        else:
            print(f"  {sid:<12} {s['tier']:<10} {'N/A':>7} {'N/A':>8} {'?':>7}")

    print()
    print("── Ordering accuracy (FIDELITY) ──────────────────────────────────")
    for group, o in analysis["ordering"].items():
        fmt = {"format1": "F1 Metadata", "format2": "F2 Narrative", "format3": "F3 Behavioral"}[group]
        ok = "✓ CORRECT" if o["correct_order"] else "✗ INVERTED"
        print(f"  {fmt}: HOWL={o['howl_q']:.3f} > AMB={o['ambiguous_q']:.3f} > BARK={o['bark_q']:.3f}"
              f"  Δ(H-B)={o['delta_h_b']:.3f}  {ok}")

    n_ok = sum(1 for o in analysis["ordering"].values() if o["correct_order"])
    print(f"\n  Ordering score: {n_ok}/{len(analysis['ordering'])} formats correct")

    n_verify = sum(1 for v in analysis["sigma_intra"].values() if v < 0.10)
    print(f"  VERIFY (σ<0.10): {n_verify}/{len(analysis['sigma_intra'])} stimuli stable")
    print()


# ── CCM feed from SolRPDS ──────────────────────────────────────

SOLRPDS_PATHS = [
    Path(__file__).parent.parent / "data" / "SolRPDS" / "dataset" / "CSV" / "Jan_2024-Nov_2024.csv",
    Path(__file__).parent.parent / "data" / "SolRPDS" / "dataset" / "CSV" / "2023.csv",
    Path(__file__).parent.parent / "data" / "SolRPDS" / "dataset" / "CSV" / "2022.csv",
    Path(__file__).parent.parent / "data" / "SolRPDS" / "dataset" / "CSV" / "2021.csv",
]


def format_rug_token(row: dict) -> str:
    """Format a SolRPDS row as a token-analysis content string."""
    from datetime import datetime, timezone

    mint = row.get("MINT", "unknown")
    lp = row.get("LIQUIDITY_POOL_ADDRESS", "unknown")
    added = float(row.get("TOTAL_ADDED_LIQUIDITY", 0))
    removed = float(row.get("TOTAL_REMOVED_LIQUIDITY", 0))
    n_adds = row.get("NUM_LIQUIDITY_ADDS", "?")
    n_removes = row.get("NUM_LIQUIDITY_REMOVES", "?")
    ratio = float(row.get("ADD_TO_REMOVE_RATIO", 0))
    status = row.get("INACTIVITY_STATUS", "unknown")
    drain_pct = (removed / added * 100) if added > 0 else 0

    # Timestamp signals — pool lifetime and staleness
    now = datetime.now(tz=timezone.utc)
    def parse_ts(s: str) -> datetime | None:
        if not s:
            return None
        for fmt in ("%Y-%m-%d %H:%M:%S.%f", "%Y-%m-%d %H:%M:%S"):
            try:
                return datetime.strptime(s.strip(), fmt).replace(tzinfo=timezone.utc)
            except ValueError:
                continue
        return None

    first_ts = parse_ts(row.get("FIRST_POOL_ACTIVITY_TIMESTAMP", ""))
    last_ts = parse_ts(row.get("LAST_POOL_ACTIVITY_TIMESTAMP", ""))
    last_swap_ts = parse_ts(row.get("LAST_SWAP_TIMESTAMP", ""))

    pool_age_days: float | None = None
    lifetime_minutes: float | None = None
    days_since_activity: float | None = None

    if first_ts:
        pool_age_days = (now - first_ts).total_seconds() / 86400
    if first_ts and last_ts:
        lifetime_minutes = (last_ts - first_ts).total_seconds() / 60
    if last_swap_ts:
        days_since_activity = (now - last_swap_ts).total_seconds() / 86400

    # Lifetime signal: short-lived pools are textbook rugs
    if lifetime_minutes is not None:
        if lifetime_minutes < 60:
            lifetime_label = f"CRITICAL: pool active for only {lifetime_minutes:.0f} minutes (textbook rug window)"
        elif lifetime_minutes < 24 * 60:
            lifetime_label = f"WARNING: pool active for only {lifetime_minutes/60:.1f} hours"
        else:
            lifetime_label = f"{lifetime_minutes/60/24:.1f} days total pool activity window"
    else:
        lifetime_label = "unknown"

    # Staleness: long-inactive pools are dead/abandoned
    if days_since_activity is not None:
        if days_since_activity > 365:
            stale_label = f"ABANDONED: no swap activity for {days_since_activity:.0f} days (>{days_since_activity/365:.1f} years)"
        elif days_since_activity > 90:
            stale_label = f"INACTIVE: no swap activity for {days_since_activity:.0f} days"
        else:
            stale_label = f"{days_since_activity:.0f} days since last swap"
    else:
        stale_label = "unknown"

    # Launch origin: pump.fun tokens have 98.6% failure rate
    is_pumpfun = mint.endswith("pump")
    origin_label = "pump.fun (98.6% rug rate per Solidus Labs 2025)" if is_pumpfun else "unknown launchpad"

    return (
        f"Solana Token Liquidity Analysis — SolRPDS Historical Record.\n"
        f"Mint: {mint}\n"
        f"Launch origin: {origin_label}\n"
        f"Liquidity pool: {lp}\n"
        f"Pool created: {first_ts.strftime('%Y-%m-%d') if first_ts else 'unknown'}"
        f"{f' ({pool_age_days:.0f} days ago)' if pool_age_days is not None else ''}\n"
        f"Pool lifetime: {lifetime_label}\n"
        f"Last swap activity: {stale_label}\n"
        f"Pool status: {status}\n"
        f"Total liquidity added: {added:,.0f} SOL equivalent\n"
        f"Total liquidity removed: {removed:,.0f} SOL ({drain_pct:.1f}% of added)\n"
        f"Add/remove events: {n_adds} adds, {n_removes} removes\n"
        f"Add-to-remove ratio: {ratio:.3f} (< 1.0 = more removed than added)\n"
        f"Add-to-remove ratio interpretation: {'ratio < 0.5 — majority of liquidity withdrawn' if ratio < 0.5 else 'ratio < 1.0 — more withdrawn than added' if ratio < 1.0 else 'ratio >= 1.0 — net liquidity positive (but check lifetime)'}"
    )


def build_rug_token_index(max_freq: int = 5) -> list[dict]:
    """
    Build a deduplicated index of actual rug tokens from all SolRPDS CSVs.

    Each liquidity pool row pairs a rug token with a well-known counterpart
    (SOL, USDC, USDT, etc.). Tokens appearing > max_freq times are almost
    certainly well-known paired tokens, not rugs. We exclude them.

    Returns: list of unique row dicts for the rug token side of each pool.
    """
    from collections import Counter

    # Pass 1: count mint frequencies across the entire dataset
    freq: Counter = Counter()
    all_rows: list[dict] = []
    for csv_path in SOLRPDS_PATHS:
        if not csv_path.exists():
            continue
        with open(csv_path, newline="") as f:
            for row in csv.DictReader(f):
                mint = row.get("MINT", "")
                freq[mint] += 1
                all_rows.append(row)

    excluded = {mint for mint, count in freq.items() if count > max_freq}
    print(
        f"Dataset: {len(all_rows)} rows | {len(freq)} unique mints | "
        f"{len(excluded)} excluded as paired tokens (freq > {max_freq})",
        file=sys.stderr,
    )

    # Pass 2: deduplicate + filter
    seen: set[str] = set()
    rug_rows: list[dict] = []
    for row in all_rows:
        mint = row.get("MINT", "")
        if mint in excluded or mint in seen:
            continue
        seen.add(mint)
        rug_rows.append(row)

    print(f"Rug token candidates: {len(rug_rows)} unique", file=sys.stderr)
    return rug_rows


def feed_ccm(max_tokens: int, verbose: bool = False, dogs_override: list[str] | None = None):
    """Feed SolRPDS rug-pull tokens to /judge (crystals=true) to build CCM observations."""
    import urllib.request
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"

    def get_crystal_count():
        req = urllib.request.Request(
            f"{addr}/health",
            headers={"Authorization": f"Bearer {API_KEY}"},
        )
        try:
            with urllib.request.urlopen(req, timeout=10) as r:
                data = json.loads(r.read())
            return data.get("crystals", {})
        except Exception:
            return {}

    crystals_before = get_crystal_count()
    print(f"Crystal state before: {crystals_before}", file=sys.stderr)

    if dogs_override is not None:
        active_dogs = dogs_override
        print(f"Dogs filter: {active_dogs} (--dogs override)", file=sys.stderr)
    else:
        active_dogs = get_healthy_dogs()
        if active_dogs:
            print(f"Dogs filter: {active_dogs} (from /health)", file=sys.stderr)
        else:
            print("Dogs filter: none (using all)", file=sys.stderr)

    # Build deduplicated rug token index (excludes SOL/USDC/USDT/etc.)
    rug_rows = build_rug_token_index(max_freq=5)
    total = min(max_tokens, len(rug_rows))
    print(f"Running {total} rug tokens ...", file=sys.stderr)

    fed = 0
    verdicts = []
    for row in rug_rows:
        if fed >= max_tokens:
            break
        content = format_rug_token(row)
        mint = row.get("MINT", "?")[:20]
        print(f"  [{fed+1:4d}/{total}] {mint} ...", end="", flush=True, file=sys.stderr)
        verdict = judge(content, domain="token-analysis", crystals=True, dogs=active_dogs or None)
        if verdict:
            q = verdict.get("q_score", {}).get("total", 0)
            vtype = verdict.get("verdict", "?")
            print(f" {vtype:<5} Q={q:.3f}", file=sys.stderr)
            verdicts.append({"mint": mint, "verdict": vtype, "q": q})
        else:
            print(" FAIL", file=sys.stderr)
        fed += 1

    crystals_after = get_crystal_count()
    print(f"\nCrystal state after: {crystals_after}", file=sys.stderr)
    print(f"Tokens fed: {fed}", file=sys.stderr)

    crystallized = crystals_after.get("crystallized", 0)
    if crystallized > 0:
        print(f"✓ FIRST CRYSTALS: {crystallized} crystallized!", file=sys.stderr)
    else:
        forming = crystals_after.get("forming", 0)
        print(f"Still forming: {forming}. Max obs needed: 21.", file=sys.stderr)

    # Detection rate: SolRPDS = confirmed rugs. BARK (≤0.236) or GROWL (≤0.382) = correct signal.
    if verdicts:
        bark  = sum(1 for v in verdicts if v["q"] <= 0.236)
        growl = sum(1 for v in verdicts if 0.236 < v["q"] <= 0.382)
        wag   = sum(1 for v in verdicts if 0.382 < v["q"] <= 0.528)
        howl  = sum(1 for v in verdicts if v["q"] > 0.528)
        n = len(verdicts)
        detected = bark + growl
        print(f"\n── Detection rate (confirmed rugs, blind signal) ──", file=sys.stderr)
        print(f"  BARK  (≤0.236): {bark:4d} / {n} = {bark/n*100:.1f}%", file=sys.stderr)
        print(f"  GROWL (≤0.382): {growl:4d} / {n} = {growl/n*100:.1f}%", file=sys.stderr)
        print(f"  WAG   (≤0.528): {wag:4d} / {n} = {wag/n*100:.1f}%", file=sys.stderr)
        print(f"  HOWL  (>0.528): {howl:4d} / {n} = {howl/n*100:.1f}%", file=sys.stderr)
        print(f"  ── BARK+GROWL:  {detected:4d} / {n} = {detected/n*100:.1f}% (correct rug signal)", file=sys.stderr)

    return verdicts


# ── Main ───────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=3, help="Runs per stimulus (default 3)")
    parser.add_argument("--feed-ccm", action="store_true", help="Feed SolRPDS to CCM instead")
    parser.add_argument("--max", type=int, default=100, help="Max tokens to feed (--feed-ccm)")
    parser.add_argument("--json", action="store_true", help="Output raw JSON")
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument(
        "--dogs",
        type=str,
        default=None,
        help="Comma-separated Dog IDs to use (overrides health check). "
             "Use to exclude quota-constrained Dogs: "
             "'qwen35-9b-gpu,gemma-4-e4b-core,deterministic-dog'",
    )
    args = parser.parse_args()

    if not KERNEL_ADDR:
        print("ERROR: CYNIC_REST_ADDR not set", file=sys.stderr)
        sys.exit(1)

    # --dogs overrides the automatic health-check filter
    explicit_dogs: list[str] | None = None
    if args.dogs:
        explicit_dogs = [d.strip() for d in args.dogs.split(",") if d.strip()]
        print(f"  Dogs override: {explicit_dogs}", file=sys.stderr)

    if args.feed_ccm:
        results = feed_ccm(args.max, verbose=args.verbose, dogs_override=explicit_dogs)
        if args.json:
            print(json.dumps(results, indent=2))
        return

    print(f"Running calibration: {len(STIMULI)} stimuli × {args.runs} runs = "
          f"{len(STIMULI) * args.runs} kernel calls", file=sys.stderr)
    print(f"Kernel: {KERNEL_ADDR}", file=sys.stderr)
    print(f"crystals=false (calibration mode — no CCM injection)\n", file=sys.stderr)

    records = run_calibration(args.runs, verbose=args.verbose)
    analysis = analyze(records)

    if args.json:
        print(json.dumps({"records": records, "analysis": analysis}, indent=2, default=str))
        return

    print_report(records, analysis)


if __name__ == "__main__":
    main()
