# Soma Ground Truth — Infrastructure Needs (2026-05-03)

**Status:** Analysis complete. Defining what Soma must manage.

---

## Current State (Observed)

**llama-server processes:** NONE RUNNING
```bash
$ ps aux | grep llama
# (empty)
```

**Systemd units:** NONE DEFINED
```bash
$ systemctl --user list-units | grep llama
# (empty)
```

**Kernel:** RUNNING (PID 878792)
- Accepts `/judge` requests ✓
- Issues verdicts ✓
- Degrades gracefully (qwen-7b-hf fallback) ✓

**Backend configuration:**
- GPU: `http://<TAILSCALE_GPU>:8080/v1` (titou, Windows, schtasks)
- Core: DISABLED (commented out, model file missing)
- Embedding: NOT CONFIGURED (logs: "no embedding server configured")

**Actual routing:**
- HuggingFace qwen-7b-hf: ✓ responding (3182ms latency, quality 0.357)
- Deterministic-dog: ✓ responding (0ms latency, quality 0.05)
- GPU qwen35-9b-gpu: ⚠ not responding (context drift detected every 20s)
- Embedding: ✗ unreachable (disabled in pipeline)

---

## What's Missing (Gap Analysis)

### 1. Backend Lifecycle Management
**What needs to happen:** llama-server processes must be started, monitored, restarted on failure.

**Current state:** No systemd units, no startup script, no health monitoring.

**Evidence:**
- No processes running despite kernel expecting them
- Kernel detects "CONTEXT DRIFT" = GPU backend not running correctly
- Core backend disabled (no service to launch it)
- Embedding not configured at all

**What Soma Layer 1 must do:**
- Start llama-server on GPU (<TAILSCALE_GPU>:8080) with `-c 131072` flag
- Start llama-server on Core (<TAILSCALE_CORE>:8081) for embedding with `--embedding` flag
- Verify both are healthy before signaling kernel "ready"
- Monitor both continuously (health probes every 30s)
- Restart if they crash or become unresponsive

---

### 2. Configuration Correctness
**What needs to happen:** Actual running state must match backends.toml.

**Current mismatch:**
- backends.toml says qwen35-9b-gpu context = 131072
- Actual running context = 65536 (or process not running at all)
- backends.toml has no embedding entry
- Logs say: "no embedding server configured"

**What Soma Layer 1 must do:**
- Before kernel boot, verify: configured context = actual context
- Before kernel boot, verify: all configured backends are reachable
- If mismatch detected, either:
  a) Fix the launch flags and restart
  b) Update backends.toml to match actual state
  c) Return UNRECOVERABLE error and block kernel boot

---

### 3. Dependency Discovery (Pre-flight)
**What needs to happen:** Check all backend dependencies before kernel starts.

**Current state:** Kernel starts, tries to use backends, logs failures, degrades.

**What Soma Layer 0 must do:**
```
Before /judge accepts request:
  1. Is embedding server at 8081 reachable?
  2. Is GPU backend at <TAILSCALE_GPU>:8080 reachable with correct context?
  3. Are hermes-x organs alive (heartbeat timestamp < 1h)?
  
If any check fails:
  - Log blocker
  - Return 503 or route around failure
  - Schedule recovery attempt
```

---

### 4. Explicit Fallback Routing
**What needs to happen:** When backends fail, routing must be explicit and logged.

**Current state:** Implicit graceful degradation.
- Logs show: "Dog skipped — organ quality degraded" (gemini-cli)
- Logs show: "embedding failed — cache + crystal merge disabled"
- Kernel continues with 2/4 Dogs (deterministic + qwen-7b-hf)

**What Soma Layer 2 must do:**
```
Fallback map:
  - If embedding unavailable: use FNV hash instead of semantic slug ✓ (already done)
  - If qwen35-9b-gpu unavailable: route to qwen-7b-hf ✓ (already done)
  - If qwen-7b-hf unavailable: use deterministic-dog only + BARK everything
  - If gemini-cli unavailable: skip (already done, circuit breaker active)

Log explicitly:
  - Which Dogs are available
  - Which Dogs are degraded
  - Reason (embedding? context drift? circuit breaker?)
  - Fallback strategy active
```

---

### 5. Auto-recovery & Escalation
**What needs to happen:** Detect failures, attempt remediation, escalate if unrecoverable.

**Current state:** Partial (PatternHealing verifier already running, but no llama-server restart logic).

**What Soma Layer 4 must do:**
```
Detect failure:
  - Backend unresponsive for >60s
  - Context drift persisting for >5 probes
  - Embedding server down for >30s

Attempt recovery (3 retries, 60s cooldown):
  1. Restart llama-server with correct flags
  2. Wait for health check to pass
  3. Re-probe context window
  4. If OK, return to ready state
  5. If fail, try again (up to 3 times)

Escalate if unrecoverable:
  - Log CRITICAL alert
  - Disable the Dog (circuit breaker)
  - Continue with degraded Dogs
  - Post alert to /health endpoint
```

---

## What Soma MUST Manage (Ordered by Criticality)

| Layer | Priority | What | How | Status |
|-------|----------|------|-----|--------|
| **0** | CRITICAL | Pre-flight dependency check | Probe embedding + GPU context before `/judge` | ✗ Missing |
| **1** | CRITICAL | llama-server lifecycle | Start/monitor/restart servers, validate config | ✗ Missing |
| **2** | HIGH | Graceful degradation | Explicit fallback routing, log degradation state | ✓ Partial (implicit) |
| **4** | HIGH | Auto-recovery | Detect failure, retry 3×, escalate | ✓ Partial (PatternHealing exists) |
| **3** | MEDIUM | Resource allocation | GPU budget, context tradeoffs | Deferred |

---

## Dependencies (What Must Start First)

```
1. Soma bootstrap (Layer 0 + 1) starts
   ├─ Start llama-server on GPU (http://<TAILSCALE_GPU>:8080)
   ├─ Wait for /health response
   ├─ Verify context = 131072
   ├─ Start llama-server on Core (http://<TAILSCALE_CORE>:8081)
   ├─ Wait for /health response
   └─ Report: all backends ready

2. Kernel starts
   ├─ Receives signal: backends ready
   ├─ Initializes Dogs (all 4 available)
   ├─ Initializes embedding pipeline
   └─ Accepts /judge requests with full functionality

3. Soma monitor (Layer 4) runs continuously
   ├─ Probe embedding every 30s
   ├─ Probe GPU context every 30s
   ├─ Detect failures, attempt recovery
   └─ Escalate if unrecoverable
```

**Current broken state:**
- Kernel starts immediately (no dependency check)
- Backends not running (or running with wrong config)
- Kernel detects failures mid-flight
- Degrades silently

---

## Ground Truth Summary

**Soma is NOT architecture. Soma is STABILITY ORCHESTRATION.**

Needed now:
1. **systemd units** or **manager process** to keep llama-servers alive
2. **Pre-flight probe** before kernel boot to verify backends are ready
3. **Configuration validator** to ensure actual state matches backends.toml
4. **Recovery loop** to restart failed backends and escalate on persistence

Falsifiable: If Soma Layer 0-2 deployed correctly, kernel should:
- Never boot when backends are unavailable
- Transition from degraded → healthy once backends recover
- Log degradation explicitly (not silently fall back)

---

**Confidence:** φ⁻¹ (0.618) — Ground truth observed from logs. Implementation plan pending.
