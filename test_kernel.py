import asyncio
import sys

# Fix path to use local cynic package (not installed version)
to_remove = [k for k in sys.modules if k.startswith('cynic')]
for k in to_remove:
    del sys.modules[k]
sys.path.insert(0, 'c:/Users/zeyxm/Desktop/asdfasdfa/CYNIC/cynic')

from cynic.dogs.sage import SageDog
from cynic.kernel.types import Cell

async def main():
    dog = SageDog()
    await dog.on_activate()
    
    cell = Cell(cell_id='test-001', content='def hello(): print("world")', cell_type='code')
    judgment = await dog.judge(cell)
    
    print("=== CYNIC Python Kernel Test ===")
    print("Q-Score: %.2f" % judgment.q_score)
    print("Verdict: %s" % judgment.verdict.value)
    print("Confidence: %.3f (max 0.618)" % judgment.confidence)
    print("Reasoning: %s" % judgment.reasoning)
    
    if judgment.confidence <= 0.618:
        print("\n=== PASS: Phi-bound enforced ===")
        return 0
    else:
        print("\n=== FAIL: Phi-bound violated ===")
        return 1

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
