"""Filesystem adapters for Gemini verdict learning."""

from __future__ import annotations

import logging
from pathlib import Path

from organs.contracts.src.surface_contracts import iter_json_objects, read_json_file_or_default
from organs.twitter.src.core.verdict_learning import (
    render_learned_patterns_section,
    replace_learned_patterns_section,
)


logger = logging.getLogger("gemini-learn")


def load_narrative_index(organ_dir: str) -> dict[str, list[str]]:
    """Build tweet_id -> narratives index from dataset.jsonl."""
    dataset_path = Path(organ_dir) / "dataset.jsonl"
    index = {}
    if not dataset_path.exists():
        return index
    try:
        with dataset_path.open(encoding="utf-8") as handle:
            for row in iter_json_objects(handle):
                tid = row.get("tweet_id", "")
                narrs = row.get("narratives", [])
                if tid and narrs:
                    index[str(tid)] = narrs
    except OSError:
        pass
    return index


def load_observation_verdicts(organ_dir: str) -> list[dict]:
    """Load native and tweet-level verdicts from organ-local state."""
    verdicts = []

    obs_dir = Path(organ_dir) / "observation-verdicts"
    if obs_dir.exists():
        try:
            for verdict_file in sorted(obs_dir.glob("*.json")):
                verdict = read_json_file_or_default(verdict_file, None)
                if isinstance(verdict, dict):
                    verdicts.append(verdict)
                else:
                    logger.warning("failed to parse %s", verdict_file.name)
        except OSError as exc:
            logger.warning("failed to scan observation-verdicts: %s", exc)

    obs_count = len(verdicts)
    tweet_dir = Path(organ_dir) / "verdicts"
    if tweet_dir.exists():
        narr_index = load_narrative_index(organ_dir)
        tweet_count = 0
        try:
            for verdict_file in sorted(tweet_dir.glob("*.json")):
                raw = read_json_file_or_default(verdict_file, None)
                if not isinstance(raw, dict):
                    continue
                tid = raw.get("tweet_id", "")
                verdict_obj = raw.get("verdict", {})
                domain = verdict_obj.get("domain", "D1") if isinstance(verdict_obj, dict) else "D1"
                verdicts.append(
                    {
                        "observation_id": f"tweet_{tid}",
                        "observation_domain": domain,
                        "observation_narratives": narr_index.get(str(tid), []),
                        "verdict": verdict_obj,
                    }
                )
                tweet_count += 1
        except OSError as exc:
            logger.warning("failed to scan verdicts: %s", exc)
        if tweet_count:
            logger.info("loaded %d tweet verdicts (+ %d observation verdicts)", tweet_count, obs_count)

    if not verdicts:
        logger.info("no verdicts found in either directory")

    return verdicts


def update_skill_md(organ_dir: str, patterns: dict) -> bool:
    """Update SKILL.md with learned verdict patterns."""
    skill_path = Path(organ_dir) / "SKILL.md"
    if not skill_path.exists():
        logger.warning("SKILL.md not found at %s", skill_path)
        return False

    try:
        current_content = skill_path.read_text(encoding="utf-8")
    except IOError as exc:
        logger.error("failed to read SKILL.md: %s", exc)
        return False

    learned_section = render_learned_patterns_section(patterns)
    new_content = replace_learned_patterns_section(current_content, learned_section)

    try:
        skill_path.write_text(new_content, encoding="utf-8")
        logger.info("updated SKILL.md with learned patterns")
        return True
    except IOError as exc:
        logger.error("failed to write SKILL.md: %s", exc)
        return False
