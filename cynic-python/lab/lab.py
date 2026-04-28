#!/usr/bin/env python3
"""
CYNIC Data Lab — Orchestrate analyses, produce briefings.

Minimal Phase Lab-1 implementation:
1. Standardize dataset (2,287 tweets)
2. Run 3 core analyses (distribution, disagreement, false_negatives)
3. Write briefing for Hermes/Claude

Usage:
  python lab.py --dataset ~/.cynic/organs/hermes/x/dataset.jsonl \
                --output ~/.cynic/organs/hermes/x/lab_briefing_latest.json
"""

import json
import os
from collections import Counter
from datetime import datetime
from pathlib import Path
from statistics import mean, stdev, median
from typing import Optional

import yaml


def pearson_correlation(x: list, y: list) -> float:
    """Calculate Pearson correlation coefficient between two lists."""
    if len(x) < 2 or len(y) < 2 or len(x) != len(y):
        return 0.0

    mean_x = sum(x) / len(x)
    mean_y = sum(y) / len(y)

    cov = sum((x[i] - mean_x) * (y[i] - mean_y) for i in range(len(x))) / (len(x) - 1)
    var_x = sum((xi - mean_x) ** 2 for xi in x) / (len(x) - 1)
    var_y = sum((yi - mean_y) ** 2 for yi in y) / (len(y) - 1)

    if var_x == 0 or var_y == 0:
        return 0.0

    return cov / (var_x ** 0.5 * var_y ** 0.5)


# ── CONFIG ──

def load_config(config_dir: str = "config") -> dict:
    """Load domains, axioms, Dogs from YAML."""
    config = {}
    for fname in ["domains.yaml", "axioms.yaml"]:
        fpath = Path(config_dir) / fname
        if fpath.exists():
            with open(fpath) as f:
                config[fname.replace(".yaml", "")] = yaml.safe_load(f)
    return config


def load_curated_ground_truth(organ_dir: str = None) -> dict:
    """Load curated D*.jsonl files as ground truth per domain.

    Returns: {domain_id: [curated_tweet_dicts]}
    """
    if organ_dir is None:
        organ_dir = str(Path.home() / ".cynic" / "organs" / "hermes" / "x")

    curated_dir = Path(organ_dir) / "curated"
    curated = {}

    if not curated_dir.exists():
        return curated

    for jsonl_file in sorted(curated_dir.glob("D*_curated.jsonl")):
        domain_id = jsonl_file.stem.replace("_curated", "")
        curated[domain_id] = []

        try:
            with open(jsonl_file) as f:
                for line in f:
                    try:
                        curated[domain_id].append(json.loads(line.strip()))
                    except json.JSONDecodeError:
                        continue
        except IOError:
            continue

    return curated


def load_verdicts_from_organ(organ_dir: str = None) -> list:
    """Load verdicts from organ directory (verdicts/ subdir).

    K15 consumer: reads verdicts written by daemon to measure signal_score quality.
    Schema per file: {tweet_id, signal_score, verdict: {q_score, ...}, ...}
    """
    if organ_dir is None:
        organ_dir = str(Path.home() / ".cynic" / "organs" / "hermes" / "x")

    verdict_dir = Path(organ_dir) / "verdicts"
    verdicts = []

    if verdict_dir.exists():
        for json_file in sorted(verdict_dir.glob("*.json")):
            try:
                with open(json_file) as f:
                    verdict = json.load(f)
                    verdicts.append(verdict)
            except (json.JSONDecodeError, IOError):
                continue

    return verdicts


def load_observations_from_organ(organ_dir: str = None) -> list:
    """Load observations from organ directory (JSON files in observations/ subdir).
    K15 workaround: Since /observe routing to organ is broken (issue TBD),
    also attempt to load from kernel via REST fallback.
    """
    if organ_dir is None:
        organ_dir = str(Path.home() / ".cynic" / "organs" / "hermes" / "x")

    obs_dir = Path(organ_dir) / "observations"
    observations = []

    if obs_dir.exists():
        for json_file in sorted(obs_dir.glob("*.json")):
            try:
                with open(json_file) as f:
                    obs = json.load(f)
                    observations.append(obs)
            except (json.JSONDecodeError, IOError):
                continue

    return observations


# ── STANDARDIZE ──

def standardize_tweet(raw: dict) -> dict:
    """Normalize raw tweet to canonical format."""
    return {
        "id": raw.get("tweet_id", raw.get("id")),
        "text": raw.get("text", ""),
        "signal": {
            "version": "2",  # Current heuristic version
            "score": raw.get("signal_score", 0),
        },
        "author": {
            "screen_name": raw.get("author_screen_name"),
            "tier": raw.get("author_tier", "unknown"),
            "verified": raw.get("author_verified", False),
            "followers": raw.get("author_followers_count", 0),
        },
        "engagement": {
            "views": raw.get("views", 0),
            "likes": raw.get("likes", 0),
            "retweets": raw.get("retweets", 0),
            "replies": raw.get("replies", 0),
            "bookmarks": raw.get("bookmarks", 0),
        },
        "metadata": {
            "source": raw.get("sampling_bias", "unknown"),
            "domain": "twitter",
            "cashtags": raw.get("cashtags", []),
            "narratives": raw.get("narratives", []),
            "captured_at": raw.get("capture_ts"),
        },
    }


# ── ANALYSES ──

def analyze_distribution(tweets: list) -> dict:
    """Signal score distribution per domain (inferred from keywords)."""
    config = load_config()
    domains = config.get("domains", {})

    # Filter out non-domain keys (e.g., "version")
    domain_signals = {d: [] for d in domains.keys() if isinstance(domains[d], dict)}

    # Ensure D1 exists as fallback
    if "D1" not in domain_signals:
        domain_signals["D1"] = []

    for tweet in tweets:
        text = tweet.get("text", "").lower()
        score = tweet.get("signal", {}).get("score", 0)

        # Simple domain mapping (keyword-based)
        found = False
        for domain_id, domain_info in domains.items():
            if not isinstance(domain_info, dict):
                continue
            keywords = domain_info.get("keywords", [])
            if any(kw.lower() in text for kw in keywords):
                domain_signals[domain_id].append(score)
                found = True
                break

        if not found:
            # Default to D1 (tokens)
            domain_signals["D1"].append(score)

    result = {}
    for domain_id, scores in domain_signals.items():
        if scores:
            result[domain_id] = {
                "count": len(scores),
                "mean": round(mean(scores), 2),
                "median": round(median(scores), 2),
                "stdev": round(stdev(scores), 2) if len(scores) > 1 else 0,
                "min": min(scores),
                "max": max(scores),
            }
        else:
            result[domain_id] = {"count": 0}

    return result


def analyze_false_negatives(tweets: list) -> dict:
    """High-quality tweets that got low signal_score."""
    false_negatives = []

    for tweet in tweets:
        text = tweet.get("text", "")
        signal = tweet.get("signal", {}).get("score", 0)

        # Heuristic: high-quality indicators despite low signal
        quality_indicators = [
            "analysis" in text.lower(),
            "thread" in text.lower(),
            "research" in text.lower(),
            "data" in text.lower(),
            "on-chain" in text.lower() or "onchain" in text.lower(),
        ]

        high_engagement = (
            tweet.get("engagement", {}).get("likes", 0) > 500
            or tweet.get("engagement", {}).get("replies", 0) > 100
        )

        if (sum(quality_indicators) >= 2 or high_engagement) and signal < 2:
            false_negatives.append({
                "id": tweet.get("id"),
                "text": text[:80],
                "signal_score": signal,
                "author_tier": tweet.get("author", {}).get("tier"),
                "engagement": tweet.get("engagement", {}),
                "quality_indicators": quality_indicators,
            })

    return {
        "count": len(false_negatives),
        "rate": round(len(false_negatives) / len(tweets) * 100, 1) if tweets else 0,
        "examples": false_negatives[:5],
    }


def analyze_calibration_heuristic(verdicts: list) -> dict:
    """Measure signal_score quality via Pearson r vs q_score.total.

    K15 consumer: joins on tweet_id, measures agreement.
    Falsification: if r < 0.4 with N > 50, signal_score is a poor proxy for Dogs.
    """
    if not verdicts:
        return {
            "pairs_found": 0,
            "status": "no_verdicts",
            "r": None,
            "confidence": 0.0,
        }

    signal_scores = []
    q_scores = []

    for v in verdicts:
        signal = v.get("signal_score", 0)
        q_score = v.get("verdict", {}).get("q_score", {})
        q_total = q_score.get("total", 0) if isinstance(q_score, dict) else 0

        if q_total is not None and q_total != "":
            signal_scores.append(float(signal))
            q_scores.append(float(q_total))

    if len(signal_scores) < 2:
        return {
            "pairs_found": len(signal_scores),
            "status": "insufficient_data",
            "r": None,
            "confidence": 0.0,
        }

    r = pearson_correlation(signal_scores, q_scores)

    # Confidence in result: higher with more pairs
    confidence = min(1.0, len(signal_scores) / 50.0)

    # Falsification: r < 0.4 with N > 50 means signal_score is poor
    is_valid = r >= 0.4 or len(signal_scores) <= 50
    status = "valid" if is_valid else "signal_score_poor"

    return {
        "pairs_found": len(signal_scores),
        "r": round(r, 3),
        "status": status,
        "confidence": round(confidence, 2),
        "interpretation": (
            f"signal_score is a {'good' if r >= 0.4 else 'poor'} proxy for Dog judgment (r={r:.3f})"
        ),
    }


def analyze_signal_patterns(verdicts: list, tweets_raw: list) -> dict:
    """Extract what signal patterns Dogs actually value.

    Compares high-Q verdicts against low-Q verdicts.
    Identifies: cashtags, narratives, author_tier that correlate with high q_score.
    """
    if not verdicts or not tweets_raw:
        return {"status": "insufficient_data", "patterns": {}}

    # Build lookup: tweet_id → raw tweet
    tweets_by_id = {t.get("tweet_id"): t for t in tweets_raw}

    high_q = []  # q_score >= 0.5
    low_q = []   # q_score < 0.3

    for v in verdicts:
        tweet_id = v.get("tweet_id")
        q_total = v.get("verdict", {}).get("q_score", {}).get("total", 0)

        if q_total is None:
            continue

        if tweet_id not in tweets_by_id:
            continue

        tweet = tweets_by_id[tweet_id]

        if q_total >= 0.5:
            high_q.append(tweet)
        elif q_total < 0.3:
            low_q.append(tweet)

    if len(high_q) < 2 or len(low_q) < 2:
        return {
            "status": "insufficient_high_q_verdicts",
            "high_q_count": len(high_q),
            "low_q_count": len(low_q),
        }

    # Count patterns in high-Q vs low-Q
    high_cashtags = Counter()
    high_narratives = Counter()
    high_author_tiers = Counter()

    low_cashtags = Counter()
    low_narratives = Counter()
    low_author_tiers = Counter()

    for t in high_q:
        high_cashtags.update(t.get("cashtags", []))
        high_narratives.update(t.get("narratives", []))
        high_author_tiers.update([t.get("author_tier", "unknown")])

    for t in low_q:
        low_cashtags.update(t.get("cashtags", []))
        low_narratives.update(t.get("narratives", []))
        low_author_tiers.update([t.get("author_tier", "unknown")])

    # Compute lift: (high_count + 1) / (low_count + 1) to avoid division by zero
    def compute_lift(high_counter, low_counter):
        lift = {}
        for key in set(high_counter.keys()) | set(low_counter.keys()):
            h = high_counter.get(key, 0) + 1
            l = low_counter.get(key, 0) + 1
            lift[key] = round(h / l, 2)
        return lift

    return {
        "status": "ok",
        "high_q_verdicts": len(high_q),
        "low_q_verdicts": len(low_q),
        "cashtags_lift": compute_lift(high_cashtags, low_cashtags),
        "narratives_lift": compute_lift(high_narratives, low_narratives),
        "author_tiers_lift": compute_lift(high_author_tiers, low_author_tiers),
        "interpretation": "Lift > 1.0 = present in high-Q verdicts more often",
    }


def analyze_coverage_vs_ground_truth(tweets: list, curated: dict) -> dict:
    """Compare lab's domain assignment against curated ground truth.

    Identifies misalignment: where does lab assign vs curated ground truth.
    """
    if not curated:
        return {"status": "no_ground_truth", "coverage": {}}

    config = load_config()
    domains = config.get("domains", {})

    # Count lab's assignments
    lab_assignments = {d: 0 for d in domains.keys() if isinstance(domains[d], dict)}
    if "D1" not in lab_assignments:
        lab_assignments["D1"] = 0

    for tweet in tweets:
        text = tweet.get("text", "").lower()
        found = False
        for domain_id, domain_info in domains.items():
            if not isinstance(domain_info, dict):
                continue
            keywords = domain_info.get("keywords", [])
            if any(kw.lower() in text for kw in keywords):
                lab_assignments[domain_id] += 1
                found = True
                break
        if not found:
            lab_assignments["D1"] += 1

    # Curated ground truth counts
    curated_counts = {d: len(tweets) for d, tweets in curated.items()}

    # Coverage analysis
    coverage = {}
    for domain_id in set(lab_assignments.keys()) | set(curated_counts.keys()):
        lab_count = lab_assignments.get(domain_id, 0)
        curated_count = curated_counts.get(domain_id, 0)
        delta = lab_count - curated_count

        coverage[domain_id] = {
            "lab_assigned": lab_count,
            "curated_count": curated_count,
            "delta": delta,
            "status": (
                "over-assigned" if delta > curated_count * 0.5 else
                "under-assigned" if delta < -curated_count * 0.5 else
                "aligned"
            ),
        }

    return {
        "status": "ok",
        "coverage": coverage,
        "notes": "delta > 0: lab over-assigned vs curated. delta < 0: lab under-assigned.",
    }


def analyze_disagreement_zones(tweets: list) -> dict:
    """Where Dogs would likely disagree (high uncertainty indicators)."""
    disagreement_tweets = []

    for tweet in tweets:
        text = tweet.get("text", "").lower()
        signal = tweet.get("signal", {}).get("score", 0)
        author_tier = tweet.get("author", {}).get("tier")

        # Disagreement happens when:
        # - Hype language but low signal (Dogs read text, heuristic disagrees)
        # - Unverified but high-signal (authority mismatch)
        # - Mixed signals (both quality + spam keywords)

        has_hype = any(w in text for w in ["moon", "100x", "pump", "gem"])
        has_warning = any(w in text for w in ["rug", "scam", "honeypot", "fraud"])
        has_analysis = any(w in text for w in ["analysis", "research", "data"])

        uncertainty = 0
        if has_hype and signal < 3:
            uncertainty += 1  # Dogs might see hype differently
        if has_warning and signal < 4:
            uncertainty += 1  # Dogs may weigh warnings higher
        if author_tier == "unknown" and signal > 3:
            uncertainty += 1  # Dogs might penalize unverified
        if has_analysis and has_hype:
            uncertainty += 1  # Mixed signals confuse heuristics

        if uncertainty >= 2:
            disagreement_tweets.append({
                "id": tweet.get("id"),
                "text": text[:80],
                "signal_score": signal,
                "author_tier": author_tier,
                "uncertainty_factors": [
                    f"has_hype={has_hype}",
                    f"has_warning={has_warning}",
                    f"has_analysis={has_analysis}",
                ],
            })

    return {
        "count": len(disagreement_tweets),
        "rate": round(len(disagreement_tweets) / len(tweets) * 100, 1) if tweets else 0,
        "examples": disagreement_tweets[:5],
    }


# ── BRIEFING ──

def generate_briefing(dataset_path: str, organ_dir: str = None) -> dict:
    """Full analysis → briefing for Hermes/Claude.

    Loads initial dataset + verdicts + observations from both organ (hermes-x data-centric)
    and kernel (general observations via /observe or MCP cynic_observe).
    K15: verdicts and observations flow back into analysis.
    """
    print(f"Loading dataset: {dataset_path}")
    tweets_raw = []
    with open(dataset_path) as f:
        for line in f:
            try:
                tweets_raw.append(json.loads(line.strip()))
            except json.JSONDecodeError:
                continue

    # K15: Load verdicts from organ (written by daemon after /judge)
    verdicts = load_verdicts_from_organ(organ_dir)
    print(f"Loading {len(verdicts)} verdicts from organ...")

    # Load curated ground truth for coverage comparison
    curated = load_curated_ground_truth(organ_dir)
    if curated:
        curated_total = sum(len(tweets) for tweets in curated.values())
        print(f"Loading {curated_total} curated tweets ({len(curated)} domains)...")

    # K15: Load observations from organ (stored via /observe → HermesXReader)
    organ_obs = load_observations_from_organ(organ_dir)
    print(f"Loading {len(organ_obs)} observations from organ...")
    tweets_raw.extend(organ_obs)

    # K15: Load observations from kernel (REST /observe without hermes-x, MCP cynic_observe)
    # Observations are stored in SurrealDB and accessed via REST /observations endpoint.
    kernel_obs_count = 0
    cynic_rest_addr = os.environ.get("CYNIC_REST_ADDR", "")
    cynic_api_key = os.environ.get("CYNIC_API_KEY", "")
    if cynic_rest_addr and cynic_api_key:
        try:
            import urllib.request
            import urllib.error
            # Ensure URL has protocol (CYNIC_REST_ADDR may omit http://)
            if not cynic_rest_addr.startswith("http"):
                cynic_rest_addr = f"http://{cynic_rest_addr}"

            req = urllib.request.Request(
                f"{cynic_rest_addr}/observations?limit=1000",
                headers={"Authorization": f"Bearer {cynic_api_key}"}
            )
            with urllib.request.urlopen(req, timeout=5) as response:
                kernel_obs_raw = json.loads(response.read().decode())
                if isinstance(kernel_obs_raw, list):
                    for obs in kernel_obs_raw:
                        tweets_raw.append(obs)
                        kernel_obs_count += 1
        except (urllib.error.URLError, json.JSONDecodeError, Exception) as e:
            print(f"Warning: Failed to load kernel observations: {e}")

    if kernel_obs_count > 0:
        print(f"Loading {kernel_obs_count} observations from kernel...")

    print(f"Standardizing {len(tweets_raw)} tweets...")
    tweets = [standardize_tweet(t) for t in tweets_raw]

    print("Running analyses...")
    distribution = analyze_distribution(tweets)
    false_negatives = analyze_false_negatives(tweets)
    disagreement = analyze_disagreement_zones(tweets)
    calibration = analyze_calibration_heuristic(verdicts)
    signal_patterns = analyze_signal_patterns(verdicts, tweets_raw)
    coverage = analyze_coverage_vs_ground_truth(tweets, curated)

    # Generate recommendation
    config = load_config()
    domains = config.get("domains", {})

    domain_coverage = {}
    for domain_id, domain_info in domains.items():
        if not isinstance(domain_info, dict):
            continue
        curated = distribution.get(domain_id, {}).get("count", 0)
        target = domain_info.get("target", 100)
        domain_coverage[domain_id] = {
            "curated": curated,
            "target": target,
            "deficit": max(0, target - curated),
        }

    # Pick domain with highest deficit + HIGH priority
    ranked = sorted(
        domain_coverage.items(),
        key=lambda x: (
            x[1]["deficit"],
            -1 if domains[x[0]].get("priority") == "HIGH" else 1,
        ),
        reverse=True,
    )

    suggested_domain = ranked[0][0] if ranked else "D1"
    suggested_info = domains.get(suggested_domain, {})
    deficit_reason = f"Deficit: {ranked[0][1]['deficit']} tweets" if ranked else "Insufficient domain data"

    briefing = {
        "timestamp": datetime.now().isoformat(),
        "dataset_id": Path(dataset_path).stem,
        "tweet_count": len(tweets),
        "verdict_count": len(verdicts),
        "curated_domains": len(curated),
        "analyses": {
            "distribution": distribution,
            "false_negatives": false_negatives,
            "disagreement": disagreement,
            "calibration": calibration,
            "signal_patterns": signal_patterns,
            "coverage_vs_ground_truth": coverage,
        },
        "recommendation": {
            "domain": suggested_domain,
            "name": suggested_info.get("name"),
            "reason": f"{deficit_reason}, Priority: {suggested_info.get('priority')}",
        },
        "briefing": f"""
Dataset Analysis: {len(tweets)} tweets

KEY FINDINGS:
- Signal distribution by domain: D1={distribution.get('D1', {}).get('mean')} avg, D4={distribution.get('D4', {}).get('mean')} avg
- False negatives: {false_negatives['count']} tweets ({false_negatives['rate']}%) have quality but low signal
- Disagreement zones: {disagreement['count']} tweets ({disagreement['rate']}%) likely to confuse Dogs

RECOMMENDATION:
Explore {suggested_domain} ({suggested_info.get('name')}) next.
Reason: {deficit_reason}, {suggested_info.get('priority')} priority.

NEXT ACTIONS:
1. Review false_negatives examples → improve signal_score heuristic
2. Browse recommended domain (@gcrtrd for D4, etc.)
3. Post ≥3 observations to cynic_observe
4. Re-run lab after new observations
""",
        "uncertainties": [
            "Domain assignment is keyword-based; may misclassify borderline tweets",
            "Calibration: " + (
                f"signal_score appears valid (r={calibration.get('r')}, N={calibration.get('pairs_found')})"
                if calibration.get("status") == "valid"
                else f"insufficient verdict data (N={calibration.get('pairs_found')})"
            ),
        ],
        "actionable_insights": [
            f"Signal patterns: {signal_patterns.get('status')}",
            f"Coverage alignment: {sum(1 for c in coverage.get('coverage', {}).values() if c.get('status') == 'aligned')} / {len(coverage.get('coverage', {}))} domains aligned with ground truth",
            f"Calibration status: {calibration.get('interpretation')}",
        ] if signal_patterns.get("status") == "ok" else [],
        "_meta": {
            "lab_version": "0.3.0",
            "signal_version": "2",
            "verdict_consumer": "enabled",
            "verdicts_loaded": len(verdicts),
            "curated_loaded": len(curated),
            "ground_truth_comparison": "enabled",
            "signal_patterns_enabled": True,
            "reproducibility_token": f"lab-0.3.0-signal-2-tweets-{len(tweets)}-verdicts-{len(verdicts)}-curated-{len(curated)}",
        },
    }

    return briefing


def main():
    dataset_path = os.environ.get(
        "X_DATASET_PATH",
        Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl",
    )

    if not Path(dataset_path).exists():
        print(f"ERROR: Dataset not found: {dataset_path}")
        return

    briefing = generate_briefing(str(dataset_path))

    # Save
    output_path = (
        Path.home()
        / ".cynic"
        / "organs"
        / "hermes"
        / "x"
        / "lab_briefing_latest.json"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w") as f:
        json.dump(briefing, f, indent=2)

    print(f"\n✓ Lab briefing saved to {output_path}")
    print(f"\nRECOMMENDATION: {briefing['recommendation']['domain']} ({briefing['recommendation']['name']})")
    print(f"Reason: {briefing['recommendation']['reason']}")


if __name__ == "__main__":
    main()
