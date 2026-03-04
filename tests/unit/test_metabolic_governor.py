
import unittest
import asyncio
from unittest.mock import AsyncMock, MagicMock
from cynic.kernel.organism.metabolism.governor import MetabolicGovernor, ResourceProvider

class MockProvider(ResourceProvider):
    def __init__(self):
        self.loaded_models = ["old-model-1", "old-model-2"]
        self.unloaded = []

    async def get_loaded_models(self):
        return self.loaded_models

    async def unload_model(self, model_id):
        self.unloaded.append(model_id)
        return True

class TestMetabolicGovernor(unittest.TestCase):
    def setUp(self):
        self.provider = MockProvider()
        self.gov = MetabolicGovernor(provider=self.provider)

    def test_eviction_on_stress(self):
        # Force stress
        self.gov.vram_threshold = 0.1  # Threshold 0.1% means always stressed
        
        async def run_test():
            await self.gov.allocate("new-model")
            return self.provider.unloaded

        unloaded = asyncio.run(run_test())
        print(f"DEBUG: Unloaded models: {unloaded}")
        self.assertIn("old-model-1", unloaded)
        self.assertIn("old-model-2", unloaded)
        print("✅ MetabolicGovernor: Resource Appropriation Validated.")

if __name__ == "__main__":
    unittest.main()
