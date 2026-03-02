# P10 CLI Self-Probes Interface — Design Document

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create implementation plan.

**Date:** 2026-03-02
**Status:** Approved for Implementation
**Priority:** P10
**Scope:** REST API + CLI for SelfProber probe management
**Goal:** Expose self-improvement proposals via operational REST API and user-friendly CLI commands

---

## Executive Summary

P10 builds a three-layer interface to CYNIC's self-improvement proposal system (SelfProber). Users can list, inspect, apply, and dismiss proposals via CLI. The REST API enables future integrations (P9 metrics bridge, dashboards, remote management).

**Key insight:** Observability requires operational surfaces. P10 makes SelfProber observable and actionable through both programmatic (REST) and interactive (CLI) interfaces.

---

## Architecture Overview

### Three-Layer Design

```
┌─────────────────────────────────────────────────────────┐
│ Layer 3: CLI Client (User Interaction)                  │
│ cynic/interfaces/cli/probes.py                          │
│ ├─ probes list [--status PENDING|APPLIED|DISMISSED|ALL]│
│ ├─ probes inspect <probe_id>                           │
│ ├─ probes apply <probe_id>                             │
│ ├─ probes dismiss <probe_id>                           │
│ └─ probes stats                                        │
└─────────────────────────────────────────────────────────┘
                         ↓ httpx client
┌─────────────────────────────────────────────────────────┐
│ Layer 2: REST API (Operational Surface)                 │
│ cynic/api/endpoints/probes.py                           │
│ ├─ GET /self-probes/list (query: status)              │
│ ├─ GET /self-probes/{probe_id}                        │
│ ├─ POST /self-probes/{probe_id}/apply                 │
│ ├─ POST /self-probes/{probe_id}/dismiss               │
│ └─ GET /self-probes/stats                             │
└─────────────────────────────────────────────────────────┘
                         ↓ sync/async
┌─────────────────────────────────────────────────────────┐
│ Layer 1: Service Logic (Business Rules)                 │
│ cynic/kernel/organism/brain/cognition/cortex/           │
│   probes_service.py                                     │
│ ├─ list_probes(status_filter)                          │
│ ├─ get_probe(probe_id)                                 │
│ ├─ apply_probe(probe_id) → async execution             │
│ ├─ dismiss_probe(probe_id)                             │
│ └─ get_stats()                                         │
└─────────────────────────────────────────────────────────┘
                         ↓ direct access
┌─────────────────────────────────────────────────────────┐
│ Layer 0: SelfProber (Source of Truth)                   │
│ cynic/kernel/organism/brain/cognition/cortex/           │
│   self_probe.py (existing)                              │
│ ├─ all_proposals()                                      │
│ ├─ pending()                                            │
│ ├─ get(probe_id)                                        │
│ ├─ apply_async(probe_id)                               │
│ ├─ dismiss(probe_id)                                    │
│ └─ stats()                                              │
└─────────────────────────────────────────────────────────┘
```

---

## Components

### Component 1: Service Layer (`probes_service.py`)

**Purpose:** Centralized business logic, decouples API from SelfProber, handles async coordination.

**Responsibility:**
- Read operations (sync): list, get, stats
- Write operations (async): apply_probe, dismiss_probe
- Status filtering (PENDING, APPLIED, DISMISSED, ALL)
- Error handling (probe not found, invalid state transitions)

**Key Methods:**
- `list_probes(status: str) → list[dict]` — Filter by status, return dicts
- `get_probe(probe_id: str) → dict | None` — Single probe details
- `get_stats() → dict` — Aggregate counts from SelfProber
- `async apply_probe(probe_id: str) → dict` — Mark APPLIED, execute if LOW_RISK
- `dismiss_probe(probe_id: str) → dict` — Mark DISMISSED

**Dependencies:**
- `SelfProber` (injected)
- `EventBus` (for future event emission)

---

### Component 2: REST API Endpoints (`cynic/api/endpoints/probes.py`)

**Purpose:** FastAPI routes exposing probes as operational REST surface.

**Routes:**

| Endpoint | Method | Purpose | Query/Body | Response |
|----------|--------|---------|-----------|----------|
| `/self-probes/list` | GET | List proposals | `status=PENDING\|APPLIED\|DISMISSED\|ALL` | `ListResponse` |
| `/self-probes/{probe_id}` | GET | Get single probe | — | `ProbeResponse` |
| `/self-probes/{probe_id}/apply` | POST | Apply proposal | — | `{"status": "success", ...}` |
| `/self-probes/{probe_id}/dismiss` | POST | Dismiss proposal | — | `{"status": "success", ...}` |
| `/self-probes/stats` | GET | Aggregate stats | — | `StatsResponse` |

**Response Schemas (Pydantic):**

```python
class ProbeResponse(BaseModel):
    probe_id: str
    trigger: str  # EMERGENCE | SCHEDULE | MANUAL | ANOMALY_DETECTED
    pattern_type: str  # SPIKE | RISING | STABLE_HIGH | QTABLE | ESCORE | CONFIG | ...
    severity: float  # [0, 1]
    dimension: str  # QTABLE | ESCORE | RESIDUAL | CONFIG | COUPLING | METRICS
    target: str  # Dog name, state_key:action, or parameter
    recommendation: str
    current_value: float
    suggested_value: float
    proposed_at: float  # Unix timestamp
    status: str  # PENDING | APPLIED | DISMISSED

class ListResponse(BaseModel):
    count: int
    status_filter: str
    probes: list[ProbeResponse]

class StatsResponse(BaseModel):
    proposed_total: int
    queue_size: int
    pending: int
    applied: int
    dismissed: int
```

**Error Responses:**
- `404 Not Found` — Probe ID doesn't exist
- `422 Unprocessable Entity` — Invalid status filter
- `500 Internal Server Error` — Execution failure

---

### Component 3: CLI Client (`cynic/interfaces/cli/probes.py`)

**Purpose:** Click command group for interactive probe management.

**Commands:**

```bash
cynic probes list [--status PENDING|APPLIED|DISMISSED|ALL] [--json]
cynic probes inspect <probe_id> [--json]
cynic probes apply <probe_id>
cynic probes dismiss <probe_id>
cynic probes stats [--json]
```

**Output Formats:**
- Default: Human-readable tables (tabulate), confirmation prompts
- `--json`: JSON output (passthrough from API)

**Error Handling:**
- Network errors → "Error: Connection refused"
- HTTP 404 → "Probe ID not found"
- HTTP 422 → "Invalid status filter"
- Confirmation prompt cancelled → Exit cleanly

---

## Data Flow

### Apply Command (Async)

```
User: cynic probes apply 12345678
  ↓
CLI: POST /self-probes/12345678/apply
  ↓
API route handler
  ↓
service.apply_probe(probe_id)
  ↓
prober.apply_async(probe_id):
  ├─ Mark status = "APPLIED"
  ├─ If executor available:
  │  ├─ Classify risk
  │  └─ If LOW_RISK: executor.execute(proposal)
  │     └─ Emit PROPOSAL_EXECUTED or PROPOSAL_FAILED
  └─ Save to ~/.cynic/self_proposals.json
  ↓
Return: {"status": "success", "probe_id": "12345678", ...}
  ↓
CLI: ✅ Proposal 12345678 applied
```

### List Command (Sync)

```
User: cynic probes list --status PENDING
  ↓
CLI: GET /self-probes/list?status=PENDING
  ↓
API route
  ↓
service.list_probes("PENDING")
  ↓
prober.pending() → [SelfProposal, ...]
  ↓
Convert to JSON dicts
  ↓
CLI: Format and display table
```

---

## Error Handling Strategy

### Service Layer
- Probe not found → Raise `ValueError` (caught by API)
- Invalid status → Caught by Pydantic query validation
- Apply async failure → Logged, proposal marked APPLIED anyway (operation succeeded)

### API Layer
- Pydantic validation errors → HTTP 422
- ValueError from service → HTTP 404
- Unexpected exceptions → HTTP 500

### CLI Layer
- Network errors → Display error message, exit code 1
- HTTP errors → Display status code + message
- Confirmation cancel → Exit code 0 (normal)

---

## Testing: Formal Specification Approach

### Test Types

**1. Formal Specification Tests**
- Exhaustive verification of spec behavior
- Example: `test_list_filters_by_status_exhaustively()` verifies all 4 status values
- Example: `test_apply_is_idempotent()` verifies multiple applies = same state

**2. Property-Based Tests (Hypothesis)**
- Generate 100+ test cases automatically
- Verify properties hold across all inputs
- Example: `test_list_always_returns_valid_proposals(status, num_proposals)`
- Example: `test_apply_respects_low_risk_execution(probe_indices)`

**3. End-to-End Scenario Tests**
- Real workflows: list → inspect → apply → stats
- Concurrent operations (10 proposals applied simultaneously)
- Full system integration with real SelfProber, EventBus, Executor

**4. API Contract Tests**
- Pydantic schema validation on all responses
- All error cases return correct HTTP status codes
- Response structure matches OpenAPI spec

**5. CLI Integration Tests**
- Real CliRunner with actual test API server
- Parse output (tables, JSON) and verify content
- Test confirmation prompts and error messages

### Test Structure

```
tests/integration/
├─ test_probes_formal_spec.py        # 8 formal spec tests
├─ test_probes_properties.py         # 3 property-based (100+ cases each)
├─ test_probes_scenarios.py          # 4 e2e workflows
├─ test_probes_api_contracts.py      # 6 API schema tests
└─ test_probes_cli_e2e.py           # 5 CLI integration tests
```

**Total Tests:** ~26 formal tests, generating 300+ total test cases (with Hypothesis)

---

## Integration Points

### With Existing Systems

**SelfProber:**
- Direct read access: `all_proposals()`, `pending()`, `get()`
- Direct write access: `apply_async()`, `dismiss()`
- Listen to stats: `stats()`

**ProposalExecutor:**
- Indirectly via `SelfProber.apply_async()` when applying proposals
- Executor classifies risk (LOW_RISK, REVIEW_REQUIRED, NOT_EXECUTABLE)

**EventBus:**
- Service receives bus reference for future event emission
- Currently: SelfProber emits PROPOSAL_EXECUTED/FAILED on its own

**Factory/ArchiveCore:**
- Factory creates ProbesService with (prober, bus)
- Factory injects into API router initialization
- No circular dependencies

---

## Files to Create / Modify

### New Files
- `cynic/kernel/organism/brain/cognition/cortex/probes_service.py` (120 lines)
- `cynic/api/endpoints/probes.py` (180 lines)
- `cynic/interfaces/cli/probes.py` (200 lines)

### Modified Files
- `cynic/api/server.py` — Register probes router
- `cynic/kernel/organism/anatomy.py` — Factory wiring for ProbesService
- `cynic/interfaces/cli/__init__.py` — Register probes command group

### Test Files (New)
- `tests/integration/test_probes_formal_spec.py`
- `tests/integration/test_probes_properties.py`
- `tests/integration/test_probes_scenarios.py`
- `tests/integration/test_probes_api_contracts.py`
- `tests/integration/test_probes_cli_e2e.py`

---

## Success Criteria

- ✅ All 5 commands work: list, inspect, apply, dismiss, stats
- ✅ `--status` filter works for all 4 values (PENDING, APPLIED, DISMISSED, ALL)
- ✅ Apply command triggers async execution for LOW_RISK proposals
- ✅ All API responses pass Pydantic validation
- ✅ All 26+ formal tests passing
- ✅ Property tests generate 300+ test cases with no failures
- ✅ End-to-end scenarios verified with real SelfProber
- ✅ CLI handles network errors gracefully
- ✅ No circular imports introduced
- ✅ Pre-commit gates pass (encoding, imports, factory, tests ≥87% coverage)
- ✅ /docs endpoint auto-generates OpenAPI schema for REST API

---

## Design Decisions

**Q: Why three layers?**
A: Separation of concerns. Service = business logic, API = HTTP contract, CLI = UX. Enables future clients (dashboard, automation) without rewriting.

**Q: Why REST API before only CLI?**
A: Operational observability requires programmatic access. P9 (metrics bridge) needs to query probes. Rest API is first-class.

**Q: Why async apply but sync dismiss?**
A: Apply triggers execution (potentially slow), dismiss is immediate state change. Async apply allows showing real-time feedback.

**Q: Why formal tests instead of mocks?**
A: CYNIC is self-improving system. Tests must verify behavior holds across real state. Property tests catch edge cases unit tests miss.

---

## Timeline

- **Week 1, Day 1:** Implement service layer + API routes (4-6 hours)
- **Week 1, Day 2:** Implement CLI client (2-3 hours)
- **Week 1, Day 3:** Write formal test suite (4-5 hours)
- **Week 1, Day 4:** Integration, debugging, PR review

---

## Next Steps

→ **Invoke writing-plans skill** to create detailed 10-12 task implementation plan with TDD structure (test → implement → commit per task)
