#!/usr/bin/env python3
"""
Filter decisions using heuristic scoring (no Qwen dependency).
Fallback for MVP when Qwen unavailable.
"""

import json
import re
from pathlib import Path
import logging

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

def heuristic_decision_score(text: str) -> float:
    """Score decision-ness using keyword heuristics (0-1)"""
    score = 0.0

    # Strong decision indicators
    if re.search(r'axiome|verdict|évaluation|conseil|décision|stratégi', text, re.I):
        score += 0.3
    if re.search(r'trade-off|tradeoff|choix|choice|option', text, re.I):
        score += 0.2
    if re.search(r'donc|thus|so|par conséquent|résultat', text, re.I):
        score += 0.15

    # Reasoning depth
    if len(text) > 300:  # Longer = more likely decision
        score += 0.1

    # Weak signals (already present = less decisive)
    if re.search(r'waiting|attendant|pending|think|réfléchi', text, re.I):
        score += 0.05

    return min(1.0, score)

def main():
    input_file = Path('/home/user/Bureau/CYNIC/cynic-python/text_blocks_raw.jsonl')

    if not input_file.exists():
        log.error(f"Input file not found: {input_file}")
        return

    log.info("Filtering decisions using heuristic scoring (Qwen fallback)...\n")

    # Load and score
    all_blocks = []
    with open(input_file) as f:
        blocks = [json.loads(line) for line in f]

    for block in blocks:
        score = heuristic_decision_score(block['text'])
        block['decision_score'] = score
        all_blocks.append(block)

    # Filter to moderate-high confidence (>0.3)
    decisions = [b for b in all_blocks if b['decision_score'] > 0.3]

    log.info(f"Scored {len(all_blocks)} blocks")
    log.info(f"  Decisions (>0.3): {len(decisions)}")
    log.info(f"  Strong (>0.5): {sum(1 for b in all_blocks if b['decision_score'] > 0.5)}\n")

    # Save decisions
    output_file = Path('/home/user/Bureau/CYNIC/cynic-python/decisions_filtered.jsonl')
    with open(output_file, 'w') as f:
        for decision in decisions:
            f.write(json.dumps(decision) + '\n')

    log.info(f"✓ Saved {len(decisions)} decisions to {output_file}")

if __name__ == '__main__':
    main()
