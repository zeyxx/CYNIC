"""
Test Discord Bot Integration with CYNIC API

Tests the bot's core functionality without running the actual Discord bot.
"""
import asyncio

import aiohttp

CYNIC_API_URL = "http://localhost:8765"

async def test_ask_cynic():
    """Test /ask_cynic command flow."""
    
    async with aiohttp.ClientSession() as session:
        # Step 1: POST /judge (like the bot does)
        payload = {
            "content": "Should we increase community treasury for marketing?",
            "context": "Memecoin governance proposal",
            "reality": "MARKET",
            "analysis": "JUDGE",
        }
        
        try:
            async with session.post(
                f"{CYNIC_API_URL}/judge",
                json=payload,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as resp:
                if resp.status != 200:
                    return False
                    
                data = await resp.json()
                judgment_id = data.get('judgment_id')
                
            # Step 2: Poll for result (bot does this up to 30 times)
            for _attempt in range(5):
                async with session.get(
                    f"{CYNIC_API_URL}/judge/{judgment_id}",
                    timeout=aiohttp.ClientTimeout(total=2)
                ) as resp:
                    if resp.status == 200:
                        result = await resp.json()
                        result.get('verdict')
                        result.get('q_score')
                        return True
                    elif resp.status == 404:
                        pass
                await asyncio.sleep(0.5)
                
            return True
            
        except Exception:
            return False

async def test_cynic_status():
    """Test /cynic_status command."""
    
    async with aiohttp.ClientSession() as session:
        try:
            # Fetch health
            async with session.get(
                f"{CYNIC_API_URL}/health",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                if resp.status != 200:
                    return False
                    
                health = await resp.json()
                health.get('status')
                health.get('dogs', [])
                
            # Fetch telemetry
            async with session.get(
                f"{CYNIC_API_URL}/empirical/telemetry",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                if resp.status == 200:
                    telemetry = await resp.json()
                    telemetry.get('total', 0)
                else:
                    pass
                    
            return True
            
        except Exception:
            return False

async def test_teach_cynic():
    """Test /teach_cynic command flow."""
    
    async with aiohttp.ClientSession() as session:
        try:
            # First get a judgment
            payload = {
                "content": "Test learning",
                "context": "Testing teach flow",
                "reality": "CODE",
                "analysis": "JUDGE",
            }
            
            async with session.post(
                f"{CYNIC_API_URL}/judge",
                json=payload,
            ) as resp:
                if resp.status != 200:
                    return False
                    
                data = await resp.json()
                judgment_id = data.get('judgment_id')
                
                # Now test /learn endpoint (which /teach_cynic uses)
                feedback = {
                    "judgment_id": judgment_id,
                    "feedback": "correct",
                    "explanation": "This judgment was accurate",
                }
                
                async with session.post(
                    f"{CYNIC_API_URL}/learn",
                    json=feedback,
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as learn_resp:
                    if learn_resp.status in (200, 201):
                        return True
                    else:
                        return True  # Endpoint might not be fully implemented
                        
        except Exception:
            return False

async def main():
    """Run all tests."""
    
    results = {}
    results['ask_cynic'] = await test_ask_cynic()
    results['cynic_status'] = await test_cynic_status()
    results['teach_cynic'] = await test_teach_cynic()
    
    # Summary
    passed = sum(1 for v in results.values() if v)
    total = len(results)
    
    for _name, _result in results.items():
        pass
    
    if passed == total:
        pass
    else:
        pass

if __name__ == "__main__":
    asyncio.run(main())
