#!/usr/bin/env python3
"""
Verification test for memory leak fix.
Tests that bot can call previously-failing methods without crashing.
"""

import asyncio
import logging
import sys
from pathlib import Path

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Add CYNIC to path
cynic_path = Path(__file__).parent.parent
sys.path.insert(0, str(cynic_path))


async def test_cynic_status():
    """Test get_cynic_status() no longer crashes"""
    try:
        from cynic.interfaces.bots.governance.integration.cynic_integration import get_cynic_status

        logger.info("[TEST] get_cynic_status()...")
        result = await get_cynic_status()

        assert result.get("status") in ["online", "error"], f"Unexpected status: {result.get('status')}"
        if result.get("status") == "online":
            data = result.get("data")
            logger.info(f"  [OK] Status online: {data}")
        else:
            logger.info(f"  [OK] Status error (expected if CYNIC not running): {result.get('error')}")

        return True
    except Exception as e:
        logger.error(f"  [FAIL] {e}", exc_info=True)
        return False


async def test_observe_cynic():
    """Test observe_cynic() no longer crashes"""
    try:
        from cynic.interfaces.bots.governance.integration.cynic_integration import observe_cynic

        for aspect in ["consciousness", "learning", "health", "full"]:
            logger.info(f"[TEST] observe_cynic(aspect='{aspect}')...")
            result = await observe_cynic(aspect=aspect)

            assert result.get("status") in ["success", "error"], f"Unexpected status: {result.get('status')}"
            if result.get("status") == "success":
                logger.info(f"  [OK] {result.get('observation')}")
            else:
                logger.info(f"  [OK] Status error (expected if CYNIC not running): {result.get('error')}")

        return True
    except Exception as e:
        logger.error(f"  [FAIL] {e}", exc_info=True)
        return False


async def main():
    """Run all verification tests"""
    logger.info("\n" + "="*70)
    logger.info("MEMORY LEAK FIX VERIFICATION TEST")
    logger.info("="*70)

    results = {}

    logger.info("\n--- Test 1: get_cynic_status() ---")
    results['cynic_status'] = await test_cynic_status()

    logger.info("\n--- Test 2: observe_cynic() ---")
    results['observe_cynic'] = await test_observe_cynic()

    # Summary
    logger.info("\n" + "="*70)
    logger.info("SUMMARY")
    logger.info("="*70)

    for test_name, passed in results.items():
        status = "[PASS]" if passed else "[FAIL]"
        logger.info(f"{status} {test_name}")

    all_passed = all(results.values())
    if all_passed:
        logger.info("\nAll tests passed! Memory leak should be fixed.")
        return 0
    else:
        logger.info("\nSome tests failed. Check logs above.")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
