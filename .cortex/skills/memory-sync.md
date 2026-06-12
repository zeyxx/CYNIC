# /memory-sync — Force Organism State Audit

**Usage:** `/memory-sync` (force full memory update against live organism state)

**When to use:**
- Before session end (handoff audit)
- Before major demo/deployment
- When you suspect MEMORY.md is stale
- As verification that memory reflects reality

**What it does:**

1. **Run organism-probe.sh** → captures live state (kernel, git, hermes, llama, system)
2. **Haiku synthesis** → compares against MEMORY.md, identifies stale/missing entries
3. **Update memories** → writes new observations about infrastructure, status, blockers
4. **Report** → shows what changed, what's outdated, what action is needed

**Output:**

```
✓ Organism Health Audit
  
Kernel: HEALTHY (5/6 dogs, crystal backlog 47)
Hermes-X: 4/4 crons running, 23K farming_log events
K15 Loop: Consumer=active, Producer=active (Seam 1+2 OK)
Git: main@a67bfd73, 3 dirty files (cymbal_findings.py, config.toml, .env)
Llama: running (pool=3 models, qwen-9b active)

Memory Sync:
  ✓ Updated: project_infrastructure.md (llama pool size, cynic-gpu temps)
  ✗ STALE: project_ccm_kill_chain.md (references 278/1934, now 312/2104)
  ✗ MISSING: session context (LSTM training on cynic-core, ETA 21:00 check)
  
Next: commit these updates to .cortex/projects/-home-user-Bureau-CYNIC/memory/
```

**Permissions required:**
- Read: `.cortex/projects/*/` (memory index)
- Read: `MEMORY.md`
- Exec: `scripts/organism-probe.sh`
- Exec: Agent `organism-health-monitor`
- Write: `.cortex/projects/*/memory/*.md` (update memories)

**Cost:** ~$0.004 (Haiku synthesis only, probes free)

**Blocking:** None. Fire and forget. Memory updates are non-blocking.

---

**Implementation notes:**

This skill is **user-facing** — gives visibility into what MEMORY.md tracks vs what's real. Builds trust that memory stays synchronized.
