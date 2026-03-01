"""
CYNIC DNA Chain Demo â€” The "Security Gene" Workflow.

Demonstrates how atomic DNA primitives are composed into a functional
thinking chain using the V3.1 unified architecture.
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
logger = logging.getLogger("cynic.dna.demo")

async def run_dna_chain():

    # 0. AWAKEN THE HOST
    organism = awaken()
    await organism.state.start_processing()
    
    try:
        # --- THE CHAIN BEGINS ---
        
        # 1. PERCEIVE: Identify a potential threat in code
        suspect_code = "eval(req.query.cmd); // Potential RCE vulnerability"
        cell = await PERCEIVE(source="code", content=suspect_code)
        
        # 2. JUDGE: Run the 11 Dogs on this cell
        dna_judgment = await JUDGE(cell, level=ConsciousnessLevel.MICRO, orchestrator=organism.orchestrator)
        
        # 3. DECIDE: Choose an action based on the PHI Axiom
        decision = DECIDE(dna_judgment, axiom="PHI")
        
        # 4. ACT: Execute the metabolic response
        # For demo, we use the 'report' executor
        act_result = await ACT(decision, executor="report")
        
        # 5. LEARN: Consolidate into the long-term memory (Q-Table)
        await LEARN(act_result, signal="success", qtable=organism.qtable)


    except Exception as e:
        logger.error(f"DNA Chain failed: {e}")
    finally:
        await organism.state.stop_processing()

if __name__ == "__main__":
    asyncio.run(run_dna_chain())
