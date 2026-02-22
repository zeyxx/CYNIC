# Phase 2: Command Palette Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Enable users to invoke CYNIC operations from the webapp UI with real-time feedback and organism metrics.

**Architecture:**
- Frontend: Type-driven form generation from OpenAPI schema + WebSocket listeners for real-time events
- Backend: Minimal — already has /api/organism/schema endpoint
- Integration: Command palette modal, metrics dashboard, error handling with retry

**Tech Stack:** TypeScript (vanilla), HTML/CSS, WebSocket, FastAPI (existing)

---

## Overview: 5 Tasks, ~7 Days

```
Task 2.1: Schema-Driven Form Builder (2 days)
├─ Load schema from GET /api/organism/schema
├─ Generate form fields dynamically
├─ Type validation (string, number, enum, boolean)
└─ Tests: 8 tests

Task 2.2: Command Palette UI (2 days)
├─ Searchable command list
├─ Modal form injection
├─ Autocomplete + filtering
└─ Tests: 6 tests

Task 2.3: Command Invocation Flow (2 days)
├─ POST /api/commands/invoke endpoint
├─ WebSocket: listen for command:complete
├─ Stream response to UI
├─ Error handling + retry
└─ Tests: 10 tests

Task 2.4: Real-Time Metrics Dashboard (1 day)
├─ GET /api/organism/state polling
├─ WebSocket: organism:state-change events
├─ Render metrics panel (balance, learn_rate, reputation)
└─ Tests: 5 tests

Task 2.5: Integration + E2E Testing (1 day)
├─ End-to-end: invoke command → see result
├─ Stress test: 100 concurrent commands
├─ Error scenarios (timeout, validation, 500)
└─ Tests: 6 tests

TOTAL: ~35 tests, frequent commits
```

---

## Task 2.1: Schema-Driven Form Builder

**Objective:** Load command schema from backend, generate HTML forms dynamically with validation.

**Files:**
- Create: cynic/webapp/src/ui/form-builder.ts
- Create: cynic/webapp/src/types/schema.ts
- Modify: cynic/webapp/src/main.ts (add form builder import)
- Test: cynic/webapp/tests/form-builder.test.ts

**Key Implementation:**
- SchemaCache class with localStorage caching (1-hour TTL)
- createFormFromSchema() function generates HTML from CommandSchema
- createFormField() for individual field rendering
- Type handling: string, number, enum, boolean
- Form validation before submit

**Tests:**
- Schema loading and caching
- Cache invalidation
- Form field generation
- Enum dropdown creation
- Required field validation

---

## Task 2.2: Command Palette UI

**Objective:** Searchable command palette modal that displays available commands and renders forms.

**Files:**
- Create: cynic/webapp/src/ui/command-palette.ts
- Create: cynic/webapp/src/ui/styles/palette.css
- Modify: cynic/webapp/src/main.ts (add palette initialization)
- Test: cynic/webapp/tests/command-palette.test.ts

**Key Implementation:**
- CommandPalette class with modal rendering
- Search input with real-time filtering
- Click to select command → inject form
- Escape key to close
- CSS: modal, backdrop, list, form container

**Tests:**
- Modal creation and rendering
- Command filtering
- Form injection on selection
- Close on escape key
- Search performance

---

## Task 2.3: Command Invocation Flow

**Objective:** Handle form submission, POST to /api/commands/invoke, listen for results via WebSocket.

**Files:**
- Create: cynic/webapp/src/api/commands.ts
- Modify: cynic/webapp/src/main.ts (wire command submission)
- Modify: cynic/webapp/src/api/ws.ts (add listener for command:complete)
- Test: cynic/webapp/tests/command-invocation.test.ts

**Key Implementation:**
- invokeCommand(operation, params, timeout) function
- POST to /api/commands/invoke with request body
- AbortController for timeout handling (30s default)
- Result display with status (success/error)
- HTML escaping to prevent XSS

**Tests:**
- Successful command invocation
- Parameter serialization (number, boolean)
- Timeout handling
- Error handling
- Result display

---

## Task 2.4: Real-Time Metrics Dashboard

**Objective:** Display organism metrics (balance, learn_rate, reputation) with real-time updates.

**Files:**
- Create: cynic/webapp/src/ui/metrics-panel.ts
- Create: cynic/webapp/src/ui/styles/metrics.css
- Modify: cynic/webapp/src/main.ts (initialize metrics)
- Test: cynic/webapp/tests/metrics-panel.test.ts

**Key Implementation:**
- MetricsPanel class with fetch and render
- GET /api/organism/account for metrics
- Metric items: balance, learn_rate, reputation
- Progress bars with dynamic width
- WebSocket listener for organism:state-change events

**Tests:**
- Metrics fetching
- Display formatting
- Progress bar calculation
- WebSocket event handling
- Auto-refresh on command complete

---

## Task 2.5: Integration & E2E Testing

**Objective:** Full end-to-end testing, stress testing, error scenarios.

**Files:**
- Create: cynic/webapp/tests/e2e.test.ts
- Modify: cynic/webapp/src/main.ts (ensure all components wired)

**Key Tests:**
- E2E: Load schema → show palette → invoke command → see result
- Stress test: 100 concurrent command invocations
- Error handling: invalid command, timeout, server error
- Metrics update after command
- Palette lifecycle: open → select → submit → close
- WebSocket connection stability

**Verification Checklist:**
- npm test: All 35+ tests passing
- npm run build: Bundle without errors
- Palette opens/closes cleanly
- Commands invoke and display results
- Metrics update in real-time
- No memory leaks
- Error handling works
- Stress test: 100 concurrent commands

---

## Summary

Phase 2 Deliverables:
- Command form builder (dynamic, type-safe)
- Command palette (searchable, real-time)
- Command invocation (error handling, retry)
- Metrics dashboard (real-time updates)
- Full E2E testing (35+ tests)

Next: Phase 3 (Skills editor) in new session after Phase 2 merge
