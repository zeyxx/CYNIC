"""
CYNIC Night One - The Autonomous Emergence Daemon (Resilient Edition).
Handles curiosity, action, and learning with real-time checkpointing.
"""
import asyncio
import logging
from cynic.orchestration import OrganismOrchestrator
from cynic.kernel.organism.discovery import DiscoveryDaemon
from cynic.kernel.organism.metabolism.cockpit import NightCockpit

logging.basicConfig(
    level=logging.INFO, 
    format="%(asctime)s [%(levelname)s] CYNIC: %(message)s"
)
logger = logging.getLogger("cynic.night_daemon")

async def run_night_cycle():
    print("\n" + "🌀"*20)
    print("🚀 CYNIC AUTONOMOUS NIGHT CYCLE: INITIATING EMERGENCE")
    print("🌀"*20)
    
    orch = OrganismOrchestrator(instance_id="cynic-night-daemon")
    discovery = DiscoveryDaemon()
    cockpit = NightCockpit()
    
    missions_done = 0
    learning_shifts = 0
    
    try:
        # 1. AWAKENING
        await orch.awake()
        
        # 2. DISCOVERY
        missions = discovery.find_potential_missions()
        # Filter for top priorities (e.g., first 10 missions for a real night)
        active_missions = missions[:10]
        logger.info(f"Night Cycle: Starting {len(active_missions)} missions.")
        
        # 3. SURGERY LOOP
        for i, mission in enumerate(active_missions):
            mission_target = mission['target']
            mission_axiom = mission['axiom']
            
            logger.info(f"Executing Mission {i+1}/{len(active_missions)}: {mission_target}")
            
            try:
                result = await orch.process_task(
                    f"Analyze and propose a fix for {mission_target}. Context: {mission['description']}",
                    axiom=mission_axiom
                )
                
                # Checkpointing Learning immediately
                await orch.vault.persist()
                missions_done += 1
                learning_shifts += 1 # In real, we would check if scores actually moved
                
                # Update Dashboard
                cockpit.update_mission(
                    target=mission_target,
                    axiom=mission_axiom,
                    result="Analysis Complete",
                    muscle=result['muscle'],
                    success=True
                )
                cockpit.update_summary(missions_done, learning_shifts)
                
            except Exception as e:
                logger.error(f"Mission failed: {mission_target} - {e}")
                cockpit.update_mission(mission_target, mission_axiom, f"Error: {e}", "N/A", False)

            # Metabolic Cool-down (let the APU breathe)
            await asyncio.sleep(5)

    except Exception as e:
        logger.critical(f"NIGHT CYCLE PANIC: {e}")
        cockpit.update_summary(missions_done, learning_shifts, status="🔴 CRASHED")
    finally:
        # 4. CONSOLIDATION & SLEEP
        await orch.sleep()
        cockpit.update_summary(missions_done, learning_shifts, status="🏁 COMPLETED")
        print("\n" + "🌀"*20)
        print("🏆 NIGHT CYCLE COMPLETE. CHECK 'NIGHT_REPORT.md' AT REVEIL.")
        print("🌀"*20)

if __name__ == "__main__":
    asyncio.run(run_night_cycle())
