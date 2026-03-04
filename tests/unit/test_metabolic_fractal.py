
import unittest
import asyncio
import os
from cynic.kernel.organism.orchestrator import MetabolicOrchestrator
from cynic.kernel.organism.experience import ExperienceVault

class TestMetabolicFractal(unittest.TestCase):
    def setUp(self):
        self.test_path = "audit/test_fractal_weights.json"
        if os.path.exists(self.test_path):
            os.remove(self.test_path)
        # Initialize orchestrator with fresh vault
        self.orch = MetabolicOrchestrator()
        self.orch.vault = ExperienceVault(storage_path=self.test_path)

    def tearDown(self):
        if os.path.exists(self.test_path):
            os.remove(self.test_path)

    def test_full_fractal_cycle(self):
        """
        Tests the complete loop: Select -> Allocate -> Execute -> Learn.
        """
        async def dummy_repair(model_id: str):
            print(f"DEBUG: Repairing code with {model_id}...")
            await asyncio.sleep(0.1)
            return "SUCCESS: Code repaired."

        async def run_test():
            # First execution: should use default or first available
            candidates = ["qwen-coder", "deepseek-r1"]
            await self.orch.execute_with_learning(
                axiom="BACKEND", 
                candidates=candidates, 
                action_func=dummy_repair
            )
            
            # Record a failure for the winner to see if it changes next time
            winner = self.orch.vault.get_best_dog_for("BACKEND", candidates)
            print(f"DEBUG: Current best dog: {winner}")
            
            # Inject a major failure for the winner
            self.orch.vault.record_experience(winner, "BACKEND", success=False, latency_ms=5000)
            
            # Second execution: should have learned and might choose another
            new_winner = self.orch.vault.get_best_dog_for("BACKEND", candidates)
            print(f"DEBUG: New best dog after learning: {new_winner}")
            
            return winner, new_winner

        winner, new_winner = asyncio.run(run_test())
        self.assertNotEqual(winner, new_winner, "Orchestrator should have learned to switch after failure.")
        print("✅ Metabolic Fractal Cycle: Fully Validated.")

if __name__ == "__main__":
    unittest.main()
