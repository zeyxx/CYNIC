
import pytest
import os
import json
from pathlib import Path
from cynic.kernel.organism.experience import ExperienceVault


@pytest.fixture
def experience_vault():
    """Fixture to create and cleanup test vault."""
    test_path = "audit/test_synaptic_weights.json"
    if os.path.exists(test_path):
        os.remove(test_path)
    vault = ExperienceVault(storage_path=test_path)
    yield vault
    if os.path.exists(test_path):
        os.remove(test_path)


@pytest.mark.asyncio
async def test_learning_and_persistence(experience_vault):
    """Test that ExperienceVault records experiences and persists them."""
    # 1. Simulate experiences
    # Model-A is reliable but slow
    await experience_vault.record_experience("model-a", "BACKEND", success=True, latency_ms=2000)
    await experience_vault.record_experience("model-a", "BACKEND", success=True, latency_ms=2000)

    # Model-B is fast but unreliable
    await experience_vault.record_experience("model-b", "BACKEND", success=False, latency_ms=100)

    # 2. Check selection
    best = experience_vault.get_best_dog_for("BACKEND", ["model-a", "model-b"])
    assert best == "model-a", "Model-A should be preferred despite latency because it succeeds."

    # 3. Verify Persistence
    new_vault = ExperienceVault(storage_path="audit/test_synaptic_weights.json")
    assert new_vault.weights["model-a"]["BACKEND"].success_count == 2
