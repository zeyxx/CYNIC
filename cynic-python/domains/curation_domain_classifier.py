#!/usr/bin/env python3
"""
Domain Curation Classifier — Keyword heuristic for D1-D6 domain mapping.
Reads raw X.com GraphQL captures, extracts tweet text, classifies per domain.
"""

import json
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Tuple

# Domain keywords
DOMAIN_KEYWORDS = {
    "D1": {
        "name": "Token/Solana",
        "keywords": [
            "solana", "token", "swap", "rug", "mint", "pump.fun", "raydium",
            "liquidity", "holders", "contract", "honeypot", "scam", "launch",
            "dex", "cex", "trading", "volume", "price", "market cap", "circulating"
        ]
    },
    "D2": {
        "name": "Inference/LLM",
        "keywords": [
            "llm", "model", "claude", "gemini", "gpt", "qwen", "reasoning",
            "inference", "latency", "cost", "context", "token", "embedding",
            "fine-tune", "training", "ai", "neural", "transformer"
        ]
    },
    "D3": {
        "name": "Sovereignty",
        "keywords": [
            "custody", "self-hosted", "decentralized", "validator", "node",
            "oracle", "incentive", "consensus", "censorship", "resistance",
            "sovereign", "peer-to-peer", "p2p", "proof", "stake"
        ]
    },
    "D4": {
        "name": "Security/Scams",
        "keywords": [
            "honeypot", "scam", "exploit", "vulnerability", "backdoor",
            "tax", "blocked", "transfer", "rug pull", "compromise",
            "security", "audit", "verified", "contract", "attack"
        ]
    },
    "D5": {
        "name": "Macro/Politics",
        "keywords": [
            "btc", "bitcoin", "volatility", "correlation", "funding", "liquidation",
            "cascade", "leverage", "short", "long", "hedge", "political",
            "regulation", "sec", "policy", "bull", "bear"
        ]
    },
    "D6": {
        "name": "Epistemology",
        "keywords": [
            "confidence", "certainty", "calibration", "probability", "likelihood",
            "doubt", "skeptic", "unknown", "uncertain", "bounds", "threshold",
            "epistemic", "knowledge", "justified", "belief"
        ]
    }
}

def extract_tweet_text(capture_file: Path) -> List[str]:
    """Extract tweet texts from a raw X.com GraphQL capture."""
    texts = []
    try:
        with open(capture_file) as f:
            data = json.load(f)

        response = data.get('response', {})
        home_data = response.get('data', {}).get('home', {})
        timeline = home_data.get('home_timeline_urt', {})
        instructions = timeline.get('instructions', [])

        for instr in instructions:
            entries = instr.get('entries', [])
            for entry in entries:
                content = entry.get('content', {})
                item = content.get('itemContent', {})
                tweet_results = item.get('tweet_results', {})
                result = tweet_results.get('result', {})
                legacy = result.get('legacy', {})
                text = legacy.get('full_text', '')

                if text.strip():
                    texts.append(text)
    except Exception as e:
        pass  # Silent fail for malformed captures

    return texts

def classify_text(text: str) -> Dict[str, int]:
    """Score text against all domain keywords."""
    scores = defaultdict(int)
    text_lower = text.lower()

    for domain, domain_info in DOMAIN_KEYWORDS.items():
        keywords = domain_info['keywords']
        for keyword in keywords:
            if keyword in text_lower:
                scores[domain] += 1

    return scores

def assign_domain(scores: Dict[str, int]) -> Tuple[str, int]:
    """Assign a single domain based on keyword scores."""
    if not scores:
        return None, 0

    domain = max(scores, key=scores.get)
    score = scores[domain]
    return domain, score

def curate_captures(captures_dir: Path, output_dir: Path) -> Dict[str, int]:
    """Process all captures and generate curated datasets per domain."""
    output_dir.mkdir(parents=True, exist_ok=True)

    # Domain datasets
    datasets = {f"D{i}": [] for i in range(1, 7)}
    stats = defaultdict(int)
    unclassified = []

    capture_files = list(captures_dir.glob('*.json'))
    print(f"Processing {len(capture_files)} captures...")

    for i, capture_file in enumerate(capture_files):
        if (i + 1) % 50 == 0:
            print(f"  {i + 1}/{len(capture_files)}")

        texts = extract_tweet_text(capture_file)

        for text in texts:
            scores = classify_text(text)
            domain, score = assign_domain(scores)

            if domain and score > 0:
                datasets[domain].append({
                    "text": text,
                    "score": score,
                    "source": capture_file.name
                })
                stats[domain] += 1
            else:
                unclassified.append(text)

            stats['total'] += 1

    # Write curated datasets
    for domain, records in datasets.items():
        output_file = output_dir / f"{domain}.jsonl"
        with open(output_file, 'w') as f:
            for record in records:
                f.write(json.dumps(record) + '\n')
        print(f"✓ {domain} ({len(records)} records) → {output_file}")

    # Unclassified log
    unclassified_file = output_dir / "unclassified.jsonl"
    with open(unclassified_file, 'w') as f:
        for text in unclassified:
            f.write(json.dumps({"text": text}) + '\n')
    print(f"✓ Unclassified ({len(unclassified)}) → {unclassified_file}")

    return stats

def main():
    captures_dir = Path.home() / ".cynic/organs/hermes/x/captures"
    output_dir = Path.home() / ".cynic/organs/hermes/x/curated"

    if not captures_dir.exists():
        print(f"Captures dir not found: {captures_dir}")
        return

    print(f"Domain Curation — Keyword Heuristic Classifier")
    print(f"Input: {captures_dir}")
    print(f"Output: {output_dir}")
    print()

    stats = curate_captures(captures_dir, output_dir)

    print()
    print("Statistics:")
    for domain in [f"D{i}" for i in range(1, 7)]:
        count = stats.get(domain, 0)
        pct = 100.0 * count / stats['total'] if stats['total'] > 0 else 0
        print(f"  {domain}: {count:6d} ({pct:5.1f}%)")

    unclassified_count = stats['total'] - sum(stats.get(f"D{i}", 0) for i in range(1, 7))
    unclassified_pct = 100.0 * unclassified_count / stats['total'] if stats['total'] > 0 else 0
    print(f"  Unclassified: {unclassified_count:3d} ({unclassified_pct:5.1f}%)")
    print(f"  Total texts:  {stats['total']:6d}")

if __name__ == "__main__":
    main()
