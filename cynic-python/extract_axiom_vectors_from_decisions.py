#!/usr/bin/env python3
"""
Extract axiom vectors from Qwen-scored decisions.
Then analyze multi-cortex divergence.
"""

import json
import re
import csv
import math
from pathlib import Path
from typing import Dict, List
import logging

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

def extract_axiom_scores(text: str) -> Dict[str, float]:
    """Extract implicit axiom scores from decision text"""
    scores = {
        'fidelity': 0.5,
        'sovereignty': 0.5,
        'burn': 0.5,
        'phi': 0.5,
        'verify': 0.5,
        'culture': 0.5,
    }

    text_lower = text.lower()

    # FIDELITY
    if re.search(r'honest|truth|authentic|radical|transparent|real|sincère|vérit', text_lower):
        scores['fidelity'] += 0.25
    if re.search(r'lie|mask|perform|hide|posture|mensonge|cacher', text_lower):
        scores['fidelity'] -= 0.15

    # SOVEREIGNTY
    if re.search(r'autonomous|freedom|agency|independent|axis|souverain|liberté|autonome', text_lower):
        scores['sovereignty'] += 0.25
    if re.search(r'depend|need|pressure|coerce|forced|dépend|obligé', text_lower):
        scores['sovereignty'] -= 0.15

    # BURN
    if re.search(r'efficient|minimal|waste|overhead|debt|cost|efficace|minimal', text_lower):
        scores['burn'] += 0.25
    if re.search(r'bloat|unnecessary|extra|over-engineer|gaspill', text_lower):
        scores['burn'] -= 0.15

    # PHI
    if re.search(r'harmony|balance|proportion|structure|coherent|harmoni|équilibre', text_lower):
        scores['phi'] += 0.25
    if re.search(r'messy|fragmented|incoherent|scattered|désordonné', text_lower):
        scores['phi'] -= 0.15

    # VERIFY
    if re.search(r'test|measure|verify|falsif|empirical|data|mesur|preuve', text_lower):
        scores['verify'] += 0.25
    if re.search(r'assume|believe|guess|hope|suppose|croire|deviner', text_lower):
        scores['verify'] -= 0.15

    # CULTURE
    if re.search(r'tradition|pattern|precedent|inherit|legacy|tradition|motif', text_lower):
        scores['culture'] += 0.25
    if re.search(r'break|new|novel|disrupt|break|nouveau|innov', text_lower):
        scores['culture'] -= 0.15

    # Clamp to 0-1
    return {k: max(0, min(1, v)) for k, v in scores.items()}

def euclidean_distance(v1: Dict[str, float], v2: Dict[str, float]) -> float:
    """Calculate axiom vector distance"""
    axioms = ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']
    sum_sq = sum((v1.get(a, 0) - v2.get(a, 0)) ** 2 for a in axioms)
    return math.sqrt(sum_sq)

def main():
    input_file = Path('/home/user/Bureau/CYNIC/cynic-python/decisions_filtered.jsonl')

    if not input_file.exists():
        log.error(f"Input file not found: {input_file}")
        log.info("Run score_decisions_qwen.py first")
        return

    # Load scored decisions
    decisions = []
    with open(input_file) as f:
        for line in f:
            decisions.append(json.loads(line))

    log.info(f"Loaded {len(decisions)} high-confidence decisions\n")

    # Extract axiom vectors
    axiom_data = []
    for decision in decisions:
        axioms = extract_axiom_scores(decision['text'])
        axiom_data.append({
            'session_id': decision['session_id'],
            'timestamp': decision['timestamp'],
            'cortex_type': decision['cortex_type'],
            'domain': decision['domain'],
            'decision_score': decision.get('decision_score', 0),
            **axioms,
            'text_preview': decision['text'][:100],
        })

    log.info(f"Extracted axiom vectors for {len(axiom_data)} decisions\n")

    # Save to CSV
    output_csv = Path('/home/user/Bureau/CYNIC/cynic-python/axiom_vectors_final.csv')
    with open(output_csv, 'w') as f:
        writer = csv.DictWriter(
            f,
            fieldnames=['session_id', 'timestamp', 'cortex_type', 'domain', 'decision_score',
                       'fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture', 'text_preview']
        )
        writer.writeheader()
        writer.writerows(axiom_data)

    log.info(f"✓ Saved axiom vectors to {output_csv}\n")

    # Analyze multi-cortex divergence
    log.info("=== MULTI-CORTEX DIVERGENCE ANALYSIS ===\n")

    # Group by timestamp + domain
    divergences = []
    processed_pairs = set()

    for i, d1 in enumerate(axiom_data):
        for j, d2 in enumerate(axiom_data):
            if i >= j:
                continue

            # Same minute + different cortex
            if d1['timestamp'][:16] == d2['timestamp'][:16] and d1['cortex_type'] != d2['cortex_type']:
                pair_key = tuple(sorted([d1['timestamp'], d1['cortex_type'], d2['cortex_type']]))
                if pair_key in processed_pairs:
                    continue
                processed_pairs.add(pair_key)

                # Extract axiom vectors
                v1 = {ax: d1[ax] for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']}
                v2 = {ax: d2[ax] for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']}
                distance = euclidean_distance(v1, v2)

                # Find max divergence axis
                max_diff = 0
                max_axiom = ""
                for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']:
                    diff = abs(v1[ax] - v2[ax])
                    if diff > max_diff:
                        max_diff = diff
                        max_axiom = ax

                divergences.append({
                    'timestamp': d1['timestamp'],
                    'distance': distance,
                    'max_axiom': max_axiom,
                    'max_diff': max_diff,
                    'cortex_1': d1['cortex_type'],
                    'cortex_2': d2['cortex_type'],
                    'score_1': d1['decision_score'],
                    'score_2': d2['decision_score'],
                })

    if divergences:
        divergences.sort(key=lambda x: x['distance'], reverse=True)

        log.info(f"Found {len(divergences)} multi-cortex moments\n")

        for i, div in enumerate(divergences[:3]):
            log.info(f"[{i+1}] {div['timestamp']}")
            log.info(f"    {div['cortex_1']} vs {div['cortex_2']}")
            log.info(f"    Distance: {div['distance']:.3f}")
            log.info(f"    Max divergence: {div['max_axiom'].upper()} (Δ={div['max_diff']:.2f})\n")

        # Summary
        distances = [d['distance'] for d in divergences]
        log.info(f"\nDIVERGENCE STATS:")
        log.info(f"  Pairs: {len(divergences)}")
        log.info(f"  Avg distance: {sum(distances) / len(distances):.3f}")
        log.info(f"  Max distance: {max(distances):.3f}")
        log.info(f"  Min distance: {min(distances):.3f}")

        high = sum(1 for d in divergences if d['distance'] > 0.618)
        med = sum(1 for d in divergences if 0.382 <= d['distance'] <= 0.618)
        low = sum(1 for d in divergences if d['distance'] < 0.382)
        log.info(f"\n  HIGH (>0.618): {high}")
        log.info(f"  MEDIUM (0.382-0.618): {med}")
        log.info(f"  LOW (<0.382): {low}\n")

    else:
        log.info("No multi-cortex moments found (expected: only Gemini CLI in MVP)")
        log.info("This will improve when Claude Code sessions are included.\n")

    # Summary statistics across all decisions
    log.info("=== AXIOM DISTRIBUTION (ALL DECISIONS) ===\n")
    axiom_means = {ax: 0 for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']}
    for decision in axiom_data:
        for ax in axiom_means:
            axiom_means[ax] += decision[ax]

    for ax, total in axiom_means.items():
        mean = total / len(axiom_data) if axiom_data else 0
        log.info(f"  {ax.upper()}: {mean:.2f}")

    log.info(f"\n✓ Analysis complete")

if __name__ == '__main__':
    main()
