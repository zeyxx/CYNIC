# Soma Reality Check — What Actually Exists (2026-05-03)

**Status:** Systemd units partially exist. Models missing. GPU backend externally managed.

---

## Existing Infrastructure

**Embedding service:**
```
File: ~/.config/systemd/user/llama-embed.service
Status: DISABLED (not running)
Config: 
  - Model: Qwen3-Embedding-0.6B-Q8_0.gguf (MISSING — "No such file or directory")
  - Host: <TAILSCALE_CORE>:8081
  - Type: Embedding only (no inference)
  - Restart: always
  - API key: /home/user/.config/cynic/llama-api-key

Last attempt (May 3 16:42):
  ERROR: failed to load model — file doesn't exist
```

**GPU service:**
```
Status: NOT FOUND
Config point: <TAILSCALE_GPU>:8080 (Windows node "titou")
Managed by: schtasks (Windows Task Scheduler, not systemd)
```

**GGUF models on disk:**
```
$ find /home/user -name "*.gguf"
# (empty — 0 files found)
```

---

## What Kernel Expects vs. Reality

| Component | Config | Running | Status |
|-----------|--------|---------|--------|
| Embedding (8081) | Qwen3-0.6B @ <TAILSCALE_CORE>:8081 | ✗ NO | Model file missing |
| GPU (8080) | Qwen3.5-9B @ <TAILSCALE_GPU>:8080 | ? UNSTABLE | Context drift detected every 20s |
| Fallback HF | qwen-7b-hf (cloud) | ✓ YES | 3182ms latency, q=0.357 |
| Deterministic | In-kernel heuristics | ✓ YES | 0ms latency, q=0.05 |

---

## What Soma Layer 1 (Backend Lifecycle) Needs

### 1. Embedding Server (Local)
**Current blocker:** Model file missing.

**To fix:**
```
1. Acquire Qwen3-Embedding-0.6B-Q8_0.gguf model file
   - Check if it exists elsewhere on system
   - Download from HuggingFace if not
   - Place at /home/user/llama.cpp/models/

2. Start service:
   systemctl --user enable llama-embed.service
   systemctl --user start llama-embed.service
   
3. Verify:
   curl http://<TAILSCALE_CORE>:8081/health
   # Should return 200 with embedding capabilities
```

### 2. GPU Server (Windows Remote)
**Current blocker:** Running externally (schtasks on Windows). Not under systemd control.

**Options:**
- **Option A:** Create systemd wrapper that calls `ssh titou@<TAILSCALE_GPU> schtasks /run /tn CynicSovereign`
  - Pro: Integrated into Soma lifecycle
  - Con: Requires SSH + schtasks permission
  
- **Option B:** Accept external management, only monitor + escalate
  - Pro: Don't reinvent Windows task scheduling
  - Con: Soma can't control recovery

**Observation:** backends.toml already has remediation config:
```toml
[backend.qwen35-9b-gpu.remediation]
node = "titou@<TAILSCALE_GPU>"
restart_command = "schtasks /run /tn CynicSovereign"
max_retries = 3
cooldown_secs = 60
```

So Soma Layer 4 recovery logic already knows how to restart GPU. Question: is this configured correctly?

---

## Ground Truth Update

**Soma L1 bottleneck:** Embedding model file missing.

**Soma L0 pre-flight will detect:**
```
- Embedding server: UNREACHABLE (file not found)
- GPU server: REACHABLE but UNSTABLE (context drift)
- Fallback HF: AVAILABLE (carrying current load)
```

**Decision point:** 
1. **Acquire embedding model** → enable service → test
2. **Investigate GPU context drift** → restart via schtasks → verify context = 131072
3. **Wire pre-flight probe** → detect failures before kernel boot

---

## What Needs to Happen Now

### Immediate (blocking Phase 2):
1. Find or download Qwen3-Embedding-0.6B-Q8_0.gguf
2. Place at /home/user/llama.cpp/models/
3. Enable and start llama-embed.service
4. Verify curl http://<TAILSCALE_CORE>:8081/health returns 200

### Secondary (unblocks GPU stability):
1. SSH to GPU node or verify schtasks command
2. Check if llama-server is actually running with `-c 131072` flag
3. If not, restart it with correct flag
4. Verify context window matches config

### Tertiary (Soma L0 implementation):
1. Build pre-flight probe that checks both backends
2. Block kernel boot if dependencies missing
3. Log degradation explicitly

---

**Falsifiable:** If embedding model acquired and service started:
- /health probe returns 200
- Kernel no longer logs "Embedding unreachable"
- Crystal formation uses semantic embedding instead of FNV hash

**Current state:** Kernel working despite missing pieces. Soma job is to make missing pieces visible and recoverable.

**Confidence:** φ⁻¹ (0.618) — infrastructure partially exists, critical piece (embedding model) missing. Recovery path clear once model acquired.
