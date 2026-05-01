#!/usr/bin/env python3
"""
Mine 5 target queries from decision axiom vectors and session metadata.

TARGETS:
1. Multi-cortex divergence: axiom vector distances between Claude/Gemini pairs at same timestamp
2. MC1-MC5 rule violations: coordination gaps (overlapping branches, shared modules, unmerged PRs)
3. FOGC inversions: axiom relevance drops to zero (axiom logic inverted from actual decision)
4. Anti-patterns: repeated mistakes with measurable cost (same error >1x, observable impact)
5. Heuristic drift: rules written vs actual behavior divergence (gate declares X, code does Y)
"""

import csv
import json
from pathlib import Path
from collections import defaultdict
import logging
import math

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

def load_axiom_vectors():
    """Load axiom vectors from CSV"""
    vectors = []
    with open('cynic-python/axiom_vectors_final.csv') as f:
        reader = csv.DictReader(f)
        for row in reader:
            vectors.append({
                'session_id': row['session_id'],
                'timestamp': row['timestamp'],
                'cortex_type': row['cortex_type'],
                'domain': row['domain'],
                'decision_score': float(row['decision_score']),
                'fidelity': float(row['fidelity']),
                'sovereignty': float(row['sovereignty']),
                'burn': float(row['burn']),
                'phi': float(row['phi']),
                'verify': float(row['verify']),
                'culture': float(row['culture']),
                'text_preview': row['text_preview'],
            })
    return vectors

def euclidean_distance(v1, v2):
    """Calculate axiom vector distance"""
    axioms = ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']
    sum_sq = sum((v1[a] - v2[a]) ** 2 for a in axioms)
    return math.sqrt(sum_sq)

def mine_multi_cortex_divergence(vectors):
    """TARGET 1: Multi-cortex divergence"""
    log.info("\n=== TARGET 1: MULTI-CORTEX DIVERGENCE ===\n")

    # Group by timestamp minute
    by_minute = defaultdict(list)
    for v in vectors:
        minute_key = v['timestamp'][:16]  # YYYY-MM-DDTHH:MM
        by_minute[minute_key].append(v)

    divergences = []
    for minute, group in by_minute.items():
        cortex_types = set(v['cortex_type'] for v in group)
        if len(cortex_types) > 1 and len(group) > 1:
            # Multi-cortex moment
            for i, v1 in enumerate(group):
                for v2 in group[i+1:]:
                    if v1['cortex_type'] != v2['cortex_type']:
                        distance = euclidean_distance(v1, v2)
                        max_diff_axiom = max(
                            ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture'],
                            key=lambda ax: abs(v1[ax] - v2[ax])
                        )
                        max_diff = abs(v1[max_diff_axiom] - v2[max_diff_axiom])
                        divergences.append({
                            'timestamp': minute,
                            'cortex_1': v1['cortex_type'],
                            'cortex_2': v2['cortex_type'],
                            'distance': distance,
                            'max_axiom': max_diff_axiom,
                            'max_diff': max_diff,
                        })

    if divergences:
        divergences.sort(key=lambda x: x['distance'], reverse=True)
        log.info(f"Found {len(divergences)} multi-cortex divergence moments\n")
        for i, d in enumerate(divergences[:5]):
            log.info(f"[{i+1}] {d['timestamp']}: {d['cortex_1']} vs {d['cortex_2']}")
            log.info(f"    Distance: {d['distance']:.3f}, Max: {d['max_axiom'].upper()} (Δ={d['max_diff']:.2f})\n")
    else:
        log.info("No multi-cortex divergence found (expected: MVP is Gemini CLI only)\n")

    return divergences

def mine_fogc_inversions(vectors):
    """TARGET 3: FOGC inversions (axiom irrelevance)"""
    log.info("\n=== TARGET 3: FOGC INVERSIONS ===\n")

    phi_const = 0.618034

    inversions = []
    for v in vectors:
        axioms = ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']
        for ax in axioms:
            score = v[ax]
            # Inversion: axiom should be high but is ≤0.4 (below median+1σ)
            if score <= 0.4 and v['decision_score'] > 0.5:
                inversions.append({
                    'timestamp': v['timestamp'],
                    'session': v['session_id'],
                    'axiom': ax,
                    'score': score,
                    'decision_score': v['decision_score'],
                    'context': v['text_preview'][:80],
                })

    if inversions:
        log.info(f"Found {len(inversions)} potential FOGC inversions\n")
        by_axiom = defaultdict(list)
        for inv in inversions:
            by_axiom[inv['axiom']].append(inv)

        for ax, items in sorted(by_axiom.items(), key=lambda x: -len(x[1]))[:3]:
            log.info(f"{ax.upper()}: {len(items)} low-signal decisions")
            for item in items[:2]:
                log.info(f"  {item['timestamp']}: score={item['score']:.2f} (context: {item['context']}...)\n")
    else:
        log.info("No FOGC inversions found\n")

    return inversions

def mine_anti_patterns(vectors):
    """TARGET 4: Anti-patterns (repeated issues with cost)"""
    log.info("\n=== TARGET 4: ANTI-PATTERNS ===\n")

    # Heuristic: same axiom combination appearing multiple times in same session
    patterns = defaultdict(list)
    for v in vectors:
        # Create pattern signature: which axioms are weak (<0.5)?
        weak_axioms = tuple(sorted([
            ax for ax in ['fidelity', 'sovereignty', 'burn', 'phi', 'verify', 'culture']
            if v[ax] < 0.5
        ]))
        patterns[weak_axioms].append({
            'session': v['session_id'],
            'timestamp': v['timestamp'],
            'score': v['decision_score'],
        })

    # Find patterns that repeat (>1 occurrence in same session is suspicious)
    anti_patterns = []
    for pattern, occurrences in patterns.items():
        if len(occurrences) > 1:
            sessions = defaultdict(int)
            for occ in occurrences:
                sessions[occ['session']] += 1

            for sess, count in sessions.items():
                if count > 1:
                    anti_patterns.append({
                        'weak_axioms': pattern,
                        'session': sess,
                        'count': count,
                        'avg_score': sum(o['score'] for o in occurrences) / len(occurrences),
                    })

    if anti_patterns:
        anti_patterns.sort(key=lambda x: -x['count'])
        log.info(f"Found {len(anti_patterns)} potential anti-patterns\n")
        for ap in anti_patterns[:5]:
            axioms = ap['weak_axioms'] if ap['weak_axioms'] else ['(all balanced)']
            log.info(f"Weak: {', '.join(axioms)} → {ap['count']}x in {ap['session'][:12]}")
            log.info(f"     Avg decision score: {ap['avg_score']:.2f}\n")
    else:
        log.info("No anti-patterns detected\n")

    return anti_patterns

def mine_heuristic_drift():
    """TARGET 5: Heuristic drift (rules declared vs behavior)"""
    log.info("\n=== TARGET 5: HEURISTIC DRIFT ===\n")

    # Check if extract_axiom_vectors_from_decisions.py heuristics match actual data distribution
    vectors = load_axiom_vectors()

    # Extract heuristics from source code
    heuristics = {
        'fidelity': {
            'positive': ['honest', 'truth', 'authentic', 'radical', 'transparent', 'real', 'sincère', 'vérit'],
            'negative': ['lie', 'mask', 'perform', 'hide', 'posture', 'mensonge', 'cacher'],
        },
        'sovereignty': {
            'positive': ['autonomous', 'freedom', 'agency', 'independent', 'axis', 'souverain', 'liberté', 'autonome'],
            'negative': ['depend', 'need', 'pressure', 'coerce', 'forced', 'dépend', 'obligé'],
        },
    }

    # For this MVP, simple report: are the declared heuristics present in the texts?
    log.info("Heuristic presence check:")
    for ax, keywords in heuristics.items():
        texts = [v['text_preview'].lower() for v in vectors]
        combined = ' '.join(texts)
        pos_count = sum(1 for kw in keywords['positive'] if kw in combined)
        neg_count = sum(1 for kw in keywords['negative'] if kw in combined)
        log.info(f"  {ax.upper()}: +{pos_count} keywords found, -{neg_count} negatives\n")

    return []

def main():
    vectors = load_axiom_vectors()
    log.info(f"Loaded {len(vectors)} decision vectors\n")

    div = mine_multi_cortex_divergence(vectors)
    invrs = mine_fogc_inversions(vectors)
    anti = mine_anti_patterns(vectors)
    drift = mine_heuristic_drift()

    # MC1-MC5 violations require git/branch data, deferred to next session
    log.info("\n=== TARGET 2: MC1-MC5 COORDINATION VIOLATIONS ===\n")
    log.info("Requires git branch/PR analysis. Deferred to dedicated session.\n")

    log.info(f"\n=== SUMMARY ===")
    log.info(f"T1 (Multi-cortex): {len(div)} divergences")
    log.info(f"T2 (MC1-MC5): Data needed from git/GitHub")
    log.info(f"T3 (FOGC): {len(invrs)} inversions")
    log.info(f"T4 (Anti): {len(anti)} patterns")
    log.info(f"T5 (Drift): Heuristic check complete\n")

if __name__ == '__main__':
    main()
