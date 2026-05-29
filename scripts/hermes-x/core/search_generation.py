#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Hermes Search Generator — SKILL.md wisdom to search queries.

Reads domain wisdom from SKILL.md and curation_yield.json, generates search
queries weighted by domain signal yield, and writes them to search_tasks.jsonl
for consumption by search_executor.py.

Architecture:
  SKILL.md (domain wisdom, updated every 4h by gemini briefing)
      +
  curation_yield.json (domain signal counts, updated every 30min by curation)
      |
      v
  search_generation.py (this script)
      |
      v
  search_tasks.jsonl (consumed by search_executor.py)

K15 Consumer: search_executor.py reads search_tasks.jsonl and executes searches.
Systemd: hermes-search-generator.service (every 5min via timer)
Stability: Tier 2 infrastructure

Failure mode: if SKILL.md or curation_yield.json missing, falls back to
static seed queries covering core domains.
"""

__version__ = "0.1.0"

import json
import logging
import os
import random
import re
import sys
from datetime import datetime
from pathlib import Path

from hermes_paths import HERMES_X_DIR, SKILL_MD, SEARCH_TASKS

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("search-generator")

# Seed queries — used when SKILL.md is empty or missing
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

MAX_TASKS = 20  # Max pending tasks in search_tasks.jsonl


def load_skill_md(skill_path: Path) -> str:
    """Load SKILL.md content. Returns empty string if missing."""
    if not skill_path.exists():
        logger.info("SKILL.md not found at %s", skill_path)
        return ""
    try:
        return skill_path.read_text(encoding="utf-8")
    except IOError as e:
        logger.warning("Failed to read SKILL.md: %s", e)
        return ""


def load_curation_yield(organ_dir: Path) -> dict:
    """Load curation_yield.json for domain weights."""
    yield_path = organ_dir / "curation_yield.json"
    if not yield_path.exists():
        return {}
    try:
        with open(yield_path) as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError) as e:
        logger.warning("Failed to load curation_yield.json: %s", e)
        return {}


def extract_domains_from_skill(skill_text: str) -> list[dict]:
    """Extract domain keywords and patterns from SKILL.md.

    Looks for markdown headers (## Domain: ...) and bullet points with
    actionable keywords that can become search queries.
    """
    queries = []
    if not skill_text:
        return queries

    current_domain = "general"
    lines = skill_text.split("\n")

    for line in lines:
        # Detect domain headers
        header_match = re.match(r"^#{1,3}\s+(?:Domain:\s*)?(.+)", line)
        if header_match:
            raw = header_match.group(1).strip().lower()
            # Clean up common header words
            for noise in ["analysis", "overview", "summary", "notes"]:
                raw = raw.replace(noise, "").strip()
            if raw:
                current_domain = raw.replace(" ", "_")[:30]
            continue

        # Extract actionable patterns from bullet points
        bullet_match = re.match(r"^\s*[-*]\s+(.+)", line)
        if bullet_match:
            content = bullet_match.group(1).strip()
            # Skip meta-commentary, keep actionable intelligence
            if len(content) < 10 or len(content) > 120:
                continue
            if any(skip in content.lower() for skip in [
                "todo", "fixme", "note:", "see also", "reference",
                "updated", "last run", "timestamp",
            ]):
                continue

            # Extract search-worthy phrases (quoted terms, cashtags, @mentions)
            cashtags = re.findall(r'\$[A-Z]{2,10}', content)
            mentions = re.findall(r'@\w{3,20}', content)
            quoted = re.findall(r'"([^"]{5,60})"', content)

            # Build queries from extracted terms
            for tag in cashtags:
                queries.append({
                    "query": f"{tag} solana",
                    "domain": current_domain,
                    "weight": 0.9,
                    "source": "skill_cashtag",
                })
            for mention in mentions:
                queries.append({
                    "query": f"{mention} crypto",
                    "domain": current_domain,
                    "weight": 0.7,
                    "source": "skill_mention",
                })
            for phrase in quoted:
                queries.append({
                    "query": phrase,
                    "domain": current_domain,
                    "weight": 0.8,
                    "source": "skill_phrase",
                })

    return queries


def apply_yield_weights(queries: list[dict], yield_data: dict) -> list[dict]:
    """Boost queries from high-yield domains, dampen low-yield ones."""
    domain_weights = {}
    domains = yield_data.get("domains", {})
    if not domains:
        return queries

    total = sum(d.get("signals", 0) for d in domains.values())
    if total == 0:
        return queries

    for domain, info in domains.items():
        pct = info.get("pct_of_total", 0)
        # Domains with >20% of signals get boosted, <5% get dampened
        if pct > 20:
            domain_weights[domain] = 1.3
        elif pct > 10:
            domain_weights[domain] = 1.1
        elif pct < 5:
            domain_weights[domain] = 0.7
        else:
            domain_weights[domain] = 1.0

    for q in queries:
        domain = q.get("domain", "")
        multiplier = domain_weights.get(domain, 1.0)
        q["weight"] = round(q.get("weight", 1.0) * multiplier, 2)

    return queries


def load_existing_tasks(tasks_path: Path) -> list[dict]:
    """Load existing search tasks to avoid duplicates."""
    tasks = []
    if not tasks_path.exists():
        return tasks
    try:
        with open(tasks_path) as f:
            for line in f:
                try:
                    tasks.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass
    return tasks


def deduplicate(new_queries: list[dict], existing: list[dict]) -> list[dict]:
    """Remove queries that already exist in search_tasks.jsonl."""
    existing_set = {t.get("query", "").lower() for t in existing}
    return [q for q in new_queries if q.get("query", "").lower() not in existing_set]


def select_queries(queries: list[dict], budget: int) -> list[dict]:
    """Select top queries by weight, with some randomness for exploration."""
    if not queries:
        return []

    # Sort by weight descending
    queries.sort(key=lambda q: q.get("weight", 0), reverse=True)

    # Take top 70% by weight, 30% random (UCB1-inspired explore/exploit)
    exploit_count = int(budget * 0.7)
    explore_count = budget - exploit_count

    selected = queries[:exploit_count]
    remaining = queries[exploit_count:]
    if remaining and explore_count > 0:
        explore = random.sample(remaining, min(explore_count, len(remaining)))
        selected.extend(explore)

    return selected


def write_tasks(tasks: list[dict], tasks_path: Path) -> int:
    """Write search tasks to search_tasks.jsonl (append mode)."""
    written = 0
    try:
        with open(tasks_path, "a") as f:
            for task in tasks:
                task["generated_at"] = datetime.now().isoformat()
                task["status"] = "pending"
                f.write(json.dumps(task, default=str) + "\n")
                written += 1
    except IOError as e:
        logger.error("Failed to write search tasks: %s", e)
    return written


def main() -> int:
    organ_dir = Path(os.environ.get("X_ORGAN_DIR", HERMES_X_DIR))

    logger.info("Search Generator v%s starting...", __version__)
    logger.info("Organ directory: %s", organ_dir)

    # Load inputs
    skill_text = load_skill_md(SKILL_MD)
    yield_data = load_curation_yield(organ_dir)
    existing_tasks = load_existing_tasks(SEARCH_TASKS)

    # Don't exceed max pending tasks
    pending_count = len([t for t in existing_tasks if t.get("status") == "pending"])
    if pending_count >= MAX_TASKS:
        logger.info("Already %d pending tasks (max %d), skipping generation",
                     pending_count, MAX_TASKS)
        return 0

    budget = MAX_TASKS - pending_count

    # Generate queries from SKILL.md
    skill_queries = extract_domains_from_skill(skill_text)
    logger.info("Extracted %d queries from SKILL.md", len(skill_queries))

    # Combine with seed queries (always available as fallback)
    all_queries = skill_queries + [
        {**q, "source": "seed"} for q in SEED_QUERIES
    ]

    # Apply curation yield weights
    all_queries = apply_yield_weights(all_queries, yield_data)

    # Deduplicate against existing tasks
    new_queries = deduplicate(all_queries, existing_tasks)
    logger.info("After dedup: %d new queries", len(new_queries))

    if not new_queries:
        logger.info("No new queries to generate")
        return 0

    # Select best queries within budget
    selected = select_queries(new_queries, budget)

    # Write to search_tasks.jsonl
    written = write_tasks(selected, SEARCH_TASKS)
    logger.info("Wrote %d search tasks to %s", written, SEARCH_TASKS)

    return 0


if __name__ == "__main__":
    sys.exit(main())
