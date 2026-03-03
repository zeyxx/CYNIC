# PHASE 2: Ralph Loop + CYNIC Judgment Framework

## The Ralph Loop: Continuous Self-Validation

**Ralph Loop** = Automated validation cycle that runs AFTER each component implementation:

```
IMPLEMENTATION
    ↓
Ralph Loop Iteration 1:
    ├─ Unit Tests (20+ per component)
    ├─ Integration Tests (with adjacent components)
    ├─ Falsification Tests (synthetic attacks)
    └─ CYNIC Judgment (φ-bounded evaluation)
        ↓
    If φ_score > 0.618: CRYSTALLIZED ✅ (merge, move forward)
    If φ_score < 0.382: FORGOTTEN ❌ (discard, restart)
    If 0.382 < φ_score < 0.618: ITERATE (improve)
```

---

## CYNIC Judgment Scoring (φ-bounded)

**5 Axioms** (Fidelity, PHI, Verify, Culture, Burn):

### Component Evaluation

**FIDELITY** (0-1): Does code match specification?
- Unit test pass rate (0.4 weight)
- Code matches decomposition design (0.3 weight)
- No breaking changes to prior components (0.3 weight)

**PHI** (0.618): Golden ratio boundary
- Performance metrics met? (0.3 weight)
- Latency targets achieved? (0.3 weight)
- Throughput targets met? (0.4 weight)

**VERIFY** (0-1): Multi-layer validation
- Unit tests passing (0.3 weight)
- Integration tests passing (0.3 weight)
- Falsification tests (attacks) passing (0.4 weight)

**CULTURE** (0-1): Follows CYNIC patterns
- Factory integration matches existing pattern (0.3 weight)
- Logging/metrics consistent with codebase (0.3 weight)
- No new circular dependencies (0.4 weight)

**BURN** (0-1): Simplicity & maintainability
- Code < 500 lines per class (0.3 weight)
- No premature abstractions (0.3 weight)
- Follows CYNIC's "don't over-engineer" philosophy (0.4 weight)

**Q-SCORE = Geometric Mean of 5 Axioms**
```python
q_score = (FIDELITY × PHI × VERIFY × CULTURE × BURN) ^ (1/5)
```

**Crystallization Threshold**: q_score ≥ 0.618 (golden ratio)
- Component is STABLE, CORRECT, MAINTAINABLE
- Safe to merge and move forward

---

## Component-by-Component Ralph Loop

### COMPONENT 4: Real-Time Detection

**Falsification Tests (Attack Scenarios):**
```python
# Test 1: Can we detect rapid voting (Stage 1: Reconnaissance)?
async def test_detect_api_scanning():
    # Simulate 100+ API calls in 5 min
    for i in range(100):
        await bus.emit(Event.typed(CoreEvent.API_REQUEST, {...}))

    # Should trigger Stage1_APIScanning rule
    assert detector.alerts_generated > 0
    assert detector.latest_alert.rule_id == "STAGE_1_API_SCANNING"

# Test 2: Can we detect suspicious proposals (Stage 2: Weaponization)?
async def test_detect_suspicious_proposal():
    # Value > 10x median + new actor + short delay
    event = {
        "type": "proposal_created",
        "proposal_value": 1000000,  # 10x+ median
        "execution_delay_hours": 0.5,  # < 1 hour
        "actor_id": "new_actor_xyz",
    }

    matched = await rule_engine.Stage2_SuspiciousProposal.evaluate(event)
    assert matched is True

# Test 3: Can we detect consensus manipulation (Stage 4)?
async def test_detect_consensus_manipulation():
    # Dogs suspiciously agreeing (variance < 10% baseline)
    event = {
        "type": "judgment_created",
        "consensus_variance": 0.01,  # Very low variance
    }
    baselines = {"consensus_variance_baseline": 0.2}

    matched = await rule_engine.Stage4_ConsensusManipulation.evaluate(event, baselines)
    assert matched is True
```

**CYNIC Judgment Criteria:**
- [ ] FIDELITY: Unit tests 20/20 passing
- [ ] PHI: Latency < 100ms (90th percentile)
- [ ] VERIFY: Integration tests 10/10 passing + 8/8 falsification passing
- [ ] CULTURE: Factory pattern matching, metrics exported
- [ ] BURN: < 800 lines total code, no over-abstraction

**Decision**: If all 5 axioms φ > 0.618 → CRYSTALLIZED ✅

---

### COMPONENT 5: Detection Rules

**Falsification Tests (Kill Chain Coverage):**
```python
# Stage 1: Reconnaissance
async def test_stage1_api_scanning():
    # 100+ API calls in 5 min
    rule = Stage1_APIScanning()
    assert await rule.evaluate(attack_event_stage1) == True

# Stage 2: Weaponization
async def test_stage2_suspicious_proposal():
    rule = Stage2_SuspiciousProposal()
    assert await rule.evaluate(attack_event_stage2) == True

# Stage 3: Delivery
async def test_stage3_voting_bloc():
    rule = Stage3_LargeVotingBloc()
    assert await rule.evaluate(attack_event_stage3) == True

# ... Stage 4-7 ...

# False positive test: Normal activity shouldn't trigger
async def test_no_false_positives():
    normal_events = [
        normal_proposal_event,
        normal_vote_event,
        normal_judgment_event,
    ]

    false_positives = 0
    for event in normal_events:
        matches = await rule_engine.evaluate_all(event)
        if matches:
            false_positives += 1

    assert false_positives == 0  # 0% false positive rate acceptable
```

**CYNIC Judgment Criteria:**
- [ ] FIDELITY: All 7 stages covered, 25-50 rules implemented
- [ ] PHI: False positive rate < 5%
- [ ] VERIFY: All falsification tests passing (8+ scenarios)
- [ ] CULTURE: Rule registry matches CYNIC's pattern
- [ ] BURN: Rule base < 1000 lines, maintainable

**Decision**: If all 5 axioms φ > 0.618 → CRYSTALLIZED ✅

---

### COMPONENT 6: Alerting & Escalation

**Falsification Tests (L1/L2/L3 Triage):**
```python
# Test L1: Deduplication prevents alert spam
async def test_deduplication():
    alert1 = create_alert(rule_id="STAGE_2", actor="attacker1")
    alert2 = create_alert(rule_id="STAGE_2", actor="attacker1")  # Duplicate

    routed1 = await deduplicator.should_route(alert1)  # True
    routed2 = await deduplicator.should_route(alert2)  # False (deduplicated)

    assert routed1 == True
    assert routed2 == False

# Test L2: Routing to correct channels by severity
async def test_routing_by_severity():
    critical_alert = create_alert(severity=AlertSeverity.CRITICAL)
    high_alert = create_alert(severity=AlertSeverity.HIGH)

    critical_channels = critical_alert.get_routing_channels()
    high_channels = high_alert.get_routing_channels()

    assert "pagerduty" in critical_channels
    assert "pagerduty" not in high_channels

# Test L3: Escalation path triggers correctly
async def test_escalation_path():
    alert = create_alert(timestamp=time.time() - 1800)  # 30 min old

    escalation_level = await escalation_path.check_escalation(alert)

    assert escalation_level == "L2"  # Should escalate after 15 min at L1
```

**CYNIC Judgment Criteria:**
- [ ] FIDELITY: All alert types route correctly
- [ ] PHI: Deduplication rate 95-99% (prevents spam)
- [ ] VERIFY: All routing tests passing + L1/L2/L3 workflow tested
- [ ] CULTURE: Alert queue, metrics, audit trail consistent
- [ ] BURN: AlertRouter < 300 lines, simple channel logic

**Decision**: If all 5 axioms φ > 0.618 → CRYSTALLIZED ✅

---

### COMPONENT 7: Compliance & Audit

**Falsification Tests (Forensics & Compliance):**
```python
# Test: Audit trail captures everything
async def test_audit_trail_completeness():
    # Simulate attack + detection + alert + response
    await event_forwarder.on_event(attack_event)
    await rule_engine.evaluate_all(attack_event)
    alert = await alert_generator.emit_alert(rule_match)
    await alert_router.route(alert)

    # Check audit trail has all steps
    audit_events = await compliance_query.all_events_by_resource(alert.alert_id)

    assert len(audit_events) >= 4  # At least: event, rule, alert, route
    assert any(e["action"] == "created" for e in audit_events)
    assert any(e["action"] == "generated" for e in audit_events)
    assert any(e["action"] == "routed" for e in audit_events)

# Test: Integrity verification works
async def test_audit_integrity_verification():
    # All events should have valid immutable hashes
    is_intact = await compliance_query.verify_integrity()
    assert is_intact == True

# Test: Retention policy enforced
async def test_retention_enforcement():
    # Add old event (> 365 days old)
    old_event = {
        "type": "audit_log",
        "timestamp": time.time() - (400 * 24 * 3600),
    }
    await storage.security_events.save_event(old_event)

    # Enforce retention
    result = await retention_enforcer.enforce_retention()

    assert result["deleted"] > 0  # Should delete old events

# Test: SOC2/GDPR reports generated
async def test_compliance_reports():
    soc2_report = await compliance_report.generate_soc2_report(days=30)

    assert "total_events" in soc2_report
    assert "by_event_type" in soc2_report
    assert "integrity_verified" in soc2_report
    assert soc2_report["integrity_verified"] == True
```

**CYNIC Judgment Criteria:**
- [ ] FIDELITY: All events logged, audit trail complete
- [ ] PHI: Integrity verification 100% (no tampering detected)
- [ ] VERIFY: All forensics tests passing + compliance reports correct
- [ ] CULTURE: AuditLogger pattern consistent with PHASE 2
- [ ] BURN: AuditLogger < 400 lines, clear event types

**Decision**: If all 5 axioms φ > 0.618 → CRYSTALLIZED ✅

---

## Ralph Loop Workflow

### Per Component (4 components × 1 day each):

**Day 1: Component 4 (Real-Time Detection)**
```
Morning:   Implement StreamDetector, BaselineCalculator, AnomalyScorer, DetectionRule
           ↓
           Create 20+ unit tests (falsification: attack scenarios)
           ↓
Afternoon: Run tests → debug → iterate
           ↓
           Generate CYNIC judgment scores (5 axioms)
           ↓
Evening:   If φ > 0.618 → CRYSTALLIZED ✅ → Merge
           If φ < 0.618 → Iterate (improve weakest axiom)
           ↓
Next Day:  Commit: feat(security-p2-component4)
```

**Days 2-4: Components 5, 6, 7** (same rhythm)

### Final Ralph Loop Integration (Day 5):

All 4 components together:
```
Integration Tests:
  ├─ Component 4 → Component 5 (rules receive scores)
  ├─ Component 5 → Component 6 (matches generate alerts)
  ├─ Component 6 → Component 7 (alerts logged)
  └─ All components → Factory (lifecycle integration)

Falsification: Full attack scenario
  ├─ Stage 1 (Reconnaissance) → Detected ✅
  ├─ Stage 2 (Weaponization) → Detected ✅
  ├─ Stage 3 (Delivery) → Detected ✅
  ├─ Stage 4 (Exploitation) → Detected ✅
  ├─ Stage 5 (Installation) → Detected ✅
  ├─ Stage 6 (C2) → Detected ✅
  ├─ Stage 7 (AoO) → Detected ✅
  └─ Alerts routed + logged ✅

CYNIC Judgment (integration):
  ├─ FIDELITY: Full pipeline working (1.0)
  ├─ PHI: End-to-end latency < 500ms (0.9)
  ├─ VERIFY: 100+ tests passing (1.0)
  ├─ CULTURE: No regressions, clean architecture (1.0)
  └─ BURN: Simple, maintainable, no over-engineering (1.0)
     → Q = (1.0 × 0.9 × 1.0 × 1.0 × 1.0)^(1/5) = 0.98 ✅✅✅

Result: PHASE 2 CRYSTALLIZED → Production Ready
```

---

## Implementation Timeline

```
Session 6Q (This Session):

Hour 1: Implement Component 4 + Tests (20)
         Ralph Loop: Score, iterate if needed
         Merge when φ > 0.618

Hour 2: Implement Component 5 + Tests (20)
         Ralph Loop: Score, iterate if needed
         Merge when φ > 0.618

Hour 3: Implement Component 6 + Tests (20)
         Ralph Loop: Score, iterate if needed
         Merge when φ > 0.618

Hour 4: Implement Component 7 + Tests (15)
         Ralph Loop: Score, iterate if needed
         Merge when φ > 0.618

Hour 5: INTEGRATION Ralph Loop
         All 4 components together
         Falsification: Full attack scenario
         Final CYNIC judgment
         If φ > 0.618 → Final merge + celebration 🎉

Total Implementation: ~4-5 hours of intense development
Total Tests: 75+ new tests
Total Commits: 4-5 feature commits
Final Status: PHASE 2 COMPLETE ✅✅✅✅✅
```

---

## CYNIC Judgment System

**Verdict Outcomes:**
- **HOWL** (φ > 0.8): Excellent, production-ready, exemplary code
- **WAG** (0.618 < φ ≤ 0.8): Good, ready to merge, acceptable quality
- **GROWL** (0.382 < φ ≤ 0.618): Mediocre, needs iteration, not ready
- **BARK** (φ ≤ 0.382): Failure, discard, restart with new approach

**Confidence Level**: Max 61.8% (φ^-1)
- We remain humble about code quality
- No false certainty
- Always room for improvement

---

## Success Definition

PHASE 2 is **COMPLETE** when:
1. All 4 components (4-7) implemented ✅
2. All 75+ tests passing ✅
3. All falsification scenarios passing ✅
4. All CYNIC judgment scores > 0.618 ✅
5. Full integration working (4→5→6→7→Factory) ✅
6. All 4 components merged to master ✅
7. PHASE 2 documented & approved ✅

**Time to completion**: 1 intense development session (~5 hours)

---

## Ready for Implementation?

**On y va?** 🐕✨
