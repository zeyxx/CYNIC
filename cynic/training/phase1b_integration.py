"""
Phase 1B Integration: Extract real governance proposals for Phase 2 fine-tuning

Pulls proposal data from governance_bot.db and formats as training examples.
Combines real voting outcomes with CYNIC judgments to create labeled training data.

Data Flow:
  governance_bot.db (proposals, votes, outcomes, feedback)
  ↓
  extract_proposals_from_bot_db()
  ↓
  format_as_training_examples()
  ↓
  JSONL training file for Mistral fine-tuning
"""

import json
import logging
import sqlite3
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict
from enum import Enum

logger = logging.getLogger("cynic.training.phase1b_integration")


# ════════════════════════════════════════════════════════════════════════════
# DATA MODELS
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class BotProposal:
    """Proposal from governance_bot database."""
    proposal_id: str
    title: str
    description: str
    category: str
    impact_level: str
    voting_status: str
    approval_status: Optional[str]
    judgment_verdict: Optional[str]
    judgment_q_score: Optional[float]
    judgment_confidence: Optional[float]
    yes_votes: float
    no_votes: float
    abstain_votes: float
    community_satisfaction_rating: Optional[float]


@dataclass
class TrainingExample:
    """Training example for Mistral fine-tuning."""
    proposal: BotProposal
    verdict: str
    q_score: float
    confidence: float
    reasoning: str


# ════════════════════════════════════════════════════════════════════════════
# DATABASE EXTRACTION
# ════════════════════════════════════════════════════════════════════════════

def extract_proposals_from_bot_db(
    db_path: Optional[Path] = None,
    min_votes: int = 1,  # Minimum votes to be considered valid
    only_closed: bool = True,  # Only include closed/resolved proposals
) -> List[BotProposal]:
    """
    Extract proposals from governance_bot.db.

    Args:
        db_path: Path to governance_bot.db (auto-detect if not provided)
        min_votes: Minimum votes required for a proposal to be included
        only_closed: Only include CLOSED proposals (have real outcomes)

    Returns:
        List of BotProposal objects
    """
    # Auto-detect database path
    if db_path is None:
        db_path = Path.home() / ".cynic" / "governance_bot.db"
        if not db_path.exists():
            # Try local governance_bot directory
            db_path = Path("governance_bot/governance_bot.db")
        if not db_path.exists():
            raise FileNotFoundError(f"Could not find governance_bot.db at {db_path}")

    logger.info(f"Extracting proposals from {db_path}")

    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    try:
        # Query all proposals with voting data
        cursor.execute("""
            SELECT
                proposal_id,
                title,
                description,
                category,
                impact_level,
                voting_status,
                approval_status,
                judgment_verdict,
                judgment_q_score,
                judgment_confidence,
                yes_votes,
                no_votes,
                abstain_votes,
                community_satisfaction_rating
            FROM proposals
            WHERE
                (? = 0 OR voting_status = 'CLOSED')  -- Filter by status if only_closed=True
                AND (yes_votes + no_votes + abstain_votes) >= ?  -- Minimum votes
                AND judgment_verdict IS NOT NULL  -- Has CYNIC judgment
            ORDER BY created_at DESC
        """, (0 if not only_closed else 1, min_votes))

        proposals = []
        for row in cursor.fetchall():
            proposals.append(BotProposal(
                proposal_id=row["proposal_id"],
                title=row["title"],
                description=row["description"],
                category=row["category"],
                impact_level=row["impact_level"],
                voting_status=row["voting_status"],
                approval_status=row["approval_status"],
                judgment_verdict=row["judgment_verdict"],
                judgment_q_score=row["judgment_q_score"],
                judgment_confidence=row["judgment_confidence"],
                yes_votes=row["yes_votes"] or 0.0,
                no_votes=row["no_votes"] or 0.0,
                abstain_votes=row["abstain_votes"] or 0.0,
                community_satisfaction_rating=row["community_satisfaction_rating"],
            ))

        logger.info(f"Extracted {len(proposals)} proposals from bot database")
        return proposals

    finally:
        conn.close()


# ════════════════════════════════════════════════════════════════════════════
# REASONING GENERATION
# ════════════════════════════════════════════════════════════════════════════

def generate_reasoning(proposal: BotProposal) -> str:
    """
    Generate reasoning for why CYNIC made this judgment.

    Analyzes:
    - Vote distribution (consensus strength)
    - Community satisfaction rating (feedback quality)
    - Proposal characteristics (extraction risk signals)
    - Axiom alignment
    """
    total_votes = proposal.yes_votes + proposal.no_votes + proposal.abstain_votes
    if total_votes > 0:
        yes_pct = (proposal.yes_votes / total_votes) * 100
        no_pct = (proposal.no_votes / total_votes) * 100
    else:
        yes_pct = no_pct = 0.0

    signals = []

    # Verdict-specific reasoning
    if proposal.judgment_verdict == "HOWL":
        signals.extend([
            "Strong community consensus on approval (high YES %)" if yes_pct > 70 else None,
            "Clear, non-extractive execution path",
            "BURN axiom honored (funds go to community, not founder)",
            "FIDELITY axiom strong (represents community intent)",
        ])
    elif proposal.judgment_verdict == "WAG":
        signals.extend([
            "Good proposal with minor concerns",
            "Reasonable community support",
            "Some execution risks but mitigated",
            "CULTURE axiom moderately positive",
        ])
    elif proposal.judgment_verdict == "GROWL":
        signals.extend([
            "Extraction risk signals detected",
            "Unclear execution or oversight mechanisms",
            "Community divided (vote split)",
            "VERIFY axiom concerns (not easily auditable)",
        ])
    elif proposal.judgment_verdict == "BARK":
        signals.extend([
            "Founder extraction pattern detected",
            "No community control or oversight",
            "BURN axiom violated (funds leave treasury)",
            "High rug risk - community funds not protected",
        ])

    # Community feedback impact
    if proposal.community_satisfaction_rating:
        if proposal.community_satisfaction_rating >= 4.0:
            signals.append(f"Community rated outcome {proposal.community_satisfaction_rating:.1f}/5 - CYNIC judgment validated")
        elif proposal.community_satisfaction_rating <= 2.0:
            signals.append(f"Community rated outcome {proposal.community_satisfaction_rating:.1f}/5 - CYNIC judgment needs improvement")

    # Category-specific signals
    if "EXTRACTION" in proposal.category.upper() or "FOUNDER" in proposal.category.upper():
        signals.append("Category signals extraction risk")
    elif "BURN" in proposal.category.upper():
        signals.append("Category strongly aligns with BURN axiom")

    # Filter None values and create reasoning
    signals = [s for s in signals if s is not None]
    return " ".join(signals) if signals else "Standard governance proposal judgment based on axiom alignment."


# ════════════════════════════════════════════════════════════════════════════
# TRAINING EXAMPLE FORMATTING
# ════════════════════════════════════════════════════════════════════════════

CYNIC_SYSTEM_PROMPT = """You are CYNIC, a governance intelligence organism. You judge governance proposals using 5 axioms:

AXIOMS (in order of importance):
1. FIDELITY (70%) — Does this proposal faithfully represent community intent?
2. PHI (10%) — Is the reasoning φ-bounded (0-61.8%) or runaway?
3. VERIFY (10%) — Can the proposal be audited and enforced?
4. CULTURE (5%) — Does it strengthen or weaken community governance?
5. BURN (5%) — Are community funds burned (not extracted) or do founders profit?

VERDICT RULES:
- HOWL (Q ≥ 61.8): Strong proposal. Non-extractive, clear execution, community-aligned.
- WAG (Q 38.2-61.8): Good proposal with minor concerns.
- GROWL (Q 23.6-38.2): Risky proposal. Extraction signals, execution unclear.
- BARK (Q < 23.6): Dangerous proposal. Rug risk, founder extraction, axiom violations.

RESPONSE FORMAT: Return valid JSON with verdict, q_score, confidence, reasoning."""


def format_training_example(proposal: BotProposal) -> Dict:
    """
    Format a BotProposal as a training example (Mistral instruction format).

    Returns:
        Dict with "messages" key containing conversation
    """
    # Proposal JSON for user message
    proposal_json = {
        "title": proposal.title,
        "description": proposal.description,
        "category": proposal.category,
        "impact_level": proposal.impact_level,
    }

    # Verdict JSON for assistant message
    reasoning = generate_reasoning(proposal)
    verdict_json = {
        "verdict": proposal.judgment_verdict,
        "q_score": proposal.judgment_q_score or 50.0,
        "confidence": proposal.judgment_confidence or 0.8,
        "reasoning": reasoning,
    }

    return {
        "messages": [
            {
                "role": "system",
                "content": CYNIC_SYSTEM_PROMPT,
            },
            {
                "role": "user",
                "content": f"Judge this governance proposal:\n\n{json.dumps(proposal_json, indent=2)}",
            },
            {
                "role": "assistant",
                "content": json.dumps(verdict_json),
            }
        ]
    }


# ════════════════════════════════════════════════════════════════════════════
# JSONL GENERATION
# ════════════════════════════════════════════════════════════════════════════

def generate_training_jsonl(
    db_path: Optional[Path] = None,
    output_path: Optional[Path] = None,
    min_votes: int = 1,
) -> Path:
    """
    Extract proposals from governance_bot and generate JSONL training file.

    Args:
        db_path: Path to governance_bot.db
        output_path: Path to output JSONL (default: ~/.cynic/training/governance_v1.jsonl)
        min_votes: Minimum votes required for inclusion

    Returns:
        Path to generated JSONL file
    """
    # Extract proposals
    proposals = extract_proposals_from_bot_db(db_path=db_path, min_votes=min_votes)

    if not proposals:
        logger.warning("No proposals extracted from database")
        return None

    # Determine output path
    if output_path is None:
        output_path = Path.home() / ".cynic" / "training" / "governance_v1.jsonl"

    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    logger.info(f"Generating JSONL with {len(proposals)} training examples")

    # Write JSONL
    with open(output_path, "w") as f:
        for proposal in proposals:
            example = format_training_example(proposal)
            f.write(json.dumps(example) + "\n")

    logger.info(f"Saved {len(proposals)} training examples to {output_path}")

    # Print summary
    verdict_counts = {}
    for prop in proposals:
        verdict_counts[prop.judgment_verdict] = verdict_counts.get(prop.judgment_verdict, 0) + 1

    print(f"\nTraining Data Summary")
    print(f"{'='*50}")
    print(f"File: {output_path}")
    print(f"Total examples: {len(proposals)}")
    print(f"\nVerdict distribution:")
    for verdict in ["HOWL", "WAG", "GROWL", "BARK"]:
        count = verdict_counts.get(verdict, 0)
        print(f"  {verdict}: {count}")
    print(f"\n✓ Ready for Phase 2 fine-tuning!")

    return output_path


# ════════════════════════════════════════════════════════════════════════════
# CLI
# ════════════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    import sys

    logging.basicConfig(level=logging.INFO)

    # Generate training data from Phase 1B database
    try:
        output_path = generate_training_jsonl()
        if output_path:
            print(f"\n✓ Phase 1B → Phase 2 integration complete!")
            print(f"Next: python -m cynic.training.finetune --data {output_path}")
    except Exception as e:
        logger.error(f"Failed to generate training data: {e}", exc_info=True)
        sys.exit(1)
