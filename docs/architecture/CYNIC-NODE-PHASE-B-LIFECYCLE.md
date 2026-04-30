# CYNIC Node Phase B — Binary Lifecycle & Integration

> **What:** Remote execution agent (cynic-node) that runs on each machine, receives orchestration commands from Soma, supervises local Dog processes, and emits K15 observations. Soma is the central orchestrator; nodes are the hands.

---

## Phase B Architecture: 4 Concerns (Soma-Centric)

```
┌────────────────────────────────────────────────────────────┐
│ cynic-node Binary (Soma's Remote Agent)                    │
│ Runs on: <TAILSCALE_GPU>, <TAILSCALE_NODE_2>, etc.        │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 1: COMMAND RECEPTION (from Soma)            │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • Every 5s: GET /node/{id}/pending_commands         │  │
│  │   (from kernel Soma)                                │  │
│  │ • Receive: {action: "load_model", model: "qwen"}   │  │
│  │ • Queue commands, execute serially                  │  │
│  │ • ACK success/failure back to Soma                  │  │
│  │                                                      │  │
│  │ Deps: reqwest, serde_json, tokio::time::interval    │  │
│  └─────────────────────────────────────────────────────┘  │
│                           ↓ (state change)                 │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 2: PROBE & ADAPT (hardware awareness)       │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • On boot: nvidia-smi, lspci, /proc/cpuinfo         │  │
│  │ • Detect: GPU type (CUDA/ROCm/Metal), VRAM, CPU%   │  │
│  │ • Choose backend: llama.cpp (CUDA/Metal/CPU)        │  │
│  │ • Adapt command: CYNIC_NODE_CMD + --n-gpu auto      │  │
│  │ • Track: models_loaded, dogs_running                │  │
│  │                                                      │  │
│  │ Deps: std::process, sysfs parsing                   │  │
│  └─────────────────────────────────────────────────────┘  │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 3: SUPERVISE (process lifecycle)            │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • Spawn subprocess with auto-detected backend       │  │
│  │ • Watch for crashes (process_wrap detects exit)     │  │
│  │ • Auto-respawn with exponential backoff (1s→30s)    │  │
│  │ • Graceful shutdown on SIGTERM (kill child)         │  │
│  │                                                      │  │
│  │ Deps: tokio::process, process-wrap (v9)             │  │
│  └─────────────────────────────────────────────────────┘  │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 4: ANNOUNCE + VERIFY (registration + hb)    │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • POST /nodes/heartbeat (every 30s to kernel)       │  │
│  │   {node_id, hardware, models_loaded, dogs_running}  │  │
│  │ • Probe Dog health: curl /health (if exposed)       │  │
│  │ • Detect hung: 20s timeout, restart if needed       │  │
│  │ • Emit K15 observations (state changes, hardware)   │  │
│  │                                                      │  │
│  │ Deps: reqwest, tokio::time::timeout                 │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                             │
│ Configuration (from Soma via /pending_commands):            │
│  • model: "qwen-3.5-9b" (Soma decides what to load)       │
│  • backend: auto-detected (CUDA/ROCm/Metal/CPU)            │
│  • n_gpu: 1 or auto (from hardware probe)                   │
│  • timeout: per-model (Soma controls)                       │
│                                                             │
└────────────────────────────────────────────────────────────┘
         ↑ Command reception & Heartbeat ↑
         │ (bidirectional to Kernel/Soma) │
         │                                │
      Tailscale Network
         │                                │
         ↓                                ↓
    ┌──────────────────────────────────────────┐
    │ CYNIC KERNEL (Soma on Ring 2)            │
    │ • Commands: load_model, switch_model     │
    │ • State: registered_nodes, hardware_map  │
    │ • K15: observations from nodes           │
    └──────────────────────────────────────────┘
```

---

## Integration: How Soma Controls Nodes

**Soma is the orchestrator; nodes are the executors.**

```
Soma Decision Loop (every 5 min on kernel):
  1. GET /health (all dogs) → see latencies, hardware
  2. GET /nodes (all nodes) → see resource availability
  3. Decide: "Load Qwen on GPU node? Do we have VRAM?"
  4. POST /nodes/{node_id}/commands {action: "load_model", model: "qwen-3.5-9b"}
  5. Poll /nodes/{node_id}/status → wait for ACK (timeout 30s)
  6. If success: update Judge to include new Dog
  7. If failure: emit K15 observation, try different node

Node Command Execution (every 5s polling loop):
  1. GET /nodes/{node_id}/pending_commands (from kernel Soma)
  2. Dequeue command {action, model, backend_hint}
  3. Execute:
     - load_model: spawn llama-server with model path
     - switch_model: kill current, spawn new (with grace period)
     - unload_model: graceful shutdown
     - get_status: return hardware + models + processes
  4. ACK success/failure: POST /nodes/{node_id}/ack {command_id, status}
  5. Emit K15 observation: "Qwen loaded, 5.2GB VRAM used"
```

**Why Soma on Kernel, Nodes on Machines?**
- **State consistency:** Soma is the SSOT (single source of truth) for which models are where
- **Resilience:** Node dies → TTL expires → Soma reruns allocation (no manual recovery)
- **Scalability:** N nodes × 1 Soma = O(N) decision time, parallelizable
- **Observability:** K15 observations flow: node → kernel → Soma (closed loop)

---

## State Machine: Node Lifecycle

```
                           ┌─────────────┐
                           │    START    │
                           └──────┬──────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
                    ↓                           ↓
            ┌───────────────────┐    ┌──────────────────┐
            │  BOOT PHASE       │    │  ERROR: Exit     │
            │                   │    │  (fatal config)  │
            │ • Load node_id    │    └──────────────────┘
            │ • Probe hardware  │
            │   (GPU, CPU, RAM) │
            │ • Detect backend  │
            │   (CUDA/Metal)    │
            └───────┬───────────┘
                    │
                    ↓
        ┌─────────────────────────────────┐
        │  COMMAND LOOP (Main)            │
        │  Every 5s:                      │
        │  • GET /pending_commands        │
        │    from Soma                    │
        │                                 │
        │  Parallel:                      │
        │  • Poll for new commands        │
        │  • Execute commands             │
        │  • Monitor child (→ respawn)    │
        │  • Heartbeat (→ hardware)       │
        │                                 │
        │  Loop: every 5s, forever        │
        └──────────────┬──────────────────┘
                       │
        ┌──────────────┴──────────────┬──────────┐
        │                             │          │
        │ Command: load_model         │ SIGTERM  │
        │ (from Soma)                 │ (user)   │
        │                             │          │
        ↓                             ↓          ↓
    ┌──────────────┐        ┌───────────────┐  │
    │SPAWN DOG     │        │GRACEFUL       │  │
    │(subprocess)  │        │SHUTDOWN       │  │
    │              │        │               │  │
    │• Start       │        │• Kill child   │  │
    │  llama-server│        │• Wait 10s     │  │
    │• Auto-detect │        │• Force kill   │  │
    │  backend     │        │• Emit obs     │  │
    │• Monitor     │        │• Exit(0)      │  │
    │  exit code   │        └───────────────┘  │
    │• Respawn on  │                           │
    │  crash       │                           │
    │  (1→30s exp) │                           │
    └──────┬───────┘                           │
           │                                   │
           ↓                                   │
    ┌──────────────────────────────┐           │
    │ANNOUNCE (Heartbeat w/        │           │
    │Hardware Snapshot)            │           │
    │                              │           │
    │Every 30s:                    │           │
    │POST /nodes/heartbeat         │           │
    │{                             │           │
    │  node_id,                    │           │
    │  hardware: {gpu, cpu, ram},  │           │
    │  models_loaded,              │           │
    │  dogs_running                │           │
    │}                             │           │
    │                              │           │
    │K15 Obs: "GPU 5.2GB used"    │           │
    └──────┬───────────────────────┘           │
           │                                   │
           └─────────────┬─────────────────────┘
                         │
                         ↓
                 ┌───────────────┐
                 │     STOP      │
                 │               │
                 │ • Deregister  │
                 │ • Clean logs  │
                 │ • Exit(0)     │
                 └───────────────┘
```

---

## Example Execution: Happy Path (Soma Commands Node)

```
T+0:00  [BOOT] Load node_id=cynic-gpu-1, kernel=cynic-core:3030
        ├─ Probe hardware: nvidia-smi → RTX 4060 Ti, 8GB total, 7.2GB free ✓
        ├─ Detect backend: CUDA (NVIDIA detected) ✓
        └─ Start command loop (every 5s polling)

T+0:05  [COMMAND POLL] GET /pending_commands
        └─ Queue empty, waiting for Soma decision

T+0:10  [COMMAND POLL] GET /pending_commands
        └─ Dequeue: {action: "load_model", model: "qwen-3.5-9b"}
           Soma decided: "GPU node is idle, load Qwen"

T+0:12  [SPAWN DOG] Start llama-server subprocess
        ├─ Cmd: llama-server -m /models/qwen-3.5-9b.gguf --n-gpu 1
        ├─ PID 12345 spawned ✓
        └─ process-wrap watching exit code

T+0:15  [MODEL LOAD] llama-server ready (loading GGUF weights)
        └─ Listening on http://127.0.0.1:8080/health

T+0:30  [HEARTBEAT] POST /nodes/heartbeat
        {
          "node_id": "cynic-gpu-1",
          "hardware": {
            "gpu": {"name": "RTX 4060 Ti", "total_gb": 8, "free_gb": 2.8, "util%": 68},
            "cpu": {"cores": 8, "util%": 12},
            "ram": {"total_gb": 16, "free_gb": 11.3}
          },
          "models_loaded": ["qwen-3.5-9b"],
          "dogs_running": 1
        }
        └─ K15 Obs: "GPU 5.2GB Qwen, 68% util"

T+0:35  [COMMAND POLL] GET /pending_commands
        └─ Queue empty, Qwen is running

T+1:00  [HEARTBEAT] POST /nodes/heartbeat (same as T+0:30)
        └─ Qwen still alive, still using 68% GPU

T+1:30  [COMMAND POLL] GET /pending_commands
        └─ Dequeue: {action: "switch_model", model: "gemini-large"}
           Soma decided: "Qwen timeout on last 3 judgments, fallback to Gemini"

T+1:32  [SWITCH MODEL] Kill qwen, start gemini-large
        ├─ SIGTERM → llama-server (graceful 10s wait)
        ├─ Force kill if needed (kill -9)
        ├─ Spawn new subprocess: gemini-large wrapper
        └─ K15 Obs: "Model switched qwen→gemini (qwen_timeout_3x)"

T+2:00  [HEARTBEAT] POST /nodes/heartbeat
        {
          "models_loaded": ["gemini-large"],
          "dogs_running": 1
        }
        └─ K15 Obs: "GPU 6.1GB gemini, 72% util"

T+5:00  [HEARTBEAT] Heartbeat repeats, Soma monitors health via /health
        └─ (Pattern continues)

T+30:00 [30-MIN MARK] Node still alive, 60 heartbeats sent, models switched once
        └─ Soma has made 6 decision cycles (every 5min), adjusted allocation
        └─ K15 observations: "gpu_utilization", "model_switch", "dog_latency"
```

---

---

## Scalability & Topology

**Question: If Soma runs without kernel, what are the implications?**

**Answer: There are two topologies. We're using Centralized (Soma on kernel).**

| Aspect | Centralized Soma (Recommended) | Distributed Soma (Autonomous) |
|--------|--------------------------------|-------------------------------|
| **Soma Location** | On kernel machine (`<TAILSCALE_CORE>`) | On each machine (local) |
| **State SSOT** | Kernel (global truth) | Each node (local truth only) |
| **Decision Scope** | Which models where (all nodes) | Per-node only |
| **Conflict Risk** | Low (Soma arbitrates) | High (independent decisions) |
| **Network** | Hub-and-spoke (kernel is hub) | Mesh (P2P node commands) |
| **Scalability** | Linear O(N) | Exponential (conflict at N>10) |
| **Observability** | Closed-loop (K15 to kernel) | Fragmented (local logs only) |
| **Best For** | Production (many nodes) | Labs (isolated machines) |

**Centralized Topology (CYNIC Production):**
- Soma lives on `<TAILSCALE_CORE>` (Ring 2)
- Soma owns registered_nodes, hardware_map, model allocation
- All nodes heartbeat to kernel (Tailscale, ~5-20ms latency)
- Soma makes decisions every 5 min, broadcasts commands to all nodes in parallel
- K15 observations flow: node → kernel → Soma (feedback loop closes)
- Bottleneck: kernel observability (at 100 nodes: 3.3 req/s, well within limits)

**Cost at scale (100 nodes):**
- Heartbeats: 100 nodes × 1 heartbeat/30s = ~3.3 req/s
- Command overhead: ~0.5 req/s (1 cmd per 10 nodes per 5min decision cycle)
- Total: <4 req/s to kernel (IO-bound, CPU negligible)
- Latency: decisions 5-20s slow (wait for heartbeats from slowest node)

**Why NOT Distributed Soma?**
- No global state → 2 nodes independently load same model → GPU OOM
- No coordination → node failure invisible until user query times out
- Knowledge scattered → "where is Qwen loaded?" → must ask all nodes
- Recovery manual → "Qwen died on node-3" → human reruns decision
- Impossible to test (10 nodes × 5 decision cycles = 50 potential divergences)

---

## Timeline

**Phase B:** 2-3 sessions post-May 10
- Session 1: Hardware probing + command reception (framework)
- Session 2: Model lifecycle (load/switch/unload) + integration tests
- Session 3: K15 observations + systemd deployment

**Then Phase C (post-hackathon):** 
- Soma orchestrator (Ring 2 service)
- Dynamic Dog registration from node commands
- Multi-model load balancing + fallback routing
