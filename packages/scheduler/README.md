# CYNIC Scheduler

> φ-weighted transaction scheduling for Solana validators

## Overview

CYNIC Scheduler is an external scheduler for Agave validators that integrates CYNIC reputation scores (K-Score, E-Score) into transaction prioritization.

```
┌───────────────┐       ┌─────────────────┐
│  tpu_to_pack  │       │ progress_tracker│
└───────┬───────┘       └───────┬─────────┘
        │                       │
        ▼                       ▼
    ┌───────────────────────────────┐
    │      CYNIC SCHEDULER          │
    │  - φ-weighted prioritization  │
    │  - Reputation lookup          │
    │  - GROWL filtering            │
    └─▲─────────▲───────────────▲───┘
      │         │               │
   ┌──▼──┐   ┌──▼──┐        ┌───▼──┐
   │wrkr1│   │wrkr2│  ...   │wrkrN │
   └─────┘   └─────┘        └──────┘
```

## Features

- **φ-weighted Priority**: Transactions from high-reputation wallets get boosted by φ (1.618)
- **GROWL Filtering**: Automatically drop transactions from flagged addresses
- **E-Score Gating**: Optionally require minimum reputation to transact
- **Reputation Caching**: Smart caching to minimize API latency
- **Scheduler Bindings**: Compatible with Agave scheduler-bindings API

## Verdict Multipliers

| Verdict | Multiplier | Effect |
|---------|------------|--------|
| WAG     | φ (1.618)  | Priority boost |
| HOWL    | 1.0        | Neutral |
| BARK    | φ⁻¹ (0.618) | Priority reduction |
| GROWL   | 0.0        | Transaction dropped |

## Installation

```bash
# Build from source
cd packages/scheduler
cargo build --release

# Install binary
cargo install --path .
```

## Usage

```bash
# Start with default config
cynic-scheduler

# With custom CYNIC endpoint
CYNIC_URL=https://cynic-mcp.onrender.com cynic-scheduler

# With API key
CYNIC_API_KEY=cynic_sk_xxx cynic-scheduler

# Full configuration
CYNIC_URL=https://cynic-mcp.onrender.com \
CYNIC_API_KEY=cynic_sk_xxx \
CYNIC_MAX_QUEUE_SIZE=100000 \
CYNIC_BATCH_SIZE=64 \
CYNIC_NUM_WORKERS=4 \
CYNIC_ENABLE_GROWL_FILTER=true \
CYNIC_ENABLE_WAG_BOOST=true \
CYNIC_MIN_E_SCORE=0 \
CYNIC_LOG_LEVEL=info \
cynic-scheduler
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CYNIC_URL` | `https://cynic-mcp.onrender.com` | CYNIC MCP server URL |
| `CYNIC_API_KEY` | - | API key for authentication |
| `CYNIC_MAX_QUEUE_SIZE` | 100000 | Maximum transactions in queue |
| `CYNIC_BATCH_SIZE` | 64 | Batch size for workers |
| `CYNIC_NUM_WORKERS` | 4 | Number of worker threads |
| `CYNIC_API_TIMEOUT_MS` | 100 | API timeout in milliseconds |
| `CYNIC_ENABLE_GROWL_FILTER` | true | Drop GROWL transactions |
| `CYNIC_ENABLE_WAG_BOOST` | true | Boost WAG transactions |
| `CYNIC_MIN_E_SCORE` | 0 | Minimum E-Score required |
| `CYNIC_LOG_LEVEL` | info | Log level (trace/debug/info/warn/error) |

## Integration with Agave

Requires Agave with scheduler-bindings support (v3.0+).

```bash
# Configure shared memory paths
export CYNIC_TPU_TO_PACK_SHM=/cynic_tpu_to_pack
export CYNIC_PACK_TO_WORKER_SHM_PREFIX=/cynic_pack_to_worker_
export CYNIC_WORKER_TO_PACK_SHM_PREFIX=/cynic_worker_to_pack_
export CYNIC_PROGRESS_SHM=/cynic_progress

# Start scheduler
cynic-scheduler &

# Start Agave validator with scheduler-bindings
agave-validator \
  --scheduler-bindings \
  --scheduler-tpu-to-pack-shm $CYNIC_TPU_TO_PACK_SHM \
  ...
```

## Statistics

The scheduler logs statistics every 10 seconds:

```
CYNIC Scheduler Stats:
├─ Slot: 123456 (Leader: yes)
├─ Queue: 5432 txs
├─ TPU → Queue: 100000 received
├─ Queue → Workers: 95000 sent
├─ Results: 95000 (94500 ok, 500 fail)
├─ Dropped (GROWL): 1234
├─ Boosted (WAG): 8765
├─ Reduced (BARK): 2345
└─ CYNIC API: 50000 calls, 45000 cache hits
```

## Philosophy

> "φ distrusts φ"
>
> Maximum confidence: 61.8%
> Loyal to truth, not to comfort.

## License

MIT
