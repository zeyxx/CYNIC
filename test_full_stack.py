"""Test full stack: LLM Adapter + SageDog"""
import asyncio
from cynic.adapters import AnthropicAdapter, AdapterConfig
from cynic.dogs import SageDog
from cynic.kernel.types import Cell

async def main():
    # Setup with real adapter
    config = AdapterConfig(name='sage', model='claude-3-5-sonnet-20241022')
    adapter = AnthropicAdapter(config)
    sage = SageDog(adapter=adapter)
    
    await sage.on_activate()
    
    cell = Cell(cell_id='test', content='def hello(): print("world")', cell_type='code')
    j = await sage.judge(cell)
    
    print(f'Q-score: {j.q_score:.1f}')
    print(f'Verdict: {j.verdict.value}')
    print(f'Confidence: {j.confidence:.4f} (φ⁻¹=0.618)')
    print(f'Reasoning: {j.reasoning}')

asyncio.run(main())
