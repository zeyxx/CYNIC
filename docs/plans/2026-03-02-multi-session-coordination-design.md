# Multi-Session Coordination Design for CYNIC

**Date:** 2026-03-02
**Status:** Approved for Implementation
**Scope:** N parallel Claude Code sessions (worktrees) coordinated via workflow gates + central observability
**Goal:** Ship CYNIC faster (velocity ↑) without architectural debt (quality ↑)

---

## Executive Summary

CYNIC needs to parallelize development across N Claude Code sessions without conflicts or debt. This design implements **Approach 3: Synchronized Trunks** with **hash-anchored edits** (inspired by oh-my-opencode) and **automated gates** (inspired by UPLINK STUDIO values: rigueur, autonomie, adaptation).

**Key insight:** Worktrees + CI/CD gates + central dashboard = observable, autonomous, high-velocity development.

---

## Architecture Overview

### Layer 1: Session Orchestration (Git Worktrees)

Each session gets an **isolated git worktree**:
- Session A: `worktree cli/priority-10-p4`
- Session B: `worktree metrics/priority-9-p3`
- Session C: `worktree tests/stability-p2`

Each worktree:
- ✅ Points to shared git origin
- ✅ Loads CLAUDE.md + MEMORY.md for context
- ✅ Validates file state before edits (hash-anchored)
- ✅ Commits independently to its branch
- ✅ Can be cleaned up or kept active

**Why worktrees?**
- Zero contamination between sessions
- Fast context switching (same user, multiple branches)
- Automatic conflict detection (git native)
- Easy rollback if session breaks

---

### Layer 2: Pre-Commit Gates (Local Validation)

Before a session commits, it runs **5 automated gates** locally (total: ~3 min):

#### Gate 1: Encoding Validation (45s)
```bash
Tool: scripts/validate_encoding.py
Checks:
  • UTF-8 integrity (no φ corruption from P7-P8 transition)
  • All Python files have correct shebang
  • No BOM markers
Fails: Blocks commit, shows exact file + line
Used by: All sessions
```

#### Gate 2: Circular Import Detection (30s)
```bash
Tool: scripts/analyze_imports.py
Checks:
  • No new circular dependency chains
  • TYPE_CHECKING guards are correct
  • Imports follow fractal hierarchy (nervous → kernel → core)
Fails: Blocks commit, shows cycle graph
Used by: All sessions (critical for 10k TPS)
```

#### Gate 3: Architecture Health (20s)
```bash
Tool: scripts/audit_factory.py
Checks:
  • No new global state introduced
  • Immutability patterns respected (P4 tuples, MappingProxyType)
  • Factory wiring is internally consistent
  • Bus event handlers registered correctly
Fails: Blocks commit, shows audit report
Used by: All sessions
```

#### Gate 4: Code Quality (90s)
```bash
Tool: pytest + coverage (existing)
Checks:
  • No new linting errors
  • All existing tests still pass
  • Test coverage doesn't drop >5%
Fails: Blocks commit, shows test output
Used by: All sessions
```

#### Gate 5: Decision Logging (10s)
```bash
Tool: commit-message-validator (to be created)
Checks:
  • Commit message has format: "type(priority-X-pY): description"
  • Message includes WHAT + WHY (not just WHAT)
  • If breaking change, it's documented
Fails: Blocks commit, shows template
Used by: All sessions
```

**Implementation:**
```bash
# .git/hooks/pre-commit (auto-installed by setup script)
#!/bin/bash
python scripts/validate_encoding.py || exit 1
python scripts/analyze_imports.py || exit 1
python scripts/audit_factory.py || exit 1
pytest tests/unit/ --cov=cynic --cov-fail-under=82 || exit 1
python scripts/validate_commit_message.py || exit 1
```

---

### Layer 3: Daily Sync (GitHub Actions CI/CD)

When a session pushes to its branch and creates a PR:

#### Job 1: Full Test Suite (3 min)
```yaml
# .github/workflows/ci-gates.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Unit Tests (30s)
        run: pytest tests/unit/ -x
      - name: Integration Tests (120s)
        run: pytest tests/e2e/ tests/integration/ -x
      - name: Coverage Report (10s)
        run: pytest --cov=cynic --cov-report=html
      - name: Coverage Check
        run: |
          coverage_pct=$(python -c "import xml.etree.ElementTree as ET;
          root=ET.parse('coverage.xml').getroot();
          print(int(float(root.get('line-rate'))*100))")
          if [ $coverage_pct -lt 87 ]; then
            echo "Coverage dropped to $coverage_pct% (threshold: 87%)"
            exit 1
          fi
```

#### Job 2: Architecture Integrity (2 min)
```yaml
  architecture:
    runs-on: ubuntu-latest
    steps:
      - name: Circular Import Check
        run: python scripts/analyze_imports.py --fail-on-cycle
      - name: Factory Audit
        run: python scripts/audit_factory.py --check-wiring
      - name: Immutability Patterns
        run: python scripts/check_immutability.py
      - name: Tech Debt Report
        run: |
          python scripts/debt_analyzer.py --compare main
          # Fail if debt increased
```

#### Job 3: Performance (1 min)
```yaml
  performance:
    runs-on: ubuntu-latest
    steps:
      - name: Throughput Benchmark
        run: |
          python tests/benchmarks/throughput_test.py
          # Checks: 10k TPS target, memory < 200MB, latency < 100ms p99
      - name: Performance Report
        run: |
          echo "TPS: $(cat /tmp/bench_tps.txt)"
          echo "Memory: $(cat /tmp/bench_mem.txt)"
          # Warn if TPS < 10k (don't block, just alert)
```

#### Job 4: Build Validation (30s)
```yaml
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Start API Server
        run: |
          timeout 30 python -m cynic.api.server &
          sleep 5
          curl -f http://localhost:8000/health || exit 1
      - name: Check No Import Errors
        run: python -c "from cynic import *; print('✅ All imports OK')"
```

**When gates fail:** GitHub PR shows red X, clear failure message, blocks merge until fixed.

---

### Layer 4: Merge Gate (Code Review + Decision Logging)

Before merging to `main`, the PR requires:

#### Automated Checks (GitHub branch protection)
```
✗ MUST pass all 4 CI/CD jobs
✗ MUST have 1 approval from "architect" role (verifies no debt)
✗ MUST have 1 approval from "operator" role (verifies observability)
```

#### Manual Review Checklist
```markdown
## Architecture Review (Architect)
- [ ] No new global state
- [ ] Fractal patterns maintained
- [ ] New dependencies don't create cycles
- [ ] Tech debt stayed flat or decreased

## Operational Review (Operator)
- [ ] Observability integrated (event journal, metrics)
- [ ] Error handling is defensive
- [ ] No breaking changes to API surface
- [ ] Performance targets met or explained
```

#### Merge Commit Format (Enforced by validator)
```
commit: "feat(priority-10-p3): Add CLI review interface

Closes: #ISSUE_NUMBER (if applicable)
Impacts: P9 metrics endpoint (add status filter), P8 SelfProber.execute()
Breaking: False
Tech Debt: -3 (net improvement)

Tested:
  • 5 new CLI tests
  • 2 new integration tests
  • Performance: ✅ 10.1k TPS

Co-Authored-By: Session-A (Claude Code) <session@cynic>
Reviewed-By: @architect, @operator"
```

---

### Layer 5: Central Dashboard (Real-time Observability)

**Location:** `docs/status.md` (auto-updated by GitHub Actions after each merge/PR)

**Content:**
```markdown
# CYNIC Project Status — 2026-03-02 14:30 UTC

## Main Branch Health
- Build: ✅ PASSING (5/5 commits in last 24h)
- Tests: ✅ 103/103 passing
- Coverage: 87.2% (↑ 0.8% from yesterday)
- Architecture: ✅ HEALTHY (0 circular imports, 0 debt)
- Performance: ✅ 10.2k TPS (target: 10k)

## In-Flight Sessions (Worktrees)

### Session A: cli/priority-10-p4
- Status: PR #XX ready for architect review
- Changes: +285 lines (commands.py, execute.py)
- Risk: LOW (isolated to CLI module)
- Gates: ✅ All pre-commit gates passed
- ETA Merge: 2 hours

### Session B: metrics/priority-9-p3
- Status: 1 commit, pre-merge gates running
- Changes: +150 lines (prometheus_bridge.py)
- Risk: MEDIUM (touches export logic)
- Blocker: ⏳ Waiting for P8 MetricsAnalyzer import to merge
- Gates: ⏳ CI/CD running (ETA: 3 min)
- ETA Merge: 4 hours (after P8 merges)

### Session C: tests/stability-p2
- Status: ✅ PR merged 2 hours ago
- Changes: +8 integration tests, +12% coverage
- Risk: NONE (test-only)
- Gates: ✅ All gates passed
- Impact: Baseline tests now more comprehensive

## Recent Decisions (Last 24h)
- P10 CLI: Use --status flag (not enum) for compatibility
- P9 Metrics: Added latency histogram export (5-bucket LOD)
- P8→P10 Bridge: SelfProber.execute() now async
  → Session A PR updated to await calls

## Risks & Blockers
- ⚠️ Session B: Waiting for P8 to merge (ETA: 4h)
- ⚠️ Session A: Review pending (1/2 approvals)
- 🟢 Session C: No blockers

## Tomorrow's Forecast
- P10: Expected merge (high confidence) → unlocks P9
- P9: Expected merge (medium) → depends on P8 completion
- P8: Waiting for 1 architecture review
- P7-P1: Baseline stable (no changes planned)

## How to Help
- If you want to accelerate Session B: Review Session A PR (will unblock P9)
- If architecture review is needed: Check pending P8 PR
- Performance is stable: No optimization needed this cycle
```

**Auto-update mechanism:**
```yaml
# .github/workflows/update-status.yml
on:
  push:
    branches: [main]
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  update-dashboard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Generate Dashboard
        run: python scripts/generate_status_dashboard.py > docs/status.md
      - name: Commit Dashboard
        run: |
          git config user.name "CYNIC Dashboard"
          git config user.email "dashboard@cynic"
          git add docs/status.md
          git commit -m "chore: update project status dashboard" || true
          git push
```

---

### Layer 6: Auto-Continue Logic (Ralph Loop)

**When a session is blocked,** the system detects it:

```python
# scripts/detect_blockers.py
def detect_blockers():
    sessions = get_in_flight_sessions()
    for session in sessions:
        if session.pr.status == "waiting_for":
            blocker_pr = session.pr.blocked_by
            if blocker_pr.can_merge():
                print(f"⚡ Auto-continue: Helping {blocker_pr.id} merge")
                trigger_merge(blocker_pr)
                # Resume blocked session
                print(f"✅ {session.name} can now resume")
            else:
                notify_user(f"⏳ {session.name} blocked for 2h+",
                           f"Consider: {suggest_pivot(session)}")
```

**Runs:** Every 30 minutes (GitHub Actions cron)

**Actions:**
- ✅ If blocker can merge: Trigger merge
- ✅ If blocker can't merge: Daily standup message with pivot suggestions
- ✅ If session idle >4h: Escalate to user

---

## Design Principles

### 1. Rigueur (Rigor) → Automated Gates
Every commit validated before human eyes see it. No surprises in PRs.

### 2. Autonomie (Autonomy) → Worktrees + Parallel Merges
Each session is fully independent. No blockers except actual code conflicts (which CI/CD catches).

### 3. Adaptation (Adaptation) → Central Dashboard + Ralph Loop
See the whole project state in one place. Pivot fast based on real data.

---

## Success Metrics

| Metric | Current | Target | Gate |
|--------|---------|--------|------|
| **Velocity** | 1 priority / 3 days | 1 priority / 1 day | N/A (observing) |
| **Test Coverage** | 87.2% | 87%+ | CI/CD blocks if <87% |
| **Throughput (TPS)** | 10.2k | 10k+ | Performance job warns |
| **Circular Imports** | 0 | 0 | Pre-commit blocks |
| **Tech Debt** | Flat | Decreasing | Architecture review checks |
| **PR Merge Time** | ~2h avg | <1h avg | Dashboard tracks |
| **Session Blocking** | <2h | <30min | Auto-continue detects |

---

## Files to Create / Modify

### New Files
- `scripts/validate_encoding.py` — UTF-8 + BOM validation
- `scripts/analyze_imports.py` — Circular import detection (enhance existing)
- `scripts/audit_factory.py` — Factory wiring audit (enhance existing)
- `scripts/validate_commit_message.py` — Commit format validation
- `scripts/generate_status_dashboard.py` — Generate docs/status.md
- `scripts/detect_blockers.py` — Auto-continue logic
- `.github/workflows/ci-gates.yml` — Full CI/CD pipeline (enhance existing)
- `.github/workflows/update-status.yml` — Dashboard auto-update
- `.git/hooks/pre-commit` — Local validation (auto-installed)
- `docs/status.md` — Live project dashboard
- `docs/MULTI_SESSION_GUIDE.md` — How to use worktrees effectively

### Modified Files
- `CLAUDE.md` — Add multi-session coordination guidelines
- `MEMORY.md` — Document session coordination patterns

---

## Migration Path

**Phase 1 (Today):** Implement pre-commit gates + GitHub Actions CI/CD
**Phase 2 (Tomorrow):** Activate branch protection + dashboard
**Phase 3 (Next cycle):** Deploy auto-continue logic + Ralph Loop

---

## Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Session A merges, breaks Session B | Medium | High | CI/CD full integration tests |
| Pre-commit gates too slow | Low | Medium | Cache compiled imports, run async |
| Dashboard becomes stale | Low | Medium | Auto-update every merge + 30min cron |
| Auto-continue merges bad code | Low | High | Human review gate stays (no auto-merge) |

---

## Success Criteria for Implementation

- ✅ Pre-commit gates working locally (all 5 gates pass in <3 min)
- ✅ CI/CD pipeline green on existing main
- ✅ Status dashboard auto-updating and accurate
- ✅ Worktrees spawn cleanly and don't conflict
- ✅ First session (P10) merges with zero conflicts using new workflow
- ✅ Second session (P9) demonstrates blocking + auto-continue behavior
- ✅ No technical debt introduced (audit stays flat/negative)

---

## Next Steps

→ **Invoke writing-plans skill** to detail the implementation steps, file-by-file
