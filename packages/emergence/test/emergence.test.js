/**
 * @cynic/emergence Tests
 */

import { test, describe } from 'node:test';
import assert from 'node:assert';

import {
  // Consciousness Monitor
  ConsciousnessMonitor,
  createConsciousnessMonitor,
  ConsciousnessState,
  AWARENESS_THRESHOLDS,
  MAX_CONFIDENCE,

  // Pattern Detector
  PatternDetector,
  createPatternDetector,
  PatternType,
  SIGNIFICANCE_THRESHOLDS,

  // Dimension Discovery
  DimensionDiscovery,
  createDimensionDiscovery,
  KNOWN_AXIOMS,
  ProposalStatus,
  ACCEPTANCE_THRESHOLDS,

  // Collective State
  CollectiveState,
  createCollectiveState,
  CollectivePhase,
  PHASE_THRESHOLDS,
  QUORUM,
} from '../src/index.js';

// ═══════════════════════════════════════════════════════════════════════════
// CONSCIOUSNESS MONITOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('ConsciousnessMonitor', () => {
  test('MAX_CONFIDENCE is φ⁻¹', () => {
    assert.ok(Math.abs(MAX_CONFIDENCE - 0.618) < 0.001, 'MAX_CONFIDENCE should be ~0.618');
  });

  test('AWARENESS_THRESHOLDS are φ-aligned', () => {
    assert.ok(Math.abs(AWARENESS_THRESHOLDS.AWAKENING - 0.236) < 0.001);
    assert.ok(Math.abs(AWARENESS_THRESHOLDS.AWARE - 0.382) < 0.001);
    assert.ok(Math.abs(AWARENESS_THRESHOLDS.HEIGHTENED - 0.618) < 0.001);
  });

  test('starts in DORMANT state', () => {
    const monitor = createConsciousnessMonitor();
    assert.strictEqual(monitor.state, ConsciousnessState.DORMANT);
    assert.strictEqual(monitor.awarenessLevel, 0);
  });

  test('observe records observations', () => {
    const monitor = createConsciousnessMonitor();

    const obs = monitor.observe('TEST', { foo: 'bar' }, 0.6);

    assert.ok(obs.id, 'should have ID');
    assert.strictEqual(obs.type, 'TEST');
    assert.strictEqual(obs.confidence, 0.6);
  });

  test('confidence is capped at φ⁻¹', () => {
    const monitor = createConsciousnessMonitor();

    const obs = monitor.observe('TEST', {}, 0.99);

    assert.ok(obs.confidence <= MAX_CONFIDENCE, 'confidence should be capped');
  });

  test('state progresses with observations', () => {
    const monitor = createConsciousnessMonitor();

    // Add many observations
    for (let i = 0; i < 20; i++) {
      monitor.observe('JUDGMENT', { i }, 0.5);
      monitor.recordPrediction('pred_' + i, true, 0.5);
    }

    assert.notStrictEqual(monitor.state, ConsciousnessState.DORMANT, 'should not be dormant');
  });

  test('getInsights returns comprehensive data', () => {
    const monitor = createConsciousnessMonitor();

    for (let i = 0; i < 15; i++) {
      monitor.observe('TEST', { i }, 0.5);
    }

    const insights = monitor.getInsights();

    assert.ok(insights.state, 'should have state');
    assert.ok(insights.awarenessLevel >= 0, 'should have awarenessLevel');
    assert.ok(insights.totalObservations >= 15, 'should have observations');
    assert.ok(Array.isArray(insights.recommendations), 'should have recommendations');
  });

  test('noticePattern tracks patterns', () => {
    const monitor = createConsciousnessMonitor();

    monitor.noticePattern('pattern_1', { type: 'SEQUENCE' }, 0.7);

    const noticed = monitor.hasNoticed('pattern_1');
    assert.ok(noticed, 'should have noticed pattern');
    assert.strictEqual(noticed.noticeCount, 1);

    monitor.noticePattern('pattern_1', { type: 'SEQUENCE' }, 0.8);
    const noticed2 = monitor.hasNoticed('pattern_1');
    assert.strictEqual(noticed2.noticeCount, 2);
  });

  test('recordPrediction tracks accuracy', () => {
    const monitor = createConsciousnessMonitor();

    monitor.recordPrediction('p1', true, 0.6);
    monitor.recordPrediction('p2', true, 0.6);
    monitor.recordPrediction('p3', false, 0.6);

    const accuracy = monitor.getPredictionAccuracy();
    assert.ok(Math.abs(accuracy - 0.666) < 0.01, 'accuracy should be ~0.667');
  });

  test('export/import preserves state', () => {
    const monitor1 = createConsciousnessMonitor();

    for (let i = 0; i < 10; i++) {
      monitor1.observe('TEST', { i }, 0.5);
    }
    monitor1.noticePattern('p1', { type: 'TEST' }, 0.7);

    const exported = monitor1.export();

    const monitor2 = createConsciousnessMonitor();
    monitor2.import(exported);

    assert.strictEqual(monitor2.noticedPatterns.size, 1);
    assert.ok(monitor2.observations.length > 0);
  });

  test('recordUncertainty tracks uncertainty events', () => {
    const monitor = createConsciousnessMonitor();

    // Confidence < 0.382 (φ⁻²) triggers uncertainty zone tracking
    monitor.recordUncertainty('test_context', 0.2, { reason: 'ambiguous input' });

    const insights = monitor.getInsights();
    assert.ok(insights.uncertaintyZones.length >= 1, 'should record uncertainty zone');
    assert.strictEqual(insights.uncertaintyZones[0].context, 'test_context');
  });

  test('getMetaInsight returns meta-level analysis', () => {
    const monitor = createConsciousnessMonitor();

    for (let i = 0; i < 15; i++) {
      monitor.observe('TEST', { i }, 0.5);
      monitor.recordPrediction('p_' + i, i % 2 === 0, 0.5);
    }

    const meta = monitor.getMetaInsight();

    assert.ok(meta.selfAwareness !== undefined, 'should have selfAwareness');
    assert.ok(meta.coherence !== undefined, 'should have coherence');
    assert.ok(meta.blindSpots !== undefined, 'should have blindSpots');
    assert.ok(meta.strengths !== undefined, 'should have strengths');
  });

  test('reset clears all state', () => {
    const monitor = createConsciousnessMonitor();

    for (let i = 0; i < 10; i++) {
      monitor.observe('TEST', { i }, 0.5);
    }
    monitor.noticePattern('p1', { type: 'TEST' }, 0.7);

    assert.ok(monitor.observations.length > 0, 'should have observations before reset');

    monitor.reset();

    assert.strictEqual(monitor.observations.length, 0, 'should clear observations');
    assert.strictEqual(monitor.noticedPatterns.size, 0, 'should clear patterns');
    assert.strictEqual(monitor.state, ConsciousnessState.DORMANT, 'should reset state');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// PATTERN DETECTOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('PatternDetector', () => {
  test('PatternType values exist', () => {
    assert.ok(PatternType.SEQUENCE);
    assert.ok(PatternType.ANOMALY);
    assert.ok(PatternType.TREND);
    assert.ok(PatternType.CLUSTER);
  });

  test('SIGNIFICANCE_THRESHOLDS are φ-aligned', () => {
    assert.ok(Math.abs(SIGNIFICANCE_THRESHOLDS.TRIVIAL - 0.236) < 0.001);
    assert.ok(Math.abs(SIGNIFICANCE_THRESHOLDS.NOTABLE - 0.382) < 0.001);
    assert.ok(Math.abs(SIGNIFICANCE_THRESHOLDS.SIGNIFICANT - 0.618) < 0.001);
  });

  test('observe records data points', () => {
    const detector = createPatternDetector();

    detector.observe({ type: 'TEST', value: 50 });
    detector.observe({ type: 'TEST', value: 55 });

    const stats = detector.getStats();
    assert.strictEqual(stats.dataPoints, 2);
  });

  test('detects anomalies', () => {
    const detector = createPatternDetector({ anomalyThreshold: 2 });

    // Add normal values
    for (let i = 0; i < 30; i++) {
      detector.observe({ type: 'SCORE', value: 50 + Math.random() * 10 });
    }

    // Add anomaly
    detector.observe({ type: 'SCORE', value: 150 });

    const patterns = detector.detect();
    const anomalies = patterns.filter(p => p.type === PatternType.ANOMALY);

    assert.ok(anomalies.length > 0, 'should detect anomaly');
  });

  test('detects trends', () => {
    const detector = createPatternDetector();

    // Add increasing values
    for (let i = 0; i < 30; i++) {
      detector.observe({ type: 'SCORE', value: 30 + i * 2 });
    }

    const patterns = detector.detect();
    const trends = patterns.filter(p => p.type === PatternType.TREND);

    assert.ok(trends.length > 0, 'should detect trend');
    assert.strictEqual(trends[0].data.direction, 'INCREASING');
  });

  test('detects sequences', () => {
    const detector = createPatternDetector();

    // Add repeating sequence
    for (let i = 0; i < 20; i++) {
      detector.observe({ type: 'A', verdict: 'GROWL' });
      detector.observe({ type: 'B', verdict: 'WAG' });
    }

    const patterns = detector.detect();
    const sequences = patterns.filter(p => p.type === PatternType.SEQUENCE);

    assert.ok(sequences.length > 0, 'should detect sequence');
  });

  test('getTopPatterns returns sorted by significance', () => {
    const detector = createPatternDetector();

    // Generate various patterns
    for (let i = 0; i < 50; i++) {
      detector.observe({ type: 'TEST', value: i % 10 === 0 ? 100 : 50 });
    }

    const top = detector.getTopPatterns(5);

    assert.ok(top.length <= 5);
    for (let i = 1; i < top.length; i++) {
      assert.ok(top[i - 1].significance >= top[i].significance, 'should be sorted');
    }
  });

  test('export/import preserves state', () => {
    const detector1 = createPatternDetector();

    for (let i = 0; i < 20; i++) {
      detector1.observe({ type: 'TEST', value: 50 });
    }

    const exported = detector1.export();

    const detector2 = createPatternDetector();
    detector2.import(exported);

    assert.ok(detector2.dataPoints.length > 0);
    assert.strictEqual(detector2.stats.count, detector1.stats.count);
  });

  test('getPatterns filters by type and significance', () => {
    const detector = createPatternDetector();

    // Generate patterns
    for (let i = 0; i < 30; i++) {
      detector.observe({ type: 'SCORE', value: 30 + i * 2 }); // Creates trend
    }

    const allPatterns = detector.getPatterns();
    assert.ok(Array.isArray(allPatterns), 'should return array');

    const trends = detector.getPatterns(PatternType.TREND);
    assert.ok(trends.every(p => p.type === PatternType.TREND), 'should filter by type');

    const significant = detector.getPatterns(null, 0.5);
    assert.ok(significant.every(p => p.significance >= 0.5), 'should filter by significance');
  });

  test('hasPattern checks pattern existence', () => {
    const detector = createPatternDetector();

    // Generate some patterns
    for (let i = 0; i < 30; i++) {
      detector.observe({ type: 'SCORE', value: 50 + (i % 2) * 20 });
    }

    const patterns = detector.detect();
    if (patterns.length > 0) {
      const patternId = patterns[0].id;
      assert.ok(detector.hasPattern(patternId), 'should find existing pattern');
    }

    assert.strictEqual(detector.hasPattern('nonexistent_pattern'), null, 'should return null for unknown pattern');
  });

  test('clear resets detector state', () => {
    const detector = createPatternDetector();

    for (let i = 0; i < 20; i++) {
      detector.observe({ type: 'TEST', value: 50 });
    }

    assert.ok(detector.dataPoints.length > 0, 'should have data before clear');

    detector.clear();

    assert.strictEqual(detector.dataPoints.length, 0, 'should clear data points');
    assert.strictEqual(detector.stats.count, 0, 'should reset stats');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// DIMENSION DISCOVERY TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('DimensionDiscovery', () => {
  test('KNOWN_AXIOMS has 4 axioms with 6 dimensions each', () => {
    assert.strictEqual(Object.keys(KNOWN_AXIOMS).length, 4);
    assert.strictEqual(KNOWN_AXIOMS.PHI.length, 6);
    assert.strictEqual(KNOWN_AXIOMS.VERIFY.length, 6);
    assert.strictEqual(KNOWN_AXIOMS.CULTURE.length, 6);
    assert.strictEqual(KNOWN_AXIOMS.BURN.length, 6);
  });

  test('ACCEPTANCE_THRESHOLDS are φ-aligned', () => {
    assert.strictEqual(ACCEPTANCE_THRESHOLDS.MIN_VOTES, 7); // 7 Sefirot
    assert.ok(Math.abs(ACCEPTANCE_THRESHOLDS.APPROVAL_RATIO - 0.618) < 0.001);
  });

  test('getDimensionsForAxiom returns known dimensions', () => {
    const discovery = createDimensionDiscovery();

    const phiDims = discovery.getDimensionsForAxiom('PHI');
    assert.ok(phiDims.includes('COHERENCE'));
    assert.ok(phiDims.includes('HARMONY'));
  });

  test('analyzeJudgment processes judgments', () => {
    const discovery = createDimensionDiscovery();

    const detected = discovery.analyzeJudgment({
      scores: { PHI: 45, VERIFY: 72, CULTURE: 38, BURN: 60 },
      rawAssessment: 'Lacks transparency in methodology and reproducibility.',
    });

    assert.ok(Array.isArray(detected));
  });

  test('propose creates a proposal', () => {
    const discovery = createDimensionDiscovery({ nodeId: 'test_node' });

    const proposal = discovery.propose(
      'METHODOLOGY',
      'VERIFY',
      'Measures clarity of process/approach',
      'Recurring pattern of methodology critiques'
    );

    assert.strictEqual(proposal.name, 'METHODOLOGY');
    assert.strictEqual(proposal.axiom, 'VERIFY');
    assert.strictEqual(proposal.status, ProposalStatus.DRAFT);
    assert.strictEqual(proposal.proposer, 'test_node');
  });

  test('proposal workflow: draft -> review -> voting', () => {
    const discovery = createDimensionDiscovery();

    const proposal = discovery.propose('TEST_DIM', 'PHI', 'Test', 'Test reason');
    assert.strictEqual(proposal.status, ProposalStatus.DRAFT);

    discovery.submitForReview(proposal.id);
    assert.strictEqual(proposal.status, ProposalStatus.UNDER_REVIEW);

    discovery.openVoting(proposal.id);
    assert.strictEqual(proposal.status, ProposalStatus.VOTING);
  });

  test('voting requires minimum votes', () => {
    const discovery = createDimensionDiscovery();

    const proposal = discovery.propose('TEST_DIM', 'PHI', 'Test', 'Test reason');
    discovery.submitForReview(proposal.id);
    discovery.openVoting(proposal.id);

    // Vote but not enough for conclusion
    const result = discovery.vote(proposal.id, 'node1', true);

    assert.strictEqual(result.result.concluded, false);
    assert.strictEqual(result.result.reason, 'INSUFFICIENT_VOTES');
  });

  test('proposal accepted with sufficient approval', () => {
    const discovery = createDimensionDiscovery();

    const proposal = discovery.propose('TEST_DIM', 'VERIFY', 'Test', 'Test reason');
    discovery.submitForReview(proposal.id);
    discovery.openVoting(proposal.id);

    // 7 votes, all approve (100% > 61.8%)
    for (let i = 1; i <= 7; i++) {
      discovery.vote(proposal.id, `node${i}`, true);
    }

    assert.strictEqual(proposal.status, ProposalStatus.ACCEPTED);
    assert.strictEqual(discovery.getAdopted().length, 1);
  });

  test('duplicate proposals are rejected', () => {
    const discovery = createDimensionDiscovery();

    discovery.propose('UNIQUE', 'PHI', 'Test', 'Test');

    assert.throws(() => {
      discovery.propose('UNIQUE', 'PHI', 'Test 2', 'Test 2');
    }, /already proposed/);
  });

  test('export/import preserves state', () => {
    const discovery1 = createDimensionDiscovery();

    discovery1.propose('TEST', 'BURN', 'Test', 'Test');

    const exported = discovery1.export();

    const discovery2 = createDimensionDiscovery();
    discovery2.import(exported);

    assert.strictEqual(discovery2.getProposals().length, 1);
  });

  test('getCandidates returns potential new dimensions', () => {
    const discovery = createDimensionDiscovery();

    // Analyze judgments to generate candidates
    for (let i = 0; i < 10; i++) {
      discovery.analyzeJudgment({
        scores: { PHI: 45, VERIFY: 30, CULTURE: 38, BURN: 60 },
        rawAssessment: 'Issues with methodology and reproducibility detected.',
      });
    }

    const candidates = discovery.getCandidates();
    assert.ok(Array.isArray(candidates), 'should return array');

    const verifyCandidates = discovery.getCandidates('VERIFY');
    assert.ok(verifyCandidates.every(c => c.axiom === 'VERIFY'), 'should filter by axiom');
  });

  test('getStats returns discovery statistics', () => {
    const discovery = createDimensionDiscovery();

    discovery.propose('DIM1', 'PHI', 'Test 1', 'Reason 1');
    discovery.propose('DIM2', 'VERIFY', 'Test 2', 'Reason 2');

    const stats = discovery.getStats();

    assert.ok(stats.proposals.total >= 2, 'should count proposals');
    assert.ok(stats.proposals.draft >= 0, 'should have draft count');
    assert.ok(stats.candidates !== undefined, 'should have candidates count');
    assert.ok(stats.termsTracked !== undefined, 'should have termsTracked');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// COLLECTIVE STATE TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('CollectiveState', () => {
  test('QUORUM values', () => {
    assert.strictEqual(QUORUM.MINIMUM, 3);
    assert.strictEqual(QUORUM.STANDARD, 5);
    assert.strictEqual(QUORUM.CRITICAL, 7);
  });

  test('PHASE_THRESHOLDS use φ-aligned values', () => {
    assert.ok(Math.abs(PHASE_THRESHOLDS.COHERENT.minCoherence - 0.236) < 0.001);
    assert.ok(Math.abs(PHASE_THRESHOLDS.RESONANT.minCoherence - 0.618) < 0.001);
  });

  test('starts in ISOLATED phase', () => {
    const collective = createCollectiveState({ nodeId: 'test' });

    assert.strictEqual(collective.phase, CollectivePhase.ISOLATED);
    assert.strictEqual(collective.activeNodes, 0);
  });

  test('reportState adds local node', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.reportState({
      eScore: 72,
      awarenessLevel: 0.58,
      consciousnessState: 'AWARE',
    });

    assert.strictEqual(collective.activeNodes, 1);
  });

  test('receiveState adds remote nodes', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('remote1', { eScore: 75, awarenessLevel: 0.6, consciousnessState: 'AWARE' });
    collective.receiveState('remote2', { eScore: 68, awarenessLevel: 0.45, consciousnessState: 'AWARE' });

    assert.strictEqual(collective.activeNodes, 3);
  });

  test('phase progresses with more nodes', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    // Add nodes
    collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('n1', { eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });

    assert.strictEqual(collective.phase, CollectivePhase.FORMING);

    collective.receiveState('n2', { eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });

    // Should be at least FORMING with 3 nodes
    assert.notStrictEqual(collective.phase, CollectivePhase.ISOLATED);
  });

  test('hasQuorum checks node count', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    assert.strictEqual(collective.hasQuorum('MINIMUM'), false);

    collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('n1', { eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('n2', { eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });

    assert.strictEqual(collective.hasQuorum('MINIMUM'), true);
    assert.strictEqual(collective.hasQuorum('CRITICAL'), false);
  });

  test('getCollectiveVerdict aggregates opinions', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    // Add nodes
    collective.reportState({ eScore: 80, awarenessLevel: 0.6, consciousnessState: 'AWARE' });
    collective.receiveState('n1', { eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('n2', { eScore: 75, awarenessLevel: 0.55, consciousnessState: 'AWARE' });

    const verdict = collective.getCollectiveVerdict({
      'local': { verdict: 'GROWL', confidence: 0.7 },
      'n1': { verdict: 'GROWL', confidence: 0.6 },
      'n2': { verdict: 'WAG', confidence: 0.5 },
    });

    assert.strictEqual(verdict.verdict, 'GROWL');
    assert.ok(verdict.confidence > 0, 'should have confidence');
    assert.strictEqual(verdict.participants, 3);
  });

  test('getCollectiveVerdict requires quorum', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });

    const verdict = collective.getCollectiveVerdict({
      'local': { verdict: 'GROWL', confidence: 0.7 },
    });

    assert.strictEqual(verdict.verdict, null);
    assert.strictEqual(verdict.reason, 'INSUFFICIENT_QUORUM');
  });

  test('remember/recall collective memory', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.remember('important_fact', { value: 42 }, 0.8);

    const recalled = collective.recall('important_fact');
    assert.deepStrictEqual(recalled, { value: 42 });

    const strength = collective.getMemoryStrength('important_fact');
    assert.strictEqual(strength, 0.8);
  });

  test('recordConsensus tracks decisions', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.recordConsensus('topic1', { result: 'accepted' }, ['n1', 'n2', 'n3'], 0.9);
    collective.recordConsensus('topic2', { result: 'rejected' }, ['n1', 'n2', 'n3'], 0.3);

    const stats = collective.getStats();
    assert.strictEqual(stats.metrics.totalJudgments, 2);
    assert.strictEqual(stats.metrics.consensusCount, 1);
    assert.strictEqual(stats.metrics.divergenceCount, 1);
  });

  test('getCollectiveInsight returns comprehensive data', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('n1', { eScore: 75, awarenessLevel: 0.55, consciousnessState: 'AWARE' });
    collective.receiveState('n2', { eScore: 72, awarenessLevel: 0.52, consciousnessState: 'AWARE' });

    const insight = collective.getCollectiveInsight();

    assert.ok(insight.phase, 'should have phase');
    assert.ok(insight.coherence >= 0, 'should have coherence');
    assert.ok(insight.nodes, 'should have nodes');
    assert.ok(insight.health, 'should have health');
  });

  test('export/import preserves state', () => {
    const collective1 = createCollectiveState({ nodeId: 'local' });

    collective1.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective1.remember('test', { value: 123 }, 0.9);

    const exported = collective1.export();

    const collective2 = createCollectiveState({ nodeId: 'local' });
    collective2.import(exported);

    assert.strictEqual(collective2.recall('test').value, 123);
  });

  test('removeNode removes disconnected nodes', () => {
    const collective = createCollectiveState({ nodeId: 'local' });

    collective.reportState({ eScore: 70, awarenessLevel: 0.5, consciousnessState: 'AWARE' });
    collective.receiveState('n1', { eScore: 75, awarenessLevel: 0.55, consciousnessState: 'AWARE' });
    collective.receiveState('n2', { eScore: 72, awarenessLevel: 0.52, consciousnessState: 'AWARE' });

    assert.strictEqual(collective.activeNodes, 3, 'should have 3 nodes');

    collective.removeNode('n1');

    assert.strictEqual(collective.activeNodes, 2, 'should have 2 nodes after removal');

    // Removing the same node again should be safe
    collective.removeNode('n1');
    assert.strictEqual(collective.activeNodes, 2, 'should still have 2 nodes');

    // Removing non-existent node should be safe
    collective.removeNode('nonexistent');
    assert.strictEqual(collective.activeNodes, 2, 'should still have 2 nodes');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Integration', () => {
  test('ConsciousnessMonitor + PatternDetector integration', () => {
    const monitor = createConsciousnessMonitor();
    const detector = createPatternDetector();

    // Generate observations that also feed the pattern detector
    for (let i = 0; i < 30; i++) {
      const value = 50 + (i % 3) * 10; // Creates pattern
      monitor.observe('SCORE', { value }, 0.5);
      detector.observe({ type: 'SCORE', value });
    }

    // Detect patterns
    const patterns = detector.detect();

    // Feed patterns to consciousness monitor
    for (const pattern of patterns) {
      if (pattern.significance > SIGNIFICANCE_THRESHOLDS.NOTABLE) {
        monitor.noticePattern(pattern.id, pattern, pattern.significance);
      }
    }

    // Monitor should have noticed patterns
    assert.ok(monitor.noticedPatterns.size > 0, 'should have noticed patterns');
  });

  test('Full emergence pipeline', () => {
    // Create all components
    const monitor = createConsciousnessMonitor();
    const detector = createPatternDetector();
    const discovery = createDimensionDiscovery({ nodeId: 'test_node' });
    const collective = createCollectiveState({ nodeId: 'test_node' });

    // 1. Generate observations
    for (let i = 0; i < 20; i++) {
      const judgment = {
        verdict: i % 3 === 0 ? 'GROWL' : 'WAG',
        score: 40 + Math.random() * 40,
      };

      monitor.observe('JUDGMENT', judgment, 0.5);
      detector.observe({ ...judgment, value: judgment.score });
      discovery.analyzeJudgment({
        scores: { PHI: judgment.score, VERIFY: judgment.score },
        rawAssessment: 'Test assessment with methodology concerns.',
      });
    }

    // 2. Report to collective
    collective.reportState({
      eScore: 70,
      awarenessLevel: monitor.awarenessLevel,
      consciousnessState: monitor.state,
      patterns: detector.getTopPatterns(5),
    });

    // 3. Verify integration
    assert.ok(monitor.state !== ConsciousnessState.DORMANT || monitor.awarenessLevel > 0);
    assert.ok(detector.getStats().dataPoints > 0);
    assert.strictEqual(collective.activeNodes, 1);
  });
});
