"""
CYNIC Cognitive Resilience Test — Proving Survival in Finite Hardware.

Simulates hardware stress and verifies the Law of Sovereignty:
1. Optimal: Ryzen (llama_cpp) is used.
2. RAM Stress: System switches to lighter local service (Ollama).
3. Critical: System falls back to local CLI or Cloud.
"""
import pytest
import asyncio
from unittest.mock import MagicMock, patch
from cynic.kernel.organism.brain.llm.adapter import LLMRegistry, LLMAdapter
from cynic.kernel.core.consciousness import ConsciousnessLevel

class MockAdapter(LLMAdapter):
    async def complete(self, req): return MagicMock()
    async def check_available(self): return True

@pytest.mark.asyncio
async def test_sovereignty_fallback_logic():
    registry = LLMRegistry()
    
    # Register different "muscles"
    llama = MockAdapter("llama3", "llama_cpp")
    ollama = MockAdapter("mistral", "ollama")
    claude_cli = MockAdapter("claude-code", "claude_cli")
    claude_cloud = MockAdapter("sonnet", "claude")
    
    registry.register(llama)
    registry.register(ollama)
    registry.register(claude_cli)
    registry.register(claude_cloud)
    
    # 1. TEST: SOVEREIGNTY PRIORITY
    # With all available, llama_cpp (pure hardware) must win.
    best = registry.get_best_for("SAGE", "judgment")
    assert best.provider == "llama_cpp"
    
    # 2. TEST: FALLBACK TO LOCAL SERVICE
    # Disable llama_cpp (simulate hardware failure or lack of fit)
    registry._available[llama.adapter_id] = False
    best = registry.get_best_for("SAGE", "judgment")
    assert best.provider == "ollama"
    
    # 3. TEST: FALLBACK TO LOCAL CLI
    # Disable ollama
    registry._available[ollama.adapter_id] = False
    best = registry.get_best_for("SAGE", "judgment")
    assert best.provider == "claude_cli"
    
    # 4. TEST: LAST RESORT CLOUD
    # Disable CLI
    registry._available[claude_cli.adapter_id] = False
    best = registry.get_best_for("SAGE", "judgment")
    assert best.provider == "claude"

@pytest.mark.asyncio
async def test_hardware_aware_discovery():
    registry = LLMRegistry()
    
    # Mock profiler to simulate low RAM (2GB available)
    mock_profiler = MagicMock()
    mock_profiler.profile.available_ram_gb = 2.0
    mock_profiler.announce_limits.return_value = "LOW RAM SIMULATION"
    
    # Mock list_local_models to return a "heavy" model (10GB)
    with patch("cynic.kernel.organism.brain.llm.llama_cpp.list_local_models", return_value=["heavy_model.gguf"]):
        with patch("os.path.getsize", return_value=10 * 1024**3):
            with patch("cynic.kernel.organism.metabolism.model_profiler.ModelProfiler", return_value=mock_profiler):
                manifest = await registry.discover(models_dir="/tmp/models")
                
                # The heavy model should be rejected due to RAM limits
                assert len(manifest["rejected"]) > 0
                assert manifest["rejected"][0]["reason"] == "RAM limit"
                assert "llama_cpp:heavy_model" not in [a.adapter_id for a in registry.get_available()]
