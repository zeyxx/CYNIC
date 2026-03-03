"""
CYNIC SurrealDB Live Query Probe — Technical Validation.

This script tests the behavior of the SurrealDB Python SDK (v0.3.2) 
regarding LIVE SELECT queries. We need to confirm if it uses 
an internal queue, an async iterator, or a callback mechanism.

Pattern:
  1. Connect to SurrealDB.
  2. Start LIVE SELECT on 'probe_table'.
  3. Emit a change in background.
  4. Measure latency and delivery reliability.

Lentilles : Data Engineer (Stream), SRE (Reliability).
"""

import asyncio
import logging
import os
import time
import uuid

from surrealdb import Surreal

# Setup Noble Logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
logger = logging.getLogger("cynic.probe.surreal_live")

async def probe_live_query():
    url = os.getenv("SURREAL_URL", "ws://localhost:8000/rpc")
    user = os.getenv("SURREAL_USER", "root")
    password = os.getenv("SURREAL_PASS", "root")
    ns = "probe_ns"
    db_name = "probe_db"

    logger.info(f"Connecting to SurrealDB at {url}...")
    
    async with Surreal(url) as db:
        try:
            await db.signin({"username": user, "password": password})
            await db.use(ns, db_name)
            logger.info("Connection established. Cleaning probe table...")
            await db.query("DELETE probe_table")

            # --- THE EXPERIMENT ---
            
            logger.info("Starting LIVE SELECT...")
            # In SDK 0.3.x, query() for LIVE SELECT returns a UUID (query_id)
            result = await db.query("LIVE SELECT * FROM probe_table")
            # SurrealDB returns a list of results for each statement. 
            # The first one should contain our query_id.
            query_id = result[0]['result']
            logger.info(f"Live Query ID: {query_id}")

            # Define a listener task
            # We need to find how the SDK exposes the stream.
            # According to SDK patterns, we might need to check if 'db' 
            # has a specific subscription handler.
            
            async def stimulator():
                await asyncio.sleep(1)
                logger.info("Stimulus: Inserting record...")
                t_start = time.time()
                await db.create("probe_table", {
                    "probe_id": str(uuid.uuid4()),
                    "val": 42,
                    "ts": t_start
                })
                return t_start

            # Attempt to listen. 
            # Note: Many async SDKs for SurrealDB use a background task 
            # that pushes to a queue or a callback.
            
            logger.info("Waiting for notifications (timeout 5s)...")
            
            # Start stimulus
            stimulus_task = asyncio.create_task(stimulator())
            
            # SURREALDB PYTHON SDK 0.3.x ANALYSIS:
            # We check if the database object provides a way to receive notifications.
            # Most likely it's via a method like `db.subscribe(query_id)` or an iterator.
            
            # Since I don't want to guess, I'll probe the object structure if it fails.
            try:
                # Some versions use a specific property or queue
                # We'll try to poll for 5 seconds
                start_time = time.time()
                received = False
                
                while time.time() - start_time < 5:
                    # PROBE: Look for 'notifications' or similar in db object
                    # This is the 'Expert Analysis' phase
                    if hasattr(db, '_notifications'):
                        queue = getattr(db, '_notifications').get(query_id)
                        if queue and not queue.empty():
                            msg = await queue.get()
                            logger.info(f"✨ NOTIFICATION RECEIVED: {msg}")
                            received = True
                            break
                    
                    await asyncio.sleep(0.1)
                
                if not received:
                    logger.warning("No notification received via internal queue probe.")
                    logger.info(f"Available DB attributes: {dir(db)}")

            except Exception as e:
            logger.error(f"Error during notification probe: {e}")

            await stimulus_task
            
        except Exception as e:
            logger.error(f"Probe failed: {e}")
        finally:
            logger.info("Probe terminated.")

if __name__ == "__main__":
    asyncio.run(probe_live_query())
