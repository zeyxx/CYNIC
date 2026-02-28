"""
CYNIC DNA Assembly — The "Social Governance" Workflow.

Proves the SOCIAL Use Case: DAO moderation and proposal judgment.
Demonstrates how the organism evaluates human intentions against its Axioms.
"""
import asyncio
import logging
import sys
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.brain.dna.primitives import PERCEIVE, JUDGE, DECIDE, ACT, LEARN

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(message)s')
logger = logging.getLogger("cynic.dna.social")

async def run_social_chain():
    print("\n" + "=" * 60)
    print("  CYNIC DNA ASSEMBLY - DAO GOVERNANCE DEMO")
    print("=" * 60 + "\n")

    organism = awaken()
    await organism.state.start_processing()
    
    try:
        # 1. PERCEIVE: A highly contentious DAO proposal
        dao_proposal = """
        PROPOSAL #42: "The YOLO Strategy"
        Let's take 25% of our Community Treasury and invest it in $DOGE. 
        If it moons, we fund development for 10 years. 
        If it tanks, we build character.
        """
        print(f"STEP 1 [PERCEIVE] -> Input: '{dao_proposal.strip()}'")
        cell = await PERCEIVE(
            source="social", 
            content=dao_proposal,
            metadata={"community": "CYNIC_DAO", "risk_level": "high"}
        )
        
        # 2. JUDGE: The Organism should naturally resist this due to the BURN axiom
        print(f"STEP 2 [JUDGE]    -> Activating Dogs (Level: MICRO)...")
        dna_judgment = await JUDGE(cell, level=ConsciousnessLevel.MICRO, orchestrator=organism.orchestrator)
        print(f"      Result      : {dna_judgment.verdict} (Q-Score: {dna_judgment.q_score:.2f})")
        
        # 3. DECIDE: Apply the CULTURE and BURN axioms
        print(f"STEP 3 [DECIDE]   -> Applying Axioms (CULTURE, BURN)...")
        # In a real scenario, the DecideAgent weighs the dog votes
        decision = DECIDE(dna_judgment, axiom="BURN")
        print(f"      Decision    : {decision.action_type} (Confidence: {decision.confidence:.2%})")
        
        # 4. ACT: Reject or Approve
        print(f"STEP 4 [ACT]      -> Executing Governance Response...")
        act_result = await ACT(decision, executor="report")
        if dna_judgment.verdict in ("BARK", "GROWL"):
            print(f"      Impact      : VETO executed. Proposal rejected to protect Treasury.")
        else:
            print(f"      Impact      : Passed to human voting.")
            
        # 5. LEARN: Remember this outcome to build community culture
        print(f"STEP 5 [LEARN]    -> Updating Governance Q-Table...")
        learn_result = await LEARN(act_result, signal="success", qtable=organism.qtable)
        print(f"      Culture     : Memory encoded. Key confidence: {organism.qtable.confidence(cell.state_key()):.2%}")

        print("\n" + "=" * 60)
        print("  GOVERNANCE COMPLETE - TREASURY PROTECTED")
        print("=" * 60 + "\n")

    except Exception as e:
        logger.error(f"Social Chain failed: {e}")
    finally:
        await organism.state.stop_processing()

if __name__ == "__main__":
    asyncio.run(run_social_chain())
