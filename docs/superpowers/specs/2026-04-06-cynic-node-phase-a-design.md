# cynic-node Phase A: Kernel Dog Lifecycle Management

**Status:** Approved (CYNIC HOWL Q=0.563)
**Scope:** Kernel-only changes. No new binary.
**Estimate:** ~200 lines across 7 files.

## Problem

Dogs registered via `POST /dogs/register` have no lifecycle management. They persist in the roster forever, even after the backend crashes or the node shuts down. The kernel has no way to expire or remove them.

This blocks Phase B (cynic-node binary) and Phase C (push mode) — both require the kernel to manage Dog lifetimes.

## Design

### New Endpoints

#### `POST /dogs/{id}/heartbeat`

Refresh the TTL of a registered Dog. Lightweight (no recalibration).

**Auth:** Bearer token required.
**Response 200:**
```json
{ "dog_id": "gemma-4-e4b-gpu", "status": "alive", "ttl_remaining_secs": 95 }
```
**Response 404:** Dog not found in registered_dogs map. Node must re-register.
**Response 400:** Invalid dog_id format.

#### `DELETE /dogs/{id}`

Remove a dynamically registered Dog from the roster. Atomic via ArcSwap.

**Auth:** Bearer token required.
**Response 200:**
```json
{ "dog_id": "gemma-4-e4b-gpu", "status": "deregistered", "roster_size": 4 }
```
**Response 404:** Dog not found (already expired or never registered).

Config-based Dogs (from backends.toml) cannot be deleted via this endpoint — returns 400.

### State

New field in `AppState`:

```rust
pub registered_dogs: Arc<std::sync::RwLock<HashMap<String, RegisteredDog>>>,
```

```rust
pub struct RegisteredDog {
    pub registered_at: std::time::Instant,
    pub last_heartbeat: std::time::Instant,
    pub ttl_secs: u64,
}
```

Only Dogs created via `POST /dogs/register` are tracked here. Config-based Dogs are permanent.

### Background Task: `spawn_dog_ttl_checker`

- Tick: every 30 seconds (same pattern as coord_expiry, usage_flush).
- On each tick: scan `registered_dogs`, find entries where `now - last_heartbeat > ttl_secs`.
- For each expired Dog:
  1. Remove from `registered_dogs` map.
  2. Load current Judge from ArcSwap.
  3. Clone all Arcs except the expired Dog.
  4. Rebuild Judge, preserve chain hash, store via ArcSwap.
  5. Emit `KernelEvent::DogExpired { dog_id }`.
  6. Log at warn level.

### Changes to `POST /dogs/register`

After successful registration, insert into `registered_dogs` map:
```rust
registered_dogs.insert(name.clone(), RegisteredDog {
    registered_at: Instant::now(),
    last_heartbeat: Instant::now(),
    ttl_secs: 120, // 2 missed heartbeats @ 30s interval + margin
});
```

### Judge: `remove_dog` method

New method on Judge (not &mut self — builds a new Judge):

```rust
pub fn without_dog(current: &Judge, dog_id: &str) -> Option<Judge>
```

Clones all Arcs from `dogs()`, `breakers()`, `organ_handles()` except the named Dog. Returns `None` if the Dog wasn't found. Preserves chain hash via `last_hash_snapshot()` + `seed_chain()`.

## Files Changed

| File | Change |
|---|---|
| `api/rest/health.rs` | +heartbeat_handler, +deregister_handler (~60 lines) |
| `api/rest/types.rs` | +RegisteredDog struct, +HeartbeatResponse, +DeregisterResponse, +registered_dogs in AppState (~30 lines) |
| `api/rest/mod.rs` | +2 routes |
| `judge.rs` | +without_dog() method (~25 lines) |
| `infra/tasks.rs` | +spawn_dog_ttl_checker (~40 lines) |
| `main.rs` | Wire registered_dogs + spawn TTL checker (~10 lines) |
| `API.md` | +2 endpoints documented |

## Edge Cases Handled

| Case | Solution |
|---|---|
| Kernel restart | registered_dogs map is empty on boot. Nodes get 404 on heartbeat, re-register. |
| Network partition | Heartbeat fails, TTL expires, Dog removed. When network heals, node re-registers. |
| Graceful shutdown | Node calls DELETE. Immediate removal. |
| Crash (no DELETE) | TTL expires after 120s. Circuit breaker opens faster (~30-60s). |
| Double registration | POST /dogs/register returns 409 if name exists. Heartbeat refreshes TTL. |
| Config Dog DELETE attempt | Returns 400 "config-based Dogs cannot be deregistered via API". |
| Thundering herd | Not addressed in Phase A. Phase B: jitter on heartbeat interval. |

## Testing

- Unit: heartbeat refreshes TTL, heartbeat returns 404 for unknown Dog.
- Unit: TTL checker expires stale Dogs, preserves active ones.
- Unit: deregister removes Dog from roster, config Dog rejection.
- Unit: without_dog builds correct Judge, preserves chain hash.
- Integration: register → heartbeat → let expire → verify removed from /dogs.

## Not In Scope

- New binary (Phase B)
- Push mode (Phase C)
- Discovery, federation, compliance metadata
- Jitter/thundering herd protection (Phase B)
