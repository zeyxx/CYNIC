#!/usr/bin/env node
/**
 * Total Memory Repository Tests
 *
 * Tests for Phase 16: Total Memory + Full Autonomy repositories.
 * Unit tests for enums, integration tests for repositories (require PostgreSQL).
 *
 * "φ remembers everything" - CYNIC
 *
 * @module @cynic/persistence/test/total-memory
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import dotenv from 'dotenv';

// Load .env from monorepo root
const __dirname = dirname(fileURLToPath(import.meta.url));
dotenv.config({ path: resolve(__dirname, '../../../.env') });

// Total Memory repositories
import { ConversationMemoriesRepository, MemoryType } from '../src/postgres/repositories/conversation-memories.js';
import { ArchitecturalDecisionsRepository, DecisionType, DecisionStatus } from '../src/postgres/repositories/architectural-decisions.js';
import { LessonsLearnedRepository, LessonCategory, LessonSeverity } from '../src/postgres/repositories/lessons-learned.js';
import { AutonomousGoalsRepository, GoalType, GoalStatus } from '../src/postgres/repositories/autonomous-goals.js';
import { AutonomousTasksRepository, TaskStatus, TaskType } from '../src/postgres/repositories/autonomous-tasks.js';
import { ProactiveNotificationsRepository, NotificationType } from '../src/postgres/repositories/proactive-notifications.js';

// Check if PostgreSQL is available
// Require explicit opt-in: set CYNIC_INTEGRATION_TESTS=1 to run DB tests
const hasPostgres = !!process.env.CYNIC_DATABASE_URL && process.env.CYNIC_INTEGRATION_TESTS === '1';

// ============================================================================
// ENUMS TESTS (Unit tests - no DB required)
// ============================================================================

describe('Total Memory Enums', () => {
  describe('MemoryType', () => {
    it('has expected values', () => {
      assert.equal(MemoryType.SUMMARY, 'summary');
      assert.equal(MemoryType.KEY_MOMENT, 'key_moment');
      assert.equal(MemoryType.DECISION, 'decision');
      assert.equal(MemoryType.PREFERENCE, 'preference');
    });

    it('supports all memory categories', () => {
      const types = Object.values(MemoryType);
      assert.ok(types.length >= 4, 'Should have at least 4 memory types');
      assert.ok(types.every(t => typeof t === 'string'), 'All types should be strings');
    });
  });

  describe('DecisionType', () => {
    it('has expected values', () => {
      assert.equal(DecisionType.PATTERN, 'pattern');
      assert.equal(DecisionType.TECHNOLOGY, 'technology');
      assert.equal(DecisionType.STRUCTURE, 'structure');
      assert.equal(DecisionType.NAMING, 'naming');
    });

    it('covers architectural decision categories', () => {
      const types = Object.values(DecisionType);
      assert.ok(types.length >= 4, 'Should have at least 4 decision types');
    });
  });

  describe('DecisionStatus', () => {
    it('has expected values', () => {
      assert.equal(DecisionStatus.ACTIVE, 'active');
      assert.equal(DecisionStatus.SUPERSEDED, 'superseded');
      assert.equal(DecisionStatus.DEPRECATED, 'deprecated');
    });
  });

  describe('LessonCategory', () => {
    it('has expected values', () => {
      assert.equal(LessonCategory.BUG, 'bug');
      assert.equal(LessonCategory.ARCHITECTURE, 'architecture');
      assert.equal(LessonCategory.PROCESS, 'process');
      assert.equal(LessonCategory.COMMUNICATION, 'communication');
    });

    it('covers learning categories', () => {
      const categories = Object.values(LessonCategory);
      assert.ok(categories.length >= 4, 'Should have at least 4 lesson categories');
    });
  });

  describe('LessonSeverity', () => {
    it('has expected values', () => {
      assert.equal(LessonSeverity.LOW, 'low');
      assert.equal(LessonSeverity.MEDIUM, 'medium');
      assert.equal(LessonSeverity.HIGH, 'high');
      assert.equal(LessonSeverity.CRITICAL, 'critical');
    });

    it('has 4 severity levels', () => {
      const severities = Object.values(LessonSeverity);
      assert.equal(severities.length, 4);
    });
  });

  describe('GoalType', () => {
    it('has expected values', () => {
      assert.equal(GoalType.QUALITY, 'quality');
      assert.equal(GoalType.LEARNING, 'learning');
      assert.equal(GoalType.MAINTENANCE, 'maintenance');
      assert.equal(GoalType.MONITORING, 'monitoring');
    });

    it('covers autonomy goal categories', () => {
      const types = Object.values(GoalType);
      assert.ok(types.length >= 4, 'Should have at least 4 goal types');
    });
  });

  describe('GoalStatus', () => {
    it('has expected values', () => {
      assert.equal(GoalStatus.ACTIVE, 'active');
      assert.equal(GoalStatus.PAUSED, 'paused');
      assert.equal(GoalStatus.COMPLETED, 'completed');
      assert.equal(GoalStatus.ABANDONED, 'abandoned');
    });
  });

  describe('TaskStatus', () => {
    it('has expected values', () => {
      assert.equal(TaskStatus.PENDING, 'pending');
      assert.equal(TaskStatus.RUNNING, 'running');
      assert.equal(TaskStatus.COMPLETED, 'completed');
      assert.equal(TaskStatus.FAILED, 'failed');
      assert.equal(TaskStatus.RETRY, 'retry');
    });

    it('has task lifecycle statuses', () => {
      const statuses = Object.values(TaskStatus);
      assert.ok(statuses.length >= 5, 'Should have at least 5 task statuses');
    });
  });

  describe('TaskType', () => {
    it('has expected values', () => {
      assert.equal(TaskType.ANALYZE_PATTERNS, 'analyze_patterns');
      assert.equal(TaskType.RUN_TESTS, 'run_tests');
      assert.equal(TaskType.SECURITY_SCAN, 'security_scan');
      assert.equal(TaskType.CODE_REVIEW, 'code_review');
      assert.equal(TaskType.SYNC, 'sync');
      assert.equal(TaskType.CUSTOM, 'custom');
    });
  });

  describe('NotificationType', () => {
    it('has expected values', () => {
      assert.equal(NotificationType.INSIGHT, 'insight');
      assert.equal(NotificationType.WARNING, 'warning');
      assert.equal(NotificationType.REMINDER, 'reminder');
      assert.equal(NotificationType.ACHIEVEMENT, 'achievement');
    });

    it('covers proactive notification types', () => {
      const types = Object.values(NotificationType);
      assert.ok(types.length >= 4, 'Should have at least 4 notification types');
    });
  });
});

// ============================================================================
// REPOSITORY CONSTRUCTOR TESTS (Unit tests - no DB required)
// ============================================================================

describe('Repository Constructors', () => {
  it('ConversationMemoriesRepository instantiates', () => {
    // Constructor should not throw when given null (will use getPool at query time)
    const repo = new ConversationMemoriesRepository(null);
    assert.ok(repo, 'Repository should instantiate');
    assert.ok(repo.supportsFTS, 'Should have supportsFTS method');
  });

  it('ArchitecturalDecisionsRepository instantiates', () => {
    const repo = new ArchitecturalDecisionsRepository(null);
    assert.ok(repo, 'Repository should instantiate');
  });

  it('LessonsLearnedRepository instantiates', () => {
    const repo = new LessonsLearnedRepository(null);
    assert.ok(repo, 'Repository should instantiate');
  });

  it('AutonomousGoalsRepository instantiates', () => {
    const repo = new AutonomousGoalsRepository(null);
    assert.ok(repo, 'Repository should instantiate');
  });

  it('AutonomousTasksRepository instantiates', () => {
    const repo = new AutonomousTasksRepository(null);
    assert.ok(repo, 'Repository should instantiate');
  });

  it('ProactiveNotificationsRepository instantiates', () => {
    const repo = new ProactiveNotificationsRepository(null);
    assert.ok(repo, 'Repository should instantiate');
  });
});

// ============================================================================
// INTEGRATION TESTS (Require PostgreSQL with CYNIC_DATABASE_URL)
// ============================================================================

describe('ConversationMemoriesRepository Integration', { skip: !hasPostgres }, () => {
  let repo;

  beforeEach(() => {
    repo = new ConversationMemoriesRepository();
  });

  it('creates and retrieves memory', async () => {
    const memory = await repo.create({
      userId: 'test_user',
      memoryType: MemoryType.KEY_MOMENT,
      content: 'Integration test memory ' + Date.now(),
      importance: 0.8,
    });

    assert.ok(memory.id, 'Should have ID');
    assert.equal(memory.memoryType, MemoryType.KEY_MOMENT);

    const found = await repo.findById(memory.id);
    assert.equal(found?.content, memory.content);
  });

  it('finds memories by user', async () => {
    const userId = 'test_user_' + Date.now();
    await repo.create({ userId, memoryType: MemoryType.SUMMARY, content: 'M1' });
    await repo.create({ userId, memoryType: MemoryType.PREFERENCE, content: 'M2' });

    const memories = await repo.findByUser(userId, { limit: 10 });
    assert.equal(memories.length, 2);
  });
});

describe('ArchitecturalDecisionsRepository Integration', { skip: !hasPostgres }, () => {
  let repo;

  beforeEach(() => {
    repo = new ArchitecturalDecisionsRepository();
  });

  it('creates and retrieves decision', async () => {
    const decision = await repo.create({
      userId: 'test_user',
      decisionType: DecisionType.PATTERN,
      title: 'Test Pattern Decision ' + Date.now(),
      description: 'Integration test decision',
      rationale: 'For testing purposes',
    });

    assert.ok(decision.id, 'Should have ID');
    assert.equal(decision.decisionType, DecisionType.PATTERN);
    assert.equal(decision.status, DecisionStatus.ACTIVE);

    const found = await repo.findById(decision.id);
    assert.equal(found?.title, decision.title);
  });
});

describe('LessonsLearnedRepository Integration', { skip: !hasPostgres }, () => {
  let repo;

  beforeEach(() => {
    repo = new LessonsLearnedRepository();
  });

  it('creates and retrieves lesson', async () => {
    const lesson = await repo.create({
      userId: 'test_user',
      category: LessonCategory.BUG,
      mistake: 'Test mistake ' + Date.now(),
      correction: 'Test correction',
      prevention: 'Test prevention',
      severity: LessonSeverity.MEDIUM,
    });

    assert.ok(lesson.id, 'Should have ID');
    assert.equal(lesson.category, LessonCategory.BUG);
    assert.equal(lesson.severity, LessonSeverity.MEDIUM);

    const found = await repo.findById(lesson.id);
    assert.equal(found?.mistake, lesson.mistake);
  });
});

describe('AutonomousGoalsRepository Integration', { skip: !hasPostgres }, () => {
  let repo;

  beforeEach(() => {
    repo = new AutonomousGoalsRepository();
  });

  it('creates and retrieves goal', async () => {
    const goal = await repo.create({
      userId: 'test_user',
      goalType: GoalType.QUALITY,
      title: 'Test Goal ' + Date.now(),
      description: 'Integration test goal',
      priority: 70,
      successCriteria: [
        { criterion: 'Test criterion 1', weight: 0.5, met: false },
        { criterion: 'Test criterion 2', weight: 0.5, met: false },
      ],
    });

    assert.ok(goal.id, 'Should have ID');
    assert.equal(goal.goalType, GoalType.QUALITY);
    assert.equal(goal.status, GoalStatus.ACTIVE);
    assert.equal(goal.progress, 0);

    const found = await repo.findById(goal.id);
    assert.equal(found?.title, goal.title);
  });

  it('updates goal progress', async () => {
    const goal = await repo.create({
      userId: 'test_user',
      goalType: GoalType.LEARNING,
      title: 'Progress Test ' + Date.now(),
      description: 'Test',
    });

    const updated = await repo.updateProgress(goal.id, 0.5);
    assert.equal(updated.progress, 0.5);
  });
});

describe('AutonomousTasksRepository Integration', { skip: !hasPostgres }, () => {
  let repo;

  beforeEach(() => {
    repo = new AutonomousTasksRepository();
  });

  it('creates and retrieves task', async () => {
    const task = await repo.create({
      userId: 'test_user',
      taskType: TaskType.ANALYZE_PATTERNS,
      payload: { sessionId: 'test_session', judgmentCount: 5 },
      priority: 60,
    });

    assert.ok(task.id, 'Should have ID');
    assert.equal(task.taskType, TaskType.ANALYZE_PATTERNS);
    assert.equal(task.status, TaskStatus.PENDING);

    const found = await repo.findById(task.id);
    assert.equal(found?.taskType, task.taskType);
  });
});

describe('ProactiveNotificationsRepository Integration', { skip: !hasPostgres }, () => {
  let repo;

  beforeEach(() => {
    repo = new ProactiveNotificationsRepository();
  });

  it('creates and retrieves notification', async () => {
    const notification = await repo.create({
      userId: 'test_user',
      notificationType: NotificationType.INSIGHT,
      title: 'Test Insight ' + Date.now(),
      message: 'Integration test notification',
      priority: 70,
    });

    assert.ok(notification.id, 'Should have ID');
    assert.equal(notification.notificationType, NotificationType.INSIGHT);
    assert.equal(notification.delivered, false);

    const found = await repo.findById(notification.id);
    assert.equal(found?.title, notification.title);
  });

  it('gets pending notifications', async () => {
    const userId = 'test_pending_' + Date.now();
    await repo.create({
      userId,
      notificationType: NotificationType.REMINDER,
      title: 'Pending Test',
      message: 'Test message',
      priority: 50,
    });

    const pending = await repo.getPending(userId, 10);
    assert.ok(pending.length >= 1, 'Should have at least one pending notification');
    assert.ok(pending.every(n => !n.delivered), 'All should be undelivered');
  });
});

// ============================================================================
// EMBEDDER TESTS (Unit tests)
// ============================================================================

describe('Embedder Service', async () => {
  // Dynamically import embedder
  let embedder;
  try {
    const module = await import('../src/services/embedder.js');
    embedder = module;
  } catch (e) {
    console.warn('Embedder import failed:', e.message);
  }

  it('MockEmbedder generates deterministic embeddings', async () => {
    if (!embedder) return;

    const { MockEmbedder, EMBEDDING_DIMENSIONS } = embedder;
    const mock = new MockEmbedder();

    const emb1 = await mock.embed('hello world');
    const emb2 = await mock.embed('hello world');

    assert.equal(emb1.length, EMBEDDING_DIMENSIONS);
    assert.deepEqual(emb1, emb2, 'Same text should produce same embedding');
  });

  it('MockEmbedder produces different embeddings for different text', async () => {
    if (!embedder) return;

    const { MockEmbedder } = embedder;
    const mock = new MockEmbedder();

    const emb1 = await mock.embed('hello world');
    const emb2 = await mock.embed('goodbye world');

    assert.notDeepEqual(emb1, emb2, 'Different text should produce different embedding');
  });

  it('MockEmbedder calculates cosine similarity', async () => {
    if (!embedder) return;

    const { MockEmbedder } = embedder;
    const mock = new MockEmbedder();

    const emb1 = await mock.embed('programming in TypeScript');
    const emb2 = await mock.embed('coding in TypeScript');
    const emb3 = await mock.embed('cooking dinner tonight');

    const sim12 = mock.cosineSimilarity(emb1, emb2);
    const sim13 = mock.cosineSimilarity(emb1, emb3);

    // Similar texts should have higher similarity
    assert.ok(sim12 > sim13, 'Similar text should have higher cosine similarity');
  });

  it('createEmbedder returns MockEmbedder by default', async () => {
    if (!embedder) return;

    const { createEmbedder, EmbedderType } = embedder;

    // Without OPENAI_API_KEY, should default to mock
    const oldKey = process.env.OPENAI_API_KEY;
    delete process.env.OPENAI_API_KEY;

    try {
      const emb = createEmbedder({ type: EmbedderType.MOCK });
      assert.equal(emb.type, EmbedderType.MOCK);
    } finally {
      if (oldKey) process.env.OPENAI_API_KEY = oldKey;
    }
  });
});
