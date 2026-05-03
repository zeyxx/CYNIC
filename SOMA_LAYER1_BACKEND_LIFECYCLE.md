# Soma Layer 1 — Backend Lifecycle Management (2026-05-03)

**Status:** Implementing backend startup, health check, restart logic.

---

## Current Observation

**Layer 0 probe result:**
```
✓ GPU backend (<TAILSCALE_GPU>:8080) IS RUNNING
  - Model: Qwen3.5-9B-Q4_K_M.gguf
  - Trained context: 262144 (256K)
  - But kernel reports: actual = 65536 (65K)
  
✗ Embedding server (<TAILSCALE_CORE>:8081) NOT RUNNING
  - Service disabled (model file issue)
  - Fallback: FNV hash, works
```

**Problem:** GPU running but context mismatch detected by kernel every 20s.

**Hypothesis:** llama-server running with `-c 65536` flag instead of `-c 131072` (configured).

---

## Soma Layer 1 Responsibilities

### 1. Backend Startup
```bash
# GPU backend — already running externally (schtasks on Windows)
# Soma monitors but doesn't control

# Embedding backend — systemd unit exists
systemctl --user enable llama-embed.service
systemctl --user start llama-embed.service
```

### 2. Health Probes
```
Every 30 seconds:
  - POST http://GPU:8080/health
  - GET  http://EMBEDDING:8081/health
  - Verify response = 200
```

### 3. Configuration Verification
```
Before declaring "ready":
  1. GPU context window actual == configured (131072)
  2. Embedding service responsive
  3. All Dogs reachable (qwen-7b-hf, deterministic, etc.)
```

### 4. Recovery Logic
```
If probe fails:
  a) Retry 3 times (60s cooldown between retries)
  b) If GPU: call schtasks /run /tn CynicSovereign
  c) If Embedding: systemctl --user restart llama-embed.service
  d) Re-probe
  e) If still failing: escalate to kernel via /health alert
```

---

## Implementation: Layer 1 Manager

**File:** `~/.config/systemd/user/soma-layer1.service`

```ini
[Unit]
Description=CYNIC Soma Layer 1 — Backend Lifecycle Manager
After=network.target
StartLimitIntervalSec=300
StartLimitBurst=3

[Service]
Type=simple
ExecStart=/usr/bin/python3 /home/user/Bureau/CYNIC/infra/soma/layer1_manager.py
Restart=always
RestartSec=10
KillMode=mixed
TimeoutStopSec=30
StandardOutput=journal
StandardError=journal

Environment="PYTHONUNBUFFERED=1"
Environment="PYTHONPATH=/home/user/Bureau/CYNIC"

[Install]
WantedBy=default.target
```

**File:** `/home/user/Bureau/CYNIC/infra/soma/layer1_manager.py`

```python
#!/usr/bin/env python3
"""Soma Layer 1 — Backend Lifecycle Manager"""

import asyncio
import httpx
import logging
import subprocess
import sys
from datetime import datetime, timedelta
from typing import Optional

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [SOMA-L1] %(levelname)s: %(message)s'
)
log = logging.getLogger(__name__)

class BackendProbe:
    """Probe a single backend service."""
    
    def __init__(self, name: str, url: str, timeout: float = 2.0):
        self.name = name
        self.url = url
        self.timeout = timeout
        self.healthy = False
        self.last_check = None
        self.consecutive_failures = 0
    
    async def check(self) -> bool:
        """Probe backend health endpoint."""
        try:
            async with httpx.AsyncClient(timeout=self.timeout) as client:
                response = await client.get(f"{self.url}/health")
                if response.status_code == 200:
                    self.healthy = True
                    self.consecutive_failures = 0
                    self.last_check = datetime.utcnow()
                    log.info(f"{self.name}: ✓ healthy")
                    return True
        except Exception as e:
            log.warning(f"{self.name}: probe error: {e}")
        
        self.consecutive_failures += 1
        self.healthy = False
        return False

class SomaLayer1:
    """Backend lifecycle orchestrator."""
    
    def __init__(self):
        self.gpu = BackendProbe("GPU (qwen35-9b)", "http://<TAILSCALE_GPU>:8080")
        self.embedding = BackendProbe("Embedding (8081)", "http://<TAILSCALE_CORE>:8081")
        self.probes = [self.gpu, self.embedding]
        self.ready = False
        self.probe_interval = 30  # seconds
    
    async def recover_embedding(self) -> bool:
        """Attempt to recover embedding service."""
        log.info("Attempting to recover embedding service...")
        try:
            result = subprocess.run(
                ["systemctl", "--user", "restart", "llama-embed.service"],
                capture_output=True,
                timeout=30
            )
            if result.returncode == 0:
                log.info("Embedding service restarted")
                await asyncio.sleep(5)  # Wait for service to come up
                return await self.embedding.check()
        except Exception as e:
            log.error(f"Failed to restart embedding: {e}")
        return False
    
    async def recover_gpu(self) -> bool:
        """Attempt to recover GPU backend (Windows remote restart)."""
        log.info("Attempting to recover GPU backend...")
        try:
            # This assumes SSH key is configured
            result = subprocess.run(
                ["ssh", "S.@<TAILSCALE_GPU>", "schtasks", "/run", "/tn", "CynicSovereign"],
                capture_output=True,
                timeout=30
            )
            if result.returncode == 0:
                log.info("GPU backend restart command sent")
                await asyncio.sleep(10)  # Wait for service to come up
                return await self.gpu.check()
        except Exception as e:
            log.error(f"Failed to restart GPU: {e}")
        return False
    
    async def probe_loop(self):
        """Main probe loop — check backends every 30s."""
        log.info("Starting backend lifecycle manager...")
        
        while True:
            all_healthy = True
            
            for probe in self.probes:
                healthy = await probe.check()
                if not healthy:
                    all_healthy = False
                    
                    # Retry logic
                    if probe.consecutive_failures <= 3:
                        log.warning(f"{probe.name}: failure {probe.consecutive_failures}, will retry")
                    elif probe.consecutive_failures == 4:
                        # Attempt recovery
                        if probe == self.embedding:
                            await self.recover_embedding()
                        elif probe == self.gpu:
                            await self.recover_gpu()
                    elif probe.consecutive_failures > 6:
                        log.error(f"{probe.name}: unrecoverable, escalating")
                        # TODO: post alert to kernel /health endpoint
            
            # Update readiness state
            old_ready = self.ready
            self.ready = all_healthy
            
            if self.ready != old_ready:
                status = "READY" if self.ready else "DEGRADED"
                log.warning(f"Backend state changed: {status}")
            
            await asyncio.sleep(self.probe_interval)
    
    async def run(self):
        """Start lifecycle manager."""
        try:
            await self.probe_loop()
        except KeyboardInterrupt:
            log.info("Shutting down...")
        except Exception as e:
            log.error(f"Fatal error: {e}")
            sys.exit(1)

async def main():
    soma = SomaLayer1()
    await soma.run()

if __name__ == "__main__":
    asyncio.run(main())
```

---

## Integration Point

**Soma Layer 1 readiness → Kernel boot:**

Currently: Kernel boots immediately, tries to use backends, logs failures.

Should be:
```
1. Soma Layer 1 starts
2. Probes all backends
3. When all healthy:
   - Write to /run/soma-layer1.ready
   - Kernel checks this file before /ready endpoint
4. When any backend fails:
   - Delete /run/soma-layer1.ready
   - Kernel returns 503 /ready (not ready)
```

---

## Next Step

**Before implementing Layer 1 manager:**
1. Fix embedding model file OR accept embedding disabled for now
2. Verify GPU context window — why does kernel see 65K when server reports 262K?
3. Enable llama-embed.service (once model acquired)

**Falsifiable:** If Layer 1 running:
- Embedding server stays up (or auto-restarts on failure)
- GPU backend stays up (or auto-restarts via schtasks)
- Kernel /ready returns 200 only when both backends healthy
- Kernel degrades gracefully if backend fails

---

**Confidence:** φ⁻¹ (0.618) — Layer 1 logic clear, but need to resolve GPU context drift root cause first.
