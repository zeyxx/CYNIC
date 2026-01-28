/**
 * Tests for Orchestration Module (Task Management, Consultation, Circuit Breaker)
 *
 * @module @cynic/core/test/orchestration
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  Task,
  Orchestrator,
  TaskPriority,
  TaskStatus,
  AggregationStrategy,
  ORCHESTRATION_CONSTANTS,
  CONSULTATION_MATRIX,
  CIRCUIT_BREAKER_CONSTANTS,
  ConsultationCircuitBreaker,
  getConsultants,
  shouldConsult,
  calculatePackEffectiveness,
} from '../src/orchestration/index.js';

import { PHI_INV, PHI_INV_2 } from '../src/axioms/constants.js';

// =============================================================================
// TASK TESTS
// =============================================================================

describe('Orchestration: Task', () => {
  it('should create task with unique ID', () => {
    const task1 = new Task({ type: 'test' });
    const task2 = new Task({ type: 'test' });
    assert.notEqual(task1.id, task2.id);
  });

  it('should set default values', () => {
    const task = new Task({ type: 'analyze' });
    assert.equal(task.type, 'analyze');
    assert.equal(task.priority, TaskPriority.NORMAL);
    assert.equal(task.status, TaskStatus.PENDING);
    assert.equal(task.timeout, ORCHESTRATION_CONSTANTS.TASK_TIMEOUT_MS);
  });

  it('should track lifecycle', () => {
    const task = new Task({ type: 'test' });
    assert.equal(task.status, TaskStatus.PENDING);

    task.start('agent-1');
    assert.equal(task.status, TaskStatus.RUNNING);
    assert.equal(task.assignedAgent, 'agent-1');
    assert.ok(task.startedAt);

    task.complete({ result: 'done' });
    assert.equal(task.status, TaskStatus.COMPLETED);
    assert.ok(task.completedAt);
  });

  it('should track failure', () => {
    const task = new Task({ type: 'test' });
    task.start('agent-1');
    task.fail('Something went wrong');
    assert.equal(task.status, TaskStatus.FAILED);
    assert.equal(task.error, 'Something went wrong');
  });

  it('should calculate duration', () => {
    const task = new Task({ type: 'test' });
    assert.equal(task.getDuration(), 0);

    task.start('agent-1');
    // Small delay to ensure some duration
    const startTime = Date.now();
    while (Date.now() - startTime < 10) { /* wait */ }
    assert.ok(task.getDuration() >= 0);
  });

  it('should detect timeout', () => {
    const task = new Task({ type: 'test', timeout: 100 });
    assert.ok(!task.isTimedOut());

    task.start('agent-1');
    // Simulate timeout
    task.startedAt = Date.now() - 200;
    assert.ok(task.isTimedOut());
  });
});

// =============================================================================
// ORCHESTRATOR TESTS
// =============================================================================

describe('Orchestration: Orchestrator', () => {
  let orchestrator;

  beforeEach(() => {
    orchestrator = new Orchestrator();
  });

  describe('agent management', () => {
    it('should register agents', () => {
      const agent = { process: async () => ({ result: 'ok' }) };
      orchestrator.registerAgent('agent-1', agent, { taskTypes: ['analyze'] });

      const status = orchestrator.getStatus();
      assert.equal(status.agents.total, 1);
    });

    it('should unregister agents', () => {
      const agent = { process: async () => ({ result: 'ok' }) };
      orchestrator.registerAgent('agent-1', agent);
      orchestrator.unregisterAgent('agent-1');

      const status = orchestrator.getStatus();
      assert.equal(status.agents.total, 0);
    });

    it('should get available agents', () => {
      const agent1 = { process: async () => ({}) };
      const agent2 = { process: async () => ({}) };

      orchestrator.registerAgent('agent-1', agent1, { taskTypes: ['analyze'] });
      orchestrator.registerAgent('agent-2', agent2, { taskTypes: ['search'] });

      const available = orchestrator.getAvailableAgents('analyze');
      assert.equal(available.length, 1);
      assert.equal(available[0].agentId, 'agent-1');
    });

    it('should prioritize preferred agents', () => {
      const agent1 = { process: async () => ({}) };
      const agent2 = { process: async () => ({}) };

      orchestrator.registerAgent('agent-1', agent1);
      orchestrator.registerAgent('agent-2', agent2);

      const available = orchestrator.getAvailableAgents('test', ['agent-2']);
      assert.equal(available[0].agentId, 'agent-2');
    });
  });

  describe('task submission', () => {
    it('should submit tasks to queue', () => {
      const task = orchestrator.submit({ type: 'test', payload: {} });
      assert.ok(task.id);
      assert.equal(orchestrator.stats.tasksCreated, 1);
    });

    it('should prioritize by priority level', () => {
      orchestrator.submit({ type: 'test', priority: TaskPriority.LOW });
      orchestrator.submit({ type: 'test', priority: TaskPriority.CRITICAL });
      orchestrator.submit({ type: 'test', priority: TaskPriority.NORMAL });

      // Queue should be: CRITICAL, NORMAL, LOW
      assert.equal(orchestrator.queue[0].priority, TaskPriority.CRITICAL);
      assert.equal(orchestrator.queue[1].priority, TaskPriority.NORMAL);
      assert.equal(orchestrator.queue[2].priority, TaskPriority.LOW);
    });
  });

  describe('result aggregation', () => {
    it('should aggregate with FIRST strategy', () => {
      const results = [
        { agentId: 'a1', result: { score: 80 }, error: null },
        { agentId: 'a2', result: { score: 90 }, error: null },
      ];

      const aggregated = orchestrator.aggregate(results, AggregationStrategy.FIRST);
      assert.ok(aggregated.success);
      assert.equal(aggregated.result.score, 80);
    });

    it('should aggregate with BEST strategy', () => {
      const results = [
        { agentId: 'a1', result: { score: 80 }, error: null },
        { agentId: 'a2', result: { score: 90 }, error: null },
      ];

      const aggregated = orchestrator.aggregate(results, AggregationStrategy.BEST);
      assert.ok(aggregated.success);
      assert.equal(aggregated.agentId, 'a2');
    });

    it('should aggregate with CONSENSUS strategy', () => {
      const results = [
        { agentId: 'a1', result: { verdict: 'PASS', confidence: 0.6 }, error: null },
        { agentId: 'a2', result: { verdict: 'PASS', confidence: 0.5 }, error: null },
        { agentId: 'a3', result: { verdict: 'FAIL', confidence: 0.4 }, error: null },
      ];

      const aggregated = orchestrator.aggregate(results, AggregationStrategy.CONSENSUS);
      assert.ok(aggregated.success);
      assert.equal(aggregated.verdict, 'PASS');
      assert.ok(aggregated.agreementRatio >= 0.66);
    });

    it('should aggregate with ALL strategy', () => {
      const results = [
        { agentId: 'a1', result: { data: 1 }, error: null },
        { agentId: 'a2', result: { data: 2 }, error: null },
      ];

      const aggregated = orchestrator.aggregate(results, AggregationStrategy.ALL);
      assert.ok(aggregated.success);
      assert.equal(aggregated.results.length, 2);
    });

    it('should handle all failures', () => {
      const results = [
        { agentId: 'a1', result: null, error: 'Error 1' },
        { agentId: 'a2', result: null, error: 'Error 2' },
      ];

      const aggregated = orchestrator.aggregate(results, AggregationStrategy.FIRST);
      assert.ok(!aggregated.success);
      assert.equal(aggregated.agentErrors.length, 2);
    });
  });

  describe('status', () => {
    it('should return correct status', () => {
      orchestrator.registerAgent('a1', { process: async () => ({}) });
      orchestrator.submit({ type: 'test' });

      const status = orchestrator.getStatus();
      assert.equal(status.agents.total, 1);
      assert.ok(status.tasks.queued >= 0);
      assert.ok('stats' in status);
    });
  });
});

// =============================================================================
// CONSULTATION MATRIX TESTS
// =============================================================================

describe('Orchestration: Consultation Matrix', () => {
  it('should have all agent roles defined', () => {
    const expectedRoles = [
      'architect', 'reviewer', 'guardian', 'scout', 'tester',
      'simplifier', 'deployer', 'oracle', 'integrator', 'doc',
      'librarian', 'cartographer', 'archivist', 'solana-expert',
    ];

    for (const role of expectedRoles) {
      assert.ok(role in CONSULTATION_MATRIX, `Missing role: ${role}`);
    }
  });

  it('should not include self-consultation', () => {
    for (const [role, tasks] of Object.entries(CONSULTATION_MATRIX)) {
      for (const [taskType, consultants] of Object.entries(tasks)) {
        assert.ok(!consultants.includes(role),
          `${role} should not consult itself for ${taskType}`);
      }
    }
  });

  it('should have max 2 consultants per task type', () => {
    for (const [role, tasks] of Object.entries(CONSULTATION_MATRIX)) {
      for (const [taskType, consultants] of Object.entries(tasks)) {
        assert.ok(consultants.length <= 2,
          `${role}.${taskType} has ${consultants.length} consultants (max 2)`);
      }
    }
  });

  describe('getConsultants', () => {
    it('should return consultants for known role and task', () => {
      const consultants = getConsultants('architect', 'design');
      assert.ok(consultants.length > 0);
      assert.ok(consultants.includes('reviewer'));
    });

    it('should return empty array for unknown role', () => {
      const consultants = getConsultants('unknown', 'design');
      assert.deepEqual(consultants, []);
    });

    it('should return empty array for unknown task', () => {
      const consultants = getConsultants('architect', 'unknown_task');
      assert.deepEqual(consultants, []);
    });
  });

  describe('shouldConsult', () => {
    it('should require consultation for high-risk operations', () => {
      const result = shouldConsult('deployer', 'deploy', { isHighRisk: true });
      assert.ok(result.needed);
      assert.ok(result.consultants.includes('guardian'));
    });

    it('should require consultation for low confidence', () => {
      const result = shouldConsult('architect', 'design', { confidence: 0.2 });
      assert.ok(result.needed);
      assert.ok(result.consultants.length > 0);
    });

    it('should consult archivist for pattern matches', () => {
      const result = shouldConsult('reviewer', 'quality', { hasPatternMatch: true });
      assert.ok(result.needed);
      assert.ok(result.consultants.includes('archivist'));
    });

    it('should not require consultation for high confidence normal ops', () => {
      const result = shouldConsult('scout', 'search', {
        confidence: 0.6,
        isHighRisk: false,
        hasPatternMatch: false,
      });
      assert.ok(!result.needed);
    });
  });
});

// =============================================================================
// CIRCUIT BREAKER TESTS
// =============================================================================

describe('Orchestration: Circuit Breaker', () => {
  let breaker;

  beforeEach(() => {
    breaker = new ConsultationCircuitBreaker();
  });

  describe('basic checks', () => {
    it('should block self-consultation', () => {
      const result = breaker.canConsult('architect', 'architect');
      assert.ok(!result.allowed);
      assert.ok(result.reason.includes('self'));
    });

    it('should allow valid consultation', () => {
      const result = breaker.canConsult('architect', 'reviewer');
      assert.ok(result.allowed);
    });
  });

  describe('depth limiting', () => {
    it('should track consultation depth', () => {
      breaker.enterConsultation('a', 'b');
      assert.equal(breaker.currentDepth, 1);

      breaker.enterConsultation('b', 'c');
      assert.equal(breaker.currentDepth, 2);

      breaker.exitConsultation('b', 'c');
      assert.equal(breaker.currentDepth, 1);
    });

    it('should block at max depth', () => {
      // Fill to max depth
      for (let i = 0; i < CIRCUIT_BREAKER_CONSTANTS.MAX_DEPTH; i++) {
        breaker.enterConsultation(`agent-${i}`, `agent-${i + 1}`);
      }

      const result = breaker.canConsult('agent-x', 'agent-y');
      assert.ok(!result.allowed);
      assert.ok(result.reason.includes('depth'));
    });
  });

  describe('cycle detection', () => {
    it('should detect direct cycles', () => {
      breaker.enterConsultation('a', 'b');

      const result = breaker.canConsult('a', 'b');
      assert.ok(!result.allowed);
      assert.ok(result.reason.includes('Cycle'));
    });

    it('should allow different edges', () => {
      breaker.enterConsultation('a', 'b');

      const result = breaker.canConsult('b', 'c');
      assert.ok(result.allowed);
    });
  });

  describe('consultation counting', () => {
    it('should count consultations', () => {
      breaker.enterConsultation('a', 'b');
      breaker.enterConsultation('b', 'c');
      assert.equal(breaker.consultationCount, 2);
    });

    it('should block at max consultations', () => {
      const max = CIRCUIT_BREAKER_CONSTANTS.MAX_CONSULTATIONS;
      for (let i = 0; i < max; i++) {
        breaker.enterConsultation(`from-${i}`, `to-${i}`);
      }

      const result = breaker.canConsult('new', 'agent');
      assert.ok(!result.allowed);
      // Could be blocked by max consultations OR max depth (whichever hits first)
      assert.ok(
        result.reason.includes('consultations') || result.reason.includes('depth'),
        `Expected consultations/depth block, got: ${result.reason}`
      );
    });
  });

  describe('token budget', () => {
    it('should track tokens used', () => {
      breaker.enterConsultation('a', 'b');
      breaker.exitConsultation('a', 'b', { tokensUsed: 1000 });
      assert.equal(breaker.tokensUsed, 1000);
    });

    it('should block when budget exceeded', () => {
      breaker.tokensUsed = breaker.tokenBudget - 100;

      const result = breaker.canConsult('a', 'b', { estimatedTokens: 200 });
      assert.ok(!result.allowed);
      assert.ok(result.reason.includes('budget'));
    });
  });

  describe('status and reset', () => {
    it('should return status', () => {
      breaker.enterConsultation('a', 'b');
      const status = breaker.getStatus();

      assert.equal(status.depth, 1);
      assert.equal(status.consultations, 1);
      assert.ok(status.visitedEdges.includes('a->b'));
    });

    it('should reset state', () => {
      breaker.enterConsultation('a', 'b');
      breaker.reset();

      assert.equal(breaker.currentDepth, 0);
      assert.equal(breaker.consultationCount, 0);
      assert.equal(breaker.visited.size, 0);
    });

    it('should preserve cooldowns on reset', () => {
      breaker.exitConsultation('a', 'b');
      breaker.reset();

      assert.ok(breaker.lastConsultations.has('b'));
    });

    it('should clear cooldowns on fullReset', () => {
      breaker.exitConsultation('a', 'b');
      breaker.fullReset();

      assert.ok(!breaker.lastConsultations.has('b'));
    });
  });

  describe('isOpen detection', () => {
    it('should report open when depth exceeded', () => {
      for (let i = 0; i < CIRCUIT_BREAKER_CONSTANTS.MAX_DEPTH; i++) {
        breaker.enterConsultation(`a-${i}`, `b-${i}`);
      }
      assert.ok(breaker.getStatus().isOpen);
    });

    it('should report open when consultations exceeded', () => {
      for (let i = 0; i < CIRCUIT_BREAKER_CONSTANTS.MAX_CONSULTATIONS; i++) {
        breaker.enterConsultation(`from-${i}`, `to-${i}`);
        breaker.exitConsultation(`from-${i}`, `to-${i}`);
      }
      assert.ok(breaker.getStatus().isOpen);
    });
  });
});

// =============================================================================
// PACK EFFECTIVENESS TESTS
// =============================================================================

describe('Orchestration: Pack Effectiveness', () => {
  it('should calculate effectiveness', () => {
    const result = calculatePackEffectiveness({
      avgQScore: 70,
      avgResponseTime: 10000,
      consensusRate: 0.8,
      consultationSuccess: 0.9,
    });

    assert.ok(result.E > 0);
    assert.ok(result.E <= 100);
    assert.ok('breakdown' in result);
    assert.ok('formula' in result);
  });

  it('should penalize low quality', () => {
    const high = calculatePackEffectiveness({ avgQScore: 90 });
    const low = calculatePackEffectiveness({ avgQScore: 30 });

    assert.ok(high.E > low.E);
  });

  it('should penalize slow response', () => {
    const fast = calculatePackEffectiveness({ avgResponseTime: 5000 });
    const slow = calculatePackEffectiveness({ avgResponseTime: 60000 });

    assert.ok(fast.E > slow.E);
  });

  it('should penalize low coherence', () => {
    const coherent = calculatePackEffectiveness({
      consensusRate: 0.9,
      consultationSuccess: 0.9,
    });
    const incoherent = calculatePackEffectiveness({
      consensusRate: 0.2,
      consultationSuccess: 0.2,
    });

    assert.ok(coherent.E > incoherent.E);
  });

  it('should use geometric mean (balanced scoring)', () => {
    // One weak component should drag down overall score
    const balanced = calculatePackEffectiveness({
      avgQScore: 70,
      avgResponseTime: 10000,
      consensusRate: 0.7,
      consultationSuccess: 0.7,
    });

    const unbalanced = calculatePackEffectiveness({
      avgQScore: 90,
      avgResponseTime: 10000,
      consensusRate: 0.2,
      consultationSuccess: 0.2,
    });

    // Unbalanced should score lower due to geometric mean
    assert.ok(balanced.E > unbalanced.E);
  });
});

// =============================================================================
// PHI ALIGNMENT TESTS
// =============================================================================

describe('Orchestration: PHI Alignment', () => {
  it('should use phi for consensus threshold', () => {
    assert.equal(ORCHESTRATION_CONSTANTS.CONSENSUS_THRESHOLD, PHI_INV);
  });

  it('should use phi^-2 for minimum confidence', () => {
    assert.equal(ORCHESTRATION_CONSTANTS.MIN_CONFIDENCE, PHI_INV_2);
  });

  it('should use phi^-2 for consultation budget ratio', () => {
    assert.equal(CIRCUIT_BREAKER_CONSTANTS.CONSULTATION_BUDGET_RATIO, PHI_INV_2);
  });
});
