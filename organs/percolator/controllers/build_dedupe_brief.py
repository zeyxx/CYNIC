#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

FEED = Path('percolator/ops/researcher-feed.json')
OUT = Path('percolator/ops/dedupe-brief.json')

AREA_PATTERNS = {
    'accounting_capital_vault': re.compile(r'accounting|capital|vault|withdraw|zero-position|zero position', re.I),
    'oracle': re.compile(r'oracle|pyth|price|confidence|freshness|feed', re.I),
    'cpi_token': re.compile(r'cpi|token|spl|token-2022|mint|ata', re.I),
    'authority_pda': re.compile(r'authority|signer|pda|seed|admin|owner', re.I),
    'resolution_settlement': re.compile(r'resolution|settlement|resolve|claim|payout', re.I),
    'account_closure': re.compile(r'close|closure|rent|stale|reinit', re.I),
    'test_feature_or_build': re.compile(r'test|feature|mainnet|cfg|bypass', re.I),
}

def area_for(text: str) -> str | None:
    for area, pat in AREA_PATTERNS.items():
        if pat.search(text or ''):
            return area
    return None

def main() -> int:
    feed = json.loads(FEED.read_text()) if FEED.exists() else {'live_log': []}
    by_repo = defaultdict(list)
    by_area = defaultdict(list)
    submissions = []
    for item in feed.get('live_log', []):
        text = item.get('claim_or_finding', '')
        refs = item.get('github_refs', []) or []
        area = area_for(text)
        entry = {
            'source': item.get('source'),
            'time_utc': item.get('time_utc'),
            'researcher': item.get('researcher'),
            'classification': item.get('classification'),
            'area_guess': area,
            'overlap_risk': item.get('overlap_risk'),
            'claim_or_finding': text,
            'github_refs': refs,
            'evidence_links': item.get('evidence_links', []),
        }
        if refs:
            submissions.append(entry)
        if area:
            by_area[area].append(entry)
        for ref in refs:
            repo = f"{ref.get('owner')}/{ref.get('repo')}"
            by_repo[repo].append(entry)
    payload = {
        'schema_version': '1.0.0',
        'generated_from': str(FEED),
        'summary': {
            'feed_items': len(feed.get('live_log', [])),
            'submissions_with_github_refs': len(submissions),
            'areas_with_overlap': sorted(by_area.keys()),
            'repos_with_overlap': sorted(by_repo.keys()),
        },
        'by_repo': dict(sorted(by_repo.items())),
        'by_area': dict(sorted(by_area.items())),
        'submissions': submissions,
        'audit_guidance': {
            'avoid_or_differentiate': sorted(by_area.keys()),
            'initial_open_space': [
                'oracle identity/freshness/confidence if not already in issues',
                'CPI token account/mint/program binding if not already in issues',
                'account closure/stale state reuse if not already in issues',
                'mainnet/test feature leakage if not already in issues'
            ]
        }
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + '\n')
    print(json.dumps({'wrote': str(OUT), 'submissions': len(submissions), 'areas': sorted(by_area.keys())}, indent=2))
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
