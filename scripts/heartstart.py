"""
CYNIC Heartstart — Sovereign Infrastructure Provisioner.

This script acts as the 'defibrillator' for CYNIC.
1. Checks if SurrealDB and Redis are responsive.
2. If not, attempts to start them via docker-compose.
3. Waits for healthchecks to pass.

Lentilles : SRE (Resilience), Robotics (Self-Maintenance).
"""

import asyncio
import logging
import subprocess
import socket

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("cynic.heartstart")

def is_port_open(host: str, port: int) -> bool:
    """Check if a TCP port is open."""
    try:
        with socket.create_connection((host, port), timeout=1):
            return True
    except (ConnectionRefusedError, socket.timeout):
        return False

async def start_infra():
    # 1. Diagnostic
    surreal_up = is_port_open("localhost", 8000)
    redis_up = is_port_open("localhost", 6379)
    
    if surreal_up and redis_up:
        logger.info("✅ Infrastructure is already breathing. No action needed.")
        return True

    # 2. Intervention
    logger.info("🚩 Infrastructure weakness detected. Initiating Heartstart (Docker Compose)...")
    try:
        # Launch only the DB and Network components
        subprocess.run(["docker-compose", "up", "-d", "surrealdb", "redis"], check=True)
        logger.info("🚀 Docker containers launched.")
    except Exception as e:
        logger.error(f"❌ Heartstart failed: {e}")
        return False

    # 3. Stabilization (Wait for life signs)
    logger.info("Waiting for SurrealDB and Redis to stabilize...")
    for _ in range(30): # 30 seconds timeout
        if is_port_open("localhost", 8000) and is_port_open("localhost", 6379):
            logger.info("✨ Infrastructure is now stable. CYNIC can think.")
            return True
        await asyncio.sleep(1)
    
    logger.error("❌ Infrastructure failed to stabilize in time.")
    return False
if __name__ == "__main__":
    asyncio.run(start_infra())
