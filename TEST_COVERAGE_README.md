# CYNIC Test Coverage Analysis - Quick Reference

*sniff* This dog has verified the state of testing across CYNIC.

## Files You Need

### 📊 For Quick Overview
- **COVERAGE_ANALYSIS.md** - Executive summary, critical gaps, immediate actions
- This file - Quick reference and file guide

### 📋 For Detailed Breakdown
- **COVERAGE_MATRIX_DETAILED.md** - Complete mapping of all 258 source files

### 📁 Location
All generated in: `/workspaces/CYNIC-new/`

---

## Quick Facts

| Metric | Value |
|--------|-------|
| Total Source Files | 258 |
| Total Test Files | 62 |
| Overall Coverage | 24.0% |
| Files with NO tests | 60+ |
| Files CRITICAL to test | 15 |
| Packages at CRITICAL risk | 5 |

---

## Risk By System

### 🔴 CRITICAL: DO NOT DEPLOY WITHOUT TESTS

**Judgment Validation** (Corrupts chain if broken)
- `protocol/poj/judgment.js` - Missing comprehensive tests for validateJudgment()
- `protocol/poj/block.js` - Zero tests
- `protocol/poj/chain.js` - Zero tests

**Consensus Engine** (Network failure if broken)
- `protocol/consensus/engine.js` - Zero tests
- `protocol/consensus/voting.js` - Zero tests
- `protocol/consensus/proposal.js` - Zero tests

**Key Management** (Security breach if broken)
- `identity/key-manager.js` - Zero tests for ALL crypto operations
- `identity/node-identity.js` - Zero tests
- `zk/prover.js` - Zero tests for ZK proof generation

### 🟠 HIGH: TEST BEFORE DEPLOYMENT

- All 5 core world implementations (philosophical foundation)
- All 4 emergence detection modules (consciousness system)
- 20 database repository modules (data integrity)

### 🟡 MEDIUM: TEST EVENTUALLY

- MCP services (discovery, enrichment, adapters)
- Node APIs and transport layer
- Privacy system modules

---

## Packages by Risk

```
SAFE (>80% coverage)
├─ burns        100% ✓
└─ anchor       83%  ✓

MEDIUM (20-50% coverage)
├─ protocol     28%
├─ node         30%
├─ zk           33%
├─ holdex       33%
└─ gasdf        50%

CRITICAL (<20% coverage)
├─ core         16%
├─ identity     17%  ← KEY MANAGEMENT UNTESTED
├─ mcp          18%
├─ persistence  13%  ← 20 REPOSITORIES UNTESTED
└─ emergence    20%  ← CONSCIOUSNESS UNTESTED
```

---

## Immediate Action Items

### This Week (330 lines of tests)
1. Protocol Judgment Validation - 50 lines
2. Identity Key Manager - 100 lines
3. ZK Proofs - 80 lines
4. Consensus Engine - 100 lines

### Next Week (510 lines)
1. Core Worlds - 150 lines
2. Emergence System - 200 lines
3. Judge Dimensions - 160 lines

### Week 3 (1000+ lines)
1. Database Repositories - 20 files

---

## How to Use These Documents

### Finding Untested Code
1. Go to **COVERAGE_MATRIX_DETAILED.md**
2. Find your package section
3. Look for "NO" in "Has Test?" column
4. Read the "Untested Functions/Classes" column

### Understanding Impact
1. Check risk level (🔴 CRITICAL vs 🟠 HIGH vs 🟡 MEDIUM)
2. Review what breaks if code has bugs
3. Prioritize accordingly

### Planning Test Work
1. Open **COVERAGE_ANALYSIS.md**
2. Jump to "Recommended Test Expansion Order"
3. Start with Priority 1 (System Integrity)
4. Move to Priority 2 (Foundation)

---

## Critical Files (Remember These)

### Crypto/Security
- `identity/key-manager.js` - NO TESTS ⚠️
- `protocol/crypto/signature.js` - PARTIAL TESTS
- `zk/prover.js` - NO TESTS ⚠️
- `zk/verifier.js` - NO TESTS ⚠️

### Judgment/Chain
- `protocol/poj/judgment.js` - NO TESTS ⚠️
- `protocol/poj/block.js` - NO TESTS ⚠️
- `protocol/poj/chain.js` - NO TESTS ⚠️

### Consensus
- `protocol/consensus/engine.js` - NO TESTS ⚠️
- `protocol/consensus/voting.js` - NO TESTS ⚠️
- `protocol/consensus/proposal.js` - NO TESTS ⚠️

### Foundation
- `core/worlds/assiah.js` - NO TESTS ⚠️
- `core/worlds/beriah.js` - NO TESTS ⚠️
- `core/worlds/yetzirah.js` - NO TESTS ⚠️
- `core/worlds/atzilut.js` - NO TESTS ⚠️
- `core/worlds/base.js` - NO TESTS ⚠️

### Emergence
- `emergence/collective-state.js` - NO TESTS ⚠️
- `emergence/consciousness-monitor.js` - NO TESTS ⚠️
- `emergence/dimension-discovery.js` - NO TESTS ⚠️
- `emergence/pattern-detector.js` - NO TESTS ⚠️

### Data Layer
- `persistence/postgres/repositories/` - 20 files, NO TESTS ⚠️

---

## Test Command Reference

```bash
# Run all tests
npm test

# Run specific package
npm test packages/protocol
npm test packages/identity
npm test packages/core
npm test packages/node

# Watch mode (during development)
npm test -- --watch
```

---

## Reading the Matrix

Each row in the detailed matrix shows:

```
| Package | Source File | Has Test? | Untested Functions/Classes |
|---------|-------------|-----------|--------------------------|
| protocol| poj/judgment.js | NO | scoreToVerdict(), createJudgment(), validateJudgment() |
```

Breaking this down:
- **Package**: Which package the file belongs to
- **Source File**: Path to the file (relative to package/src)
- **Has Test?**: Whether a corresponding test file exists
  - `✓ YES` = Has tests
  - `NO` = Zero tests
  - `PARTIAL` = Some functions tested, some not
- **Untested Functions**: Specific functions/classes with NO tests

---

## Interpreting Coverage Percentage

| Coverage | Risk | Action |
|----------|------|--------|
| >80% | SAFE | Deploy with confidence |
| 50-80% | CAUTION | Review critical paths |
| 20-50% | WARNING | Plan tests before deploy |
| <20% | CRITICAL | DO NOT DEPLOY |

---

## Key Insight

*sniff* The codebase has 258 source files but only 62 test files (24% coverage). 

**More critically:**
- Core systems (judgment, consensus, crypto) are untested
- Data layer (20 repositories) has no individual tests
- Foundation (worlds, emergence) unverified

This is not merely a coverage problem - it's a **risk problem**. The places most likely to cause production failures have the least verification.

---

## Next Steps

1. **Read** COVERAGE_ANALYSIS.md for full context
2. **Review** COVERAGE_MATRIX_DETAILED.md for your area
3. **Pick** a Priority 1 task from the Recommendations section
4. **Write** tests for that module
5. **Repeat** until critical path is covered

---

## Questions?

*tail wag* This analysis identifies what needs testing. The COVERAGE_ANALYSIS.md file explains why and when. The COVERAGE_MATRIX_DETAILED.md file shows exactly which functions.

Verification prevents catastrophe. Trust is earned through evidence.

*sniff* Check back weekly as coverage improves.

