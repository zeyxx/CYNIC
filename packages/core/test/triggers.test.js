/**
 * @cynic/core - Triggers Tests
 *
 * Tests auto-judgment trigger system:
 * - Trigger types and events
 * - Trigger matching
 * - TriggerManager
 * - PeriodicScheduler
 *
 * "The watchdog that never sleeps" - κυνικός
 *
 * @module @cynic/core/test/triggers
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  TRIGGER_CONSTANTS,
  TriggerType,
  TriggerEvent,
  TriggerAction,
  FibonacciIntervals,
  Trigger,
  TriggerManager,
  PeriodicScheduler,
} from '../src/triggers/index.js';

import { THRESHOLDS } from '../src/axioms/constants.js';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('TRIGGER_CONSTANTS', () => {
  it('should have Fibonacci-aligned values', () => {
    assert.strictEqual(TRIGGER_CONSTANTS.MAX_TRIGGERS, 21); // Fib(8)
    assert.strictEqual(TRIGGER_CONSTANTS.DEFAULT_DEBOUNCE_MS, 8000); // Fib(6) * 1000
    assert.strictEqual(TRIGGER_CONSTANTS.DEFAULT_COOLDOWN_MS, 34000); // Fib(9) * 1000
    assert.strictEqual(TRIGGER_CONSTANTS.MAX_TRIGGERS_PER_MINUTE, 5); // Fib(5)
    assert.strictEqual(TRIGGER_CONSTANTS.PATTERN_WINDOW, 13); // Fib(7)
  });
});

describe('TriggerType', () => {
  it('should have all expected types', () => {
    assert.strictEqual(TriggerType.EVENT, 'event');
    assert.strictEqual(TriggerType.PERIODIC, 'periodic');
    assert.strictEqual(TriggerType.PATTERN, 'pattern');
    assert.strictEqual(TriggerType.THRESHOLD, 'threshold');
    assert.strictEqual(TriggerType.COMPOSITE, 'composite');
  });
});

describe('TriggerEvent', () => {
  it('should have code events', () => {
    assert.strictEqual(TriggerEvent.COMMIT, 'commit');
    assert.strictEqual(TriggerEvent.PUSH, 'push');
    assert.strictEqual(TriggerEvent.MERGE, 'merge');
    assert.strictEqual(TriggerEvent.CODE_CHANGE, 'code_change');
  });

  it('should have error events', () => {
    assert.strictEqual(TriggerEvent.ERROR, 'error');
    assert.strictEqual(TriggerEvent.EXCEPTION, 'exception');
    assert.strictEqual(TriggerEvent.FAILURE, 'failure');
  });

  it('should have session events', () => {
    assert.strictEqual(TriggerEvent.SESSION_START, 'session_start');
    assert.strictEqual(TriggerEvent.SESSION_END, 'session_end');
  });
});

describe('TriggerAction', () => {
  it('should have all expected actions', () => {
    assert.strictEqual(TriggerAction.JUDGE, 'judge');
    assert.strictEqual(TriggerAction.LOG, 'log');
    assert.strictEqual(TriggerAction.ALERT, 'alert');
    assert.strictEqual(TriggerAction.BLOCK, 'block');
    assert.strictEqual(TriggerAction.REVIEW, 'review');
    assert.strictEqual(TriggerAction.NOTIFY, 'notify');
  });
});

describe('FibonacciIntervals', () => {
  it('should have Fibonacci-aligned intervals', () => {
    assert.strictEqual(FibonacciIntervals.QUICK, 5 * 1000);
    assert.strictEqual(FibonacciIntervals.SHORT, 8 * 1000);
    assert.strictEqual(FibonacciIntervals.BRIEF, 13 * 1000);
    assert.strictEqual(FibonacciIntervals.NORMAL, 21 * 1000);
    assert.strictEqual(FibonacciIntervals.MEDIUM, 34 * 1000);
    assert.strictEqual(FibonacciIntervals.STANDARD, 55 * 1000);
  });

  it('should have longer intervals', () => {
    assert.strictEqual(FibonacciIntervals.MODERATE, 89 * 1000);
    assert.strictEqual(FibonacciIntervals.EXTENDED, 144 * 1000);
    assert.strictEqual(FibonacciIntervals.LONG, 233 * 1000);
  });

  it('should have daily interval', () => {
    assert.strictEqual(FibonacciIntervals.DAILY, 24 * 60 * 60 * 1000);
  });
});

// =============================================================================
// TRIGGER CLASS TESTS
// =============================================================================

describe('Trigger', () => {
  describe('Construction', () => {
    it('should create with minimal options', () => {
      const trigger = new Trigger({
        name: 'Test Trigger',
      });

      assert.ok(trigger.id);
      assert.strictEqual(trigger.name, 'Test Trigger');
      assert.strictEqual(trigger.type, TriggerType.EVENT);
      assert.strictEqual(trigger.action, TriggerAction.JUDGE);
    });

    it('should create with full options', () => {
      const trigger = new Trigger({
        id: 'trg_custom',
        name: 'Custom Trigger',
        type: TriggerType.THRESHOLD,
        condition: { field: 'score', operator: 'lt', value: 50 },
        action: TriggerAction.ALERT,
        config: { priority: 'high' },
      });

      assert.strictEqual(trigger.id, 'trg_custom');
      assert.strictEqual(trigger.type, TriggerType.THRESHOLD);
      assert.strictEqual(trigger.condition.field, 'score');
      assert.strictEqual(trigger.config.priority, 'high');
    });

    it('should have default config', () => {
      const trigger = new Trigger({ name: 'Test' });

      assert.strictEqual(trigger.config.enabled, true);
      assert.strictEqual(trigger.config.debounceMs, TRIGGER_CONSTANTS.DEFAULT_DEBOUNCE_MS);
      assert.strictEqual(trigger.config.cooldownMs, TRIGGER_CONSTANTS.DEFAULT_COOLDOWN_MS);
      assert.strictEqual(trigger.config.priority, 'normal');
    });

    it('should have initial runtime state', () => {
      const trigger = new Trigger({ name: 'Test' });

      assert.strictEqual(trigger.lastTriggered, null);
      assert.strictEqual(trigger.triggerCount, 0);
      assert.strictEqual(trigger.lastResult, null);
    });
  });

  describe('Cooldown', () => {
    it('should not be on cooldown initially', () => {
      const trigger = new Trigger({ name: 'Test' });
      assert.strictEqual(trigger.isOnCooldown(), false);
    });

    it('should be on cooldown after trigger', () => {
      const trigger = new Trigger({
        name: 'Test',
        config: { cooldownMs: 1000 },
      });
      trigger.markTriggered();
      assert.strictEqual(trigger.isOnCooldown(), true);
    });

    it('should exit cooldown after time', async () => {
      const trigger = new Trigger({
        name: 'Test',
        config: { cooldownMs: 50 },
      });
      trigger.markTriggered();
      assert.strictEqual(trigger.isOnCooldown(), true);

      await new Promise(r => setTimeout(r, 60));
      assert.strictEqual(trigger.isOnCooldown(), false);
    });
  });

  describe('Event Matching', () => {
    it('should match by event type', () => {
      const trigger = new Trigger({
        name: 'Error Handler',
        type: TriggerType.EVENT,
        condition: { eventType: TriggerEvent.ERROR },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.COMMIT }), false);
    });

    it('should match with filter', () => {
      const trigger = new Trigger({
        name: 'Filtered',
        type: TriggerType.EVENT,
        condition: {
          eventType: TriggerEvent.ERROR,
          filter: { severity: 'critical' },
        },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR, severity: 'critical' }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR, severity: 'low' }), false);
    });

    it('should match with regex filter', () => {
      const trigger = new Trigger({
        name: 'Regex Filter',
        type: TriggerType.EVENT,
        condition: {
          eventType: TriggerEvent.COMMIT,
          filter: { message: /feat:/i },
        },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.COMMIT, message: 'feat: add feature' }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.COMMIT, message: 'fix: bug' }), false);
    });

    it('should match with function filter', () => {
      const trigger = new Trigger({
        name: 'Function Filter',
        type: TriggerType.EVENT,
        condition: {
          eventType: TriggerEvent.CODE_CHANGE,
          filter: { lines: (v) => v > 100 },
        },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.CODE_CHANGE, lines: 150 }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.CODE_CHANGE, lines: 50 }), false);
    });

    it('should not match when disabled', () => {
      const trigger = new Trigger({
        name: 'Disabled',
        condition: { eventType: TriggerEvent.ERROR },
        config: { enabled: false },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR }), false);
    });

    it('should not match when on cooldown', () => {
      const trigger = new Trigger({
        name: 'Cooldown',
        condition: { eventType: TriggerEvent.ERROR },
      });
      trigger.markTriggered();

      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR }), false);
    });
  });

  describe('Threshold Matching', () => {
    it('should match gt operator', () => {
      const trigger = new Trigger({
        name: 'Score High',
        type: TriggerType.THRESHOLD,
        condition: { field: 'score', operator: 'gt', value: 50 },
      });

      assert.strictEqual(trigger.matches({ score: 60 }), true);
      assert.strictEqual(trigger.matches({ score: 50 }), false);
      assert.strictEqual(trigger.matches({ score: 40 }), false);
    });

    it('should match gte operator', () => {
      const trigger = new Trigger({
        name: 'Score GTE',
        type: TriggerType.THRESHOLD,
        condition: { field: 'score', operator: 'gte', value: 50 },
      });

      assert.strictEqual(trigger.matches({ score: 50 }), true);
      assert.strictEqual(trigger.matches({ score: 60 }), true);
      assert.strictEqual(trigger.matches({ score: 40 }), false);
    });

    it('should match lt operator', () => {
      const trigger = new Trigger({
        name: 'Low Score',
        type: TriggerType.THRESHOLD,
        condition: { field: 'qScore', operator: 'lt', value: THRESHOLDS.GROWL },
      });

      assert.strictEqual(trigger.matches({ qScore: 20 }), true);
      assert.strictEqual(trigger.matches({ qScore: 60 }), false);
    });

    it('should match between operator', () => {
      const trigger = new Trigger({
        name: 'Range',
        type: TriggerType.THRESHOLD,
        condition: { field: 'value', operator: 'between', value: [10, 20] },
      });

      assert.strictEqual(trigger.matches({ value: 15 }), true);
      assert.strictEqual(trigger.matches({ value: 10 }), true);
      assert.strictEqual(trigger.matches({ value: 20 }), true);
      assert.strictEqual(trigger.matches({ value: 5 }), false);
    });

    it('should return false for missing field', () => {
      const trigger = new Trigger({
        name: 'Missing',
        type: TriggerType.THRESHOLD,
        condition: { field: 'score', operator: 'gt', value: 50 },
      });

      assert.strictEqual(trigger.matches({ other: 100 }), false);
    });
  });

  describe('Pattern Matching', () => {
    it('should match pattern occurrences', () => {
      const trigger = new Trigger({
        name: 'Pattern',
        type: TriggerType.PATTERN,
        condition: { minOccurrences: 3 },
      });

      assert.strictEqual(trigger.matches({ _patternMatches: 5 }), true);
      assert.strictEqual(trigger.matches({ _patternMatches: 2 }), false);
    });
  });

  describe('Composite Matching', () => {
    it('should match AND conditions', () => {
      const trigger = new Trigger({
        name: 'Composite AND',
        type: TriggerType.COMPOSITE,
        condition: {
          operator: 'AND',
          conditions: [
            { eventType: TriggerEvent.ERROR },
            { type: TriggerType.THRESHOLD, field: 'severity', operator: 'gte', value: 3 },
          ],
        },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR, severity: 5 }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR, severity: 2 }), false);
    });

    it('should match OR conditions', () => {
      const trigger = new Trigger({
        name: 'Composite OR',
        type: TriggerType.COMPOSITE,
        condition: {
          operator: 'OR',
          conditions: [
            { eventType: TriggerEvent.ERROR },
            { eventType: TriggerEvent.FAILURE },
          ],
        },
      });

      assert.strictEqual(trigger.matches({ type: TriggerEvent.ERROR }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.FAILURE }), true);
      assert.strictEqual(trigger.matches({ type: TriggerEvent.COMMIT }), false);
    });
  });

  describe('markTriggered', () => {
    it('should update runtime state', () => {
      const trigger = new Trigger({ name: 'Test' });
      const before = Date.now();

      trigger.markTriggered({ success: true });

      assert.ok(trigger.lastTriggered >= before);
      assert.strictEqual(trigger.triggerCount, 1);
      assert.deepStrictEqual(trigger.lastResult, { success: true });
    });

    it('should increment trigger count', () => {
      const trigger = new Trigger({ name: 'Test', config: { cooldownMs: 0 } });

      trigger.markTriggered();
      trigger.markTriggered();
      trigger.markTriggered();

      assert.strictEqual(trigger.triggerCount, 3);
    });
  });

  describe('toJSON', () => {
    it('should serialize trigger', () => {
      const trigger = new Trigger({
        id: 'trg_test',
        name: 'Test',
        type: TriggerType.EVENT,
        action: TriggerAction.LOG,
      });

      const json = trigger.toJSON();

      assert.strictEqual(json.id, 'trg_test');
      assert.strictEqual(json.name, 'Test');
      assert.strictEqual(json.type, TriggerType.EVENT);
      assert.strictEqual(json.action, TriggerAction.LOG);
      assert.ok(json.stats);
    });
  });
});

// =============================================================================
// TRIGGER MANAGER TESTS
// =============================================================================

describe('TriggerManager', () => {
  let manager;

  beforeEach(() => {
    manager = new TriggerManager({
      logCallback: () => {}, // Suppress logs
    });
  });

  describe('Construction', () => {
    it('should create with default options', () => {
      const m = new TriggerManager();
      assert.ok(m.triggers instanceof Map);
      assert.deepStrictEqual(m.stats.eventsProcessed, 0);
    });

    it('should accept callbacks', () => {
      let judgeCalled = false;
      const m = new TriggerManager({
        judgeCallback: () => { judgeCalled = true; },
      });
      assert.ok(m.judgeCallback);
    });
  });

  describe('Trigger Registration', () => {
    it('should register trigger', () => {
      const trigger = manager.register({
        name: 'Test',
        condition: { eventType: TriggerEvent.ERROR },
      });

      assert.ok(trigger instanceof Trigger);
      assert.strictEqual(manager.triggers.size, 1);
    });

    it('should accept Trigger instance', () => {
      const trigger = new Trigger({ name: 'Existing' });
      manager.register(trigger);

      assert.strictEqual(manager.getTrigger(trigger.id), trigger);
    });

    it('should enforce max triggers', () => {
      for (let i = 0; i < TRIGGER_CONSTANTS.MAX_TRIGGERS; i++) {
        manager.register({ id: `trg_max_${i}`, name: `Trigger ${i}` });
      }

      assert.throws(
        () => manager.register({ id: 'trg_overflow', name: 'One Too Many' }),
        /Max triggers/
      );
    });
  });

  describe('Trigger Management', () => {
    it('should unregister trigger', () => {
      const trigger = manager.register({ name: 'Test' });
      const result = manager.unregister(trigger.id);

      assert.strictEqual(result, true);
      assert.strictEqual(manager.triggers.size, 0);
    });

    it('should get trigger by ID', () => {
      const trigger = manager.register({ id: 'trg_123', name: 'Test' });
      assert.strictEqual(manager.getTrigger('trg_123'), trigger);
    });

    it('should list triggers', () => {
      manager.register({ id: 'trg_list_a', name: 'A' });
      manager.register({ id: 'trg_list_b', name: 'B' });

      const list = manager.listTriggers();
      assert.strictEqual(list.length, 2);
    });

    it('should enable/disable trigger', () => {
      const trigger = manager.register({ name: 'Test' });

      manager.setEnabled(trigger.id, false);
      assert.strictEqual(trigger.config.enabled, false);

      manager.setEnabled(trigger.id, true);
      assert.strictEqual(trigger.config.enabled, true);
    });
  });

  describe('Event Processing', () => {
    it('should process event and match trigger', async () => {
      manager.register({
        name: 'Error Handler',
        condition: { eventType: TriggerEvent.ERROR },
        action: TriggerAction.LOG,
      });

      const results = await manager.processEvent({ type: TriggerEvent.ERROR });

      assert.strictEqual(results.length, 1);
      assert.strictEqual(results[0].logged, true);
    });

    it('should increment events processed', async () => {
      await manager.processEvent({ type: 'test' });
      await manager.processEvent({ type: 'test' });

      assert.strictEqual(manager.stats.eventsProcessed, 2);
    });

    it('should track trigger activations', async () => {
      manager.register({
        name: 'Test',
        condition: { eventType: 'test' },
        action: TriggerAction.LOG,
        config: { cooldownMs: 0 },
      });

      await manager.processEvent({ type: 'test' });
      await manager.processEvent({ type: 'test' });

      assert.strictEqual(manager.stats.triggersActivated, 2);
    });

    it('should execute judge callback', async () => {
      let judgmentInput = null;
      manager.judgeCallback = (input) => {
        judgmentInput = input;
        return { verdict: 'WAG' };
      };

      manager.register({
        name: 'Judge',
        condition: { eventType: TriggerEvent.COMMIT },
        action: TriggerAction.JUDGE,
      });

      const results = await manager.processEvent({ type: TriggerEvent.COMMIT, data: { files: 5 } });

      assert.ok(judgmentInput);
      assert.strictEqual(results[0].judgment.verdict, 'WAG');
      assert.strictEqual(manager.stats.judgmentsGenerated, 1);
    });

    it('should execute alert callback', async () => {
      let alertData = null;
      manager.alertCallback = async (data) => {
        alertData = data;
      };

      manager.register({
        name: 'Alert',
        condition: { eventType: TriggerEvent.ERROR },
        action: TriggerAction.ALERT,
        config: { priority: 'high' },
      });

      await manager.processEvent({ type: TriggerEvent.ERROR });

      assert.ok(alertData);
      assert.strictEqual(alertData.severity, 'high');
      assert.strictEqual(manager.stats.alertsSent, 1);
    });

    it('should handle BLOCK action', async () => {
      manager.register({
        name: 'Blocker',
        condition: { eventType: 'dangerous' },
        action: TriggerAction.BLOCK,
      });

      const results = await manager.processEvent({ type: 'dangerous' });

      assert.strictEqual(results[0].blocked, true);
    });

    it('should sort by priority', async () => {
      manager.register({
        id: 'low',
        name: 'Low',
        condition: { eventType: 'test' },
        action: TriggerAction.LOG,
        config: { priority: 'low', cooldownMs: 0 },
      });
      manager.register({
        id: 'critical',
        name: 'Critical',
        condition: { eventType: 'test' },
        action: TriggerAction.LOG,
        config: { priority: 'critical', cooldownMs: 0 },
      });
      manager.register({
        id: 'normal',
        name: 'Normal',
        condition: { eventType: 'test' },
        action: TriggerAction.LOG,
        config: { priority: 'normal', cooldownMs: 0 },
      });

      const results = await manager.processEvent({ type: 'test' });

      assert.strictEqual(results[0].triggerId, 'critical');
      assert.strictEqual(results[1].triggerId, 'normal');
      assert.strictEqual(results[2].triggerId, 'low');
    });
  });

  describe('Rate Limiting', () => {
    it('should skip when rate limited', async () => {
      // Fill up rate limit
      for (let i = 0; i < TRIGGER_CONSTANTS.MAX_TRIGGERS_PER_MINUTE; i++) {
        manager.triggerTimestamps.push(Date.now());
      }

      const results = await manager.processEvent({ type: 'test' });

      assert.strictEqual(results[0].skipped, true);
      assert.strictEqual(results[0].reason, 'rate_limited');
    });
  });

  describe('Default Triggers', () => {
    it('should register default triggers', () => {
      manager.registerDefaults();

      assert.ok(manager.getTrigger('trg_error'));
      assert.ok(manager.getTrigger('trg_commit'));
      assert.ok(manager.getTrigger('trg_decision'));
      assert.ok(manager.getTrigger('trg_error_pattern'));
      assert.ok(manager.getTrigger('trg_low_score'));
    });

    it('should return manager for chaining', () => {
      const result = manager.registerDefaults();
      assert.strictEqual(result, manager);
    });
  });

  describe('Status & Export', () => {
    it('should get status', () => {
      manager.register({ name: 'Test' });
      const status = manager.getStatus();

      assert.ok(status.triggers);
      assert.ok(status.stats);
      assert.ok(status.rateLimitStatus);
    });

    it('should export state', () => {
      manager.register({ name: 'Test' });
      const state = manager.export();

      assert.ok(Array.isArray(state.triggers));
      assert.ok(state.stats);
    });

    it('should import state', () => {
      manager.register({ id: 'trg_test', name: 'Test' });
      const state = manager.export();

      const newManager = new TriggerManager();
      newManager.import(state);

      assert.ok(newManager.getTrigger('trg_test'));
    });

    it('should reset manager', () => {
      manager.register({ name: 'Test' });
      manager.eventHistory.push({ type: 'test' });

      manager.reset();

      assert.strictEqual(manager.triggers.size, 0);
      assert.strictEqual(manager.eventHistory.length, 0);
      assert.strictEqual(manager.stats.eventsProcessed, 0);
    });
  });
});

// =============================================================================
// PERIODIC SCHEDULER TESTS
// =============================================================================

describe('PeriodicScheduler', () => {
  let scheduler;

  beforeEach(() => {
    scheduler = new PeriodicScheduler({
      onError: () => {}, // Suppress errors
    });
  });

  afterEach(() => {
    scheduler.stopAll();
  });

  describe('Construction', () => {
    it('should create empty scheduler', () => {
      assert.strictEqual(scheduler.tasks.size, 0);
      assert.strictEqual(scheduler.intervals.size, 0);
    });
  });

  describe('Task Registration', () => {
    it('should register task', () => {
      const task = scheduler.register({
        id: 'task1',
        name: 'Test Task',
        intervalMs: 1000,
        handler: async () => {},
        enabled: false, // Don't start
      });

      assert.strictEqual(task.id, 'task1');
      assert.strictEqual(task.name, 'Test Task');
      assert.strictEqual(scheduler.tasks.size, 1);
    });

    it('should generate task ID if not provided', () => {
      const task = scheduler.register({
        name: 'No ID',
        intervalMs: 1000,
        handler: async () => {},
        enabled: false,
      });

      assert.ok(task.id.startsWith('task_'));
    });

    it('should start task when enabled', () => {
      scheduler.register({
        id: 'auto',
        name: 'Auto Start',
        intervalMs: 10000,
        handler: async () => {},
        enabled: true,
      });

      assert.strictEqual(scheduler.intervals.size, 1);
    });
  });

  describe('Task Execution', () => {
    it('should run immediately if configured', async () => {
      let ran = false;
      scheduler.register({
        name: 'Immediate',
        intervalMs: 100000,
        handler: async () => { ran = true; },
        runImmediately: true,
        enabled: true,
      });

      // Give it a moment to execute
      await new Promise(r => setTimeout(r, 50));
      assert.strictEqual(ran, true);
    });

    it('should execute on interval', async () => {
      let runCount = 0;
      scheduler.register({
        name: 'Interval',
        intervalMs: 30,
        handler: async () => { runCount++; },
        enabled: true,
      });

      await new Promise(r => setTimeout(r, 100));
      assert.ok(runCount >= 2, `Expected at least 2 runs, got ${runCount}`);
    });

    it('should track execution stats', async () => {
      scheduler.register({
        name: 'Stats',
        intervalMs: 20,
        handler: async () => {},
        runImmediately: true,
        enabled: true,
      });

      await new Promise(r => setTimeout(r, 50));

      const task = scheduler.tasks.values().next().value;
      assert.ok(task.runCount > 0);
      assert.ok(task.lastRun);
    });

    it('should handle errors gracefully', async () => {
      scheduler.register({
        name: 'Failing',
        intervalMs: 20,
        handler: async () => { throw new Error('Task failed'); },
        runImmediately: true,
        enabled: true,
      });

      await new Promise(r => setTimeout(r, 50));

      const task = scheduler.tasks.values().next().value;
      assert.ok(task.errorCount > 0);
      assert.ok(task.lastError);
    });
  });

  describe('Task Control', () => {
    it('should enable task', () => {
      scheduler.register({
        id: 'ctrl',
        name: 'Control',
        intervalMs: 10000,
        handler: async () => {},
        enabled: false,
      });

      scheduler.enable('ctrl');

      assert.strictEqual(scheduler.intervals.has('ctrl'), true);
    });

    it('should disable task', () => {
      scheduler.register({
        id: 'ctrl',
        name: 'Control',
        intervalMs: 10000,
        handler: async () => {},
        enabled: true,
      });

      scheduler.disable('ctrl');

      assert.strictEqual(scheduler.intervals.has('ctrl'), false);
    });

    it('should unregister task', () => {
      scheduler.register({
        id: 'rm',
        name: 'Remove',
        intervalMs: 10000,
        handler: async () => {},
        enabled: true,
      });

      const result = scheduler.unregister('rm');

      assert.strictEqual(result, true);
      assert.strictEqual(scheduler.tasks.size, 0);
      assert.strictEqual(scheduler.intervals.size, 0);
    });

    it('should manually trigger task', async () => {
      let ran = false;
      scheduler.register({
        id: 'manual',
        name: 'Manual',
        intervalMs: 100000,
        handler: async () => { ran = true; },
        enabled: false,
      });

      await scheduler.trigger('manual');

      assert.strictEqual(ran, true);
    });

    it('should stop all tasks', () => {
      scheduler.register({ id: 'a', name: 'A', intervalMs: 1000, handler: async () => {}, enabled: true });
      scheduler.register({ id: 'b', name: 'B', intervalMs: 1000, handler: async () => {}, enabled: true });

      scheduler.stopAll();

      assert.strictEqual(scheduler.intervals.size, 0);
    });

    it('should start all enabled tasks', () => {
      scheduler.register({ id: 'a', name: 'A', intervalMs: 1000, handler: async () => {}, enabled: true });
      scheduler.register({ id: 'b', name: 'B', intervalMs: 1000, handler: async () => {}, enabled: false });

      scheduler.stopAll();
      scheduler.startAll();

      assert.strictEqual(scheduler.intervals.has('a'), true);
      assert.strictEqual(scheduler.intervals.has('b'), false);
    });
  });

  describe('Listing & Status', () => {
    it('should list tasks', () => {
      scheduler.register({ id: 'a', name: 'A', intervalMs: 1000, handler: async () => {}, enabled: false });
      scheduler.register({ id: 'b', name: 'B', intervalMs: 2000, handler: async () => {}, enabled: false });

      const list = scheduler.list();

      assert.strictEqual(list.length, 2);
      assert.ok(list[0].id);
      assert.ok(list[0].name);
      assert.ok(list[0].intervalMs);
    });

    it('should get status', () => {
      scheduler.register({ name: 'Test', intervalMs: 1000, handler: async () => {}, enabled: true });

      const status = scheduler.getStatus();

      assert.ok(status.tasks);
      assert.ok(status.stats);
      assert.strictEqual(status.running, 1);
    });
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('Trigger Edge Cases', () => {
  it('should handle empty condition', () => {
    const trigger = new Trigger({
      name: 'Empty',
      type: TriggerType.EVENT,
      condition: {},
    });

    // Should match anything when no condition specified
    assert.strictEqual(trigger.matches({ type: 'any' }), true);
  });

  it('should handle unknown trigger type', () => {
    const trigger = new Trigger({
      name: 'Unknown',
      type: 'unknown_type',
    });

    assert.strictEqual(trigger.matches({ type: 'test' }), false);
  });

  it('should handle unknown threshold operator', () => {
    const trigger = new Trigger({
      name: 'Unknown Op',
      type: TriggerType.THRESHOLD,
      condition: { field: 'x', operator: 'unknown', value: 5 },
    });

    assert.strictEqual(trigger.matches({ x: 10 }), false);
  });

  it('should handle composite without conditions array', () => {
    const trigger = new Trigger({
      name: 'Bad Composite',
      type: TriggerType.COMPOSITE,
      condition: { operator: 'AND' },
    });

    assert.strictEqual(trigger.matches({ type: 'test' }), false);
  });
});
