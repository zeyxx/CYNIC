# CYNIC Node + Soma Orchestrator — System Architecture

> **Context:** Phase A (TTL janitor) is foundational for Soma's Dog lifecycle management. Diagram shows the integration points.

---

## System Layers (High to Low)

```
┌─────────────────────────────────────────────────────────────┐
│ CLIENT LAYER (REST + MCP)                                   │
│ - POST /judge {token, domain, context}                      │
│ - GET /dogs (all registered Dogs, including from nodes)     │
│ - GET /nodes (all registered nodes, hardware, models)       │
│ - GET /health (circuit breaker, Dog quality, node status)   │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────v──────────────────────────────────────┐
│ JUDGE LAYER (Consensus + Filtering)                         │
│ - ArcSwap<Judge> (atomic Dog roster swaps)                  │
│ - filter_dogs_by_request() (explicit only, no forced)       │
│ - evaluate() (3-way consensus, φ-bounded)                   │
│ - HealthGate per Dog (circuit breaker)                       │
│ - Dogs registered by: config + nodes (dynamic)              │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────v──────────────────────────────────────┐
│ ORCHESTRATOR LAYER (Soma — Phase C)                         │
│ - registered_nodes: RwLock<Map<String, RegisteredNode>>     │
│ - hardware_map: {node_id → GPU/CPU/RAM snapshot}             │
│ - model_allocation: {model → [node_ids]}                     │
│ - Decision loop: every 5 min                                │
│   1. Measure: Dog latencies, node hardware, failures         │
│   2. Decide: which models to load where                      │
│   3. Command: POST /nodes/{id}/pending_commands              │
│   4. Wait: ACK from nodes (timeout 30s)                      │
│   5. Update Judge: add new Dogs from nodes                   │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────v──────────────────────────────────────┐
│ ROSTER MANAGEMENT LAYER (Phase A — TTL Janitor)            │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ registered_dogs: RwLock<Map<String, RegisteredDog>>   │  │
│ │  ├─ name: String                                       │  │
│ │  ├─ registered_at: Instant                             │  │
│ │  ├─ last_heartbeat: Instant (refreshed by client)      │  │
│ │  └─ ttl_secs: u64 (default 120)                        │  │
│ │                                                         │  │
│ │ PHASE A MISSING: Background TTL Janitor                │  │
│ │  └─ Every 30s: scan registered_dogs                    │  │
│ │     ├─ Find expired entries (now - last_hb > ttl)      │  │
│ │     ├─ Remove from registered_dogs map                 │  │
│ │     ├─ Rebuild Judge (without expired Dogs)            │  │
│ │     └─ Emit observation ("Dog expired: {id}")          │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ Endpoints:                                                   │
│  - POST /dogs/register → add to registered_dogs + Judge     │
│  - POST /dogs/{id}/heartbeat → refresh last_heartbeat TTL   │
│  - DELETE /dogs/{id} → remove from both                      │
│  - GET /dogs → list active Dog IDs (from Judge)             │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────v──────────────────────────────────────┐
│ DOG IMPLEMENTATIONS (Config + Dynamic)                       │
│                                                              │
│ CONFIG DOGS (Permanent, no TTL):                             │
│  ├─ deterministic-dog (heuristics, 0ms latency)             │
│  ├─ qwen-7b-hf (HF Inference API)                           │
│  ├─ qwen35-9b-gpu (RTX 4060 Ti, Tailscale)                  │
│  └─ qwen-9b-core (CPU + Vulkan, Tailscale)                  │
│                                                              │
│ DYNAMIC DOGS (Registered at runtime, TTL-managed):          │
│  ├─ Local subprocess (supervised by Node binary, Phase B)    │
│  ├─ Remote API (base_url, Phase C = push mode)              │
│  └─ Custom domain specialist (calibrated on test set)        │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────v──────────────────────────────────────┐
│ SOMA ORCHESTRATOR (FUTURE — Coordinates Dogs + Resources)   │
│                                                              │
│ Responsibilities:                                            │
│  ├─ Resource pool (GPU vram, CPU%, memory)                  │
│  ├─ Dog lifecycle (which Dogs to spawn, when to kill)       │
│  ├─ Load balancing (route to healthy Dog)                   │
│  ├─ Fallback routing (qwen35 down? use qwen7)               │
│  ├─ Cache warming (pre-load models on GPU startup)          │
│  └─ Orphan management (detect unresponsive Dogs)            │
│                                                              │
│ Integration with Phase A:                                   │
│  ├─ Soma watches /health → sees circuit breaker state       │
│  ├─ Soma receives Dog expiration obs → adjusts pool         │
│  ├─ Phase A TTL janitor auto-removes dead Dogs              │
│  └─ Soma can POST /dogs/register + DELETE /dogs/{id}        │
│     to dynamically manage roster                            │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────v──────────────────────────────────────┐
│ INFRASTRUCTURE (Fleet, Processes, Storage)                  │
│                                                              │
│ KERNEL (cynic-core, Tailscale):                             │
│  ├─ REST API (port 3030)                                    │
│  ├─ Circuit breaker state (per Dog, observable)             │
│  ├─ Observation producer (K15 seam)                         │
│  └─ ArcSwap Judge (atomic roster swaps)                      │
│                                                              │
│ NODES (cynic-node binary, Phase B):                         │
│  ├─ Supervise local Dog subprocess (ProcessGroup/JobObject) │
│  ├─ POST heartbeat every 30s (keep-alive)                   │
│  └─ Auto-respawn on crash (with exponential backoff)        │
│                                                              │
│ FLEET:                                                       │
│  ├─ cynic-core (kernel, llama-server on GPU)                │
│  └─ cynic-gpu (RTX 4060 Ti, Windows schtask)                │
│  └─ cynic-node-1, cynic-node-2, ... (future)                │
└──────────────────────────────────────────────────────────────┘
```

---

## Phase A: TTL Background Janitor

**What exists:**
- `registered_dogs: RwLock<Map>` stores TTL metadata
- Endpoints: `/dogs/register`, `/dogs/{id}/heartbeat`, `DELETE /dogs/{id}`
- Each Dog has `ttl_secs` (default 120) + `last_heartbeat` timestamp

**What's missing (Phase A implementation):**
```rust
// Background task (spawn on kernel startup)
tokio::spawn(async {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        
        // 1. Read registered_dogs, find expired entries
        let mut map = state.registered_dogs.write().unwrap();
        let now = Instant::now();
        let mut expired = Vec::new();
        
        for (dog_id, entry) in map.iter() {
            if now.duration_since(entry.last_heartbeat).as_secs() > entry.ttl_secs {
                expired.push(dog_id.clone());
            }
        }
        
        // 2. Remove expired Dogs
        for dog_id in &expired {
            map.remove(dog_id);
            tracing::info!("Dog expired (TTL): {}", dog_id);
        }
        drop(map);
        
        // 3. Rebuild Judge if any dogs expired
        if !expired.is_empty() {
            let current = state.judge.load_full();
            if let Some(new_judge) = Judge::without_dogs(&current, &expired) {
                state.judge.store(Arc::new(new_judge));
                
                // 4. Emit observation (K15 producer)
                for dog_id in &expired {
                    let _ = state.observe.emit_event(Observation {
                        tool: "kernel/roster",
                        target: "judge",
                        domain: "infrastructure",
                        context: format!("Dog expired (no heartbeat): {}", dog_id),
                        tags: vec!["dog_lifecycle", "ttl_expiration"],
                    }).await;
                }
            }
        }
    }
});
```

**Why this matters for Soma:**
1. **Auto-cleanup:** Soma doesn't need to manually remove dead Dogs—TTL janitor handles it
2. **Observability:** K15 observations ("Dog expired") feed Soma's decision engine
3. **Orchestration:** Soma can dynamically register/deregister Dogs; Phase A ensures consistency
4. **Resilience:** If a Dog process dies without heartbeat, it's auto-removed instead of deadlocking the Judge

---

## Integration with Soma (Post-Phase A)

Once Phase A is live, Soma orchestrator can:

```
Soma Decision Cycle (every 5 min):
  1. GET /health → see circuit breaker states, Dog latencies
  2. If Dog X is degraded (>80% failures) → POST DELETE /dogs/X
  3. If Dog pool <3 healthy → POST /dogs/register {new_dog}
  4. Watch observations → adjust resource allocation
  5. Emit decision log (which Dogs spawned/killed, why)
```

This creates a **feedback loop:**
- Phase A (Phase A) = foundation: TTL + auto-cleanup
- Soma = intelligent orchestrator: decisions + lifecycle
- K15 = observability: decisions emit observations → improve future decisions

---

## Edge Cases Addressed by Phase A

| Case | Phase A Handling |
|------|-----------------|
| **Dog crashes, no heartbeat** | TTL expiration → auto-remove → Judge rebuild |
| **Network partition** | Node can't heartbeat → TTL expires → Soma sees observation |
| **Thundering herd** | Expire stale Dogs in batch (30s tick), one rebuild |
| **Config Dogs vs Dynamic** | Config Dogs skip TTL (permanent), Dynamic Dogs have TTL |
| **Deregister while expired** | DELETE checks registered_dogs, returns 404 if already expired |

---

---

## Scalability: Centralized vs Distributed Soma

**User Question: If Soma runs without kernel on individual machines, what are the implications?**

We designed TWO topologies. We're deploying CENTRALIZED (Soma on kernel). Here's why:

**CENTRALIZED SOMA (Production):**
```
                   ┌─────────────────────────────┐
                   │ CYNIC KERNEL                │
                   │ - Soma orchestrator (Ring 2)│
                   │ - registered_nodes state    │
                   │ - hardware_map (all nodes)  │
                   │ - Judge (all Dogs)          │
                   │ - K15 observations          │
                   └────────────┬────────────────┘
                                │ Tailscale (5-20ms)
                   ┌────────────┼────────────────┐
                   │            │                │
                ┌──v──┐      ┌──v──┐          ┌─v──┐
                │GPU-1│      │Node2│ ··· ····  │N100│
                │(cmd)│      │(cmd)│          │(cmd)│
                │(hb) │      │(hb) │          │(hb) │
                └─────┘      └─────┘          └─────┘

Soma Decision: "GPU has 6GB free, load Qwen"
  → POST /nodes/GPU-1/pending_commands {action, model}
  ← All nodes heartbeat (3.3 req/s at N=100)
  → K15 obs: "Qwen loaded, GPU 5.2GB"
  → Update Judge with new Dog
```

**Scalability at N nodes:**
- Heartbeats: O(N) per 30s
- Command overhead: O(1) per 5min (batched to all nodes)
- Decision time: O(N) (wait for slowest heartbeat)
- Bottleneck: **not the network**, but **when to make decisions** (Soma batches every 5min)

**Example at N=100:**
- 100 heartbeats / 30s = 3.3 req/s (well within kernel capacity)
- Soma decision: "load on 10 nodes in parallel"
- ACK wait: max 30s per command (with timeout)
- New Dogs appear in Judge within 1 min
- Total latency: heartbeat + decision + command + Dog registration = ~2 min

**DISTRIBUTED SOMA (Anti-pattern, avoided):**
```
Each machine autonomous (no kernel):
  ┌──────────┐ ┌──────────┐ ┌──────────┐
  │GPU-1     │ │Node2     │ │Node3     │
  │Soma(local)│ │Soma(local)│ │Soma(local)│
  │ "load"   │ │ "load"   │ │ "load"   │
  │ Qwen ✓   │ │ Qwen ✓   │ │ Qwen ✓   │  ← ALL load same model!
  │ 5.2GB GPU│ │ GPU OOM! │ │ 3.1GB GPU│  ← GPU-2 crashes
  └──────────┘ └──────────┘ └──────────┘

Problems:
  1. Independent decisions → resource conflicts (GPU OOM)
  2. No global truth → "where is Qwen?" requires asking all nodes
  3. Failure invisible → GPU-2 dies, no central observer detects
  4. Knowledge scattered → recovery is manual per node
  5. Impossible to reason about → 100 nodes × independent decisions
```

**Why Centralized Wins:**
- Soma is the SSOT (single source of truth)
- Conflicts resolved by one orchestrator
- K15 observations close the loop (node → kernel → Soma)
- Resilience: node dies → TTL expires → Soma reruns allocation
- Observability: all hardware snapshots visible to Soma
- Testing: reproducible decisions (same inputs → same command)

**Cost of Centralization:**
- Kernel becomes a bottleneck IF Soma can't keep up (mitigated by batching)
- Kernel downtime = no new commands (mitigation: caching commands locally, fallback to last known state)
- Tailscale dependency (but you already have it)

**When to Reconsider Distributed:**
- Private fleet <5 machines (autonomous is simpler, no coordination cost)
- Isolated labs with no shared resources (no conflict risk)
- **NOT for production >10 machines**

---

## Timeline

**Phase A (Done):** TTL background task + integration test

**Phase B (Next: 2-3 sessions):**
- cynic-node binary: hardware probing, command reception, Dog supervision
- Integration tests: load model, switch model, hardware monitoring

**Phase C (Post-May 10):**
- Soma orchestrator on kernel (decision engine, command broadcaster)
- Node discovery endpoint (GET /nodes with hardware)
- Dynamic Dog registration from node commands

**Phase D (Post-hackathon):**
- Fallback routing (if Qwen times out, try Gemini)
- Cache warming (preload models on GPU startup)
- Multi-tenant isolation (resource quotas per client)
