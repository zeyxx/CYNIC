"""Search query generation rules for Hermes X."""

from __future__ import annotations

import random
import re


SEED_QUERIES = [
    {"query": "solana rug pull", "domain": "security", "weight": 1.0},
    {"query": "solana new token launch", "domain": "token_launch", "weight": 1.0},
    {"query": "crypto airdrop scam", "domain": "security", "weight": 0.8},
    {"query": "solana meme coin", "domain": "memecoin", "weight": 0.7},
    {"query": "defi exploit hack", "domain": "security", "weight": 0.9},
    {"query": "solana whale wallet", "domain": "whale_activity", "weight": 0.6},
    {"query": "jupiter solana swap", "domain": "dex", "weight": 0.5},
    {"query": "phantom wallet update", "domain": "infrastructure", "weight": 0.4},
    {"query": "crypto market sentiment", "domain": "sentiment", "weight": 0.5},
    {"query": "NFT solana mint", "domain": "nft", "weight": 0.3},
]


def extract_domains_from_skill(skill_text: str) -> list[dict]:
    """Extract domain keywords and patterns from SKILL.md."""
    queries = []
    if not skill_text:
        return queries

    current_domain = "general"
    for line in skill_text.split("\n"):
        header_match = re.match(r"^#{1,3}\s+(?:Domain:\s*)?(.+)", line)
        if header_match:
            raw = header_match.group(1).strip().lower()
            for noise in ["analysis", "overview", "summary", "notes"]:
                raw = raw.replace(noise, "").strip()
            if raw:
                current_domain = raw.replace(" ", "_")[:30]
            continue

        bullet_match = re.match(r"^\s*[-*]\s+(.+)", line)
        if not bullet_match:
            continue

        content = bullet_match.group(1).strip()
        if len(content) < 10 or len(content) > 120:
            continue
        if any(
            skip in content.lower()
            for skip in ["todo", "fixme", "note:", "see also", "reference", "updated", "last run", "timestamp"]
        ):
            continue

        for tag in re.findall(r"\$[A-Z]{2,10}", content):
            queries.append(
                {"query": f"{tag} solana", "domain": current_domain, "weight": 0.9, "source": "skill_cashtag"}
            )
        for mention in re.findall(r"@\w{3,20}", content):
            queries.append(
                {"query": f"{mention} crypto", "domain": current_domain, "weight": 0.7, "source": "skill_mention"}
            )
        for phrase in re.findall(r'"([^"]{5,60})"', content):
            queries.append({"query": phrase, "domain": current_domain, "weight": 0.8, "source": "skill_phrase"})

    return queries


def apply_yield_weights(queries: list[dict], yield_data: dict) -> list[dict]:
    """Boost queries from high-yield domains and dampen low-yield ones."""
    domain_weights = {}
    domains = yield_data.get("domains", {})
    if not domains:
        return queries

    total = sum(d.get("signals", 0) for d in domains.values())
    if total == 0:
        return queries

    for domain, info in domains.items():
        pct = info.get("pct_of_total", 0)
        if pct > 20:
            domain_weights[domain] = 1.3
        elif pct > 10:
            domain_weights[domain] = 1.1
        elif pct < 5:
            domain_weights[domain] = 0.7
        else:
            domain_weights[domain] = 1.0

    for query in queries:
        multiplier = domain_weights.get(query.get("domain", ""), 1.0)
        query["weight"] = round(query.get("weight", 1.0) * multiplier, 2)

    return queries


def deduplicate(new_queries: list[dict], existing: list[dict]) -> list[dict]:
    """Remove queries that already exist in search_tasks.jsonl."""
    existing_set = {task.get("query", "").lower() for task in existing}
    return [query for query in new_queries if query.get("query", "").lower() not in existing_set]


def select_queries(queries: list[dict], budget: int) -> list[dict]:
    """Select top queries by weight, with randomness for exploration."""
    if not queries:
        return []

    queries.sort(key=lambda query: query.get("weight", 0), reverse=True)
    exploit_count = int(budget * 0.7)
    explore_count = budget - exploit_count

    selected = queries[:exploit_count]
    remaining = queries[exploit_count:]
    if remaining and explore_count > 0:
        selected.extend(random.sample(remaining, min(explore_count, len(remaining))))

    return selected


def build_candidate_queries(skill_text: str, yield_data: dict) -> list[dict]:
    """Build weighted search candidates from SKILL.md plus seed queries."""
    skill_queries = extract_domains_from_skill(skill_text)
    all_queries = skill_queries + [{**query, "source": "seed"} for query in SEED_QUERIES]
    return apply_yield_weights(all_queries, yield_data)
