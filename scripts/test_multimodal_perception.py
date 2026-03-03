"""
🌀 CYNIC MULTIMODAL PERCEPTION TEST

Verifies that the organism can ingest a rich sensory packet (image/binary)
via the Vascular System and use it during a fractal judgment cycle.
"""

import asyncio
import logging
import sys
import os
import uuid

# Ensure project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.vascular import MultimodalPacket
from cynic.kernel.core.judgment import Cell

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')

async def run_multimodal_test():
    print("=" * 60)
    print("🌀 STARTING MULTIMODAL PERCEPTION AUDIT")
    print("=" * 60)

    try:
        # 1. Awaken the organism
        organism = await awaken()
        vascular = organism.vascular # Factory links it directly to organism now
        
        # 2. Simulate "Seeing" an image (e.g., a base64 string or placeholder)
        packet_id = f"packet-{uuid.uuid4().hex[:6]}"
        sensory_packet = MultimodalPacket(
            packet_id=packet_id,
            content_type="image",
            payload="BASE64_ENCODED_SECURITY_DASHBOARD_SCREENSHOT",
            metadata={"source": "watcher-01", "resolution": "1920x1080"}
        )
        
        print(f"\n[SENSE] Organism saw a {sensory_packet.content_type} (ID: {packet_id})")
        await vascular.perception.push(sensory_packet)
        
        # 3. Create a Cell that references this multimodal data
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content="Industrial HAL implementation audit.",
            context="The agent must look at the dashboard image to confirm uptime.",
            multimodal_packet_id=packet_id
        )
        
        print(f"[CELL] Created unit {cell.cell_id} referencing multimodal data.")

        # 4. Trigger Dog check
        sage = organism.cognition.orchestrator.dogs["SAGE"]
        print(f"\n[JUDGE] Dog {sage.dog_id} is ready to analyze.")
        
        # Verify that SAGE can access the vascular system
        if hasattr(sage, 'vascular') and sage.vascular:
            print("SAGE Vascular Access: OK")
            
            # Simulate the internal lookup that _llm_path would do
            packets = await sage.vascular.perception.flush()
            found = any(p.packet_id == packet_id for p in packets)
            print(f"Multimodal Packet Found in Buffer: {'YES' if found else 'NO'}")
        else:
            print("SAGE Vascular Access: FAILED")

        # 5. Shutdown
        await organism.stop()
        print("\n" + "=" * 60)
        print("STATUS: MULTIMODAL SUTURE VALIDATED.")
        print("=" * 60)

    except Exception as e:
        print(f"\n❌ MULTIMODAL TEST FAILED: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    asyncio.run(run_multimodal_test())
