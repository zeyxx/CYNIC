/**
 * CYNIC E2E Integration Tests - Phase 20
 *
 * Tests the full flow:
 * - Session start → profile load
 * - Hook events → Collective → globalEventBus → Learning
 * - Orchestration routing → skill invocation
 * - Decision persistence
 *
 * "φ verifies the whole" - κυνικός
 *
 * @module @cynic/node/test/integration-e2e
 */

import { describe, it, before, after, mock } from 'node:test';
import assert from 'node:assert/strict';

// Core imports
import { globalEventBus, EventType, createLogger } from '@cynic/core';

// Orchestration imports
import {
  UnifiedOrchestrator,
  DecisionEvent,
  EventSource,
  DecisionOutcome,
  SkillRegistry,
  DecisionTracer,
  createUnifiedOrchestrator,
  createSkillRegistry,
  createDecisionTracer,
} from '../src/orchestration/index.js';

// Collective imports
import { createCollectivePack } from '../src/agents/collective/index.js';

// Services imports
import { EventBus, EventType as NodeEventType, createEventBus } from '../src/services/event-bus.js';
import { LearningManager } from '../src/judge/learning-manager.js';
import { LearningService } from '../src/judge/learning-service.js';

const log = createLogger('E2E-Test');

// TODO: Fix cleanup - CollectivePack keeps Node running
describe.skip('Phase 20: E2E Integration', () => {
  let orchestrator;
  let collective;
  let tracer;
  let skillRegistry;
  let learningManager;
  let nodeEventBus;
  let globalEventsSeen;

  before(async () => {
    // Track events on globalEventBus
    globalEventsSeen = [];
    globalEventBus.subscribe('*', (event) => {
      globalEventsSeen.push(event);
    });

    // Create services
    nodeEventBus = createEventBus();
    tracer = createDecisionTracer({ maxTraces: 100 });
    skillRegistry = createSkillRegistry();

    // Create collective
    collective = await createCollectivePack({
      enableEventBus: true,
    });

    // Create learning manager
    learningManager = new LearningManager({
      autoLearn: false, // Manual for testing
      minSamples: 1,
    });

    // Create orchestrator
    orchestrator = createUnifiedOrchestrator({
      tracer,
      skillRegistry,
    });
  });

  after(() => {
    // Cleanup
    if (collective?.stop) collective.stop();
    if (learningManager?.destroy) learningManager.destroy();
  });

  describe('DecisionEvent Flow', () => {
    it('should create decision event with proper stages', () => {
      const event = new DecisionEvent({
        eventType: 'user_prompt',
        source: EventSource.HOOK_PERCEIVE, // Use valid EventSource
      });

      assert.ok(event.id.startsWith('dec_'), 'ID should have dec_ prefix');
      assert.equal(event.eventType, 'user_prompt');
      assert.equal(event.source, EventSource.HOOK_PERCEIVE);
      assert.equal(event.outcome, 'pending'); // lowercase
      assert.ok(Array.isArray(event.trace), 'Trace should be array');
    });

    it('should track routing through trace', () => {
      const event = new DecisionEvent({ eventType: 'test' });

      event.setRouting({
        sefirah: 'Gevurah',
        domain: 'protection',
        suggestedAgent: 'cynic-guardian',
      });

      assert.equal(event.routing.sefirah, 'Gevurah');
      assert.equal(event.routing.domain, 'protection');
      assert.ok(event.trace.length > 0, 'Trace should have entries');
    });

    it('should finalize with outcome', () => {
      const event = new DecisionEvent({ eventType: 'test' });
      event.setRouting({ sefirah: 'Keter', domain: 'general' });
      event.finalize('allow', ['Test passed']); // lowercase outcome, array for reasoning

      assert.equal(event.outcome, 'allow');
      assert.ok(event.reasoning.includes('Test passed'));
    });
  });

  describe('DecisionTracer Recording', () => {
    it('should record and retrieve decisions', async () => {
      const event = new DecisionEvent({
        eventType: 'test_record',
        source: EventSource.MCP_TOOL,
      });
      event.setRouting({ sefirah: 'Binah', domain: 'design' });
      event.finalize('allow', ['Design approved']);

      await tracer.record(event);

      const retrieved = tracer.get(event.id);
      assert.ok(retrieved, 'Should retrieve recorded event');
      assert.equal(retrieved.routing.domain, 'design');
    });

    it('should index by outcome', async () => {
      // Create a blocked event
      const blockedEvent = new DecisionEvent({ eventType: 'test_blocked' });
      blockedEvent.finalize('block', ['Dangerous operation']); // lowercase, array
      await tracer.record(blockedEvent);

      const blocked = tracer.getBlocked(10);
      assert.ok(blocked.length > 0, 'Should find blocked events');
      assert.equal(blocked[0].outcome, 'block');
    });

    it('should get recent decisions', () => {
      const recent = tracer.getRecent(5);
      assert.ok(Array.isArray(recent));
      assert.ok(recent.length > 0, 'Should have recent decisions');
    });
  });

  describe('SkillRegistry Mapping', () => {
    it('should map domain to skill', () => {
      const skill = skillRegistry.getSkillForDomain('protection');
      assert.ok(skill, 'Should find skill for protection');
      assert.equal(skill.name, 'judge');
      assert.equal(skill.mcpTool, 'brain_cynic_judge');
    });

    it('should return null for unknown domain', () => {
      const skill = skillRegistry.getSkillForDomain('nonexistent');
      assert.equal(skill, null);
    });

    it('should list all skills', () => {
      const skills = skillRegistry.listSkills();
      assert.ok(Array.isArray(skills));
      assert.ok(skills.length > 0, 'Should have registered skills');

      const domains = skills.map(s => s.domain);
      assert.ok(domains.includes('protection'));
      assert.ok(domains.includes('wisdom'));
    });
  });

  describe('Collective Hook Processing', () => {
    it('should process hook event through agents', async () => {
      const hookData = {
        hookType: 'PreToolUse',
        payload: {
          tool: 'Write',
          input: { file_path: '/test/safe.txt' },
        },
        userId: 'test-user',
        sessionId: 'test-session',
      };

      const result = await collective.receiveHookEvent(hookData);

      assert.ok(result, 'Should return result');
      assert.equal(result.blocked, false, 'Safe operation should not be blocked');
      assert.ok(result.agentResults, 'Should have agent results');
    });

    it('should block dangerous operations', async () => {
      const hookData = {
        hookType: 'PreToolUse',
        payload: {
          tool: 'Bash',
          input: { command: 'rm -rf /' },
        },
        userId: 'test-user',
        sessionId: 'test-session',
      };

      const result = await collective.receiveHookEvent(hookData);

      assert.equal(result.blocked, true, 'Dangerous operation should be blocked');
      assert.ok(result.blockedBy, 'Should identify blocker');
    });

    it('should emit to globalEventBus', async () => {
      const beforeCount = globalEventsSeen.length;

      await collective.receiveHookEvent({
        hookType: 'PostToolUse',
        payload: {
          tool: 'Read',
          success: true,
          duration: 100,
        },
        userId: 'test-user',
        sessionId: 'test-session',
      });

      // Give async events time to propagate
      await new Promise(r => setTimeout(r, 50));

      const afterCount = globalEventsSeen.length;
      assert.ok(afterCount > beforeCount, 'Should emit events to globalEventBus');

      // Check for specific event types
      const recentEvents = globalEventsSeen.slice(beforeCount);
      const eventTypes = recentEvents.map(e => e.type);
      assert.ok(
        eventTypes.includes(EventType.TOOL_COMPLETED) || eventTypes.includes(EventType.USER_FEEDBACK),
        'Should emit TOOL_COMPLETED or USER_FEEDBACK'
      );
    });
  });

  describe('Orchestrator Processing', () => {
    it('should process content through orchestrator', async () => {
      const result = await orchestrator.process({
        eventType: 'user_prompt',
        content: 'analyze this code for security issues',
        source: EventSource.HOOK_PERCEIVE,
        userContext: { userId: 'test-user' },
      });

      assert.ok(result, 'Should return result');
      assert.ok(result.id, 'Should have decision ID');
      assert.ok(result.routing, 'Should have routing decision');
    });

    it('should route protection requests correctly', async () => {
      const result = await orchestrator.process({
        eventType: 'user_prompt',
        content: 'delete all files in the directory',
        source: EventSource.MCP_TOOL,
        userContext: { userId: 'test-user' },
      });

      // Should route to protection/danger detection
      assert.ok(result.routing, 'Should have routing');
      // Risk should be detected (intervention is part of routing)
      assert.ok(
        result.routing?.risk === 'high' ||
        result.routing?.risk === 'critical' ||
        result.routing?.domain === 'protection',
        'Should detect danger'
      );
    });
  });

  describe('Learning Integration', () => {
    it('should track feedback through learning manager', async () => {
      const feedback = {
        source: 'test',
        itemType: 'code',
        positive: true,
        context: { test: true },
      };

      await learningManager.addFeedback(feedback);
      // Feedback is added to pending queue
      const pendingCount = learningManager.getPendingFeedbackCount();
      assert.ok(pendingCount > 0, 'Should have pending feedback');
    });
  });

  describe('End-to-End Flow', () => {
    it('should complete full decision flow', async () => {
      // 1. Create decision event
      const event = new DecisionEvent({
        eventType: 'e2e_test',
        source: EventSource.HOOK_PERCEIVE,
      });

      // 2. Set routing (includes intervention in routing)
      event.setRouting({
        sefirah: 'Chesed',
        domain: 'analysis',
        suggestedAgent: 'cynic-analyst',
        suggestedTools: ['brain_patterns'],
        intervention: 'silent', // intervention is part of routing
        risk: 'low',
      });

      // 3. Finalize (lowercase, array for reasoning)
      event.finalize('allow', ['E2E test completed successfully']);

      // 4. Record to tracer
      await tracer.record(event);

      // 5. Verify retrieval
      const retrieved = tracer.get(event.id);
      assert.ok(retrieved, 'Should retrieve from tracer');
      assert.equal(retrieved.outcome, 'allow');
      assert.equal(retrieved.routing.domain, 'analysis');

      // 6. Verify trace has all stages
      const trace = event.getFormattedTrace();
      assert.ok(trace.includes('ROUTING'), 'Trace should include routing');
      assert.ok(trace.includes('FINALIZED'), 'Trace should include finalization');
    });

    it('should handle blocked flow', async () => {
      const event = new DecisionEvent({
        eventType: 'e2e_blocked_test',
        source: EventSource.MCP_TOOL,
      });

      event.setRouting({
        sefirah: 'Gevurah',
        domain: 'protection',
        intervention: 'block',
        risk: 'critical',
      });

      event.finalize('block', ['Dangerous operation blocked']);

      await tracer.record(event);

      const blocked = tracer.getBlocked(10);
      const found = blocked.find(b => b.id === event.id);
      assert.ok(found, 'Should be in blocked list');
    });
  });

  describe('Summary Statistics', () => {
    it('should generate tracer summary', () => {
      const summary = tracer.getSummary();

      assert.ok(summary.total > 0, 'Should have recorded decisions');
      assert.ok(summary.outcomes, 'Should have outcome counts');
      assert.ok(summary.domains, 'Should have domain counts');
      assert.ok(summary.stats, 'Should have stats');
    });

    it('should track skill registry stats', () => {
      const stats = skillRegistry.getStats();

      assert.ok(typeof stats.invocations === 'number');
      assert.ok(typeof stats.successes === 'number');
      assert.ok(typeof stats.failures === 'number');
    });
  });
});

describe('Phase 20: Integration Smoke Test', () => {
  it('should import all orchestration components', async () => {
    const orchestration = await import('../src/orchestration/index.js');

    assert.ok(orchestration.UnifiedOrchestrator, 'UnifiedOrchestrator');
    assert.ok(orchestration.DecisionEvent, 'DecisionEvent');
    assert.ok(orchestration.SkillRegistry, 'SkillRegistry');
    assert.ok(orchestration.DecisionTracer, 'DecisionTracer');
    assert.ok(orchestration.createUnifiedOrchestrator, 'createUnifiedOrchestrator');
    assert.ok(orchestration.createSkillRegistry, 'createSkillRegistry');
    assert.ok(orchestration.createDecisionTracer, 'createDecisionTracer');
    assert.ok(orchestration.CircuitBreaker, 'CircuitBreaker');
    assert.ok(orchestration.createCircuitBreaker, 'createCircuitBreaker');
  });

  it('should have EventType constants in core', async () => {
    const core = await import('@cynic/core');

    assert.ok(core.EventType.USER_FEEDBACK, 'USER_FEEDBACK');
    assert.ok(core.EventType.TOOL_COMPLETED, 'TOOL_COMPLETED');
    assert.ok(core.EventType.SESSION_STARTED, 'SESSION_STARTED');
    assert.ok(core.EventType.SESSION_ENDED, 'SESSION_ENDED');
  });
});

describe('Phase 20: Circuit Breaker', () => {
  let CircuitBreaker, CircuitState, createCircuitBreaker;

  before(async () => {
    const mod = await import('../src/orchestration/circuit-breaker.js');
    CircuitBreaker = mod.CircuitBreaker;
    CircuitState = mod.CircuitState;
    createCircuitBreaker = mod.createCircuitBreaker;
  });

  it('should start in closed state', () => {
    const cb = createCircuitBreaker({ name: 'test' });
    assert.equal(cb.state, CircuitState.CLOSED);
    assert.equal(cb.isAllowed(), true);
  });

  it('should track successes', () => {
    const cb = createCircuitBreaker({ name: 'test-success' });
    cb.recordSuccess();
    cb.recordSuccess();

    const stats = cb.getStats();
    assert.equal(stats.successes, 2);
    assert.equal(stats.totalCalls, 2);
    assert.equal(stats.state, CircuitState.CLOSED);
  });

  it('should open after exceeding failure threshold', () => {
    const cb = createCircuitBreaker({
      name: 'test-open',
      failureThreshold: 0.5, // 50% for testing
      minSamples: 3,
    });

    // Record 3 failures to meet minSamples and exceed threshold
    cb.recordFailure();
    cb.recordFailure();
    cb.recordFailure(); // Opens here (100% failure rate > 50%)

    assert.equal(cb.state, CircuitState.OPEN);
    assert.equal(cb.isAllowed(), false);
  });

  it('should reject calls when open', () => {
    const cb = createCircuitBreaker({
      name: 'test-reject-2', // Unique name to avoid registry conflicts
      failureThreshold: 0.5,
      minSamples: 2,
    });

    cb.recordFailure();
    cb.recordFailure();

    assert.equal(cb.state, CircuitState.OPEN);
    cb.isAllowed(); // First rejection
    assert.equal(cb.isAllowed(), false);
    assert.ok(cb.getStats().rejectedCalls >= 1, 'Should have rejected calls');
  });

  it('should transition to half-open after timeout', async () => {
    const cb = createCircuitBreaker({
      name: 'test-halfopen',
      failureThreshold: 0.5,
      minSamples: 2,
      timeout: 50, // Short timeout for testing
    });

    cb.recordFailure();
    cb.recordFailure();
    assert.equal(cb.state, CircuitState.OPEN);

    // Wait for timeout
    await new Promise(r => setTimeout(r, 60));

    assert.equal(cb.state, CircuitState.HALF_OPEN);
    assert.equal(cb.isAllowed(), true);
  });

  it('should close after success in half-open', async () => {
    const cb = createCircuitBreaker({
      name: 'test-close',
      failureThreshold: 0.5,
      minSamples: 2,
      timeout: 30,
    });

    cb.recordFailure();
    cb.recordFailure();
    await new Promise(r => setTimeout(r, 40));

    assert.equal(cb.state, CircuitState.HALF_OPEN);

    cb.recordSuccess();
    assert.equal(cb.state, CircuitState.CLOSED);
  });

  it('should re-open on failure in half-open', async () => {
    const cb = createCircuitBreaker({
      name: 'test-reopen',
      failureThreshold: 0.5,
      minSamples: 2,
      timeout: 30,
    });

    cb.recordFailure();
    cb.recordFailure();
    await new Promise(r => setTimeout(r, 40));

    assert.equal(cb.state, CircuitState.HALF_OPEN);

    cb.recordFailure();
    assert.equal(cb.state, CircuitState.OPEN);
  });

  it('should reset properly', () => {
    const cb = createCircuitBreaker({
      name: 'test-reset',
      failureThreshold: 0.5,
      minSamples: 2,
    });

    cb.recordFailure();
    cb.recordFailure();
    assert.equal(cb.state, CircuitState.OPEN);

    cb.reset();
    assert.equal(cb.state, CircuitState.CLOSED);
    assert.equal(cb.isAllowed(), true);
  });

  it('should provide health status', () => {
    const cb = createCircuitBreaker({ name: 'test-health' });
    const health = cb.getHealth();

    assert.equal(health.name, 'test-health');
    assert.equal(health.state, CircuitState.CLOSED);
    assert.equal(health.healthy, true);
  });
});
