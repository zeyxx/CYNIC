# Soma L1 Alpha — Resource Orchestration

**Soma L1 Alpha** prevents GPU starvation between high-contention tasks (Hermes agent, nightshift Dog evaluations).

## Architecture

Soma L1 has 4 levers (Soma design from CYNIC Constitution):

1. **GPU budget allocation** — reserved slots per task priority
2. **Task priority** — Hermes > Nightshift > Background (from ResourceRequest)
3. **Load awareness** — probe llama-server utilization before dispatch
4. **Audit trail** — observe decisions to `/observe domain=soma` (TODO)

Currently implemented: **Lever 3 (load awareness) + basic Lever 2 (priority-based backoff)**

## Public API: `POST /soma/request`

**Request:**
```json
{
  "task_name": "hermes-chat-1",
  "priority": "hermes",
  "estimated_duration_secs": 300,
  "llama_url": "http://127.0.0.1:8080"
}
```

**Response (200) — Allocate immediately:**
```json
{
  "decision": "allocate",
  "data": {
    "slot_id": "hermes-chat-1-1777805919853"
  }
}
```

**Response (200) — Queue with backoff:**
```json
{
  "decision": "queue",
  "data": {
    "wait_secs": 5
  }
}
```

### Logic

- If GPU utilization **< 80%** (from llama-server `/health`): return `allocate` with unique slot_id
- If GPU utilization **≥ 80%**: return `queue` with priority-based wait:
  - **Hermes** (priority 2): 5 seconds
  - **Nightshift** (priority 1): 15 seconds
  - **Background** (priority 0): 30 seconds

### Fallback Behavior

If llama-server is unreachable: estimate utilization = 0.5 (conservative), allocate if healthy.

## Integration Points

### 1. Nightshift (Kernel-side) ✓ LIVE

Integrated in `cynic-kernel/src/infra/tasks/nightshift.rs::judge_commit()`:

```rust
let gate_req = ResourceRequest {
    task_name: format!("nightshift-commit-{}", &commit.hash[..7]),
    priority: Priority::Nightshift,
    estimated_duration_secs: 120,
    llama_url: llama_url.to_string(),
};

let decision = soma_gate.request(gate_req).await;
match decision {
    GateDecision::Queue { wait_secs } => {
        return Err(format!("GPU saturated — retry in {}s", wait_secs));
    }
    GateDecision::Allocate { slot_id } => {
        // proceed to judge
    }
}
```

**Activation:** Uncommented in `main.rs` with `LLAMA_SERVER_URL` env var (defaults to `http://127.0.0.1:8080`).

---

### 2. Hermes (External Python Agent) — TODO

Hermes agent polls `/agent-tasks?kind=hermes&limit=10` to fetch pending tasks.

**Integration point:** Before spawning the actual task execution, call `/soma/request`:

```python
# hermes_agent_task_executor.py (pseudocode)

async def dispatch_task(task: AgentTask):
    # Consult Soma gate
    soma_req = {
        "task_name": task.id,
        "priority": "hermes",
        "estimated_duration_secs": 300,  # typical chat runtime
        "llama_url": os.getenv("LLAMA_SERVER_URL", "http://127.0.0.1:8080")
    }
    
    resp = httpx.post(f"{KERNEL_URL}/soma/request", json=soma_req, 
                      headers={"Authorization": f"Bearer {API_KEY}"})
    decision = resp.json()
    
    if decision["decision"] == "allocate":
        slot_id = decision["data"]["slot_id"]
        # Proceed with execution, using slot_id for observability
        await execute_hermes_chat(task, slot_id)
    else:
        # Queue: reschedule
        wait_secs = decision["data"]["wait_secs"]
        await asyncio.sleep(wait_secs)
        # Re-queue the task or retry
```

**Why external?** Hermes runs outside the kernel (autonomous agent). The gate is REST-accessible, so Hermes can check it before dispatch without blocking the kernel.

---

## Observation Emission (Lever 4) — TODO

Each gate decision should be observed for audit trail:

```rust
// In domain/orchestrator.rs or via soma.rs handler
kernel_events.emit({
    "domain": "soma",
    "event_type": decision_type,  // "allocate" or "queue"
    "task_name": request.task_name,
    "priority": request.priority,
    "utilization": current_utilization,
    "wait_secs": wait_secs_if_queued,
    "timestamp": now(),
})
```

This feeds K15 seam 2 (producer-consumer audit) and enables metrics on contention patterns.

---

## Production Refinements (Alpha → Beta)

1. **Actual utilization parsing** — currently returns conservative 0.5 if reachable.
   - Parse llama-server response for queue depth, active inference sessions, etc.
   - Region: `probe_llama_utilization()` in `domain/orchestrator.rs`

2. **Satellite probes** — multiple llama-server endpoints (GPU, CPU, remote)
   - Gate per endpoint (route allocation decision by server capacity)
   - Current: single endpoint assumption

3. **Memory + context budgeting** — reserve slots by estimated token consumption
   - Extend `ResourceRequest` with `context_tokens`, `max_output_tokens`
   - Allocate from a shared 128K context pool

4. **Preemption / priority escalation** — if Hermes starves >10min, raise to priority 3
   - Requires task tracking (current gate is stateless)

---

## Testing

Unit test in `domain/orchestrator.rs`:

```rust
#[tokio::test]
async fn gate_allocates_when_utilization_low() {
    let gate = ResourceGate::new();
    let req = ResourceRequest {
        task_name: "hermes-1".into(),
        priority: Priority::Hermes,
        estimated_duration_secs: 300,
        llama_url: "http://127.0.0.1:19999".into(),  // unreachable → 0.5 default
    };
    match gate.request(req).await {
        GateDecision::Allocate { slot_id } => {
            assert!(slot_id.starts_with("hermes-1"));
        }
        _ => panic!("expected allocate"),
    }
}
```

Integration test (post-hackathon):
- Spin up llama-server at 100% utilization
- Submit concurrent Hermes + Nightshift requests
- Verify Hermes gets allocate, Nightshift gets queue with 15s backoff
- Verify observation emission to `/observe domain=soma`

---

## Disabled (Hackathon)

Nightshift was paused 2026-04-26→05-11 (GPU reserved for Hermes). Re-enabled 2026-05-03 with Soma gating.

See `TODO.md` and `memory/project_orchestration_fractal.md` for prior contention analysis.
