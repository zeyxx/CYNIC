#!/usr/bin/env python3
"""
CYNIC Hermes Curation — Transform high-signal tweets into D1-D6 domain signals.

Reads dataset.jsonl (high-signal tweets), extracts falsifiable claims,
assigns domains, outputs proper DomainSignal JSONL files matching kernel expectations.

Usage:
    python curate_domain_signals.py
    python curate_domain_signals.py --dry-run  # print only, don't write files
"""

import json
import sys
import os
import re
import urllib.request
import urllib.error
from pathlib import Path
from datetime import datetime

DATASET_PATH = Path.home() / ".cynic/organs/hermes/x/dataset.jsonl"
CURATED_DIR = Path.home() / ".cynic/organs/hermes/x/curated"
MIN_SIGNAL_SCORE = 4

# Domain assignment heuristics
DOMAIN_KEYWORDS = {
    "D1": [  # Solana/Tokens - rug/scam/pump/dump
        "rug", "scam", "pump", "dump", "liquidity", "supply", "holders",
        "bundled", "rug pull", "slow rug", "fee assignment"
    ],
    "D2": [  # Inference/LLM - model performance, optimization
        "inference", "llm", "vllm", "sglang", "quantization", "gguf", "tok/s",
        "throughput", "latency", "model", "gpu", "cuda", "kv cache", "attention",
        "tph", "tps", "agent", "thinking"
    ],
    "D4": [  # Security - exploits, vulnerabilities, scams
        "security", "exploit", "vulnerability", "hack", "phishing", "malware",
        "warning", "scam", "fraud", "bridge", "cross-chain"
    ],
    "D5": [  # Macro/Market - trends, price action, ecosystem
        "market", "price", "bull", "bear", "trend", "volatility", "cycle",
        "adoption", "ecosystem", "regulation", "gmt", "sol", "eth", "btc"
    ],
    "D6": [  # Epistemology - truth, calibration, proof
        "proof", "evidence", "verified", "confirmed", "predicted", "forecast",
        "accuracy", "epistemic", "truth", "certainty", "error"
    ]
}

def extract_keywords(text: str) -> list[str]:
    """Extract meaningful keywords from tweet text (not just "war")."""
    if not text:
        return []

    text_lower = text.lower()

    # Remove URLs, mentions, hashtags for keyword extraction
    text_clean = re.sub(r'https?://\S+', '', text_lower)
    text_clean = re.sub(r'@\w+', '', text_clean)
    text_clean = re.sub(r'#\w+', '', text_clean)
    text_clean = re.sub(r'\$[A-Z]{2,10}', '', text_clean)  # Remove cashtags

    # Split on non-alphanumeric
    words = re.findall(r'\b[a-z]{3,}\b', text_clean)

    # Filter common stopwords
    stopwords = {
        'the', 'and', 'for', 'that', 'this', 'with', 'from', 'are', 'but',
        'not', 'have', 'been', 'will', 'can', 'just', 'like', 'one', 'new',
        'who', 'what', 'when', 'where', 'why', 'how', 'all', 'some', 'any',
        'out', 'get', 'has', 'his', 'her', 'its', 'our', 'your', 'make',
        'day', 'time', 'way', 'year', 'work', 'people', 'did', 'don', 'want',
        'been', 'know', 'said', 'say', 'use', 'find', 'give', 'tell', 'come'
    }

    keywords = [w for w in words if w not in stopwords and len(w) > 2]
    return list(set(keywords[:10]))  # Top 10 unique keywords

def assign_domain(text: str, narratives: list[str], tier: str) -> str:
    """Assign tweet to a domain D1-D6."""
    text_lower = text.lower()

    # Priority: hard rules first (narratives are explicit tags)
    if any(n in narratives for n in ['rug_warning', 'warning', 'scam']):
        if 'security' in text_lower or 'exploit' in text_lower or 'vulnerability' in text_lower:
            return "D4"  # Security
        return "D1"  # Default to token scams

    if any(n in narratives for n in ['inference', 'ai_crypto', 'llm']):
        return "D2"

    if any(n in narratives for n in ['macro', 'market', 'trend']):
        return "D5"

    # Keyword matching
    for domain, keywords in DOMAIN_KEYWORDS.items():
        if any(kw in text_lower for kw in keywords):
            return domain

    # Tier-based heuristic
    if tier == "curated":
        return "D6"  # Curated sources often have epistemic/truth claims
    if tier in ["whale", "influencer"]:
        return "D5"  # Whales/influencers talk macro

    return "D1"  # Default to tokens

def extract_falsifiable_claim(text: str, domain: str) -> str:
    """Extract or generate a falsifiable claim from tweet."""

    text_lower = text.lower()

    # Step 1: Look for explicit if-then claims first (highest priority)
    if "if" in text_lower and ("then" in text_lower or ":" in text):
        match = re.search(r'if[^.!?]*[.:?!]', text, re.IGNORECASE | re.DOTALL)
        if match:
            return match.group(0)[:200]

    # Step 2: Domain-specific patterns — only if strong indicators present
    # D1: Tokens/Solana — strict pattern matching
    if domain == "D1":
        if any(kw in text_lower for kw in ["rug pull", "slow rug", "honeypot"]):
            return "If pattern manifests: token will lose >80% value within 6mo"
        if "liquidity" in text_lower and ("locked" in text_lower or "permanently" in text_lower):
            return "If verified: liquidity mechanics prevent early withdrawal"
        # Generic D1 fallback
        return "If verified: token mechanism claim is accurate"

    # D2: Inference/LLM — specific performance claims only
    if domain == "D2":
        if any(kw in text_lower for kw in ["tok/s", "tokens/sec", "throughput"]) and \
           any(num in text for num in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]):
            return "If benchmarked: claimed throughput achievable under stated conditions"
        if any(kw in text_lower for kw in ["latency", "inference time", "response time"]):
            return "If measured: latency within specified bounds"
        # Generic D2 fallback
        return "If verified: performance claim is testable"

    # D4: Security — only explicit vuln/exploit mentions
    if domain == "D4":
        if any(kw in text_lower for kw in ["vulnerability", "exploit", "cve", "hack", "breach"]):
            return "If tested: vulnerability is reproducible and exploitable"
        # Generic D4 fallback
        return "If verified: security claim can be validated"

    # D5: Macro/Market — only specific metrics
    if domain == "D5":
        if any(kw in text_lower for kw in ["tps", "tvl", "volume", "adoption", "growth"]):
            return "If measured: metric value matches on-chain data"
        # Generic D5 fallback
        return "If verified: market claim aligns with observable data"

    # D6: Epistemology — prediction/proof claims only
    if domain == "D6":
        if any(kw in text_lower for kw in ["predicted", "forecast", "accuracy"]):
            return "If verified: prediction accuracy > random baseline"
        if any(kw in text_lower for kw in ["proof", "evidence", "verified"]):
            return "If tested: claim withstands critical examination"
        # Generic D6 fallback
        return "If verified: claim's truth value is determinable"

    # Step 3: Generic fallback (safe default)
    first_sentence = text.split('.')[0].strip()
    if 20 < len(first_sentence) < 150:
        return f"If verified: {first_sentence}"
    return "If verified: claim content is accurate"

def create_signal_id(author: str, tweet_id: str, domain: str, idx: int) -> str:
    """Create a unique signal ID."""
    return f"{domain}_{author}_{tweet_id}_{idx:04d}"

def curate_tweets(dry_run=False):
    """Main curation process."""

    if not DATASET_PATH.exists():
        print(f"ERROR: Dataset not found at {DATASET_PATH}")
        return False

    with open(DATASET_PATH) as f:
        tweets = [json.loads(line) for line in f if line.strip()]

    print(f"Loaded {len(tweets)} tweets from dataset")

    # Filter high-signal tweets
    high_signal = [t for t in tweets if t.get("signal_score", 0) >= MIN_SIGNAL_SCORE]
    print(f"High-signal tweets (score >= {MIN_SIGNAL_SCORE}): {len(high_signal)}")

    # Skip known bot/scam accounts
    skip_authors = {
        "gary_recovery_", "thelipglossguy_", "assertguard", "ep_peter",
        "arthurbomb", "emcruzzz", "jeffersonyoan", "njabulobhabha"
    }

    high_signal = [
        t for t in high_signal
        if t.get("author_screen_name", "").lower() not in skip_authors
    ]
    print(f"After filtering known bots: {len(high_signal)}")

    # Group by domain
    by_domain = {}
    signal_counter = {}

    for tweet in high_signal:
        author = tweet.get("author_screen_name", "unknown")
        text = tweet.get("text", "")
        tid = tweet.get("tweet_id", "")
        tier = tweet.get("author_tier", "unknown")
        narratives = tweet.get("narratives", [])

        domain = assign_domain(text, narratives, tier)

        if domain not in by_domain:
            by_domain[domain] = []
            signal_counter[domain] = 0

        # Create domain signal
        idx = signal_counter[domain]
        signal_counter[domain] += 1

        # Strength scaling: score 4→0.4, score 7→0.9
        # K15 filter excludes < 0.4, so score-4 tweets barely pass
        raw_score = tweet.get("signal_score", 0)
        strength = min(0.95, (raw_score - MIN_SIGNAL_SCORE) / 3 * 0.5 + 0.4)

        signal = {
            "signal_id": create_signal_id(author, tid, domain, idx),
            "domain": domain,
            "pattern": text[:250],  # Tweet is the pattern
            "strength": strength,  # 0.5-1.0 range; > 0.4 threshold ensures inclusion
            "sources": [tid, author],  # Tweet ID and author
            "falsifiable_claim": extract_falsifiable_claim(text, domain),
            "narratives": narratives,
            "author_tier": tier
        }

        by_domain[domain].append(signal)

    print(f"\nCurated by domain:")
    for domain, signals in sorted(by_domain.items()):
        print(f"  {domain}: {len(signals)} signals")

    if dry_run:
        print("\n[DRY RUN] Sample signals:")
        for domain in sorted(by_domain.keys())[:1]:
            sample = by_domain[domain][0]
            print(f"\n{sample['signal_id']}:")
            print(f"  Pattern: {sample['pattern'][:100]}...")
            print(f"  Claim: {sample['falsifiable_claim']}")
        return True

    # Write curated files
    if not CURATED_DIR.exists():
        CURATED_DIR.mkdir(parents=True, exist_ok=True)

    for domain, signals in by_domain.items():
        filepath = CURATED_DIR / f"{domain}_curated.jsonl"
        with open(filepath, 'w') as f:
            for signal in signals:
                f.write(json.dumps(signal) + '\n')
        print(f"✓ Wrote {len(signals)} signals to {filepath}")

    # Write curation yield summary — consumed by search_generation.py
    # to shift search budget toward domains producing high-signal content
    yield_path = Path.home() / ".cynic/organs/hermes/x/curation_yield.json"
    yield_data = {
        "timestamp": datetime.now().isoformat(),
        "total_tweets": sum(len(s) for s in by_domain.values()),
        "high_signal_tweets": sum(len(s) for s in by_domain.values()),
        "domains": {
            domain: {
                "signals": len(sigs),
                "pct_of_total": round(len(sigs) / max(sum(len(s) for s in by_domain.values()), 1) * 100, 1),
            }
            for domain, sigs in by_domain.items()
        },
    }
    try:
        with open(yield_path, 'w') as f:
            json.dump(yield_data, f, indent=2)
        print(f"✓ Wrote curation yield to {yield_path}")
    except IOError as e:
        print(f"⚠ Failed to write curation yield: {e}")

    # Crystal bridge: feed top signals to kernel for judgment → crystallization
    post_to_kernel(by_domain)

    return True

## ── Crystal Bridge: POST top signals to kernel /observe ──
# Nightshift Phase 2 judges observations from SurrealDB → verdicts → crystals.
# This bridge feeds curated Hermes signals into that pipeline.
# JSONL stays as raw cache. Kernel becomes the single nervous system.

BRIDGE_TOP_N = 3  # top N signals per domain per cycle (max 18 total)

def post_to_kernel(signals_by_domain: dict[str, list[dict]]) -> int:
    """POST top-N curated signals per domain to kernel /observe.
    Returns count of successfully posted observations."""
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if not addr or not key:
        print("⚠ CYNIC_REST_ADDR or CYNIC_API_KEY not set — bridge skipped")
        return 0

    url = f"http://{addr}/observe"
    posted = 0
    errors = 0

    for domain, signals in sorted(signals_by_domain.items()):
        # Sort by strength descending, take top N
        top = sorted(signals, key=lambda s: s.get("strength", 0), reverse=True)[:BRIDGE_TOP_N]

        for signal in top:
            body = json.dumps({
                "tool": "hermes-curation",
                "target": signal.get("signal_id", "unknown"),
                "domain": domain.lower(),
                "context": signal.get("pattern", "")[:200],
                "agent_id": "hermes-x-organ",
                "tags": ["hermes-curated", f"domain-{domain}"],
            }).encode("utf-8")

            req = urllib.request.Request(
                url,
                data=body,
                headers={
                    "Authorization": f"Bearer {key}",
                    "Content-Type": "application/json",
                },
                method="POST",
            )

            try:
                with urllib.request.urlopen(req, timeout=5) as resp:
                    if resp.status < 300:
                        posted += 1
                    else:
                        errors += 1
            except (urllib.error.URLError, urllib.error.HTTPError, OSError) as e:
                errors += 1
                if errors <= 2:
                    print(f"⚠ Bridge POST failed for {signal.get('signal_id','?')}: {e}")

    if posted > 0:
        print(f"✓ Bridge: {posted} signals → kernel /observe ({errors} errors)")
    elif errors > 0:
        print(f"⚠ Bridge: 0 posted, {errors} errors (kernel may be down)")
    return posted


if __name__ == "__main__":
    dry_run = "--dry-run" in sys.argv
    if curate_tweets(dry_run=dry_run):
        sys.exit(0)
    else:
        sys.exit(1)
