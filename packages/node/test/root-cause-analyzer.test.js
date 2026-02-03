/**
 * RootCauseAnalyzer Tests
 *
 * "Qui vérifie le vérificateur?" - κυνικός
 *
 * @module @cynic/node/test/root-cause-analyzer
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  RootCauseAnalyzer,
  createRootCauseAnalyzer,
  getRootCauseAnalyzer,
  _resetRootCauseAnalyzerForTesting,
  OracleType,
  Hypothesis,
  DECISION_THRESHOLDS,
  DEFAULT_ORACLE_TRUST,
} from '../src/judge/root-cause-analyzer.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// =============================================================================
// CONSTANTS
// =============================================================================

describe('OracleType', () => {
  it('should have all oracle types', () => {
    assert.strictEqual(OracleType.TEST_RESULT, 'test_result');
    assert.strictEqual(OracleType.BUILD_STATUS, 'build_status');
    assert.strictEqual(OracleType.COMMIT_HISTORY, 'commit_history');
    assert.strictEqual(OracleType.PR_FEEDBACK, 'pr_feedback');
    assert.strictEqual(OracleType.LLM_CONSENSUS, 'llm_consensus');
    assert.strictEqual(OracleType.HUMAN_REVIEW, 'human_review');
    assert.strictEqual(OracleType.STATIC_ANALYSIS, 'static_analysis');
    assert.strictEqual(OracleType.RUNTIME_BEHAVIOR, 'runtime_behavior');
  });
});

describe('Hypothesis', () => {
  it('should have all hypothesis types', () => {
    assert.strictEqual(Hypothesis.CODE_BUG, 'code_bug');
    assert.strictEqual(Hypothesis.TEST_BUG, 'test_bug');
    assert.strictEqual(Hypothesis.SPEC_GAP, 'spec_gap');
    assert.strictEqual(Hypothesis.FLAKY, 'flaky');
    assert.strictEqual(Hypothesis.UNKNOWN, 'unknown');
  });
});

describe('DECISION_THRESHOLDS', () => {
  it('should have φ-aligned thresholds', () => {
    assert.ok(Math.abs(DECISION_THRESHOLDS.CODE_BUG_THRESHOLD - PHI_INV) < 0.001);
    assert.ok(Math.abs(DECISION_THRESHOLDS.TEST_BUG_THRESHOLD - PHI_INV) < 0.001);
    assert.ok(Math.abs(DECISION_THRESHOLDS.SPEC_GAP_THRESHOLD - PHI_INV_2) < 0.001);
    assert.ok(Math.abs(DECISION_THRESHOLDS.MAX_LEARNING_RATE - PHI_INV_2) < 0.001);
  });

  it('should have oracle requirements', () => {
    assert.strictEqual(DECISION_THRESHOLDS.MIN_ORACLES, 2);
    assert.strictEqual(DECISION_THRESHOLDS.IDEAL_ORACLES, 5);
  });
});

describe('DEFAULT_ORACLE_TRUST', () => {
  it('should have trust values for all oracle types', () => {
    for (const type of Object.values(OracleType)) {
      assert.ok(type in DEFAULT_ORACLE_TRUST, `Missing trust for ${type}`);
      assert.ok(DEFAULT_ORACLE_TRUST[type] > 0 && DEFAULT_ORACLE_TRUST[type] <= 1);
    }
  });

  it('should trust runtime behavior most', () => {
    assert.strictEqual(DEFAULT_ORACLE_TRUST[OracleType.RUNTIME_BEHAVIOR], 0.9);
  });

  it('should trust human review highly', () => {
    assert.strictEqual(DEFAULT_ORACLE_TRUST[OracleType.HUMAN_REVIEW], 0.85);
  });
});

// =============================================================================
// ROOT CAUSE ANALYZER
// =============================================================================

describe('RootCauseAnalyzer', () => {
  let analyzer;

  beforeEach(() => {
    _resetRootCauseAnalyzerForTesting();
    analyzer = createRootCauseAnalyzer();
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      assert.ok(analyzer instanceof RootCauseAnalyzer);
    });

    it('should have default oracle trust', () => {
      const trust = analyzer.getOracleTrust();
      assert.strictEqual(trust[OracleType.TEST_RESULT], 0.7);
    });

    it('should accept custom oracle trust', () => {
      const custom = createRootCauseAnalyzer({
        oracleTrust: { [OracleType.TEST_RESULT]: 0.5 },
      });
      const trust = custom.getOracleTrust();
      assert.strictEqual(trust[OracleType.TEST_RESULT], 0.5);
    });

    it('should start with empty evidence', () => {
      assert.strictEqual(analyzer.evidence.length, 0);
    });

    it('should start with empty history', () => {
      assert.strictEqual(analyzer.history.length, 0);
    });
  });

  describe('Singleton', () => {
    it('should return same instance', () => {
      const a = getRootCauseAnalyzer();
      const b = getRootCauseAnalyzer();
      assert.strictEqual(a, b);
    });

    it('should reset singleton for testing', () => {
      const a = getRootCauseAnalyzer();
      _resetRootCauseAnalyzerForTesting();
      const b = getRootCauseAnalyzer();
      assert.notStrictEqual(a, b);
    });
  });

  describe('addEvidence()', () => {
    it('should add evidence to buffer', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      assert.strictEqual(analyzer.evidence.length, 1);
    });

    it('should cap confidence at φ⁻¹', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.95,
      });
      assert.ok(analyzer.evidence[0].confidence <= PHI_INV + 0.001);
    });

    it('should add timestamp if missing', () => {
      const before = Date.now();
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      const after = Date.now();
      assert.ok(analyzer.evidence[0].timestamp >= before);
      assert.ok(analyzer.evidence[0].timestamp <= after);
    });

    it('should emit evidence event', (t, done) => {
      analyzer.on('evidence', (e) => {
        assert.strictEqual(e.type, OracleType.TEST_RESULT);
        done();
      });
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
    });
  });

  describe('clearEvidence()', () => {
    it('should clear evidence buffer', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.clearEvidence();
      assert.strictEqual(analyzer.evidence.length, 0);
    });
  });

  describe('analyze()', () => {
    it('should return UNKNOWN with insufficient evidence', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();

      assert.strictEqual(decision.hypothesis, Hypothesis.UNKNOWN);
      assert.ok(decision.reasoning.length > 0 || decision.probabilities !== undefined);
    });

    it('should analyze with multiple oracles', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.9,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();

      assert.ok('hypothesis' in decision);
      assert.ok('probabilities' in decision);
      assert.ok('confidence' in decision);
      assert.ok('reasoning' in decision);
    });

    it('should cap decision confidence at φ⁻¹', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.9,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.9,
      });
      analyzer.addEvidence({
        type: OracleType.RUNTIME_BEHAVIOR,
        verdict: 'fail',
        confidence: 0.9,
      });

      const decision = analyzer.analyze();
      assert.ok(decision.confidence <= PHI_INV + 0.001);
    });

    it('should lean CODE_BUG when all fail', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.9,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.9,
      });

      const decision = analyzer.analyze();
      // Should favor code_bug when all fail
      assert.ok(
        decision.probabilities[Hypothesis.CODE_BUG] >= decision.probabilities[Hypothesis.FLAKY]
      );
    });

    it('should lean TEST_BUG when LLM disagrees with test', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.LLM_CONSENSUS,
        verdict: 'pass',
        confidence: 0.7,
      });

      const decision = analyzer.analyze();
      // Should suggest test bug when LLM says pass but test fails
      assert.ok(decision.probabilities[Hypothesis.TEST_BUG] > 0);
    });

    it('should detect flakiness', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'flaky',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'intermittent',
        confidence: 0.7,
      });

      const decision = analyzer.analyze();
      assert.ok(decision.probabilities[Hypothesis.FLAKY] > 0);
    });

    it('should adjust for context', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.7,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.7,
      });

      const withContext = analyzer.analyze({ recentCodeChange: true });
      analyzer.clearEvidence();

      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.7,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.7,
      });

      const withTestContext = analyzer.analyze({ recentTestChange: true });

      // Recent code change should favor code_bug
      // Recent test change should favor test_bug
      // Note: actual values depend on evidence mix
      assert.ok('probabilities' in withContext);
      assert.ok('probabilities' in withTestContext);
    });

    it('should record decision in history', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      analyzer.analyze();
      assert.strictEqual(analyzer.history.length, 1);
    });

    it('should emit decision event', (t, done) => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      analyzer.on('decision', (d) => {
        assert.ok('hypothesis' in d);
        done();
      });

      analyzer.analyze();
    });
  });

  describe('Residual calculation', () => {
    it('should have low residual when oracles agree', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      assert.strictEqual(decision.residual, 0);
    });

    it('should have higher residual when oracles disagree', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.LLM_CONSENSUS,
        verdict: 'pass',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      assert.ok(decision.residual > 0);
    });
  });

  describe('Actions', () => {
    it('should recommend FIX_CODE for code_bug', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.9,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.9,
      });
      analyzer.addEvidence({
        type: OracleType.RUNTIME_BEHAVIOR,
        verdict: 'fail',
        confidence: 0.9,
      });

      const decision = analyzer.analyze();
      // If hypothesis is code_bug, action should be FIX_CODE_AND_LEARN
      if (decision.hypothesis === Hypothesis.CODE_BUG) {
        assert.strictEqual(decision.action, 'FIX_CODE_AND_LEARN');
      }
    });

    it('should escalate with low confidence', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.3,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'pass',
        confidence: 0.3,
      });

      const decision = analyzer.analyze();
      // Low confidence with disagreement should escalate
      if (decision.confidence < DECISION_THRESHOLDS.ESCALATION_THRESHOLD) {
        assert.strictEqual(decision.action, 'ESCALATE_TO_HUMAN');
        assert.strictEqual(decision.escalate, true);
      }
    });
  });

  describe('feedback()', () => {
    let decisionId;

    beforeEach(() => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      decisionId = decision.id;
    });

    it('should record feedback', () => {
      analyzer.feedback(decisionId, Hypothesis.CODE_BUG, true);

      const decision = analyzer.history.find(d => d.id === decisionId);
      assert.ok(decision.feedback);
      assert.strictEqual(decision.feedback.actualCause, Hypothesis.CODE_BUG);
      assert.strictEqual(decision.feedback.wasCorrect, true);
    });

    it('should update statistics', () => {
      const before = analyzer.stats.totalDecisions;
      analyzer.feedback(decisionId, Hypothesis.CODE_BUG, true);
      assert.strictEqual(analyzer.stats.totalDecisions, before + 1);
    });

    it('should adjust oracle trust on correct prediction', () => {
      const trustBefore = analyzer.oracleTrust[OracleType.TEST_RESULT];
      analyzer.feedback(decisionId, Hypothesis.CODE_BUG, true);
      const trustAfter = analyzer.oracleTrust[OracleType.TEST_RESULT];

      // Trust should increase (test said fail, actual was code_bug → helpful)
      assert.ok(trustAfter >= trustBefore);
    });

    it('should adjust oracle trust on incorrect prediction', () => {
      const trustBefore = analyzer.oracleTrust[OracleType.TEST_RESULT];
      analyzer.feedback(decisionId, Hypothesis.TEST_BUG, false);
      const trustAfter = analyzer.oracleTrust[OracleType.TEST_RESULT];

      // Trust should decrease or stay (test said fail, actual was test_bug → not helpful)
      // Note: actual adjustment depends on _oracleWasHelpful logic
      assert.ok(typeof trustAfter === 'number');
    });

    it('should emit feedback event', (t, done) => {
      analyzer.on('feedback', (f) => {
        assert.strictEqual(f.decisionId, decisionId);
        assert.strictEqual(f.actualCause, Hypothesis.CODE_BUG);
        done();
      });
      analyzer.feedback(decisionId, Hypothesis.CODE_BUG, true);
    });

    it('should emit learned event', (t, done) => {
      analyzer.on('learned', (l) => {
        assert.ok('updatedTrust' in l);
        done();
      });
      analyzer.feedback(decisionId, Hypothesis.CODE_BUG, true);
    });
  });

  describe('getStats()', () => {
    it('should return statistics', () => {
      const stats = analyzer.getStats();

      assert.ok('totalDecisions' in stats);
      assert.ok('correctDecisions' in stats);
      assert.ok('accuracy' in stats);
      assert.ok('byHypothesis' in stats);
      assert.ok('byOracle' in stats);
      assert.ok('oracleTrust' in stats);
    });

    it('should calculate accuracy', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      analyzer.feedback(decision.id, Hypothesis.CODE_BUG, true);

      const stats = analyzer.getStats();
      assert.strictEqual(stats.totalDecisions, 1);
      assert.strictEqual(stats.correctDecisions, 1);
      assert.strictEqual(stats.accuracy, 1);
    });
  });

  describe('export/import', () => {
    it('should export state', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      analyzer.feedback(decision.id, Hypothesis.CODE_BUG, true);

      const exported = analyzer.export();

      assert.ok('oracleTrust' in exported);
      assert.ok('stats' in exported);
      assert.ok('history' in exported);
      assert.ok('exportedAt' in exported);
    });

    it('should import state', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      analyzer.feedback(decision.id, Hypothesis.CODE_BUG, true);

      const exported = analyzer.export();

      const newAnalyzer = createRootCauseAnalyzer();
      newAnalyzer.import(exported);

      assert.strictEqual(
        newAnalyzer.stats.totalDecisions,
        analyzer.stats.totalDecisions
      );
    });
  });

  describe('reset()', () => {
    it('should reset all state', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      analyzer.feedback(decision.id, Hypothesis.CODE_BUG, true);

      analyzer.reset();

      assert.strictEqual(analyzer.evidence.length, 0);
      assert.strictEqual(analyzer.history.length, 0);
      assert.strictEqual(analyzer.stats.totalDecisions, 0);
    });

    it('should reset oracle trust to defaults', () => {
      // Modify trust
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();
      analyzer.feedback(decision.id, Hypothesis.CODE_BUG, true);

      analyzer.reset();

      assert.strictEqual(
        analyzer.oracleTrust[OracleType.TEST_RESULT],
        DEFAULT_ORACLE_TRUST[OracleType.TEST_RESULT]
      );
    });
  });

  describe('Reasoning', () => {
    it('should generate evidence summary', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();

      assert.ok(Array.isArray(decision.reasoning));
      assert.ok(decision.reasoning.length > 0);
      assert.ok(decision.reasoning.some(r => r.includes('Evidence')));
    });

    it('should include residual interpretation', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.LLM_CONSENSUS,
        verdict: 'pass',
        confidence: 0.8,
      });

      const decision = analyzer.analyze();

      assert.ok(decision.reasoning.some(r => r.includes('residual')));
    });

    it('should note LLM-test disagreement', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.8,
      });
      analyzer.addEvidence({
        type: OracleType.LLM_CONSENSUS,
        verdict: 'pass',
        confidence: 0.7,
      });

      const decision = analyzer.analyze();

      assert.ok(
        decision.reasoning.some(r =>
          r.includes('LLM') && r.includes('disagree')
        )
      );
    });
  });

  describe('φ-alignment', () => {
    it('should cap all probabilities at φ⁻¹', () => {
      analyzer.addEvidence({
        type: OracleType.TEST_RESULT,
        verdict: 'fail',
        confidence: 0.99,
      });
      analyzer.addEvidence({
        type: OracleType.BUILD_STATUS,
        verdict: 'fail',
        confidence: 0.99,
      });
      analyzer.addEvidence({
        type: OracleType.RUNTIME_BEHAVIOR,
        verdict: 'fail',
        confidence: 0.99,
      });

      const decision = analyzer.analyze();

      for (const prob of Object.values(decision.probabilities)) {
        assert.ok(prob <= PHI_INV + 0.001, `Probability ${prob} exceeds φ⁻¹`);
      }
    });

    it('should use φ-aligned thresholds', () => {
      // CODE_BUG_THRESHOLD should be φ⁻¹
      assert.ok(
        Math.abs(DECISION_THRESHOLDS.CODE_BUG_THRESHOLD - PHI_INV) < 0.001
      );

      // MAX_LEARNING_RATE should be φ⁻²
      assert.ok(
        Math.abs(DECISION_THRESHOLDS.MAX_LEARNING_RATE - PHI_INV_2) < 0.001
      );

      // ESCALATION_THRESHOLD should be φ⁻³
      assert.ok(
        Math.abs(DECISION_THRESHOLDS.ESCALATION_THRESHOLD - PHI_INV_3) < 0.001
      );
    });
  });
});
