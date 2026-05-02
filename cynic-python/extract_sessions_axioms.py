#!/usr/bin/env python3
"""
Extract axiom vectors from representative sessions for multi-cortex divergence analysis.
MVP: 10 sessions (5 Vanille/Erin + 5 diverse domains)
"""

import json
import re
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import List, Dict, Tuple
import logging

logging.basicConfig(level=logging.INFO, format='%(message)s')
log = logging.getLogger(__name__)

@dataclass
class DecisionPoint:
    """A single decision with axiom evaluation"""
    session_id: str
    timestamp: str
    domain: str
    cortex_type: str  # 'gemini' or 'claude' (when identifiable)
    decision_text: str
    # Axiom scores (0-1 scale)
    fidelity: float
    sovereignty: float
    burn: float
    phi: float
    verify: float
    culture: float
    # Metadata
    is_multi_cortex_moment: bool = False
    partner_cortex: str = ""  # if multi-cortex, which other cortex decided?

def extract_axiom_mentions(text: str) -> Dict[str, float]:
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

    # FIDELITY signals
    if re.search(r'honest|truth|authentic|radical|transparent|real', text_lower):
        scores['fidelity'] += 0.2
    if re.search(r'lie|mask|perform|hide|posture', text_lower):
        scores['fidelity'] -= 0.1

    # SOVEREIGNTY signals
    if re.search(r'autonomous|freedom|agency|independent|my own|axis', text_lower):
        scores['sovereignty'] += 0.2
    if re.search(r'depend|need|pressure|coerce|forced', text_lower):
        scores['sovereignty'] -= 0.1

    # BURN signals
    if re.search(r'efficient|minimal|waste|overhead|debt|cost', text_lower):
        scores['burn'] += 0.2
    if re.search(r'bloat|unnecessary|extra|over-engineer', text_lower):
        scores['burn'] -= 0.1

    # PHI signals
    if re.search(r'harmony|balance|proportion|structure|coherent', text_lower):
        scores['phi'] += 0.2
    if re.search(r'messy|fragmented|incoherent|scattered', text_lower):
        scores['phi'] -= 0.1

    # VERIFY signals
    if re.search(r'test|measure|verify|falsif|empirical|data', text_lower):
        scores['verify'] += 0.2
    if re.search(r'assume|believe|guess|hope', text_lower):
        scores['verify'] -= 0.1

    # CULTURE signals
    if re.search(r'tradition|pattern|precedent|inherit|legacy', text_lower):
        scores['culture'] += 0.2
    if re.search(r'break|new|novel|disrupt', text_lower):
        scores['culture'] -= 0.1

    # Clamp to 0-1
    return {k: max(0, min(1, v)) for k, v in scores.items()}

def extract_decisions_from_session(session_path: Path) -> List[DecisionPoint]:
    """Extract decision points from a single JSONL session"""
    decisions = []

    try:
        with open(session_path) as f:
            for line in f:
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
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

                # Identify decision blocks (keywords: "Axiome", "Verdict", "Évaluation", "Conseil")
                if re.search(r'axiome|verdict|évaluation|conseil|décision|stratégi', text, re.I):
                    timestamp = entry.get('timestamp', 'unknown')
                    cortex_type = entry.get('type', 'unknown')
                    session_id = session_path.stem
                    domain = entry.get('domain', 'general')

                    # Extract axiom scores
                    axiom_scores = extract_axiom_mentions(text)

                    decision = DecisionPoint(
                        session_id=session_id,
                        timestamp=timestamp,
                        domain=domain,
                        cortex_type=cortex_type,
                        decision_text=text[:500],  # First 500 chars
                        fidelity=axiom_scores['fidelity'],
                        sovereignty=axiom_scores['sovereignty'],
                        burn=axiom_scores['burn'],
                        phi=axiom_scores['phi'],
                        verify=axiom_scores['verify'],
                        culture=axiom_scores['culture'],
                    )
                    decisions.append(decision)

    except Exception as e:
        log.warning(f"Error processing {session_path}: {e}")

    return decisions

def select_representative_sessions() -> List[Path]:
    """Select 10 representative sessions: 5 Vanille/Erin + 5 diverse"""
    session_dir = Path.home() / '.gemini' / 'tmp' / 'cynic' / 'chats'
    session_files = sorted(session_dir.glob('session-*.jsonl'))

    # Known key sessions
    vanille_erin = [
        'session-2026-04-28T12-58-b0eba480.jsonl',  # Vanille Wu-Wei
        'session-2026-04-25T12-52-83d14cea.jsonl',  # Early Erin
        'session-2026-04-25T14-35-cbbd1ad7.jsonl',  # Erin decision
        'session-2026-04-26T12-15-119d0d35.jsonl',  # Erin reflection
        'session-2026-04-26T14-03-ad8603c3.jsonl',  # Erin follow-up
    ]

    diverse = [
        'session-2026-04-24T13-43-2986cfe1.jsonl',  # Start of month
        'session-2026-04-25T10-44-03d195b3.jsonl',  # Mid-week
        'session-2026-04-27T14-03-ad8603c3.jsonl',  # Hardware/Conseil
        'session-2026-04-29T10-13-44499c90.jsonl',  # Late month
        'session-2026-04-30T17-15-05cc8234.jsonl',  # End of month
    ]

    selected = []
    for name in vanille_erin + diverse:
        path = session_dir / name
        if path.exists():
            selected.append(path)
        else:
            log.warning(f"Session not found: {name}")

    return selected[:10]

def main():
    log.info("Extracting axiom vectors from representative sessions...")

    sessions = select_representative_sessions()
    log.info(f"Found {len(sessions)} representative sessions")

    all_decisions = []
    for session_path in sessions:
        decisions = extract_decisions_from_session(session_path)
        all_decisions.extend(decisions)
        log.info(f"  {session_path.name}: {len(decisions)} decision points")

    log.info(f"\n✓ Extracted {len(all_decisions)} total decision points\n")

    # Detect multi-cortex moments (same timestamp range, different cortex types)
    for i, decision in enumerate(all_decisions):
        for j, other in enumerate(all_decisions):
            if i < j and decision.timestamp[:16] == other.timestamp[:16]:  # Same minute
                if decision.cortex_type != other.cortex_type:
                    decision.is_multi_cortex_moment = True
                    decision.partner_cortex = other.cortex_type
                    other.is_multi_cortex_moment = True
                    other.partner_cortex = decision.cortex_type

    # Save to CSV
    output_file = Path('/home/user/Bureau/CYNIC/cynic-python/axiom_vectors_mvp.csv')
    with open(output_file, 'w') as f:
        # Header
        f.write('session_id,timestamp,domain,cortex_type,fidelity,sovereignty,burn,phi,verify,culture,is_multi_cortex,partner_cortex,decision_text\n')

        # Data
        for decision in all_decisions:
            f.write(
                f'"{decision.session_id}",'
                f'"{decision.timestamp}",'
                f'"{decision.domain}",'
                f'"{decision.cortex_type}",'
                f'{decision.fidelity:.2f},'
                f'{decision.sovereignty:.2f},'
                f'{decision.burn:.2f},'
                f'{decision.phi:.2f},'
                f'{decision.verify:.2f},'
                f'{decision.culture:.2f},'
                f'{int(decision.is_multi_cortex_moment)},'
                f'"{decision.partner_cortex}",'
                f'"{decision.decision_text[:100]}"\n'
            )

    log.info(f"✓ Saved to {output_file}")
    log.info(f"\nNext: Analyze multi-cortex divergence with:")
    log.info(f"  python3 /home/user/Bureau/CYNIC/cynic-python/analyze_divergence.py")

if __name__ == '__main__':
    main()
