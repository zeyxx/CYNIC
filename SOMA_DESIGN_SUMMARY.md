# Soma — Missing Resource Orchestrator (Summary)

**Status:** DESIGN PHASE  
**Discovery:** 2026-05-03 during protocol wiring verification  
**Impact:** Kernel boot failures, no graceful degradation, manual recovery required  
**Timeline:** Design + implementation 6-8h (post-Phase-2 priority)

---

## Why Soma Emerged Today

Protocol wiring uncovered missing piece:
- Session hook checks kernel_health=true
- Kernel fails to boot (context drift on GPU backend)
- No automated recovery → Phase 2 gate blocked

**Root cause:** No orchestrator to manage backend dependencies before kernel starts.

---

## Soma Architecture (4 Layers)

```
Soma (Missing Orchestrator)
├── Layer 0: Dependency Discovery
│   └── Pre-boot probe all backends (GPU, embedding, CPU)
├── Layer 1: Backend Lifecycle
│   └── Start/stop llama-servers with correct flags
├── Layer 2: Graceful Degradation
│   └── Fallback to healthy Dogs if one fails
├── Layer 3: Resource Allocation
│   └── GPU budget, context tradeoffs, watchdogs
└── Layer 4: Failure Recovery
    └── Detect flapping, remediate, escalate
```

---

## Current Blockers (Detailed Analysis)

**See:** `~/.cynic/organisms/artifacts/deferred/soma/SOMA_EMERGENCE_2026_05_03.md`

Summarized:
1. GPU backend context drift (65K vs 131K configured)
2. Embedding server unreachable
3. Kernel boot failure (circuit breaker triggers)

---

## Immediate Workaround

**See:** `~/.cynic/organisms/artifacts/deferred/soma/KERNEL_BOOT_FIX_2026_05_03.md`

Disable GPU Dog, boot on qwen-7b-hf (90.5% success rate):
1. Edit `~/.config/cynic/backends.toml` (comment out GPU backend section)
2. Restart kernel
3. Verify kernel is healthy before Phase 2

Kernel will boot for Phase 2 measurement without GPU inference.

---

## Design Questions (Open)

- What triggers Soma startup? (system boot, kernel hook, or standalone?)
- How does Soma report status to Kernel? (coordination file, env vars, extended /health?)
- How does Soma recover a dead backend? (systemd restart, script, operator escalation?)
- Expected boot sequence? (Soma first, then Kernel?)

See design doc for full discussion.

---

## Why Soma is Necessary (Not Optional)

1. **Scale:** Multiple cortexes need resource budgets
2. **Resilience:** Graceful degradation when backends fail
3. **Measurement:** Which Dog/cortex pair is most cost-effective?
4. **Operator burden:** Manual intervention for every failure is unsustainable

---

## Timeline

| When | What | Blocked By |
|------|------|-----------|
| Now (May 3) | Workaround: disable GPU Dog | Nothing — apply immediately |
| May 5-6 | Phase 2 measurement | Kernel must boot (workaround applies) |
| May 7+ | Phase 2 cleanup + Phase 3 planning | Phase 2 results |
| Post-Phase-2 | Soma design + implementation | Design prioritization |

---

## Confidence

**Soma existence:** OBSERVED (emerged from kernel boot failure)  
**Architecture (Layers 0-4):** INFERRED (from kernel needs + multi-cortex coordination)  
**Timeline (6-8h):** ESTIMATED (similar-scale orchestration projects)  
**Immediate workaround:** DEDUCED (disable failing component, use backup)

**Overall confidence:** φ⁻¹ (0.618) — architecture is sound, but Soma hasn't been implemented yet.

---

## Full Analysis

See external storage (protocol artifact tree):
- `~/.cynic/organisms/artifacts/deferred/soma/SOMA_EMERGENCE_2026_05_03.md` — detailed analysis
- `~/.cynic/organisms/artifacts/deferred/soma/KERNEL_BOOT_FIX_2026_05_03.md` — workaround + proper fix
