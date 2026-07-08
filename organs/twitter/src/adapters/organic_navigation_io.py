"""Filesystem adapters for organic navigation."""

from __future__ import annotations

import json
import logging
import re
from pathlib import Path

from organs.contracts.src.surface_contracts import json_object_or_default, read_json_file_or_default
from organs.twitter.src.core.organic_navigation import DEFAULT_TARGETS


logger = logging.getLogger("navigator")


def load_targets_from_skill(skill_path: Path) -> list[dict]:
    """Extract navigation targets from SKILL.md domain wisdom."""
    targets = list(DEFAULT_TARGETS)
    if not skill_path.exists():
        return targets

    try:
        content = skill_path.read_text(encoding="utf-8")
    except IOError:
        return targets

    cashtags = set(re.findall(r"\$([A-Z]{2,10})", content))
    for tag in list(cashtags)[:5]:
        targets.append(
            {
                "url": f"https://x.com/search?q=%24{tag}&f=live",
                "domain": f"token_{tag.lower()}",
                "weight": 0.7,
                "source": "skill",
            }
        )
    return targets


def apply_yield_boost(targets: list[dict], organ_dir: Path) -> list[dict]:
    """Boost weights for domains that produce high-signal content."""
    yield_path = organ_dir / "curation_yield.json"
    if not yield_path.exists():
        return targets

    try:
        yield_data = read_json_file_or_default(yield_path, {})
    except IOError:
        return targets

    domains = yield_data.get("domains", {})
    for target in targets:
        domain = target.get("domain", "")
        info = domains.get(domain, {})
        pct = info.get("pct_of_total", 0)
        if pct > 20:
            target["weight"] = round(target.get("weight", 1.0) * 1.3, 2)
        elif pct < 5 and pct > 0:
            target["weight"] = round(target.get("weight", 1.0) * 0.7, 2)
    return targets


def get_cdp_endpoint(organ_dir: Path) -> str | None:
    """Read CDP endpoint from browser-state.json."""
    candidates = [organ_dir / "browser-state.json", organ_dir.parent / "browser-state.json"]
    for browser_state in candidates:
        if browser_state.exists():
            state = json_object_or_default(browser_state.read_text(encoding="utf-8"), {})
            cdp_url = state.get("cdp_url")
            if cdp_url:
                return cdp_url
            cdp_port = state.get("cdp_port")
            if cdp_port:
                return f"http://127.0.0.1:{cdp_port}"
    logger.warning("No browser-state.json found with CDP info")
    return None


def log_navigation(nav_log: Path, entry: dict) -> None:
    """Append navigation event to log."""
    try:
        with nav_log.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(entry, default=str) + "\n")
    except IOError as exc:
        logger.error("Failed to log navigation: %s", exc)
