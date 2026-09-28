# cynic-finance — Design Doc

> **Status**: Proposal (not yet validated)
> **Authority**: Subordinate to `contracts/foundation.json` and observed runtime state
> **Date**: 2026-09-28

---

## 1. Problem Statement

CYNIC is becoming a computational agent OS. Agents consume multiple resource types (compute, storage, network, energy, tokens, context, time, actions). Today these are fragmented:

| Resource | Fragment | Location |
|---|---|---|
| Tokens / credits | `TreasuryAccount.cyd_calibration_credits` | `cynic-consensus` |
| Compute | `Budget.cpu_ms` | `cynic-agent-bus` |
| GPU (Vulkan/ROCm/CUDA) | `ComputeBackend::Vulkan`, `vram_gb` | `cynic-host-probes` |
| Channel balances | `AgentChannel.initiator_balance` | `cynic-channel` |
| Pool rewards | reward distribution | `cynic-pool` |
| Payment requests | `SolanaPayRequest` | `cynic-solana-pay` |
| Authority | `MandateAccount`, `AllowanceGrant` | `cynic-consensus`, `cynic-solana-pay` |
| Reputation | `EScoreSignal` | `cynic-intel-escore` |

No unified ledger records all resource flows. No canonical unit of value. No exchange rate table. Each crate owns its fragment with no cross-domain settlement.

---

## 2. Desired State

`cynic-finance` is a **pure L2 domain crate** that:

1. **Owns canonical resource types** — `ResourceLedger`, `ResourceUnit`, `ResourceTransaction`, `ExchangeRate`, `ExchangeRateTable`
2. **Defines typed ports** for each resource meter — `ComputeMeter`, `StorageMeter`, `NetworkMeter`, `EnergyMeter`, `TokenMeter`, `ContextMeter`, `TimeMeter`, `ActionMeter`
3. **Enforces invariants** — append-only, monotone consumption, idempotent transactions, double-entry semantics (every debit has a credit)
4. **Provides exchange rate mechanism** — measured rates, not hardcoded; rates emerge from observed usage
5. **Serves as settlement layer** — other crates (consensus, channel, pool) consume `cynic-finance` types for cross-domain value flows

---

## 3. Boundaries

### 3.1 What cynic-finance OWNS

- Canonical resource unit types and schemas
- Ledger transaction types and validation rules
- Exchange rate table structure and update rules
- Double-entry accounting invariants
- Resource transaction lifecycle (reserve → consume → settle → archive)

### 3.2 What cynic-finance does NOT own

- **Measurement** — how to read CPU-ms, bytes, watts (lives in `cynic-host`, `cynic-host-probes`, `cynic-network`, `cynic-storage`)
- **Execution** — how to spend tokens, transfer balances (lives in `cynic-consensus`, `cynic-channel`, `cynic-solana-pay`)
- **Policy** — who can spend what, budgets, mandates (lives in `cynic-protocol`, `cynic-agent-bus`)
- **Observation** — host metrics, fleet state (lives in `cynic-host`, `cynic-observer`)

### 3.3 Crate Dependency Graph

```
cynic-finance (L2, pure)
  ├── depends on: cynic-protocol (L0, types only)
  ├── consumed by: cynic-consensus (L2), cynic-channel (L2), cynic-pool (L2), cynic-solana-pay (L2)
  └── ports implemented by: cynic-host (L1), cynic-node (L3)
```

---

## 4. Core Types

### 4.1 ResourceUnit

```rust
pub enum ResourceUnit {
    ComputeCpuMs(u64),
    ComputeGpuMs(u64),
    ComputeVramBytes(u64),
    StorageBytes(u64),
    StorageIops(u64),
    NetworkBytes(u64),
    NetworkPackets(u64),
    EnergyWatts(f64),
    EnergyKwh(f64),
    TokenCyd(u64),
    ContextTokens(u64),
    TimeSeconds(u64),
    Actions(u64),
}
```

Each variant is measured independently. No implicit conversion.

### 4.2 ResourceTransaction

```rust
pub struct ResourceTransaction {
    pub schema: &'static str,           // "cynic.finance.resource_transaction.v1"
    pub transaction_id: String,
    pub timestamp: u64,                 // unix ms
    pub subject: String,                // agent_id or node_id
    pub resource: ResourceUnit,
    pub direction: TransactionDirection, // Debit | Credit
    pub counterparty: String,           // destination account
    pub proof: String,                  // hash of evidence
    pub metadata: ResourceMetadata,
}

pub enum TransactionDirection {
    Debit,   // consumption
    Credit,  // allocation / reward
}
```

### 4.3 ResourceLedger

```rust
pub struct ResourceLedger {
    pub ledger_id: String,
    pub resource_type: ResourceType,
    pub entries: Vec<ResourceTransaction>,  // append-only
    pub balance: ResourceUnit,              // derived, not stored independently
}
```

Invariants:
- Append-only: entries never modified or deleted
- Monotone: balance only decreases for debits, only increases for credits
- Idempotent: same `transaction_id` applied twice = no-op
- Double-entry: every debit has a corresponding credit (sum of debits = sum of credits)

### 4.4 ExchangeRate

```rust
pub struct ExchangeRate {
    pub schema: &'static str,           // "cynic.finance.exchange_rate.v1"
    pub from: ResourceUnit,
    pub to: ResourceUnit,
    pub rate: f64,                      // 1 from = rate to
    pub measured_at: u64,               // unix ms
    pub sample_count: u64,              // how many observations
    pub confidence: f64,                // 0.0 - 1.0
}
```

Rates are **measured**, not hardcoded. They emerge from observed usage patterns.

### 4.5 ExchangeRateTable

```rust
pub struct ExchangeRateTable {
    pub schema: &'static str,           // "cynic.finance.exchange_rate_table.v1"
    pub rates: Vec<ExchangeRate>,
    pub last_updated: u64,
}
```

---

## 5. Ports (Traits)

```rust
pub trait ComputeMeter {
    fn measure_cpu_ms(&self, agent_id: &str) -> Result<u64, MeterError>;
}

pub trait GpuMeter {
    fn measure_gpu_ms(&self, agent_id: &str) -> Result<u64, MeterError>;
    fn measure_vram_bytes(&self, agent_id: &str) -> Result<u64, MeterError>;
    fn measure_gpu_utilization(&self, agent_id: &str) -> Result<f64, MeterError>;
}

pub trait StorageMeter {
    fn measure_bytes(&self, agent_id: &str) -> Result<u64, MeterError>;
    fn measure_iops(&self, agent_id: &str) -> Result<u64, MeterError>;
}

pub trait NetworkMeter {
    fn measure_bytes(&self, agent_id: &str) -> Result<u64, MeterError>;
    fn measure_packets(&self, agent_id: &str) -> Result<u64, MeterError>;
}

pub trait EnergyMeter {
    fn measure_watts(&self, agent_id: &str) -> Result<f64, MeterError>;
    fn measure_kwh(&self, agent_id: &str) -> Result<f64, MeterError>;
}

pub trait TokenMeter {
    fn measure_cyd(&self, agent_id: &str) -> Result<u64, MeterError>;
}

pub trait ContextMeter {
    fn measure_tokens(&self, agent_id: &str) -> Result<u64, MeterError>;
}

pub trait TimeMeter {
    fn measure_seconds(&self, agent_id: &str) -> Result<u64, MeterError>;
}

pub trait ActionMeter {
    fn measure_actions(&self, agent_id: &str) -> Result<u64, MeterError>;
}
```

Ports are implemented by adapters in `cynic-host` (L1) or `cynic-node` (L3). The domain crate stays pure.

---

## 6. Invariants

1. **Append-only**: Ledger entries are never modified or deleted. Corrections are new entries.
2. **Monotone**: Balances only decrease (debits) or increase (credits). No arbitrary jumps.
3. **Idempotent**: Same `transaction_id` applied twice is a no-op.
4. **Double-entry**: Every debit has a corresponding credit. Sum of debits = sum of credits.
5. **Measured rates**: Exchange rates are computed from observed samples, never hardcoded.
6. **Explicit unknown**: A missing meter returns `MeterError::Unknown`, never a fabricated value.
7. **Zero-I/O domain**: No `std::process`, no `reqwest`, no `tokio::spawn` in the domain crate.

---

## 7. Relation to Existing Crates

| Existing crate | What it owns today | Migration path |
|---|---|---|
| `cynic-consensus` | `TreasuryAccount` | Consumes `ResourceLedger<TokenCyd>` for treasury operations |
| `cynic-channel` | `AgentChannel` balances | Consumes `ResourceLedger<TokenCyd>` for channel settlements |
| `cynic-pool` | Reward distribution | Consumes `ResourceTransaction` for reward accounting |
| `cynic-solana-pay` | `SolanaPayRequest` | Consumes `ResourceTransaction` for payment receipts |
| `cynic-agent-bus` | `Budget` | Consumes `ResourceLedger<ComputeCpuMs>` etc. for budget enforcement |
| `cynic-protocol` | `MandateAccount` | Unchanged — policy layer, not resource layer |

---

## 8. Exchange Rate Measurement

Rates are measured, not hardcoded. The measurement process:

1. **Observe**: Collect samples of resource consumption and token flows over time
2. **Correlate**: Match resource consumption to token flows (e.g., agent X consumed Y CPU-ms and was charged Z CYD)
3. **Compute**: Rate = tokens / resource_unit, averaged over N samples
4. **Confidence**: Based on sample count and variance
5. **Update**: Rates refresh periodically, with explicit `measured_at` timestamp

Initial rates are `ExchangeRate::Unknown` until sufficient samples exist.

---

## 9. Implementation Plan

### Phase 1 — Skeleton (this session)
- [ ] Create `crates/cynic-finance/` with `Cargo.toml`
- [ ] Define core types in `src/types.rs`
- [ ] Define ports in `src/ports.rs`
- [ ] Add `tests/architecture_boundary.rs`
- [ ] Add unit tests for invariants

### Phase 2 — Instrumentation (next)
- [ ] Implement `ComputeMeter` adapter in `cynic-host`
- [ ] Implement `GpuMeter` adapter in `cynic-host-probes` (Vulkan, ROCm, CUDA)
- [ ] Implement `StorageMeter` adapter in `cynic-storage`
- [ ] Implement `NetworkMeter` adapter in `cynic-network`
- [ ] Implement `EnergyMeter` adapter in `cynic-host`
- [ ] Collect baseline measurements

### Phase 3 — Integration (future)
- [ ] Migrate `cynic-consensus` TreasuryAccount to use `ResourceLedger`
- [ ] Migrate `cynic-channel` settlements to use `ResourceTransaction`
- [ ] Migrate `cynic-pool` rewards to use `ResourceTransaction`
- [ ] Build `ExchangeRateTable` from observed data

---

## 10. Open Questions

1. **Granularity**: Per-agent ledger or per-node ledger? (Proposal: per-agent, aggregated to node)
2. **Persistence**: Where are ledgers stored? (Proposal: `cynic-storage` WAL)
3. **Consensus**: Do ledger entries need fleet consensus? (Proposal: local first, federate later)
4. **Migration**: How to migrate existing `TreasuryAccount` balances? (Proposal: genesis transaction)
5. **Exchange rate oracle**: Who computes rates? (Proposal: `cynic-finance` pure function, `cynic-node` triggers)

---

## 11. References

- `contracts/foundation.json` — constitutional invariants
- `infra/ARCHITECTURE_CONSTELLATIONS.md` — crate taxonomy
- `docs/CYNIC_FEDERATED_ECONOMIC_ARCHITECTURE.md` — economic flow model
- `docs/CYNIC_PROTOCOL_TOKEN_PAPER.md` — token design
- Stripe Ledger pattern — double-entry bookkeeping for distributed systems
- MegaETH resource accounting — multi-dimensional resource tracking
