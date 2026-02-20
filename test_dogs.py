"""Quick test for all Dogs"""
import asyncio
from cynic.dogs import SageDog, ScoutDog, OracleDog, ArchitectDog, ScholarDog, CollectiveDog, ThermodynamicsDog
from cynic.kernel.types import Cell

async def main():
    cell = Cell(cell_id='test-1', content='def hello(): print("world")\n    return True', cell_type='code')
    
    dogs = [
        ('Sage', SageDog()),
        ('Scout', ScoutDog()),
        ('Oracle', OracleDog()),
        ('Architect', ArchitectDog()),
        ('Scholar', ScholarDog()),
        ('Thermodynamics', ThermodynamicsDog()),
    ]
    
    for name, dog in dogs:
        await dog.on_activate()
        j = await dog.judge(cell)
        print(f'{name}: Q={j.q_score:.1f}, verdict={j.verdict.value}')

asyncio.run(main())
