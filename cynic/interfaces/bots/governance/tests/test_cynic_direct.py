"""
Test CYNIC integration directly without Discord bot
"""
import asyncio
import sys
from pathlib import Path

# Add CYNIC to path
cynic_path = Path(__file__).parent.parent
sys.path.insert(0, str(cynic_path))

async def test_cynic_judgment():
    """Test a CYNIC judgment call"""
    from governance_bot.cynic_integration import ask_cynic
    
    print("Testing CYNIC judgment integration...")
    
    result = await ask_cynic(
        question="Should we increase community treasury allocation from 25% to 50%?",
        context="Proposal requests doubling allocation to 50% to fund development incentives",
        reality="GOVERNANCE"
    )
    
    print(f"\nCYNIC Response:")
    print(f"  Verdict: {result.get('verdict')}")
    print(f"  Q-Score: {result.get('q_score')}")
    print(f"  Confidence: {result.get('confidence'):.3f}")
    print(f"  Verdict Explanation: {result.get('verdict_explanation', 'N/A')[:200]}")
    print(f"  Dog Reasoning count: {len(result.get('dog_reasoning', {}))}")
    
    return result

if __name__ == "__main__":
    result = asyncio.run(test_cynic_judgment())
    if result.get("error"):
        print(f"\nError: {result.get('error')}")
        sys.exit(1)
    else:
        print("\n✓ CYNIC integration working!")
        sys.exit(0)
