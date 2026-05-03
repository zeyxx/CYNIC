# Soma Layer 1 — Backend Orchestration (Phase 1)

> Minimal, manifest-driven orchestrator for llama.cpp inference backends.

## Design Philosophy

**Data-centric, not speculative:**
- Current scope: 3 backends (cynic-core 8080 reasoning, cynic-core 8081 embedding, cynic-gpu 8080)
- Manifest = single source of truth (what SHOULD be running)
- Recovery = SYS4-compliant (verify actual state, don't trust command success)
- Measurement: 7-day live test, falsifiable (zero OOM = success)

**Organic growth:**
- Phase 1 (now): Manifest + apply loop for llama.cpp
- Phase 2 (when needed): Add Kairos/Hermes budgets if contention measured
- No premature abstraction (trait system) until real constraints prove it necessary

## Files

| File | Purpose |
|------|---------|
| `soma_manifest.toml` | Declarative resource config: backends, budgets, recovery strategies |
| `soma_orchestrator.py` | Runtime: probe → enforce → recover → observe |
| `__init__.py` | Python package entry point |

## Quick Start

### 1. Install dependencies

```bash
cd /home/user/Bureau/CYNIC
pip install -r cynic-python/requirements.lock
```

Verify `toml` is installed (should be in requirements.lock already).

### 2. Test the orchestrator

```bash
# Initial probe + summary (doesn't loop)
python3 -m soma.soma_orchestrator --probe-only

# Output:
# ✓ Loaded manifest with 3 backends
# Initial health check:
# {
#   "timestamp": 1234567890.0,
#   "backends": {
#     "reasoning_core": {"is_alive": true, "latency_ms": 45, ...},
#     "embedding_core": {"is_alive": true, "latency_ms": 50, ...},
#     "reasoning_gpu": {"is_alive": true, "latency_ms": 120, ...}
#   }
# }
```

### 3. Start monitoring loop (blocking, runs forever)

```bash
python3 -m soma.soma_orchestrator

# Output:
# ✓ Loaded manifest with 3 backends
# Initial health check:
# ...
# Starting monitor loop...
# --- Iteration 1 ---
# ✓ reasoning_core: alive (45ms, model=Qwen3.5-9B-Q4_K_M)
# ✓ embedding_core: alive (50ms, model=Qwen3-Embedding-0.6B-Q8_0)
# ✓ reasoning_gpu: alive (120ms, model=Qwen3.5-9B-Q4_K_M)
# ...
```

### 4. Deploy as systemd service (with integrity check)

**IMPORTANT: Run integrity check FIRST before systemd deployment**

```bash
# Step 1: Verify integrity (generates soma_integrity.sha256)
cd /home/user/Bureau/CYNIC/cynic-python/soma
python3 soma_integrity.py
# Should output: ✓ INTEGRITY CHECK PASSED

# Step 2: Create user-level systemd service
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/soma-orchestrator.service << 'EOF'
[Unit]
Description=CYNIC Soma Layer 1 Orchestrator
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=/home/user/Bureau/CYNIC/cynic-python/soma
ExecStart=/usr/bin/python3 soma_orchestrator.py
Restart=on-failure
RestartSec=10
StartLimitInterval=300
StartLimitBurst=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

# Step 3: Pre-flight validation (SYS3 rule)
systemctl --user daemon-reload
systemctl --user start soma-orchestrator
sleep 2
systemctl --user status soma-orchestrator
journalctl --user -xeu soma-orchestrator | tail -20

# If status shows "Active: active (running)" → safe to enable
systemctl --user enable soma-orchestrator

# Step 4: Monitor
journalctl --user -xeu soma-orchestrator
```

**What integrity check does:**
- ✓ Verifies soma_manifest.toml hash (detects manifest poisoning)
- ✓ Verifies soma_orchestrator.py hash (detects code tampering)
- ✓ Checks file permissions (rejects world-readable/executable)
- ✓ Rejects symlinks (symlink attack vector)
- ✓ Fails loud if ANY check fails (refuses to start orchestrator)

## How It Works

### 1. Parse Manifest

Load `soma_manifest.toml` into memory:
- 3 backends: reasoning_core, embedding_core, reasoning_gpu
- Recovery strategies: systemctl (cynic-core), windows schtasks + SSH kill (cynic-gpu)
- Monitoring config: probe every 30s, timeout 5s

### 2. Health Check Loop (Every 30s)

For each backend:
1. GET `http://{host}:{port}/health`
2. If 200 + alive → record latency, model, context_actual
3. If error/timeout → record failure

### 3. Enforce Manifest

For each backend where manifest says `enabled = true`:
1. If probe shows dead → trigger recovery
2. Run recovery strategy (systemctl OR SSH kill + schtasks)
3. Wait for post-restart delay
4. Probe again to verify success
5. Log recovery event

### 4. Observability

Every 300s (configurable):
- Build observation payload: backend_state, recovery_events, latency
- POST to kernel `/observe` (stub for now, real code reads .cynic-env for API key)

## Configuration

Edit `soma_manifest.toml` to:

**Change probe interval:**
```toml
[monitoring]
probe_interval_seconds = 60  # was 30
```

**Disable a backend:**
```toml
[[backends]]
name = "embedding_core"
enabled = false  # Soma won't monitor or recover it
```

**Add a new backend (Phase 2):**
```toml
[[backends]]
name = "hermes_browser"
enabled = true
host = "<TAILSCALE_CORE>"
port = 9090
...
```

Then add recovery strategy:
```toml
[[recovery_strategies]]
backend_name = "hermes_browser"
restart_method = "systemctl"
systemd_service = "hermes-browser.service"
```

## Falsifiable Test (7 Days)

**Hypothesis:** Manifest-driven orchestrator prevents VRAM contention crashes.

**Measurement:**
- Run Soma + behavioral analytics for 7 days
- Measure: OOM crashes, embedding timeouts, context drift

**Success criteria:**
- Zero OOM crashes (both backends stay within budget)
- Zero embedding /timeout (no requests dropped)
- GPU context matches manifest (131K, not 65K drift)

**If criteria met:**
- Design works, no trait system needed yet
- Move to Phase 2: add Kairos/Hermes when they contend

**If criteria fail:**
- Extend design: add VRAM limiter, request batching, or dynamic budget reallocation
- Then re-run 7-day test

## SYS4 Rule Implementation

GPU backend recovery enforces **SYS4 (Remote Task Restart Validation):**

```python
# Kill FIRST (don't trust process exit)
ssh S.@<TAILSCALE_GPU> taskkill /IM llama-server.exe /F /T /TIMEOUT:5

# Wait for full exit
await asyncio.sleep(10)

# THEN start (fresh process, not inherited parent)
ssh S.@<TAILSCALE_GPU> schtasks /run /tn CynicSovereign

# Verify actual state (not just command success)
await self._probe_backend("reasoning_gpu")
if actual_context != expected_context:
    # Still broken, escalate
    self.logger.error("Context drift persists, SYS4 violation")
```

This prevents the context drift bug (old process inherited 65K, new task never started).

## Next Steps (Phase 2)

When behavioral analytics or Kairos reports VRAM pressure:

1. **Add VRAM limiter**
   - Parse `/health` response for memory usage
   - If cynic-core exceeds budget → kill lower-priority service
   - Example: If reasoning > 6GB, kill embedding temporarily

2. **Add request batching**
   - Queue embedding requests when reasoning is high-load
   - Forward to embedding_gpu if available (Phase 3)

3. **Add dynamic budget reallocation**
   - Read `behavioral_profile.json` peak_hours
   - Adjust budgets: off-peak 4GB reasoning + 2GB embed, peak 6GB reasoning + 1GB embed

These are PRs (feature commits), not hackathons. Only build if measurement proves need.

## Troubleshooting

**"✗ reasoning_core: timeout after 5s"**
- Embedding or reasoning server crashed
- Check: `systemctl --user status llama-server@8080`
- Manual restart: `systemctl --user restart llama-server@8080`

**"✗ reasoning_gpu: HTTP 401"**
- GPU backend endpoint requires auth but no API key configured
- Temporary: accept degraded (context validation skipped)
- Fix: Pass GPU API key to orchestrator (later)

**"Context drift after restart: expected 131072, got 65536 (SYS4 violation)"**
- Windows schtask didn't fully kill old process, new task inherited parent context
- Orchestrator will attempt recovery on next failure
- Root: schtasks doesn't wait for process death, needs pre-kill

**"Recovery FAILED for reasoning_core, escalating to alert"**
- Restart tried but backend still dead
- Check systemd service: `journalctl --user -xeu llama-server@8080 | tail -50`
- May be model file corruption, OOM, or permission issue

## Testing

### Unit test (stub)

```python
# tests/test_soma_manifest.py
def test_manifest_loads():
    orchestrator = SomaOrchestrator(Path("soma_manifest.toml"))
    assert len(orchestrator.backends) == 3
    assert "reasoning_core" in orchestrator.backends
```

### Integration test (manual, 7 days)

```bash
# Terminal 1: Run orchestrator
python3 -m soma.soma_orchestrator

# Terminal 2: Run behavioral analytics
python3 cynic-python/domains/validation/improved_behavior_detection.py

# Monitor for crashes
journalctl --user -xeu soma-orchestrator | grep -E "Recovery|FAILED|Error"

# After 7 days: report results
```

## References

- SYS4 Rule (systemd.md): Remote task restart verification
- K15 Consumer: K15 seam closure (observation → kernel)
- soma_manifest.toml: Full configuration reference
