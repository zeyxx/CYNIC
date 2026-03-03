"""
CYNIC DNA Assembly â€” The "Social Governance" Workflow.

Proves the SOCIAL Use Case: DAO moderation and proposal judgment.
Demonstrates how the organism evaluates human intentions against its Axioms.
"""
import asyncio
import logging
import sys
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.organism.brain.dna.primitives import ACT, DECIDE, JUDGE, LEARN, PERCEIVE
from cynic.kernel.organism.organism import awaken

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(message)s')
logger = logging.getLogger("cynic.dna.social")

async def run_social_chain():

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
        cell = await PERCEIVE(
            source="social", 
            content=dao_proposal,
            metadata={"community": "CYNIC_DAO", "risk_level": "high"}
        )
        
        # 2. JUDGE: The Organism should naturally resist this due to the BURN axiom
        dna_judgment = await JUDGE(cell, level=ConsciousnessLevel.MICRO, orchestrator=organism.orchestrator)
        
        # 3. DECIDE: Apply the CULTURE and BURN axioms
        # In a real scenario, the DecideAgent weighs the dog votes
        decision = DECIDE(dna_judgment, axiom="BURN")
        
        # 4. ACT: Reject or Approve
        act_result = await ACT(decision, executor="report")
        if dna_judgment.verdict in ("BARK", "GROWL"):
            pass
        else:
            pass
            
        # 5. LEARN: Remember this outcome to build community culture
        await LEARN(act_result, signal="success", qtable=organism.qtable)


    except Exception as e:
            logger.error(f"Social Chain failed: {e}")
    finally:
        await organism.state.stop_processing()

if __name__ == "__main__":
    asyncio.run(run_social_chain())
