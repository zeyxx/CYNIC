/**
 * φ-Governor Tests
 *
 * Tests the homeostatic influence controller that maintains
 * CYNIC's influence at φ⁻¹ (61.8%).
 *
 * @module @cynic/core/test/phi-governor
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  createPhiGovernor,
  SETPOINT,
  LOWER_BOUND,
  classifyZone,
} from '../src/intelligence/phi-governor.js';

import { PHI_INV, PHI_INV_2 } from '../src/axioms/constants.js';

// =============================================================================
// Zone classification
// =============================================================================

describe('classifyZone', () => {
  it('over when above φ⁻¹', () => {
    assert.equal(classifyZone(0.7), 'over');
    assert.equal(classifyZone(0.9), 'over');
    assert.equal(classifyZone(1.0), 'over');
  });

  it('under when below φ⁻²', () => {
    assert.equal(classifyZone(0.1), 'under');
    assert.equal(classifyZone(0.3), 'under');
  });

  it('balanced in [φ⁻², φ⁻¹]', () => {
    assert.equal(classifyZone(0.5), 'balanced');
    assert.equal(classifyZone(PHI_INV_2), 'balanced');
    assert.equal(classifyZone(PHI_INV), 'balanced');
  });

  it('SETPOINT matches φ⁻¹', () => {
    assert.equal(SETPOINT, PHI_INV);
  });

  it('LOWER_BOUND matches φ⁻²', () => {
    assert.equal(LOWER_BOUND, PHI_INV_2);
  });
});

// =============================================================================
// Governor creation
// =============================================================================

describe('createPhiGovernor', () => {
  let gov;

  beforeEach(() => {
    gov = createPhiGovernor();
  });

  it('creates a governor with all methods', () => {
    assert.equal(typeof gov.measure, 'function');
    assert.equal(typeof gov.getAdjustment, 'function');
    assert.equal(typeof gov.applyToBudget, 'function');
    assert.equal(typeof gov.getState, 'function');
    assert.equal(typeof gov.getConvergence, 'function');
    assert.equal(typeof gov.getHistory, 'function');
    assert.equal(typeof gov.reset, 'function');
  });

  it('initial adjustment is 1.0 (no change)', () => {
    assert.equal(gov.getAdjustment(), 1.0);
  });

  it('initial state starts at setpoint', () => {
    const state = gov.getState();
    assert.equal(state.adjustmentFactor, 1.0);
    assert.equal(state.currentZone, 'balanced');
    assert.equal(state.historySize, 0);
  });
});

// =============================================================================
// Measurement
// =============================================================================

describe('governor measurement', () => {
  let gov;

  beforeEach(() => {
    gov = createPhiGovernor();
  });

  it('measures influence ratio correctly', () => {
    const m = gov.measure(100, 200);
    assert.equal(m.ratio, 0.5);
    assert.equal(m.zone, 'balanced');
    assert.ok(m.timestamp > 0);
  });

  it('handles zero total tokens', () => {
    const m = gov.measure(100, 0);
    assert.equal(m.ratio, 0);
    assert.equal(m.zone, 'empty');
  });

  it('caps ratio at 1', () => {
    const m = gov.measure(300, 200);
    assert.equal(m.ratio, 1);
  });

  it('detects over-influence', () => {
    const m = gov.measure(800, 1000); // 80%
    assert.equal(m.zone, 'over');
  });

  it('detects under-influence', () => {
    const m = gov.measure(100, 1000); // 10%
    assert.equal(m.zone, 'under');
  });

  it('detects balanced influence', () => {
    const m = gov.measure(500, 1000); // 50%
    assert.equal(m.zone, 'balanced');
  });
});

// =============================================================================
// Adjustment behavior
// =============================================================================

describe('governor adjustment', () => {
  let gov;

  beforeEach(() => {
    gov = createPhiGovernor();
  });

  it('no adjustment in balanced zone', () => {
    gov.measure(500, 1000); // 50% — balanced
    assert.equal(gov.getAdjustment(), 1.0);
  });

  it('reduces when over-influencing', () => {
    // Pump EMA above setpoint with repeated high readings
    for (let i = 0; i < 5; i++) {
      gov.measure(800, 1000); // 80%
    }
    assert.ok(gov.getAdjustment() < 1.0, 'should reduce injection');
  });

  it('increases when under-influencing', () => {
    // Push EMA below lower bound
    for (let i = 0; i < 5; i++) {
      gov.measure(50, 1000); // 5%
    }
    assert.ok(gov.getAdjustment() > 1.0, 'should increase injection');
  });

  it('adjustment bounded between 0.5 and 1.5', () => {
    // Extreme over-influence
    for (let i = 0; i < 20; i++) {
      gov.measure(950, 1000);
    }
    assert.ok(gov.getAdjustment() >= 0.5);

    gov.reset();

    // Extreme under-influence
    for (let i = 0; i < 20; i++) {
      gov.measure(10, 1000);
    }
    assert.ok(gov.getAdjustment() <= 1.5);
  });

  it('consecutive deviations strengthen correction', () => {
    // 3 readings of over-influence
    gov.measure(800, 1000);
    gov.measure(800, 1000);
    const adj2 = gov.getAdjustment();

    gov.measure(800, 1000);
    gov.measure(800, 1000);
    const adj4 = gov.getAdjustment();

    // After more consecutive readings, correction should be stronger (or at least equal)
    assert.ok(adj4 <= adj2, 'more consecutive over-influence should not reduce correction strength');
  });
});

// =============================================================================
// Budget application
// =============================================================================

describe('governor applyToBudget', () => {
  let gov;

  beforeEach(() => {
    gov = createPhiGovernor();
  });

  it('no change when balanced', () => {
    gov.measure(500, 1000);
    assert.equal(gov.applyToBudget(1000), 1000);
  });

  it('reduces budget when over-influencing', () => {
    for (let i = 0; i < 5; i++) {
      gov.measure(800, 1000);
    }
    assert.ok(gov.applyToBudget(1000) < 1000);
  });

  it('increases budget when under-influencing', () => {
    for (let i = 0; i < 5; i++) {
      gov.measure(50, 1000);
    }
    assert.ok(gov.applyToBudget(1000) > 1000);
  });
});

// =============================================================================
// History and convergence
// =============================================================================

describe('governor history and convergence', () => {
  let gov;

  beforeEach(() => {
    gov = createPhiGovernor();
  });

  it('records history', () => {
    gov.measure(500, 1000);
    gov.measure(600, 1000);
    assert.equal(gov.getHistory().length, 2);
  });

  it('history respects limit', () => {
    for (let i = 0; i < 10; i++) {
      gov.measure(500, 1000);
    }
    assert.equal(gov.getHistory(3).length, 3);
  });

  it('convergence reports insufficient data for < 3 readings', () => {
    gov.measure(500, 1000);
    const conv = gov.getConvergence();
    assert.equal(conv.converged, false);
    assert.equal(conv.reason, 'insufficient_data');
  });

  it('convergence reports after enough readings', () => {
    for (let i = 0; i < 5; i++) {
      gov.measure(500, 1000); // 50% — balanced zone
    }
    const conv = gov.getConvergence();
    assert.ok('avgRatio' in conv);
    assert.ok('deviation' in conv);
    assert.ok('variance' in conv);
  });

  it('converges when consistently near setpoint', () => {
    // Feed ratio near setpoint: 60%, 62%, 61%, 59%, 60%
    gov.measure(600, 1000);
    gov.measure(620, 1000);
    gov.measure(610, 1000);
    gov.measure(590, 1000);
    gov.measure(600, 1000);
    const conv = gov.getConvergence();
    assert.ok(conv.converged, 'should be converged near setpoint');
  });

  it('does not converge when oscillating', () => {
    // Wild oscillation: 10%, 90%, 10%, 90%, 10%
    gov.measure(100, 1000);
    gov.measure(900, 1000);
    gov.measure(100, 1000);
    gov.measure(900, 1000);
    gov.measure(100, 1000);
    const conv = gov.getConvergence();
    assert.equal(conv.converged, false);
  });
});

// =============================================================================
// Reset
// =============================================================================

describe('governor reset', () => {
  it('clears all state', () => {
    const gov = createPhiGovernor();
    gov.measure(800, 1000);
    gov.measure(800, 1000);
    gov.reset();

    assert.equal(gov.getAdjustment(), 1.0);
    assert.equal(gov.getHistory().length, 0);
    const state = gov.getState();
    assert.equal(state.consecutiveHigh, 0);
    assert.equal(state.consecutiveLow, 0);
  });
});

// =============================================================================
// Re-export
// =============================================================================

describe('phi-governor re-export', () => {
  it('accessible from @cynic/core', async () => {
    const core = await import('../src/index.js');
    assert.equal(typeof core.createPhiGovernor, 'function');
    assert.equal(typeof core.classifyZone, 'function');
    assert.equal(core.SETPOINT, PHI_INV);
  });
});
