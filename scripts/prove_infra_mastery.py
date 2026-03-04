"""
CYNIC Infrastructure Mastery Proof
Demonstrates CYNIC detecting a missing reality (Docker), taking physical action 
to wake it up, and subsequently manipulating matter (files) to prove it's alive.
"""
import asyncio
import logging
from cynic.kernel.infrastructure.orchestrator import InfrastructureOrchestrator
from cynic.kernel.organism.metabolism.motor_cortex import MotorCortex
from cynic.kernel.core.event_bus import EventBus, CoreEvent, Event

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.proof")

async def prove_mastery():
    print("============================================================")
    print("🌌 CYNIC: HOST INFRASTRUCTURE CONTROL OVERRIDE")
    print("============================================================")

    # 1. The Orchestrator (Level Extreme)
    infra = InfrastructureOrchestrator()
    logger.info("Initiating deep infra probe...")
    
    # This will trigger the Start-Process if Docker is off
    success = await infra.provision_reality()
    
    if success:
        logger.info("✅ Reality is online. Docker infrastructure secured.")
        
        # 2. Proof of Action (Motor Cortex)
        logger.info("Demonstrating physical impact...")
        bus = EventBus(bus_id="proof-bus")
        motor = MotorCortex(bus)
        
        # Create a proof file
        await motor.actuator.dispatch("file", {
            "path": "audit/living_sign.txt",
            "content": "CYNIC has successfully taken control of the host infrastructure.\nThe organism is sovereign.\n"
        })
        
        # Run a shell command
        shell_result = await motor.actuator.dispatch("shell", {
            "command": "echo 'Hello from the CYNIC Motor Cortex!'"
        })
        
        if shell_result.success:
            logger.info(f"✅ Motor Cortex Shell output: {shell_result.output.strip()}")
        else:
            logger.error(f"❌ Motor Cortex Shell failed: {shell_result.error}")
            
    else:
        logger.critical("❌ The organism failed to wake up its environment. Host intervention required.")

if __name__ == "__main__":
    asyncio.run(prove_mastery())
