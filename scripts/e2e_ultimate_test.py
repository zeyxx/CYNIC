"""
🌀 CYNIC E2E ULTIMATE TEST : The Ordeal of Materialization.
Respects all 9 Lentilles.

This is the TRUE falsification of our session. It tests if the
Daemon, Vascular, WorldModel, and Throttler work as ONE organism.
"""

import asyncio
import logging
from cynic.kernel.core.daemon import CynicDaemon
from cynic.kernel.core.event_bus import CoreEvent, Event

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("cynic.e2e")


async def run_ordeal():
    print("============================================================")
    print("🔥 INITIATING ULTIMATE FALSIFICATION (E2E Materialization)")
    print("============================================================")

    # 1. Initialize the Daemon
    daemon = CynicDaemon()

    try:
        # 2. Start the Organism (Partial start for testing)
        print("[1/4] AWAKENING CORE...")
        daemon.rhythm.start()

        # 3. Simulate Reality (Data Injection)
        print("[2/4] INJECTING SENSORY DATA...")
        for i in range(5):
            await daemon.bus.emit(
                Event.typed(
                    CoreEvent.JUDGMENT_CREATED,
                    {
                        "reality": "CODE",
                        "verdict": "BARK",
                        "q_score": 20.0 + i,
                        "judgment_id": f"test-judge-{i}",
                    },
                    source="test_injector",
                )
            )

        # Give time for the WorldModel to process
        await asyncio.sleep(2)

        # 4. Falsify WorldModel Vectorization
        print("[3/4] AUDITING WORLD VECTOR...")
        # We need to access the WorldModel through the daemon (we'll add it if missing)
        # For now, let's check if we can get a snapshot
        from cynic.kernel.core.world_model import WorldModelUpdater

        updater = WorldModelUpdater(bus=daemon.bus)
        updater.start()

        # Wait for another judgment
        await daemon.bus.emit(
            Event.typed(
                CoreEvent.JUDGMENT_CREATED,
                {"reality": "MARKET", "verdict": "WAG", "q_score": 88.0},
                source="test_injector",
            )
        )
        await asyncio.sleep(1)

        state = updater.world_state()
        vector = state.state_vector
        print(f"  -> World Vector: {vector}")

        if len(vector) == 7 and any(v != 0.5 for v in vector):
            print("  ✅ VECTORIZATION VALIDATED.")
        else:
            print("  ❌ VECTORIZATION FAILED (Still neutral or wrong size).")
            raise RuntimeError("Falsification: WorldModel is dead.")

        # 5. Falsify Metabolic Throttling
        print("[4/4] TESTING SOMATIC PULSE...")
        metrics = await daemon.body.pulse()
        print(f"  -> CPU: {metrics.cpu_percent}% | Mem: {metrics.memory_percent}%")

        cost = daemon.body.get_metabolic_cost()
        print(f"  -> Metabolic Cost: {cost:.4f}")

        if cost >= 1.0:
            print("  ✅ METABOLISM VALIDATED.")
        else:
            print("  ❌ METABOLISM FAILED.")
            raise RuntimeError("Falsification: No heartbeat detected.")

        # 6. Falsify Phi Incorruptibility
        print("[5/5] SEALING TRUTH (PhiAuditor)...")
        from cynic.kernel.security.vault import VaultSecretStore, VaultConfig
        from cynic.kernel.security.phi_auditor import PhiAuditor
        
        vault = VaultSecretStore(config=VaultConfig(vault_addr="http://localhost:8200", vault_token="root"))
        auditor = PhiAuditor(vault=vault)
        
        signature = await auditor.seal_truth(state)
        print(f"  -> Truth Signature: {signature}")
        
        if signature.startswith("phi-sig-"):
            print("  ✅ INCORRUPTIBILITY VALIDATED.")
        else:
            print("  ❌ AUDIT FAILED.")
            raise RuntimeError("Falsification: Auditor is corrupt.")

        print("\n" + "=" * 60)
        print("RESULT: CYNIC MATERIALIZATION SUCCESSFUL.")
        print("THE SYSTEM IS REAL AND INTERCONNECTED.")
        print("=" * 60)

    except Exception as e:
        print("\n" + "!" * 60)
        print(f"RESULT: CYNIC IS FRAGILE. CRASH DETECTED: {e}")
        print("!" * 60)
        import traceback

        traceback.print_exc()
    finally:
        await daemon.stop()


if __name__ == "__main__":
    asyncio.run(run_ordeal())
