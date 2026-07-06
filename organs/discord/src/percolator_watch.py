#!/usr/bin/env python3
"""Percolator Discord watcher.

Consumes normalized passive Discord messages and exports durable, structured
Percolator bounty intelligence:

- researcher feed with classified Discord observations
- GitHub issue/PR/dossier links extracted from claims
- timeboxed bounty windows inferred from maintainer announcements
"""
from __future__ import annotations

import argparse
import json
import re
import sqlite3
from dataclasses import asdict, dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.parse import urlparse

from organs.contracts.src.surface_contracts import read_json_file_or_default

_PERCS = re.compile(
    r"\b(percolator|dcccrypto|percolator-prog|percolator-sdk|percolator-nft|bounty|audit|finding|duplicate|dupe|poc|exploit|critical|medium|med-critical|issue|pull|pr)\b",
    re.IGNORECASE,
)
_BOUNTY_WINDOW = re.compile(r"\bnext\s+(\d+)\s+hours?\b", re.IGNORECASE)
_MED_CRIT = re.compile(r"\bmed(?:ium)?[- ]?critical\b", re.IGNORECASE)
_HIGH_CRIT = re.compile(r"\b(high|critical)\b", re.IGNORECASE)
_PROOF = re.compile(r"\b(poc|proof|test|repro|reproduce|details?)\b", re.IGNORECASE)
_DUPLICATE = re.compile(r"\b(duplicate|dupe|already|same issue|known)\b", re.IGNORECASE)
_GITHUB = re.compile(r"https://github\.com/([^\s/]+)/([^\s/#?]+)(?:/(issues|pull)/(\d+))?", re.IGNORECASE)
_URL = re.compile(r"https?://\S+")
_DEFAULT_DB = Path.home() / ".cynic/organs/discord/messages.db"
_DEFAULT_FEED = Path("percolator/ops/researcher-feed.json")
_DEFAULT_RULES = Path("percolator/notes/bounty-rules.json")
_PARIS = timezone(timedelta(hours=2))


@dataclass(frozen=True)
class GitHubRef:
    url: str
    owner: str
    repo: str
    kind: str
    number: int | None


@dataclass(frozen=True)
class BountyWindow:
    source: str
    repository: str | None
    start_utc: str
    end_utc: str
    start_paris: str
    end_paris: str
    duration_hours: int
    severity: str
    payment: str | None


@dataclass(frozen=True)
class FeedItem:
    time_utc: str
    time_paris: str
    source: str
    channel_id: int
    message_id: int
    researcher: str
    classification: str
    repo_scope: list[str]
    severity_signal: str | None
    claim_or_finding: str
    evidence_links: list[str]
    github_refs: list[GitHubRef]
    bounty_window: BountyWindow | None
    overlap_risk: str
    action: str


def parse_dt(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)


def compact_text(value: str, limit: int = 260) -> str:
    clean = " ".join((value or "").split())
    if len(clean) <= limit:
        return clean
    return clean[: limit - 3].rstrip() + "..."


def extract_links(text: str) -> list[str]:
    return [link.rstrip(">").rstrip(").,;") for link in _URL.findall(text or "")]


def github_refs(text: str) -> list[GitHubRef]:
    refs: list[GitHubRef] = []
    for match in _GITHUB.finditer(text or ""):
        owner, repo, kind, number = match.groups()
        url = match.group(0).rstrip(">").rstrip(").,;")
        refs.append(
            GitHubRef(
                url=url,
                owner=owner,
                repo=repo,
                kind=kind or "repository",
                number=int(number) if number else None,
            )
        )
    return refs


def classify(text: str, refs: list[GitHubRef]) -> str:
    lower = (text or "").lower()
    if _BOUNTY_WINDOW.search(text or ""):
        return "bounty_announcement"
    if any(ref.kind == "issues" for ref in refs):
        return "github_issue_submission"
    if any(ref.kind == "pull" for ref in refs):
        return "github_pull_request"
    if "notion.site" in lower or "audit" in lower and extract_links(text):
        return "external_audit_dossier"
    if _DUPLICATE.search(text or ""):
        return "duplicate_signal"
    if _PROOF.search(text or ""):
        return "proof_or_submission_guidance"
    if _PERCS.search(text or ""):
        return "percolator_context"
    return "other"


def severity_signal(text: str) -> str | None:
    if _MED_CRIT.search(text or ""):
        return "medium_to_critical"
    if _HIGH_CRIT.search(text or ""):
        return "high_or_critical"
    return None


def repo_scope(text: str, refs: list[GitHubRef]) -> list[str]:
    repos = {f"{ref.owner}/{ref.repo}" for ref in refs}
    lower = (text or "").lower()
    if "percolator-prog" in lower:
        repos.add("dcccrypto/percolator-prog")
    if "percolator-sdk" in lower:
        repos.add("dcccrypto/percolator-sdk")
    if "percolator-nft" in lower:
        repos.add("dcccrypto/percolator-nft")
    return sorted(repos)


def bounty_window(item_time: datetime, text: str, refs: list[GitHubRef], source: str) -> BountyWindow | None:
    match = _BOUNTY_WINDOW.search(text or "")
    if not match:
        return None
    hours = int(match.group(1))
    end = item_time + timedelta(hours=hours)
    repos = repo_scope(text, refs)
    return BountyWindow(
        source=source,
        repository=repos[0] if repos else None,
        start_utc=item_time.isoformat(),
        end_utc=end.isoformat(),
        start_paris=item_time.astimezone(_PARIS).isoformat(),
        end_paris=end.astimezone(_PARIS).isoformat(),
        duration_hours=hours,
        severity=severity_signal(text) or "unknown",
        payment="instant_tonight_once_proven_real" if "instantly tonight" in (text or "").lower() else None,
    )


def overlap_risk(classification: str, refs: list[GitHubRef], text: str) -> str:
    if _DUPLICATE.search(text or ""):
        return "high"
    if any(ref.kind == "issues" for ref in refs):
        return "high"
    if classification in {"github_pull_request", "external_audit_dossier"}:
        return "medium"
    return "unknown"


def action_for(classification: str) -> str:
    if classification == "bounty_announcement":
        return "update_bounty_window"
    if classification in {"github_issue_submission", "github_pull_request", "external_audit_dossier"}:
        return "dedupe_against_candidate_findings"
    if classification == "proof_or_submission_guidance":
        return "update_submission_rules_if_authoritative"
    return "triage"


def load_items(db_path: Path, limit: int) -> list[FeedItem]:
    conn = sqlite3.connect(str(db_path.expanduser()))
    rows = conn.execute(
        """
        SELECT date, channel_id, message_id, COALESCE(author_name, ''), COALESCE(text, '')
        FROM messages
        WHERE text IS NOT NULL AND length(text) > 5
        ORDER BY date DESC
        LIMIT ?
        """,
        (limit,),
    ).fetchall()
    conn.close()

    items: list[FeedItem] = []
    for date, channel_id, message_id, author_name, text in rows:
        refs = github_refs(text)
        if not (_PERCS.search(text) or refs):
            continue
        dt = parse_dt(str(date))
        source = f"discord:{channel_id}:{message_id}"
        classification = classify(text, refs)
        window = bounty_window(dt, text, refs, source)
        items.append(
            FeedItem(
                time_utc=dt.isoformat(),
                time_paris=dt.astimezone(_PARIS).isoformat(),
                source=source,
                channel_id=int(channel_id),
                message_id=int(message_id),
                researcher=str(author_name or "unknown"),
                classification=classification,
                repo_scope=repo_scope(text, refs),
                severity_signal=severity_signal(text),
                claim_or_finding=compact_text(text),
                evidence_links=extract_links(text),
                github_refs=refs,
                bounty_window=window,
                overlap_risk=overlap_risk(classification, refs, text),
                action=action_for(classification),
            )
        )
    return items


def render_feed(items: list[FeedItem]) -> str:
    windows = [item.bounty_window for item in items if item.bounty_window is not None]
    github = [ref for item in items for ref in item.github_refs]
    payload = {
        "schema_version": "1.0.0",
        "purpose": "Track Percolator Discord bounty intelligence and duplicate risk automatically.",
        "generated_from": {
            "source": "CYNIC/organs/discord/src/percolator_watch.py",
            "input": "~/.cynic/organs/discord/messages.db"
        },
        "summary": {
            "items": len(items),
            "github_refs": len(github),
            "bounty_windows": len(windows),
            "high_overlap_items": sum(1 for item in items if item.overlap_risk == "high")
        },
        "active_bounty_windows": [asdict(window) for window in windows],
        "github_refs": [asdict(ref) for ref in github],
        "live_log": [
            {
                **asdict(item),
                "github_refs": [asdict(ref) for ref in item.github_refs],
                "bounty_window": asdict(item.bounty_window) if item.bounty_window else None,
            }
            for item in items
        ],
        "overlap_risk_scale": {
            "none": "unrelated_area",
            "low": "same_component_different_root_cause",
            "medium": "same_instruction_or_invariant_needs_differentiation",
            "high": "same_bug_class_or_public_issue_or_exploit_path_likely_duplicate",
            "duplicate": "do_not_submit_unless_materially_stronger_proof_or_distinct_impact"
        }
    }
    return json.dumps(payload, indent=2, ensure_ascii=True) + "\n"


def update_rules(rules_path: Path, items: list[FeedItem]) -> None:
    if not rules_path.exists():
        return
    rules = read_json_file_or_default(rules_path, {})
    windows = [item.bounty_window for item in items if item.bounty_window is not None]
    if windows:
        latest = sorted(windows, key=lambda window: window.start_utc)[-1]
        scope = rules.setdefault("scope", {})
        announcement = scope.setdefault("timeboxed_announcement", {})
        announcement.update(
            {
                "status": "imported_from_discord_capture",
                "source": latest.source,
                "repository": latest.repository,
                "window": f"{latest.duration_hours}_hours",
                "announcement_time_utc": latest.start_utc,
                "deadline_utc": latest.end_utc,
                "announcement_time_paris": latest.start_paris,
                "deadline_paris": latest.end_paris,
                "severity_override": latest.severity,
                "payment": latest.payment,
                "qualification_dependency": "pinned_message",
            }
        )
        rules["active_priority"] = {
            "repository": latest.repository,
            "severity": latest.severity,
            "deadline_utc": latest.end_utc,
            "deadline_paris": latest.end_paris,
            "reason": "timeboxed_announcement_detected_from_discord_capture",
            "must_follow": "pinned_message",
        }
    rules_path.write_text(json.dumps(rules, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Export Percolator Discord bounty intelligence")
    parser.add_argument("--db", default=str(_DEFAULT_DB), help="Discord organ SQLite database")
    parser.add_argument("--feed", default=str(_DEFAULT_FEED), help="JSON feed to overwrite")
    parser.add_argument("--rules", default=str(_DEFAULT_RULES), help="Bounty rules JSON to update when --update-rules is set")
    parser.add_argument("--limit", type=int, default=2000, help="Recent Discord messages to scan")
    parser.add_argument("--update-rules", action="store_true")
    args = parser.parse_args()

    items = load_items(Path(args.db), args.limit)
    feed_path = Path(args.feed)
    feed_path.parent.mkdir(parents=True, exist_ok=True)
    feed_path.write_text(render_feed(items), encoding="utf-8")
    if args.update_rules:
        update_rules(Path(args.rules), items)
    print(f"percolator feed updated: {feed_path} ({len(items)} items)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
