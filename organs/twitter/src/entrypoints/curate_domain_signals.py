#!/usr/bin/env python3
"""CYNIC Hermes curation CLI."""

import sys

from organs.contracts.src.surface_contracts import iter_json_objects
from organs.twitter.src.adapters.curation_files import (
    sync_kernel_curation,
    write_curated_files,
    write_yield_summary,
)
from organs.twitter.src.adapters.curation_kernel_bridge import post_to_kernel
from organs.twitter.src.adapters.hermes_paths import CURATED_DIR, DATASET, HERMES_X_DIR
from organs.twitter.src.core import domain_curation
from organs.twitter.src.core.domain_curation import (
    DEFAULT_NARRATIVE_MAPPINGS,
    MIN_SIGNAL_SCORE,
    create_signal_id,
    extract_falsifiable_claim,
    extract_keywords,
)

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
DATASET_PATH = DATASET


def _load_narrative_mappings() -> dict[str, list[str]]:
    """Load narrative-to-domain mappings from the organ or legacy config."""
    yaml_paths = [
        HERMES_X_DIR / "narrative_domains.yaml",
        PROJECT_ROOT / "services" / "cynic-python" / "lab" / "config" / "narrative_domains.yaml",
        PROJECT_ROOT / "cynic-python" / "lab" / "config" / "narrative_domains.yaml",
    ]
    for path in yaml_paths:
        if path.exists():
            try:
                import yaml

                with path.open(encoding="utf-8") as handle:
                    data = yaml.safe_load(handle)
                mappings = data.get("narrative_mappings", {})
                if mappings:
                    return mappings
            except Exception:
                pass
    return DEFAULT_NARRATIVE_MAPPINGS


NARRATIVE_MAPPINGS = _load_narrative_mappings()


def assign_domain(text: str, narratives: list[str], tier: str) -> str:
    """Assign tweet to domain D1-D6 using entrypoint-loaded mappings."""
    return domain_curation.assign_domain(text, narratives, tier, NARRATIVE_MAPPINGS)


def _print_dry_run_sample(by_domain: dict[str, list[dict]]) -> None:
    print("\n[DRY RUN] Sample signals:")
    for domain in sorted(by_domain.keys())[:1]:
        sample = by_domain[domain][0]
        print(f"\n{sample['signal_id']}:")
        print(f"  Pattern: {sample['pattern'][:100]}...")
        print(f"  Claim: {sample['falsifiable_claim']}")


def curate_tweets(dry_run: bool = False) -> bool:
    """Curate high-signal tweets into DomainSignal JSONL files."""
    if not DATASET_PATH.exists():
        print(f"ERROR: Dataset not found at {DATASET_PATH}")
        return False

    with DATASET_PATH.open(encoding="utf-8") as handle:
        tweets = list(iter_json_objects(handle))

    print(f"Loaded {len(tweets)} tweets from dataset")
    high_signal = [tweet for tweet in tweets if tweet.get("signal_score", 0) >= MIN_SIGNAL_SCORE]
    print(f"High-signal tweets (score >= {MIN_SIGNAL_SCORE}): {len(high_signal)}")

    by_domain = domain_curation.curate_by_domain(high_signal, NARRATIVE_MAPPINGS)
    print("\nCurated by domain:")
    for domain, signals in sorted(by_domain.items()):
        print(f"  {domain}: {len(signals)} signals")

    if dry_run:
        _print_dry_run_sample(by_domain)
        return True

    write_curated_files(CURATED_DIR, by_domain)
    sync_kernel_curation(CURATED_DIR, by_domain, PROJECT_ROOT)
    write_yield_summary(HERMES_X_DIR, by_domain)
    post_to_kernel(by_domain)
    return True


if __name__ == "__main__":
    sys.exit(0 if curate_tweets(dry_run="--dry-run" in sys.argv) else 1)
