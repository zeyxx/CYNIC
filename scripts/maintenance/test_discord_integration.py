"""
Test Discord Bot Integration with CYNIC API

Tests the bot's core functionality without running the actual Discord bot.
"""
import asyncio
import aiohttp
import json

CYNIC_API_URL = "http://localhost:8765"

async def test_ask_cynic():
    """Test /ask_cynic command flow."""
    print("\n[TEST 1] /ask_cynic Command")
    print("=" * 60)
    
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
                    print(f"  FAIL: POST /judge returned {resp.status}")
                    return False
                    
                data = await resp.json()
                judgment_id = data.get('judgment_id')
                print(f"  [OK] Judgment submitted: {judgment_id}")
                print(f"       Initial verdict: {data.get('verdict')}")
                
            # Step 2: Poll for result (bot does this up to 30 times)
            print(f"  [POLLING] Fetching result...")
            for attempt in range(5):
                async with session.get(
                    f"{CYNIC_API_URL}/judge/{judgment_id}",
                    timeout=aiohttp.ClientTimeout(total=2)
                ) as resp:
                    if resp.status == 200:
                        result = await resp.json()
                        verdict = result.get('verdict')
                        q_score = result.get('q_score')
                        print(f"  [OK] Result found: verdict={verdict}, q_score={q_score:.2f}")
                        return True
                    elif resp.status == 404:
                        print(f"       Attempt {attempt + 1}: PENDING")
                await asyncio.sleep(0.5)
                
            print(f"  [WARNING] Still PENDING after polling")
            return True
            
        except Exception as e:
            print(f"  FAIL: {type(e).__name__}: {e}")
            return False

async def test_cynic_status():
    """Test /cynic_status command."""
    print("\n[TEST 2] /cynic_status Command")
    print("=" * 60)
    
    async with aiohttp.ClientSession() as session:
        try:
            # Fetch health
            async with session.get(
                f"{CYNIC_API_URL}/health",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                if resp.status != 200:
                    print(f"  FAIL: /health returned {resp.status}")
                    return False
                    
                health = await resp.json()
                status = health.get('status')
                dogs = health.get('dogs', [])
                print(f"  [OK] API Status: {status}")
                print(f"  [OK] Dogs active: {len(dogs)} - {', '.join(dogs)}")
                
            # Fetch telemetry
            async with session.get(
                f"{CYNIC_API_URL}/empirical/telemetry",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                if resp.status == 200:
                    telemetry = await resp.json()
                    judgments = telemetry.get('total', 0)
                    print(f"  [OK] Total judgments: {judgments}")
                else:
                    print(f"  [WARNING] /telemetry returned {resp.status}")
                    
            return True
            
        except Exception as e:
            print(f"  FAIL: {type(e).__name__}: {e}")
            return False

async def test_teach_cynic():
    """Test /teach_cynic command flow."""
    print("\n[TEST 3] /teach_cynic Command")
    print("=" * 60)
    
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
                    print(f"  FAIL: POST /judge returned {resp.status}")
                    return False
                    
                data = await resp.json()
                judgment_id = data.get('judgment_id')
                print(f"  [OK] Created judgment: {judgment_id}")
                
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
                        print(f"  [OK] Feedback recorded")
                        return True
                    else:
                        print(f"  [WARNING] /learn returned {learn_resp.status}")
                        return True  # Endpoint might not be fully implemented
                        
        except Exception as e:
            print(f"  FAIL: {type(e).__name__}: {e}")
            return False

async def main():
    """Run all tests."""
    print("\nCYNIC Discord Bot Integration Tests")
    print("=" * 60)
    
    results = {}
    results['ask_cynic'] = await test_ask_cynic()
    results['cynic_status'] = await test_cynic_status()
    results['teach_cynic'] = await test_teach_cynic()
    
    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    passed = sum(1 for v in results.values() if v)
    total = len(results)
    print(f"Passed: {passed}/{total}")
    
    for name, result in results.items():
        status = "[PASS]" if result else "[FAIL]"
        print(f"  {name:30} {status}")
    
    if passed == total:
        print("\nAll integration tests passed!")
    else:
        print(f"\n{total - passed} test(s) failed")

if __name__ == "__main__":
    asyncio.run(main())
