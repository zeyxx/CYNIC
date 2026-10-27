# Business-Agnostic MMORPG / Economy Model — Inference Vertical

**Status:** proposed (design only — no code, no runtime authority)
**Parent:** `docs/design/inference-universe.md`, `docs/design/cynic-finance-design.md`, `docs/design/MARKET_ARCHITECTURE.md`
**Authority:** Subordinate to `contracts/foundation.json`, `AGENTS.md`, observed runtime state
**Date:** 2026-10-18
**Scope:** Business-agnostic resource/economy abstraction for the inference vertical. No product-specific logic. No hardcoded provider, model, or host.

---

## 1. Problem Statement

CYNIC's inference vertical (`cynic-inference`) serves LLM completions across a heterogeneous fleet. Today, resource accounting is fragmented:

| Fragment | Location | Unit |
|---|---|---|
| Compute (CPU-ms) | `cynic-workload` | `AgentBudget.cpu_ms` |
| GPU (VRAM, layers) | `cynic-inference` | `vram_total`, `n_gpu_layers` |
| Memory (RAM) | `infra/compute-policy.json` (future) | `ram_mb` |
| Storage (disk) | `infra/compute-policy.json` (future) | `disk_mb` |
| Network (bandwidth) | `cynic-network` | unstructured |
| Latency | `infra/inference-profiles.json` | `tg_tok_per_s` (proxy) |
| Cost (energy, tokens) | `cynic-finance` | `ResourceUnit` (emerging) |
| Authority | `cynic-consensus` | `MandateAccount` |

No unified economy. No scarcity model. No pricing. No allocation policy that is business-agnostic — each vertical would re-invent the same wheel.

**Goal:** Define a single MMORPG-flavored economy model that any vertical (inference, storage, networking, agent coordination) can adopt. The model is **business-agnostic**: it knows nothing about LLMs, GPUs, or Solana. It knows resources, needs, constraints, and value flow.

---

## 2. Design Principles

1. **Bottom-Up over Top-Down** — Every resource type, constraint, and price must be observable from runtime state. No speculative abstraction without a measured baseline. (Foundation invariant.)
2. **Provider-Agnostic Domain** — The model must not depend on a specific LLM, browser, host, hypervisor, or cloud provider. (Foundation boundary `provider_agnostic_domain`.)
3. **Strict DAG Downward** — Economy layer depends on resource layer, never the reverse. Resource measurement is pure domain logic; side effects live behind ports.
4. **Dynamic Registries over Hardcoded Values** — Resource catalogs, price tables, and constraint profiles are registry-driven, not compiled in.
5. **Typed Sovereign Errors** — Every economy violation surfaces as a `CynicErrorCode` (domain: `Finance`, `Market`, `Pool`; nature: `Blocked | Recoverable | Constitutional | AgentResolution`).

---

## 3. Resource Model

### 3.1 Resource Types (The "Items")

Every consumable in the system is a **Resource** — a typed, measurable, depletable quantity. Resources are the MMORPG "items": potions, crystals, scrolls, ore.

| Resource | Symbol | Unit | Observable Source | MMORPG Analogy |
|---|---|---|---|---|
| Compute | `RES_COMPUTE` | CPU-ms | `cynic-workload` `AgentBudget.cpu_ms` | Mana potion |
| GPU Compute | `RES_GPU` | VRAM-GB × seconds | `cynic-inference` `vram_total`, `n_gpu_layers` | Arcane crystal |
| Memory | `RES_MEMORY` | RAM-MB × seconds | `infra/compute-policy.json` `ram_mb` | Health crystal |
| Storage | `RES_STORAGE` | Disk-MB | `infra/compute-policy.json` `disk_mb` | Inventory slot |
| Bandwidth | `RES_BANDWIDTH` | Bytes transferred | `cynic-network` | Scroll of teleportation |
| Latency Budget | `RES_LATENCY` | Milliseconds | `infra/inference-profiles.json` `tg_tok_per_s` | Swiftness buff |
| Energy | `RES_ENERGY` | Watt-hours | `cynic-host` probes | Stamina |
| Tokens | `RES_TOKEN` | `ResourceUnit` | `cynic-finance` ledger | Gold |
| Context Window | `RES_CONTEXT` | Tokens | `infra/inference-profiles.json` `ctx_target` | Spell slot |
| Time | `RES_TIME` | Seconds | `cynic-workload` `time_min` (future) | Cooldown timer |
| Actions | `RES_ACTION` | Count | `cynic-finance` `ActionMeter` | Action points |

### 3.2 Resource Properties

Each resource instance carries:

```rust
pub struct Resource {
    pub id: ResourceId,           // content-addressed
    pub class: ResourceClass,     // Compute, Gpu, Memory, ...
    pub quantity: Quantity,       // measured amount
    pub unit: ResourceUnit,       // canonical unit
    pub origin: ResourceOrigin,   // observed | derived | projected
    pub expiry: Option<Timestamp>, // None = durable
    pub provenance: Provenance,   // which node, which measurement
}
```

**ResourceOrigin** is critical for the Bottom-Up invariant:
- `Observed` — read back from runtime (authoritative)
- `Derived` — computed from observed values (e.g., cost = quantity × rate)
- `Projected` — estimated from historical patterns (lowest authority)

### 3.3 Resource Registry

A dynamic registry (`ResourceRegistry`) maps `ResourceClass → ResourceSchema`. New resource types are registered at runtime, not compiled in. This is what makes the model **business-agnostic**: a new vertical registers its own resource classes without modifying the economy core.

---

## 4. Needs & Constraints Model

### 4.1 Constraint Types (The "Debuffs" and "Buffs")

Constraints are typed requirements that gate resource consumption. They are the MMORPG "debuffs" (SLA penalties) and "buff requirements" (minimum quality).

| Constraint | Symbol | Unit | Source | MMORPG Analogy |
|---|---|---|---|---|
| SLA Deadline | `CON_SLA` | Timestamp | `cynic-workload` `time_min` (future) | Quest timer |
| Throughput Floor | `CON_TPUT` | tokens/s | `infra/inference-profiles.json` `tg_tok_per_s` | DPS requirement |
| Quality Floor | `CON_QUAL` | score [0,1] | `cynic-inference` `token_entropy` | Accuracy buff |
| Availability | `CON_AVAIL` | % uptime | `infra/compute-policy.json` `live_guard` | Uptime requirement |
| Budget Cap | `CON_BUDGET` | `ResourceUnit` | `cynic-finance` ledger | Gold cap |
| Latency Ceiling | `CON_LATENCY` | ms | `cynic-inference` routing | Lag tolerance |
| Concurrency | `CON_CONCURRENCY` | count | `cynic-inference` `slot_*` | Party size |

### 4.2 Constraint Profiles

A **ConstraintProfile** bundles constraints for a task class. Profiles are registry-driven:

```rust
pub struct ConstraintProfile {
    pub id: ProfileId,
    pub class: TaskClass,          // rust_build, embedding_bulk, ...
    pub constraints: Vec<Constraint>,
    pub priority: Priority,        // Critical | High | Normal | Low
    pub fallback: FallbackPolicy,  // Degrade | Queue | Reject
}
```

Existing task classes from `infra/compute-policy.json` map directly:

| Task Class | SLA | Throughput | Availability | Budget |
|---|---|---|---|---|
| `rust_build` | 10 min | n/a | 4 GB free floor | 6 GB RAM |
| `rust_check_light` | 5 min | n/a | 2 GB free floor | 3 GB RAM |
| `embedding_bulk` | 45 min | n/a | 1 GB free floor | 2 GB RAM |
| `embedding_probe` | 2 min | n/a | 1 GB free floor | 200 MB RAM |

### 4.3 SLA as Quest Timer

An SLA deadline is a **quest timer**: the task must complete before the deadline or it fails. The timer is set at quest creation and checked at settlement. Partial completion is scored proportionally (XP = % of quest completed).

---

## 5. Economy Model

### 5.1 Currency: ResourceUnit (The "Gold")

`cynic-finance` defines `ResourceUnit` as the canonical unit of value. All resources are denominated in `ResourceUnit` via an **ExchangeRateTable**:

```rust
pub struct ExchangeRate {
    pub from: ResourceClass,   // e.g., Compute
    pub to: ResourceClass,     // e.g., Token
    pub rate: Rate,            // measured, not hardcoded
    pub origin: RateOrigin,    // observed | derived | projected
    pub timestamp: Timestamp,
    pub provenance: Provenance,
}
```

Rates are **measured from observed usage**, not hardcoded. The `cynic-finance` `RateCalculator` (future) computes rates from historical resource consumption and outcome quality.

### 5.2 Pricing: Dynamic Scarcity

Price emerges from **scarcity** — the ratio of demand to available supply:

```
Price(resource) = BaseRate(resource) × ScarcityMultiplier(resource)

ScarcityMultiplier = 1 + (Demand / Supply - 1) × Elasticity
```

When demand exceeds supply, price rises. When supply exceeds demand, price falls. This is the MMORPG auction house: everyone needs the same GPU, so GPU time costs more.

**Scarcity sources:**
- **Temporal** — peak hours (all agents active) vs. off-peak
- **Spatial** — one node has GPU, others don't
- **Structural** — a resource class is inherently rare (e.g., VRAM on CPU-only nodes)

### 5.3 Allocation: The Auction

Resource allocation is a **sealed-bid auction**:

1. **Quest** is posted with `ConstraintProfile` and `BudgetCap`
2. **Agents** (players) bid: `Bid { agent_id, resource_ids, price, eta }`
3. **Auctioneer** (market contract) selects winner by `Score = Quality / Price`
4. **Resources** are reserved → consumed → settled
5. **Payment** flows from agent's `ResourceUnit` balance to resource provider

This is the `cynic-market` futarchy mechanism applied to resource allocation: agents bid on tasks, the market selects the best bid, and the outcome is measured against reality.

### 5.4 Scarcity Enforcement

When a resource is scarce:

| Scarcity Level | Policy | MMORPG Analogy |
|---|---|---|
| Normal | First-come, first-served | Common herb |
| Elevated | Price × 1.5, queue allowed | Uncommon ore |
| High | Price × 3, auction required | Rare crystal |
| Critical | Price × 10, authority approval | Legendary item |

### 5.5 Settlement: Double-Entry

Every resource flow is a **double-entry transaction** (from `cynic-finance`):

```
Debit:  Agent-123 ResourceUnit  -100  (cost of GPU time)
Credit: Node-456  ResourceUnit  +100  (payment for GPU time)
```

No creation, no destruction. Conservation of value is enforced by the ledger invariant.

---

## 6. MMORPG Metaphor

### 6.1 Agents as Players

Every CYNIC agent is a **Player** with:

| Player Attribute | CYNIC Source | MMORPG Analogy |
|---|---|---|
| `agent_id` | `cynic-protocol` | Character name |
| `ResourceUnit` balance | `cynic-finance` ledger | Gold |
| `MandateAccount` | `cynic-consensus` | Guild rank |
| `AllowanceGrant` (future) | `cynic-consensus` | Inventory access |
| `EScoreSignal` | `cynic-intel` | Reputation |
| `AgentChannel.initiator_balance` | `cynic-channel` | Wallet |
| Skill tree | `cynic-workload` `agent_skill` | Talent tree |
| Level | Derived from `EScoreSignal` | Character level |

### 6.2 Resources as Items

Every resource is an **Item** with rarity, durability, and provenance:

| Item Property | CYNIC Source | MMORPG Analogy |
|---|---|---|
| `ResourceClass` | `ResourceRegistry` | Item type (weapon, potion, ore) |
| `Quantity` | measured | Stack size |
| `Expiry` | `Option<Timestamp>` | Consumable vs. durable |
| `Provenance` | node + measurement | Crafted by / dropped by |
| `Origin` | `Observed \| Derived \| Projected` | Common / Rare / Epic |

### 6.3 Quests as Tasks

Every task is a **Quest** with difficulty, rewards, and deadlines:

| Quest Property | CYNIC Source | MMORPG Analogy |
|---|---|---|
| `TaskClass` | `infra/compute-policy.json` | Quest type |
| `ConstraintProfile` | `ConstraintProfile` | Difficulty + requirements |
| `BudgetCap` | `cynic-finance` | Reward pool |
| `time_min` | `infra/compute-policy.json` | Quest timer |
| `Priority` | `ConstraintProfile.priority` | Normal / Elite / Raid |
| `FallbackPolicy` | `ConstraintProfile.fallback` | Fail / Retry / Degrade |

**Quest tiers:**

| Tier | Task Class | Reward Multiplier | Party Size |
|---|---|---|---|
| Normal | `embedding_probe` | ×1 | 1 |
| Elite | `embedding_bulk` | ×2 | 1-2 |
| Raid | `rust_build` | ×5 | 2-4 (multi-node) |
| Legendary | cross-vertical federation | ×10 | 4+ (fleet-wide) |

### 6.4 Guilds as Agent Fleets

A **Guild** is a coordinated group of agents sharing resources and reputation:

| Guild Property | CYNIC Source | MMORPG Analogy |
|---|---|---|
| Guild ID | `cynic-consensus` federation contract | Guild name |
| Members | `cynic-protocol` agent registry (future) | Guild roster |
| Shared treasury | `cynic-finance` `ResourceLedger` | Guild bank |
| Guild rank | Derived from aggregate `EScoreSignal` | Guild level |
| Federation contract | `cynic-consensus` | Guild charter |

### 6.5 Crafting as Model Publishing

**Crafting** is the process of transforming raw resources into higher-value artifacts:

| Crafting Input | Process | Output | CYNIC Source |
|---|---|---|---|
| Base model + training data | Fine-tuned model | Published model | `cynic-inference` `model_sync_pipeline` |
| Model + validation | Verified model | Trusted model | `cynic-inference` `backend_verification` |
| Agent + task history | Reputation signal | Authority | `cynic-intel` `EScoreSignal` |

### 6.6 Auction House as Resource Exchange

The **Auction House** is the `cynic-market` mechanism where agents trade resources:

- **Sell orders** — agents offer resources (e.g., "I have 10 GB VRAM free for 1 hour")
- **Buy orders** — agents request resources (e.g., "I need 5 GB VRAM for 30 min")
- **Matching** — `cynic-market` `market_loop` matches buy/sell by price and constraint
- **Settlement** — `cynic-finance` double-entry ledger records the transfer

---

## 7. Vertical Integration

The model is **business-agnostic**: any vertical can adopt it by registering its own resource classes and constraint profiles.

### 7.1 Inference Vertical (Reference Implementation)

| MMORPG Concept | Inference Implementation |
|---|---|
| Player | Inference agent (`cynic-inference` `chat_service`) |
| Item (Mana) | GPU VRAM (`RES_GPU`) |
| Item (Health) | RAM (`RES_MEMORY`) |
| Item (Gold) | `ResourceUnit` (`cynic-finance`) |
| Quest | Inference request (`chat_service` → `inference_gateway`, future) |
| Quest timer | SLA deadline (`time_min`) |
| Auction | Routing policy (`routing_policy` → `fleet_node`) |
| Guild | Inference fleet (`cynic-inference` `fleet_node`) |
| Crafting | Model publishing (`model_sync_pipeline`) |

### 7.2 Storage Vertical (Future)

| MMORPG Concept | Storage Implementation |
|---|---|
| Player | Storage agent (`cynic-storage`) |
| Item (Ore) | Disk space (`RES_STORAGE`) |
| Item (Scroll) | Bandwidth (`RES_BANDWIDTH`) |
| Quest | Replication task |
| Guild | Storage pool (`cynic-pool`) |

### 7.3 Network Vertical (Future)

| MMORPG Concept | Network Implementation |
|---|---|
| Player | Network agent (`cynic-network`) |
| Item (Teleport scroll) | Bandwidth (`RES_BANDWIDTH`) |
| Item (Swiftness) | Latency budget (`RES_LATENCY`) |
| Quest | Packet delivery SLA |
| Guild | Mesh network (`cynic-network` `FleetNetworkReport`) |

---

## 8. Crate Mapping

The model maps to existing CYNIC crates without requiring new code:

| Model Concept | Existing Crate | Module |
|---|---|---|
| Resource types | `cynic-finance` | `types.rs` (`ResourceUnit`, `ResourceTransaction`) |
| Resource registry | `cynic-finance` | `ports.rs` (`ComputeMeter`, `GpuMeter`, ...) |
| Exchange rates | `cynic-finance` | `rate_calculator.rs` |
| Ledger | `cynic-finance` | `ledger.rs` |
| Market / auction | `cynic-market` | `market_loop.rs`, `scoring.rs` |
| Pool rewards | `cynic-pool` | `reward.rs`, `difficulty.rs` |
| Workload / quests | `cynic-workload` | `workspace_claim.rs`, `settlement.rs` |
| Inference resources | `cynic-inference` | `math/roofline.rs`, `config_loader.rs` |
| Authority | `cynic-consensus` | `MandateAccount`, `AllowanceGrant` (future) |
| Reputation | `cynic-intel` | `EScoreSignal` |
| Task classes | `infra/compute-policy.json` | `task_classes` |
| Model profiles | `infra/inference-profiles.json` | `profiles` |

---

## 9. Invariants

1. **Conservation of Value** — Every resource debit has a corresponding credit. No creation, no destruction. (Enforced by `cynic-finance` double-entry.)
2. **Bottom-Up Authority** — `Observed` > `Derived` > `Projected`. No resource claim without a measurement.
3. **Scarcity Reflects Reality** — Price is a function of measured supply/demand, not a hardcoded constant.
4. **Provider Agnostic** — No resource class names a specific provider, model, or host.
5. **Typed Errors** — Every economy violation is a `CynicErrorCode`, never a free-form string.
6. **DAG Downward** — Economy depends on resource measurement, never the reverse.

---

## 10. Open Questions

1. **Rate oracle** — Who measures exchange rates? `cynic-finance` `RateCalculator` (future) is the candidate, but the measurement cadence and authority model are undefined.
2. **Auction cadence** — Real-time (per-request) vs. batch (per-time-window)? Inference latency requirements may force real-time.
3. **Reputation stake** — Should agents stake `ResourceUnit` as collateral for quest acceptance? Slashing on failure?
4. **Cross-vertical settlement** — If inference pays storage, which ledger settles? `cynic-finance` is the candidate L2 domain crate.
5. **Federation economy** — Do guilds have independent economies, or is there a single fleet-wide `ResourceUnit`?

---

## 11. Non-Goals

- **No product-specific logic** — This model does not know what an LLM is, what a GPU is, or what Solana is.
- **No runtime authority** — This is a design document. No code, no contracts, no registry changes.
- **No implementation plan** — Emergence order is deferred to a separate execution plan.
- **No new crates** — The model maps to existing crates. New crates only if a stable boundary cannot be expressed in existing types.

---

## 12. References

- `docs/design/inference-universe.md` — Inference vertical boundaries
- `docs/design/cynic-finance-design.md` — Resource ledger and exchange rate design
- `docs/design/MARKET_ARCHITECTURE.md` — Futarchy and market mechanism
- `contracts/foundation.json` — Authority, boundaries, invariants
- `infra/compute-policy.json` — Task classes and live guard
- `infra/inference-profiles.json` — Model profiles
- `AGENTS.md` — Gates, build policy, git conventions
