# CYNIC Kernel API Reference

**Version:** 2.0.0

Python kernel — φ-bounded judgment + learning

## Overview

This document describes all available endpoints in the CYNIC Kernel API.

- **Base URL:** `http://localhost:8000` (local) or deployed URL
- **Response format:** JSON (all endpoints return JSON)
- **Authentication:** None (local development — add in production)

---

## Quick Start

The CYNIC API is organized into several functional groups:

1. **Health & Observability** — System status, metrics, logs
2. **Organism State** — Consciousness, dogs, actions, account, policy
3. **Judgment Pipeline** — Accept perceptions, run judgment, learn
4. **Actions** — Propose, accept, reject, execute actions
5. **WebSocket** — Real-time event stream
6. **MCP** — Tool integration (Model Context Protocol)

---

## Health & Observability

### `GET /health`

**Summary:** Organism health status

**Description:**
Check if the CYNIC kernel is running and healthy. Returns detailed health metrics for each component.

**Response:** 200 OK
```json
{
  "overall": "healthy",
  "dogs": 11,
  "consciousness_level": "MICRO",
  "components": {
    "orchestrator": "healthy",
    "qtable": "healthy",
    "scheduler": "healthy"
  }
}
```

**Example:**
```bash
curl http://localhost:8000/health
```

---

### `GET /metrics`

**Summary:** Prometheus metrics

**Description:**
Export metrics in Prometheus text format for monitoring and alerting.

**Common metrics:**
- `cynic_requests_total` — Total HTTP requests by endpoint, method, status
- `cynic_request_duration_seconds` — Request duration distribution by endpoint
- `cynic_learning_rate` — Current learning rate [0, 0.618]
- `cynic_consciousness_level` — Current consciousness LOD [0, 3]
- `cynic_judgments_total` — Total judgments executed
- `cynic_q_entries_total` — Q-Table entries

**Response:** 200 OK (Prometheus text format)

**Example:**
```bash
curl http://localhost:8000/metrics | grep cynic_
```

---

## Organism State

These endpoints provide read-only snapshots of the organism's internal state.

### `GET /api/organism/state/snapshot`

**Summary:** Full organism state snapshot

**Description:**
Returns the complete state of the CYNIC organism including consciousness level, judgment count, dog count, Q-Table entries, residuals, and pending actions.

**Response:** 200 OK
```json
{
  "timestamp": 1708507200.123,
  "consciousness_level": "MICRO",
  "judgment_count": 127,
  "dog_count": 11,
  "qtable_entries": 89,
  "residuals_count": 3,
  "pending_actions_count": 2
}
```

**Example:**
```bash
curl http://localhost:8000/api/organism/state/snapshot
```

---

### `GET /api/organism/consciousness`

**Summary:** Current consciousness level

**Description:**
Returns the current consciousness level inferred from the organism's metabolic scheduler. Levels: REFLEX, MICRO, MACRO, META.

**Response:** 200 OK
```json
{
  "level": "MACRO"
}
```

---

### `GET /api/organism/dogs`

**Summary:** All dogs and their status

**Description:**
Returns the current status of all dogs in the organism's orchestrator, including their last verdict, Q-score, confidence, and activity.

**Response:** 200 OK
```json
{
  "count": 11,
  "dogs": {
    "GUARDIAN": {
      "q_score": 61.8,
      "verdict": "WAG",
      "confidence": 0.618,
      "activity": "monitoring"
    },
    "ANALYST": {
      "q_score": 52.5,
      "verdict": "GROWL",
      "confidence": 0.45,
      "activity": "analyzing"
    }
  }
}
```

---

### `GET /api/organism/actions`

**Summary:** Pending proposed actions

**Description:**
Returns the list of pending proposed actions from the organism's memory action proposer.

**Response:** 200 OK
```json
{
  "count": 2,
  "actions": [
    {
      "action_id": "act_123abc",
      "action_type": "INVESTIGATE",
      "priority": 2,
      "description": "Check codebase for unused imports"
    }
  ]
}
```

---

### `GET /api/organism/account`

**Summary:** Account and budget status

**Description:**
Returns the organism's account metrics including balance, spending, budget remaining, learning rate, and reputation score.

**Response:** 200 OK
```json
{
  "timestamp": 1708507200.123,
  "balance_usd": 10.0,
  "spent_usd": 2.34,
  "budget_remaining_usd": 7.66,
  "learn_rate": 0.618,
  "reputation": 75.0
}
```

---

### `GET /api/organism/policy/actions`

**Summary:** Learned best actions per state

**Description:**
Returns the learned policy from the Q-table: for each state seen during learning, what is the best action to take? Shows CYNIC's learned behavior pattern.

**Response:** 200 OK
```json
{
  "timestamp": 1708507200.123,
  "count": 45,
  "actions": [
    {
      "state_key": "CODE:test_coverage_low",
      "best_action": "REFACTOR",
      "q_value": 0.75,
      "confidence": 0.618
    }
  ]
}
```

---

### `GET /api/organism/policy/stats`

**Summary:** Policy coverage and learning statistics

**Description:**
Returns statistics about CYNIC's learned policy and Q-table including coverage, average confidence, and max Q-value.

**Response:** 200 OK
```json
{
  "timestamp": 1708507200.123,
  "total_states": 156,
  "total_actions_per_state": 3.2,
  "policy_coverage": 0.87,
  "average_confidence": 0.52,
  "max_q_value": 0.98
}
```

---

## Judgment Pipeline

### `POST /judge`

**Summary:** Run full judgment pipeline

**Description:**
Execute the complete judgment cycle (REFLEX → MICRO → MACRO → META) on the given perception. Returns judgment with verdict (WAG/GROWL/BARK/HOWL) and Q-score.

**Request Body:**
```json
{
  "perception": {
    "reality": "CODE",
    "observation": "Test coverage dropped to 45%",
    "severity": 2
  },
  "mode": "full"
}
```

**Response:** 200 OK
```json
{
  "judgment_id": "jdg_abc123xyz",
  "verdict": "WAG",
  "q_score": 62.5,
  "dogs_voted": {
    "GUARDIAN": "WAG",
    "ANALYST": "WAG",
    "SCHOLAR": "GROWL"
  },
  "reasoning": "Coverage acceptable with monitoring..."
}
```

---

## Error Handling

All errors return structured JSON with three fields:

```json
{
  "error": "User-friendly error message",
  "code": "ERROR_CODE",
  "type": "ErrorType"
}
```

### Common Error Codes

- `VALIDATION_ERROR` — Request validation failed (422)
- `NOT_FOUND` — Resource not found (404)
- `INTERNAL_ERROR` — Server error (500)
- `BUDGET_EXCEEDED` — Spend limit exceeded (429)

---

## Interactive Swagger UI

The API provides interactive Swagger documentation:

- **URL:** `http://localhost:8000/docs`
- **Features:** Try endpoints, auto-schema generation, visualization
- **Alternative:** ReDoc at `http://localhost:8000/redoc`

---

## Common Workflows

### Check organism health

```bash
curl http://localhost:8000/health
```

### Get current state snapshot

```bash
curl http://localhost:8000/api/organism/state/snapshot
```

### View learned policy

```bash
curl http://localhost:8000/api/organism/policy/actions
```

---

*Last Updated: 2026-02-22*
*CYNIC Kernel API Reference*
