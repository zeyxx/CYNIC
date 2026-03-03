"""
CYNIC Scientist Falsification Script - Empirical Stress Test.
Attempts to break the new Optic, MCTS and Surgery components.
"""
import asyncio
import logging
import os
import shutil
from pathlib import Path
from cynic.kernel.organism.brain.cognition.cortex.code_optic import CodebaseOptic
from cynic.kernel.organism.brain.cognition.cortex.mcts_scientist import ScientificMCTS, Hypothesis
from cynic.kernel.organism.brain.cognition.cortex.surgery import AutoSurgeon

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("falsify")

async def test_optic_robustness():
    logger.info("🧪 [H1] Falsifying Optic...")
    # Create a "poison" file with non-ascii content
    poison_path = Path("cynic/poison_test.py")
    with open(poison_path, "wb") as f:
        f.write(b"\xff\xfe\x00\x00massive_binary_garbage")
    
    optic = CodebaseOptic()
    try:
        code_map = optic.scan_codebase()
        # The key in code_map is relative to root, e.g. "cynic/poison_test.py"
        key = str(poison_path)
        if os.name == 'nt':
            key = key.replace("\\", "/")
            
        if key not in code_map:
            logger.error(f"Optic failed to find {key}. Discovered files: {list(code_map.keys())[:5]}...")
            assert key in code_map
        logger.info("✅ Optic survived binary injection (ASCII fallback worked).")
    finally:
        if poison_path.exists():
            os.remove(poison_path)

async def test_mcts_balance():
    logger.info("🧪 [H2] Falsifying MCTS Selection...")
    mcts = ScientificMCTS(Hypothesis("ROOT", "Base", "perf", "increase"))
    
    # Add 3 competing hypotheses
    mcts.add_hypothesis("ROOT", Hypothesis("H1", "Opt A", "perf", "increase"))
    mcts.add_hypothesis("ROOT", Hypothesis("H2", "Opt B", "perf", "increase"))
    mcts.add_hypothesis("ROOT", Hypothesis("H3", "Opt C", "perf", "increase"))
    
    selections = []
    for _ in range(10):
        node = mcts.select_next_experiment()
        selections.append(node.id)
        # Simulate quick success to force UCT to re-evaluate
        mcts.backpropagate(node.id, 0.8, "SUCCESS")
    
    # If MCTS is working, it should have visited all 3 at least once due to PHI-exploration
    unique_selections = set(selections)
    if len(unique_selections) >= 3:
        logger.info(f"✅ MCTS balanced: {unique_selections}")
    else:
        logger.error(f"❌ MCTS Biased: {selections}")

async def test_surgery_cleanup():
    logger.info("🧪 [H3] Falsifying Surgery Leakage...")
    surgeon = AutoSurgeon()
    exp_id = "falsify_leak"
    
    try:
        # 1. Create a fake git failure by messing with the path
        # But AutoSurgeon uses real subprocesses, let's try to trigger a real git error
        # by providing an invalid experiment ID (too long or invalid chars)
        path = surgeon.prepare_sandbox("!!!!!invalid!!!!!")
    except Exception as e:
        logger.info(f"✅ Captured expected Git error: {e}")
    
    # Check if .worktrees directory is clean
    worktree_dir = Path(".worktrees")
    leftovers = list(worktree_dir.glob("*falsify*"))
    if not leftovers:
        logger.info("✅ Surgery left no orphaned directories.")
    else:
        logger.error(f"❌ Surgery Leaked: {leftovers}")

async def run_falsification():
    await test_optic_robustness()
    await test_mcts_balance()
    await test_surgery_cleanup()

if __name__ == "__main__":
    asyncio.run(run_falsification())
