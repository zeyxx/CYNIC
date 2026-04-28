#!/usr/bin/env python3
import json
import re
import sys

with open('captures/dataset.jsonl', 'r') as f:
    tweets = [json.loads(line) for line in f if line.strip()]

# Find all high-signal tweets (>=3)
high_signal = []
for t in tweets:
    score = t.get('signal_score', 0)
    if score >= 3:
        high_signal.append(t)

print(f'High-signal tweets (score >= 3): {len(high_signal)}')
print()

# Group by narrative
narratives = {}
for t in high_signal:
    narrs = t.get('narratives', [])
    for n in narrs:
        if n not in narratives:
            narratives[n] = []
        narratives[n].append(t)

print('Distribution by narrative:')
for n, ts in sorted(narratives.items()):
    print(f'  {n}: {len(ts)}')
print()

# Print first 5 highest-scoring tweets
print('Top 5 highest-scored tweets:')
high_signal.sort(key=lambda x: x.get('signal_score', 0), reverse=True)
for i, t in enumerate(high_signal[:5], 1):
    print(f'{i}. Score: {t.get("signal_score")}, Author: @{t.get("author_screen_name")}, Tier: {t.get("author_tier")}')
    print(f'   Text: {t.get("text", "")[:200]}')
    print()

# Find rug warnings
print('Rug warning tweets:')
rug_warnings = [t for t in high_signal if 'rug_warning' in t.get('narratives', [])]
for t in rug_warnings[:15]:
    tokens = t.get('cashtags', []) + re.findall(r'\$([A-Z]{3,10})', t.get('text', ''))
    print(f"Score: {t['signal_score']}, Author: @{t.get('author_screen_name')}, Tokens: {tokens}")
    print(f"   {t.get('text', '')[:250]}")
    print()
