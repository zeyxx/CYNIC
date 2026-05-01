#!/usr/bin/env python3
"""
KENOSIS Pattern Extraction & Irreducibility Analysis
Mines Gemini CLI session logs for Wu-Wei / KENOSIS patterns
"""

import json
import re
from pathlib import Path
from typing import TypedDict, List
from dataclasses import dataclass, asdict
import logging

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

# Pattern matching
KENOSIS_PATTERNS = [
    r'wu-wei|Wu-Wei|WU-WEI',
    r'KENOSIS|kenosis|Kenosis',
    r'silence\s+radio|radio\s+silence',
    r'l[\'a]gir\s+non-agir|non-action|non-agir',
    r'agir en ne\s+.*agissant',
    r'espace.*important|important.*espace',
    r'7th\s+axiom|septième\s+axiome|7e\s+axiome',
    r'let.*go|abandon.*conscient',
    r'strategic.*non.*action|non.*action.*stratégique',
]

BURN_PATTERNS = [
    r'BURN|burn|Burn',
    r'efficien|efficiency|wast|moindre.*effort|least.*effort',
    r'metabolic|overhead|debt',
]

@dataclass
class KenosisPattern:
    timestamp: str
    session_id: str
    domain: str
    raw_content: str
    pattern_type: str  # 'wu-wei', 'kenosis', 'letting_go', 'strategic_silence'
    axiom_mentioned: str  # 'KENOSIS', 'BURN', 'SOVEREIGNTY', etc.
    decision_context: str  # What decision triggered this?
    is_irreducible: bool = False  # Can't be restated as BURN/FIDELITY/SOVEREIGNTY alone?
    reasoning: str = ""

def extract_kenosis_instances(session_file: Path) -> List[KenosisPattern]:
    """Extract KENOSIS instances from a single JSONL session file."""
    patterns = []

    try:
        with open(session_file) as f:
            for i, line in enumerate(f):
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue

                if 'content' not in entry:
                    continue

                # Extract text from content array
                content_array = entry.get('content', [])
                if not content_array or not isinstance(content_array, list):
                    continue

                text_parts = []
                for content_item in content_array:
                    if isinstance(content_item, dict) and 'text' in content_item:
                        text_parts.append(content_item['text'])

                text = ' '.join(text_parts)

                timestamp = entry.get('timestamp', 'unknown')
                session_id = session_file.stem.replace('session-', '').replace('.jsonl', '')
                domain = entry.get('domain', 'unknown')

                # Check for KENOSIS patterns
                for pattern in KENOSIS_PATTERNS:
                    if re.search(pattern, text, re.IGNORECASE):
                        # Extract pattern type
                        if 'wu-wei' in text.lower():
                            ptype = 'wu-wei'
                        elif 'kenosis' in text.lower():
                            ptype = 'kenosis'
                        elif 'silence' in text.lower() and 'radio' in text.lower():
                            ptype = 'strategic_silence'
                        elif 'abandon' in text.lower() or 'let go' in text.lower():
                            ptype = 'letting_go'
                        else:
                            ptype = 'non_action'

                        # Find mentioned axioms
                        axioms = []
                        for axiom in ['KENOSIS', 'BURN', 'SOVEREIGNTY', 'FIDELITY', 'PHI', 'VERIFY', 'CULTURE']:
                            if axiom in text:
                                axioms.append(axiom)

                        # Extract decision context (previous/next 200 chars)
                        snippet = text[max(0, text.lower().find(pattern)-100):min(len(text), text.lower().find(pattern)+300)]

                        patterns.append(KenosisPattern(
                            timestamp=timestamp,
                            session_id=session_id,
                            domain=domain,
                            raw_content=snippet[:500],
                            pattern_type=ptype,
                            axiom_mentioned=','.join(axioms) if axioms else 'INFERRED',
                            decision_context='',  # Will analyze manually
                        ))

    except Exception as e:
        log.warning(f"Error processing {session_file}: {e}")

    return patterns

def analyze_irreducibility(pattern: KenosisPattern) -> bool:
    """
    Check if a KENOSIS pattern is irreducible to BURN/FIDELITY/SOVEREIGNTY.

    Rubric:
    - BURN: "avoid wasted effort" → strategic efficiency
    - KENOSIS: "preserve autonomy/freedom via non-action" → higher-order strategic principle

    KENOSIS is irreducible if:
    1. The non-action preserves someone else's autonomy (not just your own efficiency)
    2. The space left is intentionally strategic (not default passivity)
    3. The cost to remain silent exceeds the cost to act (sacrifice)
    """

    content_lower = pattern.raw_content.lower()

    # Check for autonomy-preservation indicators
    autonomy_signals = [
        r'protect\s+space|preserv\s+.*freedom|autonomy',
        r'let.*her.*initiative|son\s+initiative',
        r'espace.*croi?tre|space.*grow',
        r'éviter.*pression|avoid.*pressure',
        r'non\s+forcer|don.*force',
    ]

    autonomy_score = sum(1 for sig in autonomy_signals if re.search(sig, content_lower, re.IGNORECASE))

    # Check for intentional strategy (vs passive waiting)
    strategy_signals = [
        r'stratégi|strateg|intentional|deliberat',
        r'espace.*important|important.*espace',
        r'structure.*stable|équilibre',
    ]

    strategy_score = sum(1 for sig in strategy_signals if re.search(sig, content_lower, re.IGNORECASE))

    # Check for sacrifice/cost
    cost_signals = [
        r'wait|attendre|patience',
        r'resist|résist|difficult',
        r'tempt|tentation|pressure',
    ]

    cost_score = sum(1 for sig in cost_signals if re.search(sig, content_lower, re.IGNORECASE))

    # Irreducible if: autonomy_score > 0 AND (strategy_score + cost_score) > 0
    is_irreducible = autonomy_score > 0 and (strategy_score + cost_score) > 0

    return is_irreducible

def main():
    session_dir = Path.home() / '.gemini' / 'tmp' / 'cynic' / 'chats'
    session_files = sorted(session_dir.glob('session-*.jsonl'), reverse=True)

    log.info(f"Found {len(session_files)} session files. Mining for KENOSIS patterns...")

    all_patterns = []
    for i, session_file in enumerate(session_files):
        if i % 10 == 0:
            log.info(f"  Processing {i}/{len(session_files)}: {session_file.name}")

        patterns = extract_kenosis_instances(session_file)

        # Analyze irreducibility for each pattern
        for p in patterns:
            p.is_irreducible = analyze_irreducibility(p)

        all_patterns.extend(patterns)

    log.info(f"\n✓ Found {len(all_patterns)} KENOSIS patterns across {len(session_files)} sessions\n")

    # Aggregate statistics
    irreducible_count = sum(1 for p in all_patterns if p.is_irreducible)
    reducible_count = len(all_patterns) - irreducible_count

    log.info(f"IRREDUCIBILITY VERDICT:")
    log.info(f"  Irreducible patterns (high confidence KENOSIS 7th axiom): {irreducible_count} ({100*irreducible_count/len(all_patterns):.1f}%)")
    log.info(f"  Reducible patterns (can be restated as BURN/FIDELITY): {reducible_count} ({100*reducible_count/len(all_patterns):.1f}%)")

    # Break down by pattern type
    pattern_types = {}
    for p in all_patterns:
        pattern_types[p.pattern_type] = pattern_types.get(p.pattern_type, 0) + 1

    log.info(f"\nBREAKDOWN BY PATTERN TYPE:")
    for ptype, count in sorted(pattern_types.items(), key=lambda x: -x[1]):
        irreducible_in_type = sum(1 for p in all_patterns if p.pattern_type == ptype and p.is_irreducible)
        log.info(f"  {ptype}: {count} total, {irreducible_in_type} irreducible")

    # Save detailed results
    output_file = Path('/home/user/Bureau/CYNIC/cynic-python/kenosis_analysis_results.jsonl')
    with open(output_file, 'w') as f:
        for p in all_patterns:
            f.write(json.dumps(asdict(p)) + '\n')

    log.info(f"\n✓ Results saved to {output_file}")

    # Sample irreducible patterns
    irreducible_patterns = [p for p in all_patterns if p.is_irreducible]
    if irreducible_patterns:
        log.info(f"\n=== SAMPLE IRREDUCIBLE PATTERNS (showing 3) ===")
        for p in irreducible_patterns[:3]:
            log.info(f"\n[{p.timestamp}] {p.pattern_type.upper()}")
            log.info(f"Axioms: {p.axiom_mentioned}")
            log.info(f"Content: {p.raw_content[:300]}...")

    # Falsification criterion
    log.info(f"\n=== FALSIFICATION RESULT ===")
    if irreducible_count / len(all_patterns) > 0.80:
        log.info(f"✓ KENOSIS CONFIRMED as 7th axiom (>{80}% patterns irreducible)")
        log.info(f"  Decision: Advance to 12-week empirical validation (Phase 2)")
        return 0
    elif irreducible_count / len(all_patterns) > 0.50:
        log.info(f"⚠ KENOSIS CANDIDATE (50-80% patterns irreducible)")
        log.info(f"  Decision: Extend observation period, refine irreducibility metric")
        return 1
    else:
        log.info(f"✗ KENOSIS REJECTED (<50% patterns irreducible, reducible to BURN/FIDELITY)")
        return 2

if __name__ == '__main__':
    exit(main())
