"""
CYNIC DNA Chain Demo — The "Security Gene" Workflow.

Demonstrates how atomic DNA primitives are composed into a functional
thinking chain using the V3.1 unified architecture.
"""
import asyncio
import logging
import sys
import os
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.brain.dna.primitives import PERCEIVE, JUDGE, DECIDE, ACT, LEARN
from cynic.kernel.core.phi import PHI

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(message)s')
logger = logging.getLogger("cynic.dna.demo")

async def run_dna_chain():
    print("\n" + "="*60)
    print("  CYNIC DNA ASSEMBLY — SECURITY CHAIN DEMO")
    print("="*60 + "\n")

    # 0. AWAKEN THE HOST
    organism = awaken()
    await organism.state.start_processing()
    
    try:
        # --- THE CHAIN BEGINS ---
        
        # 1. PERCEIVE: Identify a potential threat in code
        suspect_code = "eval(req.query.cmd); // Potential RCE vulnerability"
        print(f"STEP 1 [PERCEIVE] -> Input: '{suspect_code}'")
        cell = await PERCEIVE(source="code", content=suspect_code)
        
        # 2. JUDGE: Run the 11 Dogs on this cell
        print(f"STEP 2 [JUDGE]    -> Activating Dogs (Level: MICRO)...")
        dna_judgment = await JUDGE(cell, level=ConsciousnessLevel.MICRO, orchestrator=organism.orchestrator)
        print(f"      Result      : {dna_judgment.verdict} (Q-Score: {dna_judgment.q_score:.2f})")
        
        # 3. DECIDE: Choose an action based on the PHI Axiom
        print(f"STEP 3 [DECIDE]   -> Applying Axiom PHI...")
        decision = DECIDE(dna_judgment, axiom="PHI")
        print(f"      Decision    : {decision.action_type} (Confidence: {decision.confidence:.2%})")
        
        # 4. ACT: Execute the metabolic response
        print(f"STEP 4 [ACT]      -> Executing Metabolic Response...")
        # For demo, we use the 'report' executor
        act_result = await ACT(decision, executor="report")
        print(f"      Impact      : {act_result.output or 'Success'}")
        
        # 5. LEARN: Consolidate into the long-term memory (Q-Table)
        print(f"STEP 5 [LEARN]    -> Consolidating into Q-Table...")
        learn_result = await LEARN(act_result, signal="success", qtable=organism.qtable)
        print(f"      Consensus   : Q-Table updated. Key confidence now: {organism.qtable.confidence(cell.state_key()):.2%}")

        print("\n" + "="*60)
        print("  CHAIN COMPLETE — ORGANISM HAS EVOLVED")
        print("="*60 + "\n")

    except Exception as e:
        logger.error(f"DNA Chain failed: {e}")
    finally:
        await organism.state.stop_processing()

if __name__ == "__main__":
    asyncio.run(run_dna_chain())
