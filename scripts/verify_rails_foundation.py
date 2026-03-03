"""
Falsify the Rails Foundation.
Tests if the AutoSurgeon actually blocks a heresy (except: pass).
"""
import asyncio
import logging
from cynic.kernel.organism.brain.cognition.cortex.surgery import AutoSurgeon

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("rails_test")

async def test_heresy_block():
    surgeon = AutoSurgeon()
    exp_id = "heresy_test"
    
    # 1. Create Sandbox
    sandbox_path = surgeon.prepare_sandbox(exp_id)
    
    # 2. Inject Heresy (except: pass)
    heresy_code = """
def bad_function():
    try:
        1/0
    except:
        pass
"""
    mutations = {"cynic/heresy_module.py": heresy_code}
    surgeon.apply_mutations(sandbox_path, mutations)
    
    # 3. Attempt Suture
    logger.info("🔥 Attempting to suture heresy...")
    success = surgeon.suture(exp_id, mutations=mutations)
    
    if not success:
        logger.info("✅ SUCCESS: Rails foundation REJECTED the heresy.")
    else:
        logger.error("❌ FAILURE: Rails foundation ALLOWED the heresy. Foundation is performative.")
    
    # 4. Cleanup
    surgeon.cleanup_sandbox(exp_id)

if __name__ == "__main__":
    asyncio.run(test_heresy_block())
