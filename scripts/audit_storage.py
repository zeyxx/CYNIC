"""
CYNIC Multiversal Auditor — SurrealDB Port & Auth test.

Attempts connectivity on multiple ports to find the real one.
Lentilles : Data Engineer, SRE.
"""

import asyncio
import logging
from surrealdb import Surreal

logging.basicConfig(level=logging.DEBUG) # Force DEBUG for more info
logger = logging.getLogger("cynic.audit.storage")

async def test_port(port: int):
    url = f"ws://localhost:{port}/rpc"
    logger.info(f"--- TESTING PORT {port} ({url}) ---")
    
    try:
        db = Surreal(url)
        await db.connect()
        logger.info(f"✅ Port {port}: Connection successful.")
        
        try:
            await db.signin({"user": "root", "pass": "cynic_phi_618"})
            logger.info(f"✅ Port {port}: Authentication successful.")
            await db.use("cynic", "cynic")
            logger.info(f"✅ Port {port}: Namespace/DB use successful.")
            await db.close()
            return True
        except Exception as auth_err:
            logger.error(f"❌ Port {port}: Auth FAILED: {auth_err}")
            await db.close()
            return False
            
    except Exception as conn_err:
        logger.warning(f"❌ Port {port}: Connection FAILED: {conn_err}")
        return False

async def audit():
    results = []
    for port in [8000, 8080]:
        results.append(await test_port(port))
    
    if any(results):
        logger.info("🎉 STORAGE FOUND. FOUNDATION IS VALID.")
    else:
        logger.error("💀 STORAGE UNREACHABLE. INFRASTRUCTURE IS FAILED.")

if __name__ == "__main__":
    asyncio.run(audit())
