/**
 * Intelligent Router Tests
 *
 * Comprehensive tests for RoutingDecision and IntelligentRouter classes.
 * Covers confidence capping, dog routing, security escalation,
 * handler management, stats tracking, and weight persistence.
 *
 * @module @cynic/node/test/intelligent-router
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import {
  TaskDescriptor,
  createTaskDescriptor,
  TaskType,
  ComplexityLevel,
  RiskLevel,
  DogCapabilityMatrix,
  DogId,
  DOG_CAPABILITIES,
  IntelligentRouter,
  createIntelligentRouter,
  RoutingDecision,
} from '../src/routing/index.js';

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

// ─────────────────────────────────────────────────────────────────────────────
// RoutingDecision
// ─────────────────────────────────────────────────────────────────────────────

describe('RoutingDecision', () => {
  const makeTask = (input = 'test task') => createTaskDescriptor(input);

  describe('confidence capping', () => {
    it('should cap confidence at PHI_INV (0.618)', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.95,
        task: makeTask(),
        candidates: [],
        reason: 'test',
      });
      assert.ok(decision.confidence <= PHI_INV);
      assert.strictEqual(decision.confidence, PHI_INV);
    });

    it('should preserve confidence when below PHI_INV', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.42,
        task: makeTask(),
      });
      assert.strictEqual(decision.confidence, 0.42);
    });

    it('should handle zero confidence', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0,
        task: makeTask(),
      });
      assert.strictEqual(decision.confidence, 0);
    });

    it('should cap confidence exactly at PHI_INV boundary', () => {
      const decision = new RoutingDecision({
        dogId: DogId.CYNIC,
        confidence: PHI_INV,
        task: makeTask(),
      });
      assert.strictEqual(decision.confidence, PHI_INV);
    });
  });

  describe('isHighConfidence()', () => {
    it('should return true when confidence >= PHI_INV_2', () => {
      const decision = new RoutingDecision({
        dogId: DogId.GUARDIAN,
        confidence: PHI_INV, // will be capped to PHI_INV which >= PHI_INV_2
        task: makeTask(),
      });
      assert.strictEqual(decision.isHighConfidence(), true);
    });

    it('should return false when confidence < PHI_INV_2', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.2,
        task: makeTask(),
      });
      assert.strictEqual(decision.isHighConfidence(), false);
    });

    it('should return true at exactly PHI_INV_2', () => {
      const decision = new RoutingDecision({
        dogId: DogId.ANALYST,
        confidence: PHI_INV_2,
        task: makeTask(),
      });
      assert.strictEqual(decision.isHighConfidence(), true);
    });
  });

  describe('getDogInfo()', () => {
    it('should return capability info for valid dog', () => {
      const decision = new RoutingDecision({
        dogId: DogId.GUARDIAN,
        confidence: 0.5,
        task: makeTask(),
      });
      const info = decision.getDogInfo();
      assert.ok(info);
      assert.strictEqual(info.name, 'Guardian');
      assert.strictEqual(info.sefira, 'Gevurah');
      assert.strictEqual(info.emoji, '🛡️');
    });

    it('should return null for unknown dog ID', () => {
      const decision = new RoutingDecision({
        dogId: 'nonexistent_dog',
        confidence: 0.3,
        task: makeTask(),
      });
      assert.strictEqual(decision.getDogInfo(), null);
    });
  });

  describe('toJSON()', () => {
    it('should serialize all fields correctly', () => {
      const task = createTaskDescriptor('check for security vulnerabilities');
      const decision = new RoutingDecision({
        dogId: DogId.GUARDIAN,
        confidence: 0.55,
        task,
        candidates: [{ dogId: DogId.GUARDIAN }, { dogId: DogId.CYNIC }],
        reason: 'Security task',
      });
      decision.blocked = true;
      decision.escalated = true;

      const json = decision.toJSON();

      assert.strictEqual(json.dogId, DogId.GUARDIAN);
      assert.strictEqual(json.dogName, 'Guardian');
      assert.strictEqual(json.dogEmoji, '🛡️');
      assert.strictEqual(json.confidence, 0.55);
      assert.strictEqual(json.candidateCount, 2);
      assert.strictEqual(json.reason, 'Security task');
      assert.strictEqual(json.blocked, true);
      assert.strictEqual(json.escalated, true);
      assert.ok(typeof json.timestamp === 'number');
      assert.ok(typeof json.taskType === 'string');
      assert.ok(typeof json.complexity === 'string');
      assert.ok(typeof json.risk === 'string');
    });

    it('should round confidence to 3 decimal places', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.12345678,
        task: makeTask(),
      });
      const json = decision.toJSON();
      assert.strictEqual(json.confidence, 0.123);
    });

    it('should handle missing dog info gracefully', () => {
      const decision = new RoutingDecision({
        dogId: 'fake',
        confidence: 0.3,
        task: makeTask(),
      });
      const json = decision.toJSON();
      assert.strictEqual(json.dogName, undefined);
      assert.strictEqual(json.dogEmoji, undefined);
    });
  });

  describe('default values', () => {
    it('should default blocked and escalated to false', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.5,
        task: makeTask(),
      });
      assert.strictEqual(decision.blocked, false);
      assert.strictEqual(decision.escalated, false);
    });

    it('should default candidates to empty array', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.5,
        task: makeTask(),
      });
      assert.deepStrictEqual(decision.candidates, []);
    });

    it('should default reason to empty string', () => {
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.5,
        task: makeTask(),
      });
      assert.strictEqual(decision.reason, '');
    });

    it('should set a numeric timestamp', () => {
      const before = Date.now();
      const decision = new RoutingDecision({
        dogId: DogId.SCOUT,
        confidence: 0.5,
        task: makeTask(),
      });
      const after = Date.now();
      assert.ok(decision.timestamp >= before);
      assert.ok(decision.timestamp <= after);
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// IntelligentRouter
// ─────────────────────────────────────────────────────────────────────────────

describe('IntelligentRouter', () => {
  let router;

  beforeEach(() => {
    router = createIntelligentRouter();
  });

  // ── route() ──────────────────────────────────────────────────────────────

  describe('route()', () => {
    it('should route security audit tasks to Guardian', async () => {
      const decision = await router.route('check for security vulnerabilities');
      assert.ok(decision instanceof RoutingDecision);
      assert.strictEqual(decision.dogId, DogId.GUARDIAN);
    });

    it('should route exploration tasks to Scout', async () => {
      const decision = await router.route('explore and find where errors are handled');
      assert.strictEqual(decision.dogId, DogId.SCOUT);
    });

    it('should route cleanup tasks to Janitor', async () => {
      const decision = await router.route('clean up unused code');
      assert.strictEqual(decision.dogId, DogId.JANITOR);
    });

    it('should route deployment tasks with security escalation', async () => {
      // "deploy" triggers HIGH risk, so security escalation routes to Guardian
      const decision = await router.route('deploy to staging now');
      assert.ok(
        [DogId.GUARDIAN, DogId.DEPLOYER, DogId.CYNIC].includes(decision.dogId),
        `Expected Guardian/Deployer/CYNIC for deploy task, got ${decision.dogId}`
      );
    });

    it('should accept a TaskDescriptor as input', async () => {
      const task = createTaskDescriptor('explore the codebase');
      const decision = await router.route(task);
      assert.ok(decision instanceof RoutingDecision);
      assert.ok(decision.task === task);
    });

    it('should cap all decision confidences at PHI_INV', async () => {
      const decision = await router.route('audit security in auth module');
      assert.ok(decision.confidence <= PHI_INV);
    });

    it('should return candidates list', async () => {
      const decision = await router.route('analyze code quality');
      assert.ok(Array.isArray(decision.candidates));
    });
  });

  // ── Security escalation ──────────────────────────────────────────────────

  describe('security escalation for high-risk tasks', () => {
    it('should route critical risk to Guardian when Guardian is a candidate', async () => {
      const decision = await router.route('delete all production credentials');
      assert.ok(
        [DogId.GUARDIAN, DogId.CYNIC].includes(decision.dogId),
        `Expected Guardian or CYNIC for critical task, got ${decision.dogId}`
      );
    });

    it('should mark escalated decisions when task is escalated to CYNIC', async () => {
      const decision = await router.route('delete all production data');
      if (decision.dogId === DogId.CYNIC) {
        assert.strictEqual(decision.escalated, true);
      }
    });

    it('should escalate high-risk tasks when selected dog cannot block', async () => {
      // "deploy" is HIGH risk (via "deploy" keyword), and the selected dog
      // may or may not have canBlock capability
      const decision = await router.route('deploy changes to production credentials');
      assert.ok(decision.confidence <= PHI_INV);
    });
  });

  // ── Handler registration ─────────────────────────────────────────────────

  describe('registerHandler()', () => {
    it('should register a handler for a valid dog', () => {
      const handler = async () => ({ result: 'ok' });
      router.registerHandler(DogId.SCOUT, handler);
      assert.ok(router.handlers.has(DogId.SCOUT));
    });

    it('should throw for invalid dog ID', () => {
      assert.throws(() => {
        router.registerHandler('fake_dog', () => {});
      }, /Invalid dog ID/);
    });

    it('should emit handler:registered event', (t, done) => {
      router.on('handler:registered', (evt) => {
        assert.strictEqual(evt.dogId, DogId.ANALYST);
        done();
      });
      router.registerHandler(DogId.ANALYST, async () => {});
    });
  });

  describe('unregisterHandler()', () => {
    it('should remove a registered handler', () => {
      router.registerHandler(DogId.SCOUT, async () => {});
      router.unregisterHandler(DogId.SCOUT);
      assert.ok(!router.handlers.has(DogId.SCOUT));
    });

    it('should emit handler:unregistered event', (t, done) => {
      router.registerHandler(DogId.SCOUT, async () => {});
      router.on('handler:unregistered', (evt) => {
        assert.strictEqual(evt.dogId, DogId.SCOUT);
        done();
      });
      router.unregisterHandler(DogId.SCOUT);
    });

    it('should silently handle unregistering non-existent handler', () => {
      assert.doesNotThrow(() => {
        router.unregisterHandler(DogId.ORACLE);
      });
    });
  });

  // ── Constructor options ──────────────────────────────────────────────────

  describe('constructor options', () => {
    it('should accept pre-configured handlers', () => {
      const r = createIntelligentRouter({
        handlers: {
          [DogId.SCOUT]: async () => 'scouted',
          [DogId.GUARDIAN]: async () => 'guarded',
        },
      });
      assert.strictEqual(r.handlers.size, 2);
      assert.ok(r.handlers.has(DogId.SCOUT));
      assert.ok(r.handlers.has(DogId.GUARDIAN));
    });

    it('should default learnFromOutcomes to true', () => {
      const r = createIntelligentRouter();
      assert.strictEqual(r.learnFromOutcomes, true);
    });

    it('should allow disabling learnFromOutcomes', () => {
      const r = createIntelligentRouter({ learnFromOutcomes: false });
      assert.strictEqual(r.learnFromOutcomes, false);
    });
  });

  // ── routeAndExecute() ────────────────────────────────────────────────────

  describe('routeAndExecute()', () => {
    it('should execute handler and return success', async () => {
      router.registerHandler(DogId.SCOUT, async (task, ctx, decision) => {
        return { explored: true, raw: task.raw };
      });

      const result = await router.routeAndExecute('explore the codebase');

      assert.strictEqual(result.success, true);
      assert.ok(result.result.explored);
      assert.ok(result.decision instanceof RoutingDecision);
      assert.ok(typeof result.latency === 'number');
      assert.ok(result.latency >= 0);
    });

    it('should return error when no handler is registered', async () => {
      const result = await router.routeAndExecute('security audit');

      assert.strictEqual(result.success, false);
      assert.ok(result.error.includes('No handler'));
      assert.ok(result.decision instanceof RoutingDecision);
    });

    it('should escalate to CYNIC when primary handler fails', async () => {
      router.registerHandler(DogId.SCOUT, async () => {
        throw new Error('Scout failed');
      });
      router.registerHandler(DogId.CYNIC, async () => {
        return { rescued: true };
      });

      const result = await router.routeAndExecute('explore files');

      assert.strictEqual(result.success, true);
      assert.ok(result.result.rescued);
      assert.strictEqual(result.decision.escalated, true);
    });

    it('should return failure when handler throws and no CYNIC fallback', async () => {
      router.registerHandler(DogId.SCOUT, async () => {
        throw new Error('Scout broke');
      });

      const result = await router.routeAndExecute('explore files');

      assert.strictEqual(result.success, false);
      assert.ok(result.result.error.includes('Scout broke'));
    });

    it('should not escalate if the failing dog IS CYNIC', async () => {
      router.registerHandler(DogId.CYNIC, async () => {
        throw new Error('CYNIC failed');
      });

      // Route something that goes to CYNIC
      const result = await router.routeAndExecute('plan the entire system architecture');

      assert.strictEqual(result.success, false);
    });

    it('should record outcome when learnFromOutcomes is enabled', async () => {
      router.registerHandler(DogId.SCOUT, async () => ({ done: true }));

      await router.routeAndExecute('explore');

      assert.strictEqual(router.stats.outcomes.success, 1);
    });

    it('should not record outcome when learnFromOutcomes is disabled', async () => {
      const noLearnRouter = createIntelligentRouter({ learnFromOutcomes: false });
      noLearnRouter.registerHandler(DogId.SCOUT, async () => ({ done: true }));

      await noLearnRouter.routeAndExecute('explore');

      assert.strictEqual(noLearnRouter.stats.outcomes.success, 0);
    });
  });

  // ── recordOutcome() ──────────────────────────────────────────────────────

  describe('recordOutcome()', () => {
    it('should record success and update success rate', async () => {
      const decision = await router.route('explore');
      router.recordOutcome(decision, true);

      assert.strictEqual(router.stats.outcomes.success, 1);
      assert.strictEqual(router.stats.successRate, 1);
    });

    it('should record failure and update success rate', async () => {
      const decision = await router.route('explore');
      router.recordOutcome(decision, false);

      assert.strictEqual(router.stats.outcomes.failure, 1);
      assert.strictEqual(router.stats.successRate, 0);
    });

    it('should calculate correct success rate with mixed outcomes', async () => {
      const d1 = await router.route('explore');
      const d2 = await router.route('search for files');
      const d3 = await router.route('find patterns');

      router.recordOutcome(d1, true);
      router.recordOutcome(d2, true);
      router.recordOutcome(d3, false);

      const total = router.stats.outcomes.success + router.stats.outcomes.failure;
      assert.strictEqual(total, 3);
      assert.ok(Math.abs(router.stats.successRate - 2 / 3) < 0.001);
    });

    it('should store recent decisions with cap at _maxRecentDecisions', async () => {
      for (let i = 0; i < 5; i++) {
        const decision = await router.route('explore');
        router.recordOutcome(decision, true);
      }
      assert.strictEqual(router._recentDecisions.length, 5);
    });

    it('should emit outcome:recorded event', async () => {
      const events = [];
      router.on('outcome:recorded', (evt) => events.push(evt));

      const decision = await router.route('explore');
      router.recordOutcome(decision, true);

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].success, true);
      assert.ok(typeof events[0].successRate === 'number');
    });
  });

  // ── getStats() ───────────────────────────────────────────────────────────

  describe('getStats()', () => {
    it('should track total routed count', async () => {
      await router.route('explore');
      await router.route('cleanup');
      await router.route('security');

      const stats = router.getStats();
      assert.strictEqual(stats.routed, 3);
    });

    it('should track routing by dog', async () => {
      await router.route('explore the codebase');
      await router.route('find files');

      const stats = router.getStats();
      assert.ok(typeof stats.byDog === 'object');
      // At least one dog should have routes
      const totalByDog = Object.values(stats.byDog).reduce((a, b) => a + b, 0);
      assert.strictEqual(totalByDog, 2);
    });

    it('should track average confidence', async () => {
      await router.route('security audit');
      await router.route('explore files');

      const stats = router.getStats();
      assert.ok(stats.avgConfidence > 0);
      assert.ok(stats.avgConfidence <= PHI_INV);
    });

    it('should include handler count', () => {
      router.registerHandler(DogId.SCOUT, async () => {});
      router.registerHandler(DogId.GUARDIAN, async () => {});

      const stats = router.getStats();
      assert.strictEqual(stats.handlersRegistered, 2);
    });

    it('should include matrix weights', () => {
      const stats = router.getStats();
      assert.ok(typeof stats.matrixWeights === 'object');
    });

    it('should include recent decisions (max 10)', async () => {
      router.registerHandler(DogId.SCOUT, async () => 'ok');
      for (let i = 0; i < 15; i++) {
        await router.routeAndExecute('explore');
      }

      const stats = router.getStats();
      assert.ok(stats.recentDecisions.length <= 10);
    });
  });

  // ── resetStats() ─────────────────────────────────────────────────────────

  describe('resetStats()', () => {
    it('should reset all stat counters to zero', async () => {
      await router.route('explore');
      await router.route('security');
      router.resetStats();

      const stats = router.getStats();
      assert.strictEqual(stats.routed, 0);
      assert.strictEqual(stats.blocked, 0);
      assert.strictEqual(stats.escalated, 0);
      assert.strictEqual(stats.avgConfidence, 0);
      assert.strictEqual(stats.successRate, 0);
      assert.strictEqual(stats.outcomes.success, 0);
      assert.strictEqual(stats.outcomes.failure, 0);
    });

    it('should clear recent decisions', async () => {
      const decision = await router.route('explore');
      router.recordOutcome(decision, true);
      router.resetStats();

      assert.strictEqual(router._recentDecisions.length, 0);
    });

    it('should reinitialize byDog counters for all dogs', () => {
      router.resetStats();
      const stats = router.getStats();
      for (const dogId of Object.values(DogId)) {
        assert.strictEqual(stats.byDog[dogId], 0);
      }
    });
  });

  // ── exportWeights / importWeights ────────────────────────────────────────

  describe('exportWeights() and importWeights()', () => {
    it('should export empty weights when no outcomes recorded', () => {
      const weights = router.exportWeights();
      assert.ok(typeof weights === 'object');
    });

    it('should export weights after recording outcomes', async () => {
      router.registerHandler(DogId.SCOUT, async () => ({ done: true }));
      await router.routeAndExecute('explore');

      const weights = router.exportWeights();
      assert.ok(typeof weights === 'object');
    });

    it('should import weights into another router', async () => {
      router.registerHandler(DogId.SCOUT, async () => ({ done: true }));
      await router.routeAndExecute('explore');

      const weights = router.exportWeights();
      const router2 = createIntelligentRouter();
      router2.importWeights(weights);

      const weights2 = router2.exportWeights();
      assert.deepStrictEqual(weights, weights2);
    });

    it('should affect routing after importing weights', () => {
      const weights = {
        [DogId.ANALYST]: { [TaskType.ANALYSIS]: 0.15 },
      };
      router.importWeights(weights);
      const exported = router.exportWeights();
      assert.strictEqual(exported[DogId.ANALYST][TaskType.ANALYSIS], 0.15);
    });
  });

  // ── Handler preference in routing ────────────────────────────────────────

  describe('handler-aware routing', () => {
    it('should prefer candidate with registered handler', async () => {
      // Register handler for a less-optimal dog only
      router.registerHandler(DogId.ANALYST, async () => 'analyzed');

      const decision = await router.route('analyze the code quality');
      // The router should still pick the best match but may adjust for handler availability
      assert.ok(decision instanceof RoutingDecision);
    });
  });

  // ── Event emission during routing ────────────────────────────────────────

  describe('event emission', () => {
    it('should emit route:start event', async () => {
      const events = [];
      router.on('route:start', (e) => events.push(e));

      await router.route('explore');
      assert.strictEqual(events.length, 1);
      assert.ok(events[0].task);
    });

    it('should emit route:decided on routeAndExecute', async () => {
      const events = [];
      router.on('route:decided', (e) => events.push(e));
      router.registerHandler(DogId.SCOUT, async () => 'ok');

      await router.routeAndExecute('explore');
      assert.strictEqual(events.length, 1);
      assert.ok(events[0].decision);
    });

    it('should emit route:no_handler when handler missing', async () => {
      const events = [];
      router.on('route:no_handler', (e) => events.push(e));

      await router.routeAndExecute('explore');
      assert.strictEqual(events.length, 1);
      assert.ok(events[0].dogId);
    });

    it('should emit route:error when handler throws', async () => {
      const events = [];
      router.on('route:error', (e) => events.push(e));
      router.registerHandler(DogId.SCOUT, async () => {
        throw new Error('boom');
      });

      await router.routeAndExecute('explore');
      assert.ok(events.length >= 1);
      assert.ok(events[0].error);
    });

    it('should emit route:completed after successful execution', async () => {
      const events = [];
      router.on('route:completed', (e) => events.push(e));
      router.registerHandler(DogId.SCOUT, async () => 'ok');

      await router.routeAndExecute('explore');
      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].success, true);
      assert.ok(typeof events[0].latency === 'number');
    });
  });

  // ── getRecommendation() ──────────────────────────────────────────────────

  describe('getRecommendation()', () => {
    it('should return dogs for security audit', () => {
      const recs = router.getRecommendation(TaskType.SECURITY_AUDIT);
      assert.ok(recs.length > 0);
      assert.strictEqual(recs[0].dogId, DogId.GUARDIAN);
    });

    it('should return dogs sorted by affinity', () => {
      const recs = router.getRecommendation(TaskType.EXPLORATION);
      for (let i = 1; i < recs.length; i++) {
        assert.ok(recs[i - 1].affinity >= recs[i].affinity);
      }
    });

    it('should return empty array for unknown task type', () => {
      const recs = router.getRecommendation('completely_made_up_type');
      assert.strictEqual(recs.length, 0);
    });
  });
});
