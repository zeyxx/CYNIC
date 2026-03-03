import asyncio
import logging
import sys

# Configure strict logging
logging.basicConfig(level=logging.DEBUG)


async def test_awaken():
    try:
        from cynic.kernel.organism.factory import awaken

        awaken()
        return 0
    except Exception:
        import traceback

        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(asyncio.run(test_awaken()))
