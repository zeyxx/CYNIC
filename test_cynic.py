import asyncio
from cynic.dogs.sage import SageDog
from cynic.kernel.types import Cell

async def test():
    dog = SageDog()
    cell = Cell(cell_id='test-1', content='def hello(): print("world")', cell_type='code')
    j = await dog.judge(cell)
    return f'Q:{j.q_score} V:{j.verdict.value} C:{j.confidence}'

result = asyncio.run(test())
print(result)
