# Foundation Cleanup — Test Environment & Event Bus Verification Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute this plan task-by-task.

**Goal:** Fix broken test environment, verify event bus wiring, and establish debt tracking so Phase 2 can be built on clean foundation.

**Architecture:**
- Task 1: Fix test imports (mock docker module, install missing dependencies)
- Task 2: Fix pytest asyncio config (deprecation warning)
- Task 3: Verify event bus end-to-end (ResidualDetector receives JUDGMENT_CREATED → emits EMERGENCE_DETECTED)
- Task 4: Document debt in structured issue tracker

**Tech Stack:** pytest, asyncio, PostgreSQL, event bus architecture

**Expected Outcome:** All 222 tests passing, zero import errors, event bus verified working, debt tracked

---

## Task 1: Fix Test Import Error (docker module)

**Objective:** Make `test_docker_manager.py` importable without requiring docker SDK.

**Files:**
- Create: `cynic/cynic/deployment/docker_mock.py`
- Modify: `cynic/cynic/deployment/docker_manager.py` (imports section only)

**Status:** PENDING

---

## Task 2: Fix Pytest asyncio Configuration

**Objective:** Resolve deprecation warning about `asyncio_default_fixture_loop_scope`.

**Files:**
- Modify: `cynic/pyproject.toml`

**Status:** PENDING

---

## Task 3: Verify Event Bus End-to-End

**Objective:** Create a test that proves ResidualDetector receives events and emits properly.

**Files:**
- Create: `cynic/cynic/tests/test_event_bus_integration_clean.py`

**Status:** PENDING

---

## Task 4: Document Debt in Issue Tracker

**Objective:** Create structured issue tracking for known debt.

**Files:**
- Create: `cynic/TECHNICAL_DEBT.md`
- Create: `cynic/DEPLOYMENT_DEBT.md`

**Status:** PENDING
