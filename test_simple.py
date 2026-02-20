"""Simple test"""
import asyncio
from cynic.dogs import SageDog
from cynic.kernel.types import Cell

async def main():
    s = SageDog()
    await s.on_activate()
    j = await s.judge(Cell(cell_id='1', content='x=1', cell_type='code'))
    print('Q=%s V=%s C=%.4f' % (round(j.q_score,1), j.verdict.value, j.confidence))

asyncio.run(main())
