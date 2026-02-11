/**
 * Domain Wiring Factory Tests
 *
 * Tests the auto-wiring system for event listener generation.
 *
 * @module @cynic/node/test/create-domain-wiring
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import { createDomainWiring, createWiringManager, FIB_INTERVALS } from '../src/services/create-domain-wiring.js';
import { globalEventBus } from '@cynic/core';

// =============================================================================
// Test helpers
// =============================================================================

function createMockJudge() {
  const calls = [];
  return {
    calls,
    judge(data) {
      calls.push({ method: 'judge', data });
      return { verdict: 'GROWL', score: 42, source: 'test' };
    },
    assessPeriodic() {
      calls.push({ method: 'assessPeriodic' });
      return { verdict: 'WAG', score: 60 };
    },
  };
}

function createMockDecider() {
  const calls = [];
  return {
    calls,
    decide(judgment, context) {
      calls.push({ method: 'decide', judgment, context });
      return { type: 'TEST_ACTION', params: { from: 'decider' }, confidence: 0.5 };
    },
  };
}

function createMockActor() {
  const calls = [];
  return {
    calls,
    execute(type, params) {
      calls.push({ method: 'execute', type, params });
      return Promise.resolve({ success: true });
    },
  };
}

function createMockLearner() {
  const calls = [];
  return {
    calls,
    recordOutcome(outcome) {
      calls.push({ method: 'recordOutcome', outcome });
    },
  };
}

function createMockAccountant() {
  const calls = [];
  return {
    calls,
    recordTransaction(data) {
      calls.push({ method: 'recordTransaction', data });
    },
  };
}

function createMockEmergence() {
  const calls = [];
  return {
    calls,
    recordActivity(data) {
      calls.push({ method: 'recordActivity', data });
    },
    analyze() {
      calls.push({ method: 'analyze' });
      return [{ type: 'test_pattern', significance: 'medium' }];
    },
  };
}

/** Collect events from globalEventBus */
function collectEvents(eventName, timeout = 100) {
  const events = [];
  const unsub = globalEventBus.subscribe(eventName, (event) => {
    events.push(event);
  });
  return { events, cleanup: unsub };
}

// =============================================================================
// Factory creation
// =============================================================================

describe('createDomainWiring', () => {
  it('creates wiring with all methods', () => {
    const wiring = createDomainWiring({ name: 'test', cell: 'C0' });
    assert.equal(typeof wiring.wire, 'function');
    assert.equal(typeof wiring.stop, 'function');
    assert.equal(typeof wiring.getStats, 'function');
    assert.equal(typeof wiring.isWired, 'function');
    assert.equal(typeof wiring.getName, 'function');
  });

  it('starts unwired', () => {
    const wiring = createDomainWiring({ name: 'test', cell: 'C0' });
    assert.equal(wiring.isWired(), false);
  });

  it('getName returns domain name', () => {
    const wiring = createDomainWiring({ name: 'solana', cell: 'C2' });
    assert.equal(wiring.getName(), 'solana');
  });

  it('initial stats are zero', () => {
    const wiring = createDomainWiring({ name: 'test', cell: 'C0' });
    const stats = wiring.getStats();
    assert.equal(stats.judgments, 0);
    assert.equal(stats.decisions, 0);
    assert.equal(stats.actions, 0);
    assert.equal(stats.domain, 'test');
  });
});

// =============================================================================
// Perception → Judgment pipeline
// =============================================================================

describe('domain wiring: perception → judgment', () => {
  let wiring;

  afterEach(() => {
    if (wiring) wiring.stop();
  });

  it('wires perception events to judge', () => {
    const judge = createMockJudge();
    wiring = createDomainWiring({
      name: 'testdomain',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:testdomain:slot' }],
    });

    wiring.wire({ judge });
    assert.equal(wiring.isWired(), true);

    // Emit perception event
    globalEventBus.publish('perception:testdomain:slot', { slot: 42 });

    assert.equal(judge.calls.length, 1);
    assert.equal(judge.calls[0].method, 'judge');
    assert.equal(wiring.getStats().judgments, 1);
    assert.equal(wiring.getStats().perceptionEvents, 1);
  });

  it('supports Fibonacci sampling', () => {
    const judge = createMockJudge();
    wiring = createDomainWiring({
      name: 'sampled',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:sampled:tick', sampling: 3 }],
    });

    wiring.wire({ judge });

    // Emit 5 events — only every 3rd should pass
    for (let i = 0; i < 6; i++) {
      globalEventBus.publish('perception:sampled:tick', { tick: i });
    }

    // Events at index 2 and 5 pass (counter 3 and 6, mod 3 === 0)
    assert.equal(judge.calls.length, 2);
    assert.equal(wiring.getStats().perceptionEvents, 2);
  });

  it('feeds emergence on perception', () => {
    const judge = createMockJudge();
    const emergence = createMockEmergence();
    wiring = createDomainWiring({
      name: 'emtest',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:emtest:data' }],
    });

    wiring.wire({ judge, emergence });
    globalEventBus.publish('perception:emtest:data', { val: 1 });

    assert.equal(emergence.calls.length, 1);
    assert.equal(emergence.calls[0].method, 'recordActivity');
  });

  it('emits domain:judgment event', () => {
    const judge = createMockJudge();
    wiring = createDomainWiring({
      name: 'evtest',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:evtest:ping' }],
    });

    const { events, cleanup } = collectEvents('evtest:judgment');

    wiring.wire({ judge });
    globalEventBus.publish('perception:evtest:ping', { x: 1 });

    cleanup();
    assert.equal(events.length, 1);
  });

  it('skips wiring without judge', () => {
    wiring = createDomainWiring({
      name: 'nojudge',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:nojudge:x' }],
    });

    wiring.wire({}); // No judge
    assert.equal(wiring.isWired(), false);
  });

  it('custom perception handler overrides default', () => {
    const customCalls = [];
    wiring = createDomainWiring({
      name: 'custom',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:custom:data' }],
      onPerception(eventName, data, ctx) {
        customCalls.push({ eventName, data });
        ctx.stats.judgments++;
        ctx.emit({ verdict: 'CUSTOM' }, data, eventName);
      },
    });

    const judge = createMockJudge();
    wiring.wire({ judge });
    globalEventBus.publish('perception:custom:data', { test: true });

    assert.equal(customCalls.length, 1);
    assert.equal(judge.calls.length, 0); // Default judge not called
    assert.equal(wiring.getStats().judgments, 1);
  });
});

// =============================================================================
// Judgment → Decision pipeline
// =============================================================================

describe('domain wiring: judgment → decision', () => {
  let wiring;

  afterEach(() => {
    if (wiring) wiring.stop();
  });

  it('routes judgment to decider on GROWL verdict', () => {
    const decider = createMockDecider();
    wiring = createDomainWiring({
      name: 'jd',
      cell: 'C0',
    });

    wiring.wire({ judge: createMockJudge(), decider });

    globalEventBus.publish('jd:judgment', {
      judgment: { verdict: 'GROWL', score: 30 },
    });

    assert.equal(decider.calls.length, 1);
    assert.equal(wiring.getStats().decisions, 1);
  });

  it('filters out WAG verdict by default', () => {
    const decider = createMockDecider();
    wiring = createDomainWiring({
      name: 'wag',
      cell: 'C0',
    });

    wiring.wire({ judge: createMockJudge(), decider });

    globalEventBus.publish('wag:judgment', {
      judgment: { verdict: 'WAG', score: 80 },
    });

    assert.equal(decider.calls.length, 0);
  });

  it('custom verdict filter allows HOWL', () => {
    const decider = createMockDecider();
    wiring = createDomainWiring({
      name: 'howl',
      cell: 'C0',
      verdictFilter: ['HOWL'],
    });

    wiring.wire({ judge: createMockJudge(), decider });

    globalEventBus.publish('howl:judgment', {
      judgment: { verdict: 'HOWL', score: 90 },
    });

    assert.equal(decider.calls.length, 1);
  });

  it('empty verdictFilter passes all', () => {
    const decider = createMockDecider();
    wiring = createDomainWiring({
      name: 'all',
      cell: 'C0',
      verdictFilter: [],
    });

    wiring.wire({ judge: createMockJudge(), decider });

    globalEventBus.publish('all:judgment', {
      judgment: { verdict: 'WAG', score: 80 },
    });

    assert.equal(decider.calls.length, 1);
  });
});

// =============================================================================
// Decision → Action + Learning pipeline
// =============================================================================

describe('domain wiring: decision → action + learning', () => {
  let wiring;

  afterEach(() => {
    if (wiring) wiring.stop();
  });

  it('executes actor on decision', () => {
    const actor = createMockActor();
    wiring = createDomainWiring({ name: 'act', cell: 'C0' });

    wiring.wire({ judge: createMockJudge(), actor });

    globalEventBus.publish('act:decision', {
      decision: { type: 'DEPLOY', params: { target: 'devnet' } },
    });

    assert.equal(actor.calls.length, 1);
    assert.equal(actor.calls[0].type, 'DEPLOY');
    assert.equal(wiring.getStats().actions, 1);
  });

  it('records learning on decision', () => {
    const learner = createMockLearner();
    wiring = createDomainWiring({ name: 'lrn', cell: 'C0' });

    wiring.wire({ judge: createMockJudge(), learner });

    globalEventBus.publish('lrn:decision', {
      decision: { type: 'SCALE' },
    });

    assert.equal(learner.calls.length, 1);
    assert.equal(learner.calls[0].outcome.type, 'SCALE');
    assert.equal(wiring.getStats().learnings, 1);
  });

  it('respects actor safety gate env', () => {
    const actor = createMockActor();
    const originalEnv = process.env.TEST_ACTOR_LIVE;

    // Set env to false — actor should NOT execute
    process.env.TEST_ACTOR_LIVE = 'false';

    wiring = createDomainWiring({
      name: 'safe',
      cell: 'C0',
      actorSafetyEnv: 'TEST_ACTOR_LIVE',
    });

    wiring.wire({ judge: createMockJudge(), actor });

    globalEventBus.publish('safe:decision', {
      decision: { type: 'DANGEROUS' },
    });

    assert.equal(actor.calls.length, 0); // Not executed
    assert.equal(wiring.getStats().actions, 1); // Action still recorded

    // Restore
    if (originalEnv !== undefined) {
      process.env.TEST_ACTOR_LIVE = originalEnv;
    } else {
      delete process.env.TEST_ACTOR_LIVE;
    }
  });
});

// =============================================================================
// Action → Accounting pipeline
// =============================================================================

describe('domain wiring: action → accounting', () => {
  let wiring;

  afterEach(() => {
    if (wiring) wiring.stop();
  });

  it('records accounting on action', () => {
    const accountant = createMockAccountant();
    wiring = createDomainWiring({ name: 'acc', cell: 'C0' });

    wiring.wire({ judge: createMockJudge(), accountant });

    globalEventBus.publish('acc:action', {
      source: 'AccActor', decision: { type: 'TRANSFER', executed: true },
    });

    assert.equal(accountant.calls.length, 1);
    assert.equal(wiring.getStats().accountingOps, 1);
  });
});

// =============================================================================
// Full pipeline integration
// =============================================================================

describe('domain wiring: full pipeline', () => {
  let wiring;

  afterEach(() => {
    if (wiring) wiring.stop();
  });

  it('perception → judgment → decision → action → accounting (full chain)', () => {
    const judge = createMockJudge();
    const decider = createMockDecider();
    const actor = createMockActor();
    const learner = createMockLearner();
    const accountant = createMockAccountant();
    const emergence = createMockEmergence();

    wiring = createDomainWiring({
      name: 'full',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:full:data' }],
      verdictFilter: ['GROWL', 'BARK'],
    });

    wiring.wire({ judge, decider, actor, learner, accountant, emergence });

    // Trigger the pipeline
    globalEventBus.publish('perception:full:data', { value: 42 });

    const stats = wiring.getStats();
    assert.equal(stats.perceptionEvents, 1);
    assert.equal(stats.judgments, 1);
    assert.equal(stats.decisions, 1);
    assert.equal(stats.actions, 1);
    assert.equal(stats.learnings, 1);
    assert.equal(stats.accountingOps, 1);

    // All modules called
    assert.equal(judge.calls.length, 1);
    assert.equal(decider.calls.length, 1);
    assert.equal(actor.calls.length, 1);
    assert.equal(learner.calls.length, 1);
    assert.equal(accountant.calls.length, 1);
    assert.equal(emergence.calls.length, 1); // recordActivity
  });
});

// =============================================================================
// Stop / cleanup
// =============================================================================

describe('domain wiring: stop', () => {
  it('stops all listeners', () => {
    const judge = createMockJudge();
    const wiring = createDomainWiring({
      name: 'stoptest',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:stoptest:x' }],
    });

    wiring.wire({ judge });
    assert.equal(wiring.isWired(), true);

    wiring.stop();
    assert.equal(wiring.isWired(), false);

    // Events after stop should not reach judge
    globalEventBus.publish('perception:stoptest:x', { val: 1 });
    assert.equal(judge.calls.length, 0);
  });

  it('double-wire is idempotent', () => {
    const judge = createMockJudge();
    const wiring = createDomainWiring({
      name: 'dbl',
      cell: 'C0',
      perceptionEvents: [{ event: 'perception:dbl:x' }],
    });

    wiring.wire({ judge });
    wiring.wire({ judge }); // Should be no-op

    globalEventBus.publish('perception:dbl:x', {});
    assert.equal(judge.calls.length, 1); // Only 1 handler, not 2
  });
});

// =============================================================================
// Wiring Manager
// =============================================================================

describe('createWiringManager', () => {
  let manager;

  afterEach(() => {
    if (manager) manager.stopAll();
  });

  it('creates manager with all methods', () => {
    manager = createWiringManager([]);
    assert.equal(typeof manager.wireAll, 'function');
    assert.equal(typeof manager.wireDomain, 'function');
    assert.equal(typeof manager.stopAll, 'function');
    assert.equal(typeof manager.getStats, 'function');
    assert.equal(typeof manager.getWiring, 'function');
  });

  it('manages multiple domains', () => {
    manager = createWiringManager([
      { name: 'alpha', cell: 'C1', perceptionEvents: [{ event: 'perception:alpha:x' }] },
      { name: 'beta', cell: 'C2', perceptionEvents: [{ event: 'perception:beta:y' }] },
    ]);

    assert.equal(manager.getAll().length, 2);
    assert.ok(manager.getWiring('alpha'));
    assert.ok(manager.getWiring('beta'));
    assert.equal(manager.getWiring('gamma'), null);
  });

  it('wireAll wires all domains', () => {
    const judgeA = createMockJudge();
    const judgeB = createMockJudge();

    manager = createWiringManager([
      { name: 'a', cell: 'C1', perceptionEvents: [{ event: 'perception:a:x' }] },
      { name: 'b', cell: 'C2', perceptionEvents: [{ event: 'perception:b:x' }] },
    ]);

    manager.wireAll({
      a: { judge: judgeA },
      b: { judge: judgeB },
    });

    globalEventBus.publish('perception:a:x', {});
    globalEventBus.publish('perception:b:x', {});

    assert.equal(judgeA.calls.length, 1);
    assert.equal(judgeB.calls.length, 1);
  });

  it('wireDomain wires single domain', () => {
    const judge = createMockJudge();

    manager = createWiringManager([
      { name: 'single', cell: 'C1', perceptionEvents: [{ event: 'perception:single:x' }] },
    ]);

    manager.wireDomain('single', { judge });

    globalEventBus.publish('perception:single:x', {});
    assert.equal(judge.calls.length, 1);
  });

  it('getStats returns all domain stats', () => {
    manager = createWiringManager([
      { name: 'x', cell: 'C1' },
      { name: 'y', cell: 'C2' },
    ]);

    const stats = manager.getStats();
    assert.equal(stats.length, 2);
    assert.equal(stats[0].domain, 'x');
    assert.equal(stats[1].domain, 'y');
  });

  it('stopAll cleans up all wirings', () => {
    const judge = createMockJudge();
    manager = createWiringManager([
      { name: 'cleanup', cell: 'C1', perceptionEvents: [{ event: 'perception:cleanup:x' }] },
    ]);

    manager.wireAll({ cleanup: { judge } });
    manager.stopAll();

    globalEventBus.publish('perception:cleanup:x', {});
    assert.equal(judge.calls.length, 0);
  });
});

// =============================================================================
// FIB_INTERVALS
// =============================================================================

describe('FIB_INTERVALS', () => {
  it('contains Fibonacci intervals in milliseconds', () => {
    assert.equal(FIB_INTERVALS.F7, 13 * 60 * 1000);
    assert.equal(FIB_INTERVALS.F8, 21 * 60 * 1000);
    assert.equal(FIB_INTERVALS.F9, 34 * 60 * 1000);
    assert.equal(FIB_INTERVALS.F10, 55 * 60 * 1000);
    assert.equal(FIB_INTERVALS.F11, 89 * 60 * 1000);
  });
});
