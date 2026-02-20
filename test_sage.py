import asyncio
from cynic.dogs.sage import SageDog
from cynic.kernel.types import Cell

async def test():
    sage = SageDog()
    await sage.on_activate()
    cell = Cell(cell_id='test-1', content='def hello(): print(1)', cell_type='code', metadata={})
    j = await sage.judge(cell)
    
    print(f'Q-score: {j.q_score}')
    print(f'Verdict: {j.verdict}')
    print(f'Confidence: {j.confidence:.4f}')
    print(f'Reasoning: {j.reasoning}')
    print(f'Dimensions: {j.dimensions}')

asyncio.run(test())
