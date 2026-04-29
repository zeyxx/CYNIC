# CYNIC Node Phase B — Binary Lifecycle & Integration

> **What:** Standalone binary (cynic-node) supervises a local Dog subprocess, registers it with the kernel, and keeps it alive. Zero kernel dependencies (except HTTP).

---

## Phase B Architecture: 3 Concerns

```
┌────────────────────────────────────────────────────────────┐
│ cynic-node Binary (Runs on local machine/container)        │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 1: SUPERVISE (process lifecycle)            │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • Spawn subprocess: llama-server, ollama, custom     │  │
│  │ • Watch for crashes (process_wrap detects exit)     │  │
│  │ • Auto-respawn with exponential backoff (1s→30s)    │  │
│  │ • Graceful shutdown on SIGTERM (kill child)         │  │
│  │                                                      │  │
│  │ Deps: tokio::process, process-wrap (v9)             │  │
│  └─────────────────────────────────────────────────────┘  │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 2: ANNOUNCE (registration + heartbeat)      │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • POST /dogs/register {name, base_url, model}       │  │
│  │ • On success: TTL = 120s, last_heartbeat = now()    │  │
│  │ • Every 30s: POST /dogs/{id}/heartbeat              │  │
│  │ • On 404: auto re-register (kernel restarted)       │  │
│  │ • On failure: backoff, retry, log alert             │  │
│  │                                                      │  │
│  │ Deps: reqwest, tokio::time::interval                │  │
│  └─────────────────────────────────────────────────────┘  │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ CONCERN 3: VERIFY (identity + health)               │  │
│  ├─────────────────────────────────────────────────────┤  │
│  │ • Probe subprocess health (curl /health if exposed) │  │
│  │ • Verify model matches manifest (metadata check)    │  │
│  │ • Detect hung processes (20s timeout)               │  │
│  │ • Emit observations (state changes, degradation)    │  │
│  │                                                      │  │
│  │ Deps: reqwest, tokio::time::timeout                 │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                             │
│ Configuration (env vars or TOML):                           │
│  CYNIC_NODE_NAME=my-dog-1                                  │
│  CYNIC_NODE_CMD=llama-server -m /models/qwen-9b.gguf      │
│  CYNIC_NODE_BASE_URL=http://127.0.0.1:8080                │
│  CYNIC_NODE_KERNEL_ADDR=http://cynic-core:3030            │
│  CYNIC_NODE_API_KEY=<bearer_token>                         │
│  CYNIC_NODE_HEALTH_PROBE=/health (optional)                │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

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
            ┌───────────────┐         ┌───────────────┐
            │  BOOT PHASE   │         │  ERROR: Exit  │
            │               │         │  (fatal)      │
            │ • Load config │         └───────────────┘
            │ • Validate    │
            │ • Start watch │
            │   threads     │
            └───────┬───────┘
                    │
                    ↓
            ┌───────────────┐
            │  SUPERVISE    │
            │  (start child)│
            │               │
            │ [Running Loop]│
            │ • Monitor     │
            │   exit code   │
            │ • Auto-restart│
            │   on crash    │
            └───────┬───────┘
                    │
                    ↓
            ┌───────────────────────────────┐
            │  ANNOUNCE (Register + HB)     │
            │                               │
            │ • POST /dogs/register         │
            │   {name, base_url, model}     │
            │                               │
            │ Outcomes:                     │
            │  ├─ 200 OK → REGISTERED       │
            │  ├─ 409 Conflict → re-use TTL│
            │  ├─ 404 Not Found → re-boot   │
            │  └─ 5xx → backoff, retry      │
            └───────┬───────────────────────┘
                    │
                    ↓
        ┌─────────────────────────────────┐
        │  REGISTERED (HEARTBEAT LOOP)    │
        │                                 │
        │  Every 30s:                     │
        │  • POST /dogs/{id}/heartbeat    │
        │  • Refresh TTL (120s → now)     │
        │                                 │
        │  Parallel:                      │
        │  • Monitor child process (→ 1)  │
        │  • Verify health probe (→ 3)    │
        │  • Detect hangs (→ restart)     │
        │  • SIGTERM → graceful shutdown  │
        │                                 │
        │  Loop: every 30s, forever       │
        └─────────────┬───────────────────┘
                      │
        ┌─────────────┴────────────────┬──────────┐
        │                              │          │
        │ Child dies                   │ SIGTERM  │
        │ (respawn)                    │ (user)   │
        │                              │          │
        ↓                              ↓          ↓
   ┌─────────┐                   ┌──────────┐   │
   │BACKOFF  │                   │GRACEFUL  │   │
   │RESTART  │                   │SHUTDOWN  │   │
   │         │                   │          │   │
   │1→30s ex-│                   │• Kill    │   │
   │pone     │                   │  child   │   │
   │         │                   │• Wait    │   │
   │Return   │                   │  10s     │   │
   │to →SUPERV│                  │• Force   │   │
   │          │                   │  kill    │   │
   └──────────┘                   │• Exit(0)│   │
                                  └──────────┘   │
                                                 │
                            ┌────────────────────┘
                            │
                            ↓
                    ┌───────────────┐
                    │     STOP      │
                    │               │
                    │ • Deregister? │
                    │   (DEL /dogs) │
                    │ • Clean logs  │
                    │ • Exit(0)     │
                    └───────────────┘
```

---

## Example Execution: Happy Path (30 minutes)

```
T+0:00  [BOOT] Load config: name=my-qwen, cmd=llama-server, kernel=cynic-core:3030
        ├─ Validate CYNIC_NODE_NAME ✓
        ├─ Validate CYNIC_NODE_CMD ✓
        └─ Start supervise + heartbeat threads

T+0:05  [SUPERVISE] Spawned child PID 12345
        └─ process-wrap watching exit

T+0:10  [ANNOUNCE] POST /dogs/register {name: "my-qwen", base_url: "http://127.0.0.1:8080"}
        └─ Response 200 OK: ttl=120s, registered_at=T+0:10

T+0:30  [HEARTBEAT] POST /dogs/my-qwen/heartbeat
        └─ Response 200 OK: ttl_remaining=100s

T+0:31  [VERIFY] Probe subprocess health (curl http://127.0.0.1:8080/health)
        └─ Response 200: model="qwen-3.5-9b", status="ready"

T+1:00  [HEARTBEAT] POST /dogs/my-qwen/heartbeat
        └─ Response 200 OK: ttl_remaining=60s

T+1:30  [HEARTBEAT] POST /dogs/my-qwen/heartbeat
        └─ Response 200 OK: ttl_remaining=30s

T+2:00  [HEARTBEAT] POST /dogs/my-qwen/heartbeat
        └─ Response 200 OK: ttl_remaining=120s (refreshed)

        ... (repeats every 30s)

T+30:00 [HEARTBEAT] Still alive after 30 minutes
        └─ Total heartbeats sent: 60
        └─ Total registration refreshes: 2
        └─ Child process uptime: 30 min
```

---

## Timeline

**Phase B:** 2-3 sessions post-May 10
- Session 1: supervisor + config
- Session 2: announcer + integration tests
- Session 3: verifier + systemd deployment

**Then Phase C (post-hackathon):** WebSocket push mode for NAT-agnostic nodes.
