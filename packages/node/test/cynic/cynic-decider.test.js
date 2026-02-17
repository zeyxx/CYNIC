/**
 * Test suite for CynicDecider (C6.3 - CYNIC × DECIDE)
 *
 * Tests φ-bounded self-governance decisions:
 * - Budget governance (shift to Ollama at φ⁻¹)
 * - Memory governance (compress/GC at thresholds)
 * - Context governance (semantic compression)
 * - Learning governance (prioritize SONA)
 * - Event governance (investigate wiring)
 * - Pattern-based decisions (from emergence detector)
 *
 * @module @cynic/node/test/cynic/cynic-decider.test
 */

'use strict';

import assert from 'assert';
import { describe, it, beforeEach, afterEach } from 'node:test';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { getCynicDecider, resetCynicDecider, CynicDecisionType } from '../../src/cynic/cynic-decider.js';

describe('CynicDecider (C6.3 - Factory Pattern)', () => {
  let decider;

  beforeEach(() => {
    resetCynicDecider();
    decider = getCynicDecider();
  });

  afterEach(() => {
    resetCynicDecider();
  });

  // =========================================================================
  // Basic Factory Pattern
  // =========================================================================

  it('should create singleton via factory', () => {
    const d1 = getCynicDecider();
    const d2 = getCynicDecider();
    assert.strictEqual(d1, d2, 'Should return same singleton instance');
  });

  it('should reset singleton', () => {
    const d1 = getCynicDecider();
    resetCynicDecider();
    const d2 = getCynicDecider();
    assert.notStrictEqual(d1, d2, 'Should create new instance after reset');
  });

  it('should initialize with correct cell/dimension', () => {
    const judgment = { score: 50, verdict: 'WAG' };
    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.cell, 'C6.3');
    assert.strictEqual(result.dimension, 'CYNIC');
    assert.strictEqual(result.analysis, 'DECIDE');
  });

  // =========================================================================
  // Budget Governance (φ-bounded)
  // =========================================================================

  it('should SHIFT_TO_OLLAMA when budget exceeds φ⁻¹', () => {
    const judgment = { score: 50, verdict: 'WAG' };

    // Trigger enough judgments to pass MIN_JUDGMENTS threshold
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    // Now trigger budget violation
    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 6.5, limit: 10 }, // 65% > 61.8%
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.SHIFT_TO_OLLAMA);
    assert.strictEqual(result.severity, 'high');
    assert.ok(result.reason.includes('65.0%') || result.reason.includes('Budget'));
    assert.ok(result.confidence <= PHI_INV);
  });

  it('should ACKNOWLEDGE when budget within φ⁻¹', () => {
    const judgment = { score: 70, verdict: 'WAG' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 3, limit: 10 }, // 30% < 61.8%
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.ACKNOWLEDGE);
    assert.strictEqual(result.severity, 'low');
  });

  // =========================================================================
  // Memory Governance
  // =========================================================================

  it('should COMPRESS_CONTEXT when heap critical (>80%)', () => {
    const judgment = { score: 50, verdict: 'GROWL' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 850, heapTotal: 1000 }, // 85% > 80%
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.COMPRESS_CONTEXT);
    assert.strictEqual(result.severity, 'critical');
    assert.ok(result.actions.includes(CynicDecisionType.TRIGGER_GC));
  });

  it('should TRIGGER_GC when heap warning (>φ⁻¹)', () => {
    const judgment = { score: 60, verdict: 'WAG' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 650, heapTotal: 1000 }, // 65% > 61.8%
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.TRIGGER_GC);
    assert.strictEqual(result.severity, 'medium');
  });

  // =========================================================================
  // Context Governance
  // =========================================================================

  it('should SEMANTIC_COMPRESS when context >10MB', () => {
    const judgment = { score: 60, verdict: 'WAG' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 12 }, // >10MB
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.SEMANTIC_COMPRESS);
    assert.strictEqual(result.severity, 'medium');
    assert.ok(result.reason.includes('12.0MB'));
  });

  // =========================================================================
  // Learning Governance
  // =========================================================================

  it('should PRIORITIZE_SONA when learning maturity <φ⁻²', () => {
    const judgment = { score: 50, verdict: 'WAG' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 30 }, // 30% < 38.2%
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.PRIORITIZE_SONA);
    assert.strictEqual(result.severity, 'medium');
    assert.ok(result.reason.includes('30.0%'));
  });

  // =========================================================================
  // Event Governance
  // =========================================================================

  it('should INVESTIGATE_WIRING when orphan events >3', () => {
    const judgment = { score: 40, verdict: 'GROWL' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 5 }, // >3 orphans
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.INVESTIGATE_WIRING);
    assert.strictEqual(result.severity, 'high');
    assert.ok(result.reason.includes('5 orphan'));
  });

  // =========================================================================
  // Pattern-based Decisions (C6.7 → C6.3)
  // =========================================================================

  it('should handle dog_dominance_shift pattern', () => {
    const judgment = { score: 60, verdict: 'WAG', patternType: 'dog_dominance_shift' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide({ score: 60, verdict: 'WAG' }, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      patternType: 'dog_dominance_shift',
      pattern: {
        significance: 'high',
        data: { ratio: 0.7, dominantDog: 'HOWL' }, // 70% > 61.8%
      },
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.ADJUST_THRESHOLDS);
    assert.ok(result.reason.includes('dominance'));
    assert.strictEqual(result.dominantDog, 'HOWL');
  });

  it('should handle memory_pressure pattern', () => {
    const judgment = { score: 50, verdict: 'GROWL', patternType: 'memory_pressure' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide({ score: 60, verdict: 'WAG' }, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      patternType: 'memory_pressure',
      pattern: {
        significance: 'critical',
        data: { load: 0.7 }, // 70% > φ⁻¹
      },
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.strictEqual(result.decision, CynicDecisionType.COMPRESS_CONTEXT);
    assert.strictEqual(result.severity, 'high');
  });

  // =========================================================================
  // φ-Bounds (Confidence never exceeds φ⁻¹)
  // =========================================================================

  it('should never exceed φ⁻¹ confidence', () => {
    const judgment = { score: 95, verdict: 'HOWL' };

    // Pass MIN_JUDGMENTS
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 8, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const result = decider.decide(judgment, {
      state: {
        budget: { spent: 8, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    assert.ok(result.confidence <= PHI_INV, `Confidence ${result.confidence} exceeds φ⁻¹ (${PHI_INV})`);
  });

  // =========================================================================
  // Stats & Health
  // =========================================================================

  it('should track decision stats', () => {
    const judgment = { score: 60, verdict: 'WAG' };

    // Make multiple decisions
    for (let i = 0; i < 5; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    const stats = decider.getStats();
    assert.strictEqual(stats.decisionsTotal, 5);
    assert.ok(stats.byType[CynicDecisionType.ACKNOWLEDGE] >= 1);
  });

  it('should report health status', () => {
    const judgment = { score: 60, verdict: 'WAG' };

    decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });

    const health = decider.getHealth();
    assert.ok(['healthy', 'stressed', 'high_investigation'].includes(health.status));
    assert.ok(health.score <= PHI_INV);
    assert.strictEqual(typeof health.totalDecisions, 'number');
  });

  // =========================================================================
  // Event Emission
  // =========================================================================

  it('should emit cynic:decision event', (t, done) => {
    const judgment = { score: 60, verdict: 'WAG' };

    // Pass MIN_JUDGMENTS first
    for (let i = 0; i < 3; i++) {
      decider.decide(judgment, {
        state: {
          budget: { spent: 1, limit: 10 },
          memory: { heapUsed: 100, heapTotal: 1000 },
          context: { sizeMB: 1 },
          learning: { maturityPercent: 50 },
          events: { orphanCount: 0 },
        },
      });
    }

    decider.once('decision', (result) => {
      assert.strictEqual(result.cell, 'C6.3');
      assert.strictEqual(result.dimension, 'CYNIC');
      done();
    });

    decider.decide(judgment, {
      state: {
        budget: { spent: 1, limit: 10 },
        memory: { heapUsed: 100, heapTotal: 1000 },
        context: { sizeMB: 1 },
        learning: { maturityPercent: 50 },
        events: { orphanCount: 0 },
      },
    });
  });
});
