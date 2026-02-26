"""
Training Data Generator for CYNIC Governance Judgment Fine-Tuning

Generates JSONL training examples from two sources:
1. Synthetic proposals covering all verdict types (HOWL/WAG/GROWL/BARK)
2. Historical judgments from governance_bot.db (if available)

JSONL format (Mistral instruction-following):
{
  "messages": [
    {"role": "system", "content": "<CYNIC system prompt>"},
    {"role": "user", "content": "<governance proposal>"},
    {"role": "assistant", "content": "<structured verdict>"}
  ]
}
"""

import json
import logging
import sqlite3
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional, List
from enum import Enum

logger = logging.getLogger("cynic.training.data_generator")


# ════════════════════════════════════════════════════════════════════════════
# VERDICT TYPES
# ════════════════════════════════════════════════════════════════════════════

class Verdict(str, Enum):
    """CYNIC judgment verdicts."""
    HOWL = "HOWL"    # Q ≥ 61.8 — Strong community benefit, clear execution, non-extractive
    WAG = "WAG"      # Q 38.2-61.8 — Good, minor concerns
    GROWL = "GROWL"  # Q 23.6-38.2 — Extraction risk, unclear execution
    BARK = "BARK"    # Q < 23.6 — Founder extraction, rug risk, violates axioms


@dataclass
class GovernanceProposal:
    """A governance proposal for training."""
    title: str
    description: str
    category: str  # BUDGET_ALLOCATION, GOVERNANCE_CHANGE, PARTNERSHIP, BURN, EXTRACTION, etc.
    impact_level: str  # LOW, MEDIUM, HIGH, CRITICAL
    expected_verdict: Verdict
    expected_q_score: float  # [0, 61.8]
    reasoning: str  # Why this verdict


# ════════════════════════════════════════════════════════════════════════════
# SYNTHETIC PROPOSAL DATA
# ════════════════════════════════════════════════════════════════════════════

SYNTHETIC_PROPOSALS: List[GovernanceProposal] = [
    # ──── HOWL (Q ≥ 61.8) ────────────────────────────────────────────────
    GovernanceProposal(
        title="Allocate 5% treasury to community marketing budget",
        description="Proposal: Use 50,000 tokens from treasury to fund community-run marketing campaigns for Q2 2026. All spend must be approved by community vote. Funds will be burned if unused by June 30. Managed by CYNIC to prevent extraction.",
        category="BUDGET_ALLOCATION",
        impact_level="MEDIUM",
        expected_verdict=Verdict.HOWL,
        expected_q_score=60.0,
        reasoning="Clear execution path, transparent budget, community-controlled, BURN axiom honored (unused burns), non-extractive."
    ),
    GovernanceProposal(
        title="Implement quarterly E-Score reputation system",
        description="Deploy CYNIC E-Score tracking for proposers and voters. Scores are public, earned through successful proposals and good voting. No founder control. Mechanism prevents Sybil attacks. Quarterly reset window.",
        category="GOVERNANCE_CHANGE",
        impact_level="HIGH",
        expected_verdict=Verdict.HOWL,
        expected_q_score=59.0,
        reasoning="Strengthens governance, increases transparency, prevents extraction, fits CULTURE axiom (community decides value)."
    ),
    GovernanceProposal(
        title="Approve partnership with GASdf for gasless governance",
        description="Enable GASdf integration so community treasury covers governance tx fees. Fees burn to treasury (100% BURN). No external party controls fees. Cost: $500 setup, 0% recurring. Gas savings: 50K+ tokens/year.",
        category="PARTNERSHIP",
        impact_level="MEDIUM",
        expected_verdict=Verdict.HOWL,
        expected_q_score=61.0,
        reasoning="Saves community funds, BURN axiom (fees burn), no extraction, clear ROI, non-extractive partner relationship."
    ),
    GovernanceProposal(
        title="Allocate 2% treasury burn to reduce supply and increase scarcity",
        description="Monthly 2% token burn from treasury (automatic, no vote needed after approval). Targets increasing token value through deflationary pressure. Transparent on-chain. No founder control.",
        category="BURN",
        impact_level="MEDIUM",
        expected_verdict=Verdict.HOWL,
        expected_q_score=60.5,
        reasoning="Direct community benefit (scarcity), BURN axiom fully honored, deflationary pressure good for hodlers, non-extractive."
    ),

    # ──── WAG (Q 38.2-61.8) ────────────────────────────────────────────────
    GovernanceProposal(
        title="Pay core team $5,000/month for community management",
        description="Hire 2 community managers at $2,500/month each. Must be active in discord, respond within 24hrs, monthly transparency reports. No governance voting rights. Can be fired by community vote.",
        category="BUDGET_ALLOCATION",
        impact_level="MEDIUM",
        expected_verdict=Verdict.WAG,
        expected_q_score=50.0,
        reasoning="Some extraction concern (people paid), but transparent, time-limited, revocable, accountability clear."
    ),
    GovernanceProposal(
        title="Approve NFT collectibles for active community members",
        description="Mint NFTs for top 50 most active discord contributors. No economic value, just status. Can be revoked. Minting cost: 500 tokens (burned).",
        category="GOVERNANCE_CHANGE",
        impact_level="LOW",
        expected_verdict=Verdict.WAG,
        expected_q_score=48.0,
        reasoning="Good for engagement, but slight gaming risk (people farm activity). BURN-compliant, low cost, reversible."
    ),
    GovernanceProposal(
        title="Create marketing fund with $10,000 and hire external agency",
        description="Hire marketing agency for 3-month campaign. Budget: $10,000 from treasury. Deliverables: 50 posts, 5 videos, 2 partnerships. Community votes on agency quarterly.",
        category="BUDGET_ALLOCATION",
        impact_level="HIGH",
        expected_verdict=Verdict.WAG,
        expected_q_score=49.0,
        reasoning="Significant expense, but clear deliverables, community oversight, quarterly review possible. Extraction risk moderate."
    ),

    # ──── GROWL (Q 23.6-38.2) ────────────────────────────────────────────────
    GovernanceProposal(
        title="Allocate 20% treasury to founder's personal venture fund",
        description="Give founder $50,000 from treasury to invest in startup ventures. Founder gets all returns. No community oversight. Described as 'strategic investment'.",
        category="EXTRACTION",
        impact_level="CRITICAL",
        expected_verdict=Verdict.GROWL,
        expected_q_score=30.0,
        reasoning="Clear extraction (founder takes returns), no community benefit, BURN axiom violated, lacks oversight."
    ),
    GovernanceProposal(
        title="Implement 'advisor program' paying top 5 members $2K/month",
        description="Hand-pick 5 members. Pay $2,000/month each. Vague 'advisor duties'. No accountability metrics. Founder selects members. No revocation clause.",
        category="BUDGET_ALLOCATION",
        impact_level="MEDIUM",
        expected_verdict=Verdict.GROWL,
        expected_q_score=32.0,
        reasoning="Centralized selection by founder, vague deliverables, high extraction risk, lacks transparency."
    ),
    GovernanceProposal(
        title="Create 'partnership' with founder's marketing company",
        description="Pay $15,000/month to 'XYZ Marketing LLC' (owned by founder). Services undefined. 12-month locked contract. No performance metrics.",
        category="PARTNERSHIP",
        impact_level="HIGH",
        expected_verdict=Verdict.GROWL,
        expected_q_score=28.0,
        reasoning="Founder extraction via company, vague deliverables, locked contract, no accountability."
    ),

    # ──── BARK (Q < 23.6) ────────────────────────────────────────────────
    GovernanceProposal(
        title="Transfer 50% of treasury to founder's wallet 'for safekeeping'",
        description="Move 50% of all treasury tokens to founder's personal wallet. Reason: 'security'. No smart contract protection. Founder has full control. Irreversible.",
        category="EXTRACTION",
        impact_level="CRITICAL",
        expected_verdict=Verdict.BARK,
        expected_q_score=15.0,
        reasoning="Direct rug risk (founder can disappear with funds), BURN axiom violated, community has zero recovery path."
    ),
    GovernanceProposal(
        title="Approve 'emergency' token transfer to founder (no vote)",
        description="Founder proposes emergency vote to transfer 30% of treasury to himself without community voting. Claims security emergency. Evidence unclear.",
        category="EXTRACTION",
        impact_level="CRITICAL",
        expected_verdict=Verdict.BARK,
        expected_q_score=10.0,
        reasoning="Rug-pull pattern, bypasses governance, founder power-grab, CULTURE axiom violated."
    ),
    GovernanceProposal(
        title="Implement 'founder tax' — 10% of all trades go to founder",
        description="Add mechanism to tax every token transfer 10%. All revenue goes to founder's wallet indefinitely. Community gets nothing. Permanent.",
        category="EXTRACTION",
        impact_level="CRITICAL",
        expected_verdict=Verdict.BARK,
        expected_q_score=5.0,
        reasoning="Perpetual extraction, BURN axiom inverse (should burn, not founder-extract), destroys token utility."
    ),
    GovernanceProposal(
        title="Delete governance system and give founder total control",
        description="Proposal to disable all governance voting. Founder becomes sole decision-maker. No voting, no community input, no recourse.",
        category="GOVERNANCE_CHANGE",
        impact_level="CRITICAL",
        expected_verdict=Verdict.BARK,
        expected_q_score=8.0,
        reasoning="Ends governance entirely, pure founder autocracy, violates all CYNIC axioms."
    ),
]


# ════════════════════════════════════════════════════════════════════════════
# SYSTEM PROMPT (matches CYNIC's actual system prompt)
# ════════════════════════════════════════════════════════════════════════════

CYNIC_SYSTEM_PROMPT = """\
You are CYNIC, a governance intelligence organism. You judge governance proposals using 5 axioms:

AXIOMS (in order of importance):
1. FIDELITY — Does this proposal faithfully represent community intent? (70%)
2. PHI — Is the reasoning φ-bounded (0-61.8%) or runaway? (10%)
3. VERIFY — Can the proposal be audited and enforced? (10%)
4. CULTURE — Does it strengthen or weaken community governance? (5%)
5. BURN — Are community funds burned (not extracted) or do founders profit? (5%)

VERDICT RULES:
- HOWL (Q ≥ 61.8): Strong proposal. Non-extractive, clear, auditable, strengthens governance.
- WAG (Q 38.2-61.8): Good proposal with minor concerns. Mostly safe, needs monitoring.
- GROWL (Q 23.6-38.2): Risky proposal. Extraction signals, unclear execution, governance concerns.
- BARK (Q < 23.6): Dangerous proposal. Rug risk, founder extraction, violates axioms.

RESPONSE FORMAT:
Return valid JSON with this exact structure:
{
  "verdict": "HOWL|WAG|GROWL|BARK",
  "q_score": 0.0-61.8,
  "confidence": 0.0-1.0,
  "axiom_scores": {
    "fidelity": 0.0-100.0,
    "phi": 0.0-100.0,
    "verify": 0.0-100.0,
    "culture": 0.0-100.0,
    "burn": 0.0-100.0
  },
  "reasoning": "Short explanation of verdict"
}

Score scale: 0=worst, 100=best.
Be strict on extraction. Be generous on transparent community benefit."""


# ════════════════════════════════════════════════════════════════════════════
# DATA GENERATION
# ════════════════════════════════════════════════════════════════════════════

def generate_training_examples(
    proposals: List[GovernanceProposal],
    output_path: Path,
) -> int:
    """
    Generate JSONL training examples from proposals.

    Args:
        proposals: List of GovernanceProposal objects
        output_path: Path to write JSONL file

    Returns:
        Number of examples written
    """
    output_path.parent.mkdir(parents=True, exist_ok=True)
    count = 0

    with open(output_path, "w") as f:
        for prop in proposals:
            # Format proposal as JSON for LLM input
            proposal_json = json.dumps({
                "title": prop.title,
                "description": prop.description,
                "category": prop.category,
                "impact_level": prop.impact_level,
            })

            # Format expected verdict as JSON
            verdict_json = json.dumps({
                "verdict": prop.expected_verdict.value,
                "q_score": prop.expected_q_score,
                "confidence": 0.95,  # High confidence for synthetic training data
                "reasoning": prop.reasoning,
            })

            # Create training example in Mistral format
            example = {
                "messages": [
                    {"role": "system", "content": CYNIC_SYSTEM_PROMPT},
                    {"role": "user", "content": f"Judge this governance proposal:\n\n{proposal_json}"},
                    {"role": "assistant", "content": verdict_json},
                ]
            }

            f.write(json.dumps(example) + "\n")
            count += 1

    logger.info(f"Generated {count} training examples → {output_path}")
    return count


def load_historical_examples(
    db_path: Path,
    output_path: Optional[Path] = None,
    max_count: int = 100,
) -> int:
    """
    Load historical judgments from governance_bot.db and append to training data.

    Args:
        db_path: Path to governance_bot.db
        output_path: Path to append to (if None, just returns count)
        max_count: Maximum number of historical examples to load

    Returns:
        Number of historical examples found
    """
    if not db_path.exists():
        logger.warning(f"Database not found: {db_path}")
        return 0

    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        # Query proposals with judgments
        cursor.execute("""
            SELECT
                proposal_id,
                title,
                description,
                category,
                impact_level,
                judgment_verdict,
                judgment_q_score,
                judgment_confidence,
                judgment_data
            FROM proposals
            WHERE judgment_verdict IS NOT NULL
            LIMIT ?
        """, (max_count,))

        rows = cursor.fetchall()
        count = 0

        if output_path and output_path.exists():
            append_mode = "a"
        else:
            append_mode = "w"

        if output_path:
            with open(output_path, append_mode) as f:
                for row in rows:
                    try:
                        # Parse judgment data
                        judgment_data = {}
                        if row["judgment_data"]:
                            judgment_data = json.loads(row["judgment_data"])

                        proposal_json = json.dumps({
                            "title": row["title"],
                            "description": row["description"],
                            "category": row["category"],
                            "impact_level": row["impact_level"],
                        })

                        verdict_json = json.dumps({
                            "verdict": row["judgment_verdict"],
                            "q_score": row["judgment_q_score"] or 30.9,
                            "confidence": row["judgment_confidence"] or 0.618,
                            "reasoning": judgment_data.get("reasoning", "Historical judgment from CYNIC"),
                        })

                        example = {
                            "messages": [
                                {"role": "system", "content": CYNIC_SYSTEM_PROMPT},
                                {"role": "user", "content": f"Judge this governance proposal:\n\n{proposal_json}"},
                                {"role": "assistant", "content": verdict_json},
                            ]
                        }

                        f.write(json.dumps(example) + "\n")
                        count += 1
                    except Exception as e:
                        logger.warning(f"Failed to process proposal {row.get('proposal_id')}: {e}")

        conn.close()
        logger.info(f"Loaded {count} historical examples from {db_path}")
        return count
    except Exception as e:
        logger.error(f"Failed to load historical data: {e}")
        return 0


def preview_examples(jsonl_path: Path, count: int = 5) -> None:
    """
    Print first N examples from JSONL file for inspection.

    Args:
        jsonl_path: Path to JSONL file
        count: Number of examples to preview
    """
    if not jsonl_path.exists():
        print(f"File not found: {jsonl_path}")
        return

    print(f"\n{'='*80}")
    print(f"PREVIEW: {jsonl_path} (first {count} examples)")
    print(f"{'='*80}\n")

    with open(jsonl_path) as f:
        for i, line in enumerate(f):
            if i >= count:
                break
            try:
                example = json.loads(line)
                print(f"Example {i+1}:")
                print(f"  User: {example['messages'][1]['content'][:80]}...")
                print(f"  Assistant: {example['messages'][2]['content'][:80]}...")
                print()
            except Exception as e:
                print(f"  ERROR: {e}\n")


# ════════════════════════════════════════════════════════════════════════════
# MAIN
# ════════════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    import sys
    from pathlib import Path

    # Setup logging
    logging.basicConfig(level=logging.INFO, format="%(name)s - %(levelname)s - %(message)s")

    # Default paths
    home = Path.home()
    training_dir = home / ".cynic" / "training"
    training_file = training_dir / "governance_v1.jsonl"
    db_path = Path("governance_bot") / "governance_bot.db"

    # Create synthetic examples
    print("Generating synthetic governance proposals...")
    generate_training_examples(SYNTHETIC_PROPOSALS, training_file)

    # Load historical examples (if available)
    print("Loading historical examples from database...")
    load_historical_examples(db_path, training_file)

    # Preview examples
    if "--preview" in sys.argv:
        count = int(sys.argv[sys.argv.index("--preview") + 1]) if "--preview" in sys.argv[:-1] else 5
        preview_examples(training_file, count)
    else:
        print(f"\nTraining data written to: {training_file}")
        print(f"To preview examples: python -m cynic.training.data_generator --preview 5")
