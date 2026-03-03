# PHASE 2: COMPONENT 2 — EventForwarder Decomposition

## Problem Statement

**Bridge EventBus (in-memory) → SurrealDB (persistent)**

EventBus emits 30+ CoreEvent types. Currently:
- Lost after overflow (EventJournal = 89 event max)
- No persistent storage for forensics/correlation
- No integration with SecurityEventRepo (Component 3)

**Solution: EventForwarder** — autonomous subscriber that:
1. Listens to all CoreEvent types
2. Normalizes events (flatten/serialize)
3. Batches for efficiency (100 events/batch)
4. Persists to SurrealDB via SecurityEventRepo
5. Handles backpressure (pause vs drop)

---

## DOMAIN EXPERTISE DECOMPOSITION

### 1️⃣ BACKEND ENGINEER: Event Subscription & Normalization

**Problem:** How do we safely subscribe to 30+ event types without coupling?

**Architecture:**
```python
class EventForwarder(EventBusSubscriber):
    """Subscribe to EventBus, forward to storage."""

    def __init__(self, bus: EventBus, storage: StorageInterface):
        self.bus = bus
        self.storage = storage

    async def on_event(self, event: CoreEvent) -> None:
        """Triggered by EventBus for every event."""
        normalized = self._normalize(event)
        await self._queue_for_persistence(normalized)
```

**Normalization Strategy:**
```python
def _normalize(self, event: CoreEvent) -> dict:
    """Convert CoreEvent to dict for SurrealDB."""
    return {
        "type": event.__class__.__name__,
        "timestamp": event.timestamp or time.time(),
        "actor_id": getattr(event, "actor_id", None),
        "payload": self._flatten(event.__dict__),
        "version": "1.0",  # Schema version for migrations
    }
```

**Validation Checklist:**
- [ ] EventForwarder implements EventBusSubscriber interface
- [ ] Subscribes to all 30+ CoreEvent types
- [ ] Normalization tested for each event type
- [ ] No exceptions crash the forwarder
- [ ] Handles None/missing fields gracefully

---

### 2️⃣ DATA ENGINEER: Batching & Pipeline

**Problem:** Persisting every event individually = 30k queries/sec = database overload

**Solution: Batch Queue**
```python
class EventQueue:
    """Accumulate events, flush when batch is full."""

    def __init__(self, batch_size: int = 100, flush_interval_sec: float = 5.0):
        self.batch_size = batch_size
        self.flush_interval = flush_interval_sec
        self.events: list[dict] = []
        self._last_flush = time.time()

    async def add(self, event: dict) -> None:
        """Add event to batch."""
        self.events.append(event)
        if len(self.events) >= self.batch_size:
            await self.flush()

    async def flush(self) -> None:
        """Write batch to storage."""
        if not self.events:
            return
        # Batch insert via storage
        for event in self.events:
            await self.storage.security_events.save_event(event)
        self.events.clear()
        self._last_flush = time.time()
```

**Pipeline Flow:**
```
EventBus.emit(event)
    ↓
EventForwarder.on_event()
    ↓
Normalize to dict
    ↓
EventQueue.add(event)
    ↓
When batch_size reached OR flush_interval elapsed:
    ↓
Save all events to SurrealDB (single transaction)
    ↓
Clear queue, ready for next batch
```

**Validation Checklist:**
- [ ] Batch insertion works (100 events in one call)
- [ ] Flush interval works (even if < batch_size)
- [ ] No events lost on graceful shutdown
- [ ] Latency per event < 1ms (in-memory queue)
- [ ] Memory usage bounded (max batch_size in memory)

---

### 3️⃣ INFRASTRUCTURE ENGINEER: Performance & Throughput

**Problem:** How to handle 30k events/sec without blocking EventBus?

**Solution: Async Non-Blocking**
```python
# EventForwarder uses async/await, never blocks EventBus
async def on_event(self, event: CoreEvent) -> None:
    """Non-blocking event handler."""
    normalized = self._normalize(event)  # Fast, CPU-bound
    await self._queue.add(normalized)     # Async, I/O-bound
    # No await for flush() - let it run in background
```

**Performance Targets:**
- Per-event overhead: < 1ms (normalize + queue)
- Batch write latency: < 100ms (100 events to SurrealDB)
- Queue memory: < 10MB (100 events × 100KB per event)
- EventBus latency impact: < 5% (measured end-to-end)

**Load Testing:**
```python
# Test: Can we handle 10k events/sec?
async def test_throughput():
    forwarder = EventForwarder(bus, storage)
    start = time.time()

    for i in range(10000):
        event = MockEvent(actor_id=f"dog_{i % 10}")
        await forwarder.on_event(event)

    elapsed = time.time() - start
    tps = 10000 / elapsed
    assert tps > 5000, f"TPS too low: {tps}"  # 5k events/sec minimum
```

**Validation Checklist:**
- [ ] Per-event overhead < 1ms
- [ ] Batch write latency < 100ms
- [ ] Queue memory < 10MB
- [ ] TPS > 5000 (load test)
- [ ] No EventBus latency regression (< 5%)

---

### 4️⃣ SECURITY ARCHITECT: Data Integrity & Encryption

**Problem:** Events may contain sensitive data (treasury_address, proposal_details)

**Solution: Selective Encryption**
```python
def _normalize(self, event: CoreEvent) -> dict:
    """Normalize and optionally encrypt sensitive fields."""
    data = {
        "type": event.__class__.__name__,
        "timestamp": event.timestamp,
        "actor_id": getattr(event, "actor_id", None),
        "payload": {},
    }

    # Encrypt sensitive fields
    for key, value in event.__dict__.items():
        if key in SENSITIVE_FIELDS:  # e.g., treasury_address, proposal_value
            data["payload"][f"{key}_encrypted"] = await self.encryption.encrypt(value)
        else:
            data["payload"][key] = value

    return data
```

**Sensitive Fields:**
- treasury_address (blockchain address)
- proposal_value (monetary amounts > threshold)
- community_token (API keys)
- voter_id (personally identifiable)

**Validation Checklist:**
- [ ] Sensitive fields identified and documented
- [ ] Encryption applied consistently
- [ ] No plaintext sensitive data in logs
- [ ] Encryption keys rotated periodically
- [ ] Audit trail for encrypted field access

---

### 5️⃣ SRE / DEVOPS: Backpressure & Resilience

**Problem:** What if SurrealDB is slow/down? Do we drop events or pause EventBus?

**Solution: Backpressure Strategy**
```python
class EventForwarder:
    """
    Backpressure modes:
    1. QUEUE: Accumulate events in memory (up to max_queue_size)
    2. PAUSE: Stop listening to EventBus (pause upstream)
    3. DROP: Discard oldest events (FIFO drop)
    """

    async def _handle_backpressure(self) -> None:
        """When queue > 50%, apply backpressure."""
        queue_usage = len(self.queue.events) / self.queue.batch_size

        if queue_usage > 0.9:  # 90% full
            logger.warning(f"Queue near capacity: {queue_usage:.1%}")
            # Mode 1: Try to flush immediately
            await self.queue.flush()

        if queue_usage > 0.95:  # 95% full
            logger.error(f"Queue critical: {queue_usage:.1%}")
            # Mode 2: Pause EventBus subscription
            await self.bus.pause_subscriber(self)
            # Wait until queue clears
            while len(self.queue.events) > 0:
                await asyncio.sleep(0.1)
            await self.bus.resume_subscriber(self)
```

**Graceful Shutdown:**
```python
async def shutdown(self) -> None:
    """Flush remaining events before exit."""
    logger.info(f"Flushing {len(self.queue.events)} pending events...")
    await self.queue.flush()
    logger.info("EventForwarder shutdown complete")
```

**Monitoring:**
```python
async def _emit_metrics(self) -> None:
    """Report metrics every 10 seconds."""
    while True:
        metrics.gauge("eventforwarder.queue_size", len(self.queue.events))
        metrics.gauge("eventforwarder.events_persisted", self.total_persisted)
        metrics.gauge("eventforwarder.flush_latency_ms", self.last_flush_ms)
        await asyncio.sleep(10)
```

**Validation Checklist:**
- [ ] Backpressure triggers at 50% queue usage
- [ ] Pause/resume works without data loss
- [ ] Graceful shutdown flushes remaining events
- [ ] Metrics exported (queue size, throughput, latency)
- [ ] Alerts on queue > 80% or SurrealDB timeout

---

### 6️⃣ SOLUTIONS ARCHITECT: Integration & Business Logic

**Problem:** How does EventForwarder fit into CYNIC's broader system?

**System Context:**
```
EventBus (core/event.py)
    ↓ emits 30+ CoreEvent types
    ↓
EventForwarder (new)
    ↓ normalizes + batches
    ↓
SurrealDB via SecurityEventRepo (Component 3)
    ↓ persists security_event records
    ↓
Real-Time Detection (Component 4: LIVE SELECT)
    ↓ watches security_event table
    ↓
Detection Rules (Component 5)
    ↓ correlates patterns
    ↓
Alerting & Escalation (Component 6)
    ↓ routes to humans
```

**Factory Integration:**
```python
# In factory.py
class CynicFactory:
    async def _build_event_ingestion(self) -> EventForwarder:
        """Create EventForwarder with dependencies."""
        return EventForwarder(
            bus=self.event_bus,  # Already exists
            storage=self.storage,  # SurrealStorage (Component 3)
            encryption=self.encryption_service,  # From Vault
        )

    async def initialize(self):
        """Start EventForwarder as background task."""
        self.event_forwarder = await self._build_event_ingestion()
        # Subscribe to bus
        self.event_bus.subscribe(self.event_forwarder)
        # Start background metrics task
        asyncio.create_task(self.event_forwarder._emit_metrics())
```

**Validation Checklist:**
- [ ] EventForwarder initialized in factory
- [ ] Subscribed to EventBus on startup
- [ ] Metrics emitted continuously
- [ ] Shutdown handler integrated
- [ ] No circular dependencies

---

## Implementation Order (Critical Path)

### Week 1: EventForwarder Core (2 days)

**Day 1: Core Architecture**
- [ ] EventForwarder class + EventBusSubscriber interface
- [ ] Event normalization (_normalize method)
- [ ] EventQueue (batch accumulation + flush)
- [ ] 8 tests: CRUD + normalization + batching

**Day 2: Resilience & Integration**
- [ ] Backpressure handling (pause/resume/drop)
- [ ] Graceful shutdown
- [ ] Metrics & monitoring
- [ ] Factory integration
- [ ] 12 tests: backpressure + shutdown + metrics
- [ ] Load test: 10k events/sec

**Total: 20 tests, 1 commit**

---

## Validation by Domain

| Domain | Key Questions | Acceptance Criteria |
|--------|---|---|
| **Backend** | Does it normalize all 30+ event types? | All types tested, no crashes |
| **Data Engineer** | Can it batch 100 events efficiently? | Batch insert < 100ms latency |
| **Infrastructure** | Per-event overhead < 1ms? | Load test: 5k+ TPS |
| **Security** | Are sensitive fields encrypted? | Audit trail complete |
| **SRE** | What happens when DB is slow? | Backpressure tested, no data loss |
| **Solutions** | Does it fit the broader architecture? | Factory wired, metrics working |

---

## Risk Assessment

### High-Confidence Areas
- Event normalization (straightforward flattening)
- Batch queueing (proven pattern)
- Backpressure handling (pause/resume tested in EventBus)

### Medium-Confidence Areas
- Performance at 10k+ TPS (load test needed)
- SurrealDB batch insert performance (depends on DB tuning)
- Encryption impact on latency (should be negligible)

### Mitigation
- Load test with realistic event volume before deploying
- Monitor metrics on staging environment
- Graceful degradation: drop old events if queue fills
- Feature flag: enable/disable EventForwarder without redeployment

---

## Success Criteria

✅ **Component 2 is DONE when:**
1. EventForwarder normalizes 30+ CoreEvent types
2. Batches 100 events, flushes to SurrealDB
3. Handles backpressure (pause EventBus if queue > 95%)
4. Persists to security_events table (via Component 3)
5. 20 comprehensive tests passing
6. Load test: 5k+ events/sec sustained
7. Metrics exported (queue size, TPS, latency)
8. Factory integration wired + startup tested
9. Graceful shutdown flushes remaining events
10. No regression in EventBus latency (< 5%)

**Estimated effort:** 2 days
**Test count:** 20 tests
**Commit message:** feat(security-p2-component2): Event ingestion pipeline
