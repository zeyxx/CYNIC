# CYNIC Gemini 3 Readiness Implementation Plan - Part 1

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish CYNIC's "present truth" and prepare all integration points for seamless Gemini 3 hookup by March 13th.

**Architecture:** Multi-phase audit-first approach: (1) Scan codebase for current state, (2) Map Vision/Memory/Surgery integration points, (3) Prioritize critical fixes, (4) Execute fixes in parallel tracks, (5) Validate readiness.

**Tech Stack:** Python 3.13, SurrealDB, FastAPI, pytest, git, bash

**Timeline:** 10 days (March 3-13, 2026)

---

## Phase 1: Comprehensive Audit (48h, March 3-5)

### Task 1.1: Audit Infrastructure Layer

**Files:**
- Scan: `cynic/kernel/core/agents/`
- Create: `audit/infrastructure-inventory.md`

**Output:** Document all 11 agents, PBFT consensus, Vascular System, metabolic layer

---

### Task 1.2: Audit Capability Layer

**Files:**
- Scan: `cynic/kernel/core/reasoning/`
- Append to: `audit/infrastructure-inventory.md`

**Output:** Current model type, reasoning flows, blind spots, safety gates

---

### Task 1.3: Audit Technical Debt

**Files:**
- Run: Full pytest suite
- Create: `audit/technical-debt-inventory.md`

**Output:** Test failures, import cycles, unfinished components

---

### Task 1.4: Audit Skills Gap

**Files:**
- Create: `audit/skills-gap-analysis.md`

**Output:** Required skills per integration point, team capacity assessment

---

## Phase 2: Integration Point Mapping (24h, March 5-6)

### Task 2.1: Map Vision Input

Create: `docs/INTEGRATION_VISION.md`
- Where images enter CYNIC
- Required components (PerceptionBuffer, ImageProcessor, VisionAnalyzer)

### Task 2.2: Map Long-Context Memory

Create: `docs/INTEGRATION_MEMORY.md`
- Fractal Trace status & persistence
- Codebase snapshot & serialization
- Context window budget calculation

### Task 2.3: Map Auto-Surgery

Create: `docs/INTEGRATION_AUTO_SURGERY.md`
- Execution sandbox design
- Code generation flow
- Safety gates & validation
- Rollback mechanism

---

## Phase 3: Prioritization Matrix (12h, March 6-7)

Create: `audit/critical-path-matrix.md`
- Decision table: blocker status × effort × team skill
- Identify 8 days of critical work (fits in 5-day window with parallelization)
- Scope down Auto-Surgery: "validation only, no execution yet"

---

## Phase 4: Execute Critical Path (5 days, March 7-12)

Four parallel tracks:
- **Track A:** Vision Input (PerceptionBuffer + API endpoint)
- **Track B:** Long-Context Memory (FractalTrace + CodebaseSnapshot)
- **Track C:** Auto-Surgery (CodeValidator + ExecutionQueue)
- **Track D:** Documentation (Gemini 3 Integration Guide)

---

## Phase 5: Validation (1 day, March 12-13)

- Run full test suite
- Verify imports & integrations
- Final commit & readiness check

**Expected Result:** All tests passing, all integration points ready for Gemini 3 hookup.
