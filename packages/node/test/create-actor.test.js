/**
 * Actor Factory Tests
 *
 * Tests create-actor.js with both CodeActor and CosmosActor configs.
 * Verifies behavioral equivalence with the original hand-written classes.
 *
 * @module @cynic/node/test/create-actor
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { createActor } from '../src/cycle/create-actor.js';
import { ActionStatus } from '../src/cycle/shared-enums.js';
import { codeActorConfig, CodeActionType } from '../src/cycle/configs/code-actor.config.js';
import { cosmosActorConfig, CosmosActionType } from '../src/cycle/configs/cosmos-actor.config.js';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// Factory basics
// =============================================================================

describe('createActor factory', () => {
  it('returns Class, ActionStatus, getInstance, resetInstance', () => {
    const result = createActor(codeActorConfig);
    assert.equal(typeof result.Class, 'function');
    assert.equal(typeof result.getInstance, 'function');
    assert.equal(typeof result.resetInstance, 'function');
    assert.deepEqual(result.ActionStatus, ActionStatus);
    result.resetInstance();
  });

  it('getInstance returns singleton', () => {
    const { getInstance, resetInstance } = createActor(codeActorConfig);
    const a = getInstance();
    const b = getInstance();
    assert.equal(a, b);
    resetInstance();
  });

  it('resetInstance clears singleton', () => {
    const { getInstance, resetInstance } = createActor(codeActorConfig);
    const a = getInstance();
    resetInstance();
    const b = getInstance();
    assert.notEqual(a, b);
    resetInstance();
  });
});

// =============================================================================
// CodeActor via factory
// =============================================================================

describe('CodeActor (factory)', () => {
  let actor;
  let factory;

  beforeEach(() => {
    factory = createActor(codeActorConfig);
    actor = new factory.Class();
  });

  it('has correct stats initialization', () => {
    const stats = actor.getStats();
    assert.equal(stats.actionsTotal, 0);
    assert.equal(stats.delivered, 0);
    assert.equal(stats.blocksRaised, 0);
    assert.equal(stats.reviewsFlagged, 0);
    assert.equal(stats.debtsLogged, 0);
    for (const type of Object.values(CodeActionType)) {
      assert.equal(stats.byType[type], 0);
    }
  });

  it('act() returns result with correct shape', () => {
    const result = actor.act({ type: 'approve' }, { qScore: 85 });
    assert.ok(result);
    assert.equal(result.type, 'approve_commit');
    assert.equal(result.status, ActionStatus.DELIVERED);
    assert.equal(result.cell, 'C1.4');
    assert.equal(result.dimension, 'CODE');
    assert.equal(result.analysis, 'ACT');
    assert.equal(result.urgency, 'low');
    assert.ok(result.message.includes('*tail wag*'));
    assert.ok(result.message.includes('Q:85'));
    assert.ok(result.timestamp);
  });

  it('act() respects cooldown', () => {
    // FLAG_REVIEW has 5 min cooldown
    actor.act({ type: 'queue_review' });
    const second = actor.act({ type: 'queue_review' });
    assert.equal(second, null);
  });

  it('act() allows 0-cooldown actions repeatedly', () => {
    actor.act({ type: 'approve' });
    const second = actor.act({ type: 'approve' });
    assert.ok(second);
  });

  it('block decision maps to BLOCK_ALERT with critical urgency', () => {
    const result = actor.act({ type: 'block', reasoning: 'dangerous' });
    assert.equal(result.type, 'block_alert');
    assert.equal(result.urgency, 'critical');
    assert.ok(result.message.includes('*GROWL*'));
  });

  it('updates stats correctly', () => {
    actor.act({ type: 'approve' });
    actor.act({ type: 'block' });
    actor.act({ type: 'queue_review' });
    const stats = actor.getStats();
    assert.equal(stats.actionsTotal, 3);
    assert.equal(stats.delivered, 3);
    assert.equal(stats.byType.approve_commit, 1);
    assert.equal(stats.byType.block_alert, 1);
    assert.equal(stats.blocksRaised, 1);
    assert.equal(stats.reviewsFlagged, 1);
  });

  it('getHistory returns actions', () => {
    actor.act({ type: 'approve' });
    actor.act({ type: 'block' });
    const history = actor.getHistory();
    assert.equal(history.length, 2);
    assert.equal(history[0].type, 'approve_commit');
    assert.equal(history[1].type, 'block_alert');
  });

  it('getHistory respects limit', () => {
    for (let i = 0; i < 10; i++) {
      actor.act({ type: 'approve' });
    }
    assert.equal(actor.getHistory(3).length, 3);
  });

  it('history bounded by maxHistory', () => {
    // maxHistory = 233 (Fib 13)
    for (let i = 0; i < 250; i++) {
      actor.act({ type: 'approve' });
    }
    assert.equal(actor.getHistory(999).length, 233);
  });

  it('getHealth returns status', () => {
    const health = actor.getHealth();
    assert.equal(health.status, 'healthy');
    assert.ok(health.score <= PHI_INV);
  });

  it('getHealth detects unhealthy (many blocks)', () => {
    // Create many blocks vs total actions
    for (let i = 0; i < 10; i++) {
      actor.act({ type: 'block' });
    }
    const health = actor.getHealth();
    // 10 blocks / 10 total = 100% block rate > PHI_INV_2
    assert.equal(health.status, 'high_risk_codebase');
  });

  it('recordResponse updates history status', () => {
    actor.act({ type: 'approve' });
    actor.recordResponse('approve_commit', 'acted');
    const history = actor.getHistory();
    assert.equal(history[0].status, ActionStatus.ACTED_ON);
  });

  it('recordResponse dismisses correctly', () => {
    actor.act({ type: 'approve' });
    actor.recordResponse('approve_commit', 'dismiss');
    const history = actor.getHistory();
    assert.equal(history[0].status, ActionStatus.DISMISSED);
  });

  it('emits action event', () => {
    let emitted = null;
    actor.on('action', (r) => { emitted = r; });
    actor.act({ type: 'approve' });
    assert.ok(emitted);
    assert.equal(emitted.type, 'approve_commit');
  });

  it('debt tracking works via postAct', () => {
    actor.act({ type: 'defer', reasoning: 'later' });
    assert.equal(actor._debtLog.length, 1);
    assert.equal(actor._debtLog[0].type, 'log_debt');
    assert.equal(actor._stats.debtsLogged, 1);
  });
});

// =============================================================================
// CosmosActor via factory
// =============================================================================

describe('CosmosActor (factory)', () => {
  let actor;

  beforeEach(() => {
    const factory = createActor(cosmosActorConfig);
    actor = new factory.Class();
  });

  it('has correct cell and dimension', () => {
    const result = actor.act({ decision: 'intervene', reason: 'crisis' });
    assert.equal(result.cell, 'C7.4');
    assert.equal(result.dimension, 'COSMOS');
  });

  it('maps decisions correctly', () => {
    assert.equal(actor.act({ decision: 'intervene' }).type, 'signal_alert');
    assert.equal(actor.act({ decision: 'accelerate' }).type, 'log_insight');
  });

  it('wait decision maps to LOG_INSIGHT (fallback)', () => {
    // Original code: map['wait'] = null, null ?? LOG_INSIGHT = LOG_INSIGHT
    const result = actor.act({ decision: 'wait' });
    assert.ok(result);
    assert.equal(result.type, CosmosActionType.LOG_INSIGHT);
  });

  it('urgency mapping works', () => {
    assert.equal(actor.act({ decision: 'intervene' }).urgency, 'critical');
    // Reset cooldown for signal_alert
    actor._lastAction.clear();
    assert.equal(actor.act({ decision: 'diversify' }).urgency, 'medium');
  });

  it('composes messages with dog voice', () => {
    const result = actor.act({ decision: 'intervene', reason: 'test' });
    assert.ok(result.message.includes('*GROWL*'));
    assert.ok(result.message.includes('test'));
  });

  it('tracks extra stats', () => {
    actor.act({ decision: 'intervene' });
    const stats = actor.getStats();
    assert.equal(stats.alertsRaised, 1);
    assert.equal(stats.insightsLogged, 0);
  });

  it('getHealth uses alertRate', () => {
    for (let i = 0; i < 5; i++) {
      actor.act({ decision: 'intervene' });
      actor._lastAction.clear(); // bypass cooldown
    }
    const health = actor.getHealth();
    assert.equal(health.status, 'high_alert_ecosystem');
    assert.ok('alertsRaisedRate' in health);
  });

  it('history bounded by maxHistory (144)', () => {
    for (let i = 0; i < 160; i++) {
      actor.act({ decision: 'accelerate' });
      actor._lastAction.clear();
    }
    assert.equal(actor.getHistory(999).length, 144);
  });
});

// =============================================================================
// ActionStatus shared enum
// =============================================================================

describe('ActionStatus shared enum', () => {
  it('has all expected values', () => {
    assert.equal(ActionStatus.QUEUED, 'queued');
    assert.equal(ActionStatus.DELIVERED, 'delivered');
    assert.equal(ActionStatus.ACTED_ON, 'acted_on');
    assert.equal(ActionStatus.DISMISSED, 'dismissed');
    assert.equal(ActionStatus.EXPIRED, 'expired');
  });
});
