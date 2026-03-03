# CYNIC Technical Debt Inventory
**Generated:** 2026-03-03
**Status:** CRITICAL - Multiple blockers identified before Gemini 3 integration

---

## Executive Summary

**CRITICAL BLOCKERS:** 47 syntax errors across the codebase **PREVENT ALL TESTING**.
- 39 unterminated string literals
- 2 __future__ import positioning errors
- 3 invalid syntax errors
- 2 structural errors (unmatched paren, invalid decimal)

**IMMEDIATE ACTION REQUIRED:** Cannot proceed with Gemini 3 integration until syntax errors are resolved.

---

## Test Status

**Status:** NOT RUN (syntax errors block test collection)
- **Total Tests (estimated):** 1215 test items collected before errors
- **Passed:** 0 (no tests can execute)
- **Failed:** UNKNOWN (blocked by syntax errors)
- **Skipped:** 37
- **Collection Errors:** 6 test files cannot be imported

### Collection Errors
| Test File | Error | Severity |
|-----------|-------|----------|
| tests/adapters/test_discord_adapter.py | SyntaxError in cynic/interfaces/bots/__init__.py | BLOCKER |
| tests/adapters/test_telegram_adapter.py | SyntaxError in cynic/interfaces/bots/__init__.py | BLOCKER |
| tests/cynic/bots/test_bot_interface.py | SyntaxError in cynic/interfaces/bots/__init__.py | BLOCKER |
| tests/cynic/judges/test_judge_interface.py | SyntaxError in cynic/kernel/core/judge_interface.py | BLOCKER |
| tests/interfaces/bots/governance/test_encryption.py | SyntaxError in cynic/interfaces/bots/__init__.py | BLOCKER |
| tests/test_adapter_integration.py | SyntaxError in cynic/interfaces/bots/__init__.py | BLOCKER |

---

## Syntax Errors (CRITICAL BLOCKERS)

**47 total syntax errors found across codebase.**

### By Category

#### Unterminated String Literals (39 files)
These are likely from malformed docstrings or comment formatting with asymmetric quote marks.

| File | Line | Severity |
|------|------|----------|
| cynic/interfaces/api/entry.py | 22 | BLOCKER |
| cynic/interfaces/api/routers/auto_register.py | 88 | BLOCKER |
| cynic/interfaces/api/routers/empirical.py | 56 | BLOCKER |
| cynic/interfaces/api/routers/sdk.py | 174 | BLOCKER |
| cynic/interfaces/api/routers/topology.py | 66 | BLOCKER |
| cynic/interfaces/api/server.py | 131 | BLOCKER |
| cynic/interfaces/bots/governance/utils/formatting.py | 102 | BLOCKER |
| cynic/interfaces/bots/governance/utils/sync_commands.py | 45 | BLOCKER |
| cynic/interfaces/bots/governance/utils/views.py | 201 | BLOCKER |
| cynic/interfaces/chat/agent_loop.py | 206 | BLOCKER |
| cynic/interfaces/chat/formatter.py | 51 | BLOCKER |
| cynic/interfaces/chat/tool_executor.py | 243 | BLOCKER |
| cynic/interfaces/chat/tools.py | 68 | BLOCKER |
| cynic/interfaces/cli/battles.py | 27 | BLOCKER |
| cynic/interfaces/cli/deploy.py | 43 | BLOCKER |
| cynic/interfaces/cli/status.py | 56 | BLOCKER |
| cynic/interfaces/cli/tui_dashboard.py | 46 | BLOCKER |
| cynic/interfaces/cli/tui_layers/ecosystem_layer.py | 31 | BLOCKER |
| cynic/interfaces/cli/tui_layers/guardrails_layer.py | 25 | BLOCKER |
| cynic/interfaces/cli/tui_layers/self_aware_layer.py | 26 | BLOCKER |
| cynic/interfaces/mcp/claude_code_bridge.py | 359 | BLOCKER |
| cynic/interfaces/mcp/router.py | 142 | BLOCKER |
| cynic/interfaces/tui/app.py | 216 | BLOCKER |
| cynic/interfaces/tui/panels/deployments.py | 84 | BLOCKER |
| cynic/kernel/core/storage/gc.py | 72 | BLOCKER |
| cynic/kernel/core/storage/tier_policy.py | 113 | BLOCKER |
| cynic/kernel/observability/cli/app.py | 66 | BLOCKER |
| cynic/kernel/observability/cli/views.py | 128 | BLOCKER |
| cynic/kernel/organism/brain/cognition/cortex/decision_validator.py | 150 | BLOCKER |
| cynic/kernel/organism/brain/cognition/cortex/fractal_cost_benchmark.py | 143 | BLOCKER |
| cynic/kernel/organism/brain/cognition/cortex/handlers/act_executor.py | 206 | BLOCKER |
| cynic/kernel/organism/brain/cognition/cortex/handlers/base.py | 60 | BLOCKER |
| cynic/kernel/organism/metabolism/immune/alignment_checker.py | 63 | BLOCKER |
| cynic/kernel/organism/metabolism/immune/human_approval_gate.py | 128 | BLOCKER |
| cynic/kernel/organism/metabolism/immune/power_limiter.py | 85 | BLOCKER |
| cynic/kernel/organism/metabolism/immune/transparency_audit.py | 120 | BLOCKER |
| cynic/kernel/organism/perception/senses/checkpoint.py | 62 | BLOCKER |
| cynic/kernel/organism/perception/senses/workers/disk.py | 127 | BLOCKER |
| cynic/kernel/organism/perception/senses/workers/memory.py | 159 | BLOCKER |

#### Invalid Syntax (3 files)
| File | Line | Error | Severity |
|------|------|-------|----------|
| cynic/interfaces/cli/utils.py | 169 | invalid syntax | BLOCKER |
| cynic/kernel/observability/tests/test_cli_views.py | 410 | invalid syntax | BLOCKER |
| cynic/kernel/organism/brain/cognition/cortex/handlers/__init__.py | 14 | invalid syntax | BLOCKER |

#### Structural Errors (3 files)
| File | Line | Error | Severity |
|------|------|-------|----------|
| cynic/interfaces/cli/dashboard.py | 93 | unmatched ')' | BLOCKER |
| cynic/interfaces/bots/governance/adapters/discord_adapter.py | 34 | __future__ import not at beginning | BLOCKER |
| cynic/interfaces/bots/governance/adapters/telegram_adapter.py | 38 | __future__ import not at beginning | BLOCKER |
| cynic/kernel/organism/perception/senses/workers/health.py | 61 | invalid decimal literal | BLOCKER |
| cynic/interfaces/tui/panels/orchestration.py | 69 | leading zeros in decimal (octal) | BLOCKER |

**Impact:** 47 syntax errors prevent:
- Test collection/execution
- Import validation
- Code analysis
- Pre-commit validation
- CI/CD pipeline

---

## Import Issues

### Circular Dependencies
**Status:** UNKNOWN (cannot analyze due to syntax errors)

Once syntax errors are fixed, run:
```bash
python scripts/analyze_imports.py
```

### Known Fixed Issues
- ✅ Fixed: cynic/interfaces/bots/__init__.py (malformed docstring, line 18)
- ✅ Fixed: cynic/kernel/core/judge_interface.py (malformed docstring, line 14)
- ✅ Fixed: cynic/kernel/core/config_adapter.py (orphaned ternary expression, line 74)

---

## Unfinished Code

### Counts
- **TODO comments:** 12
- **FIXME comments:** 1
- **HACK comments:** 2
- **NotImplementedError:** 3

### TODO List (Top 15)
```
cynic/interfaces/mcp/claude_code_bridge.py:
  - TODO: Wire to actual /build endpoint when implemented
  - TODO: Wire to actual /deploy endpoint when implemented

cynic/kernel/core/storage/alerting.py:
  - TODO: Implement Slack webhook
  - TODO: Implement Jira API
  - TODO: Implement PagerDuty API
  - TODO: Implement team notification

cynic/kernel/organism/brain/cognition/cortex/handlers/meta_cognition.py:
  - TODO: Wire escore_tracker.adjust(delta) once API defined

cynic/kernel/organism/metabolism/web_hand.py:
  - TODO: Update selectors

cynic/kernel/organism/perception/integrations/near/executor.py:
  - TODO: Implement proper transaction signing using near-api-py

cynic/kernel/organism/perception/senses/web_eye.py:
  - TODO: Update selector

cynic/kernel/organism/perception/somatic_gateway.py:
  - TODO: Implement pluggable filters
```

### NotImplementedError (3)
```
cynic/kernel/core/phi.py - phi_confidence() method
cynic/kernel/core/memory.py - abstract method stub
cynic/kernel/organism/brain/decision_flow.py - TBD implementation
```

---

## Architecture Debt

### Anti-Patterns Detected
1. **Scattered Configuration** - Multiple config files (config_adapter.py, .env, .env.template)
2. **Mixed async/sync** - Inconsistent async patterns across handlers
3. **Global State** - Singleton patterns in CLI, observability modules
4. **Direct Imports** - Some tests directly import internals (tight coupling)

### Performance Bottlenecks
1. **Storage Layer** - No pagination on list_events (potential memory issues)
2. **Alert Processing** - Batching strategy not optimized for high throughput
3. **Encryption** - Field-level encryption may be slower than bulk operations

### Test Coverage Gaps
- **Integration Tests:** Minimal coverage for Phase 2 components (EventForwarder, SecurityEventRepo)
- **End-to-end:** No Ralph Loop validation tests written
- **Failure Scenarios:** Limited error handling tests

---

## Blockers for Gemini 3 Integration

### CRITICAL (Block Integration) - 47 Issues

**Cannot proceed with Gemini 3 until ALL syntax errors are fixed.**

#### Syntax Error Resolution Strategy
1. **Automated scan:** 47 files with syntax errors identified
2. **Manual review:** Each file needs careful fix (not bulk regex)
3. **Testing:** Re-validate compilation after each fix
4. **Validation:** Run full pytest after all fixes

#### Files Requiring Immediate Attention
**High Priority (Core Components):**
- `cynic/interfaces/api/entry.py` - Core API entry point
- `cynic/interfaces/api/routers/sdk.py` - SDK router
- `cynic/kernel/core/storage/gc.py` - Storage garbage collection
- `cynic/kernel/organism/brain/cognition/cortex/handlers/act_executor.py` - Action execution

**Medium Priority (CLI/UI):**
- `cynic/interfaces/cli/dashboard.py` - Dashboard display (unmatched paren)
- `cynic/interfaces/cli/tui_dashboard.py` - TUI dashboard
- `cynic/kernel/observability/cli/app.py` - Observability CLI

**Lower Priority (Integrations):**
- `cynic/interfaces/mcp/claude_code_bridge.py` - Claude Code Bridge
- `cynic/interfaces/bots/governance/adapters/*.py` - Bot adapters

### IMPORTANT (Should Fix Before Merging) - 3 Issues

1. **Alerting Webhooks** (cynic/kernel/core/storage/alerting.py)
   - Slack, Jira, PagerDuty APIs not wired
   - Blocks Phase 2 feature completeness
   - Estimated: 3-4 hours

2. **Transaction Signing** (cynic/kernel/organism/perception/integrations/near/executor.py)
   - NEAR transaction signing uses stub
   - Will fail on real transactions
   - Estimated: 2-3 hours

3. **Field-Level Encryption** (Phase 1 Security)
   - Not yet wired into EventForwarder
   - Critical for audit logging compliance
   - Estimated: 2 hours

### NICE-TO-HAVE (Low Priority) - 12 Issues

- MCP endpoint wiring (test-only, not user-facing)
- Selector updates for web automation
- Pluggable filter framework (not used in MVP)

---

## Phase 2 Validation Status

From MEMORY.md: Phase 2 SIEM Foundation marked as COMPLETE with 146/146 tests passing.

**However, cannot verify due to syntax errors blocking test execution.**

### Components Status (Claimed)
| Component | Status | Tests | Blocker |
|-----------|--------|-------|---------|
| 1. Data Architecture (SurrealDB) | COMPLETE | 26 | SYNTAX ERROR |
| 2. EventForwarder (Event Ingestion) | COMPLETE | 25 | SYNTAX ERROR |
| 3. SecurityEventRepo (Storage) | COMPLETE | 25 | SYNTAX ERROR |
| 4. Real-time Detection | COMPLETE | 34 | SYNTAX ERROR |
| 5. Detection Rules | COMPLETE | 33 | SYNTAX ERROR |
| 6. Alerting | COMPLETE | 36 | SYNTAX ERROR |
| 7. Compliance/Audit | COMPLETE | 35 | SYNTAX ERROR |
| Integration (Ralph Loop) | COMPLETE | 8 | SYNTAX ERROR |

**Need to verify all 146 tests still pass after syntax fixes.**

---

## Quick Fix Checklist

### Phase 1: Fix Syntax Errors
- [ ] Unterminated string literals (39 files) - **HIGH PRIORITY**
- [ ] Invalid syntax errors (3 files) - **HIGH PRIORITY**
- [ ] Structural errors (5 files) - **HIGH PRIORITY**
- [ ] Re-test compilation: `pytest --collect-only`

### Phase 2: Run Full Test Suite
- [ ] Execute: `pytest --tb=short -v`
- [ ] Target: 146+ tests passing (Phase 2)
- [ ] Document any new failures

### Phase 3: Address Remaining Debt
- [ ] Fix alerting TODOs (if blocking Phase 3)
- [ ] Wire MCP endpoints (if test-critical)
- [ ] Document NotImplementedError locations

### Phase 4: Gemini 3 Integration
- [ ] All syntax errors: FIXED
- [ ] Test suite: PASSING
- [ ] Import cycles: VERIFIED
- [ ] Ready for integration

---

## Next Steps

**IMMEDIATE (TODAY):**
1. Fix all 47 syntax errors
2. Run `pytest --collect-only` to verify collection works
3. Run `pytest --tb=short -v` to get baseline test results
4. Document any test failures

**FOLLOW-UP (NEXT SESSION):**
1. Fix test failures
2. Address critical TODOs (alerting, transaction signing)
3. Verify Phase 2 components (146 tests)
4. Begin Phase 3 integration planning

**BLOCKING GEMINI 3:**
- All syntax errors must be resolved
- Test suite must have 0 collection errors
- All Phase 2 component tests must pass

---

## References

- CLAUDE.md - Multi-session coordination guide
- ACTION_PLAN.md - Original sprint planning
- CYNIC_SIEM_ANALYSIS.md - Phase 2 SIEM requirements
- docs/status.md - Real-time project status
