#!/usr/bin/env node
/**
 * Memory Retriever Service Tests
 *
 * Tests for Phase 16: Total Memory service.
 * Unit tests for MemoryRetriever with mock repositories.
 *
 * "φ remembers everything" - CYNIC
 *
 * @module @cynic/persistence/test/memory-retriever
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  MemoryRetriever,
  createMemoryRetriever,
  MemoryType,
  DecisionType,
  LessonCategory,
  LessonSeverity,
} from '../src/services/memory-retriever.js';

// ============================================================================
// MOCK FACTORIES
// ============================================================================

function createMockPool() {
  return {
    query: async (sql) => ({ rows: [] }),
  };
}

function createMockMemoriesRepo() {
  const memories = [];
  return {
    create: async (data) => {
      const mem = { id: `mem-${memories.length + 1}`, ...data, createdAt: new Date() };
      memories.push(mem);
      return mem;
    },
    findById: async (id) => memories.find(m => m.id === id),
    findByUser: async (userId, opts = {}) => memories.filter(m => m.userId === userId).slice(0, opts.limit || 10),
    findImportant: async (userId, minImportance, limit) =>
      memories.filter(m => m.userId === userId && (m.importance || 0) >= minImportance).slice(0, limit),
    search: async (userId, query, opts = {}) =>
      memories.filter(m => m.userId === userId && m.content.includes(query)).slice(0, opts.limit || 10),
    recordAccess: async (ids) => ids.length,
    getStats: async () => ({ total: memories.length }),
    _memories: memories,
  };
}

function createMockDecisionsRepo() {
  const decisions = [];
  return {
    create: async (data) => {
      const dec = { id: `dec-${decisions.length + 1}`, ...data, createdAt: new Date() };
      decisions.push(dec);
      return dec;
    },
    findById: async (id) => decisions.find(d => d.id === id),
    findByProject: async (userId, projectPath, opts = {}) =>
      decisions.filter(d => d.userId === userId && d.projectPath === projectPath).slice(0, opts.limit || 10),
    search: async (userId, query, opts = {}) =>
      decisions.filter(d => d.userId === userId && (d.title.includes(query) || d.description.includes(query))).slice(0, opts.limit || 10),
    getStats: async () => ({ total: decisions.length }),
    _decisions: decisions,
  };
}

function createMockLessonsRepo() {
  const lessons = [];
  return {
    create: async (data) => {
      const les = { id: `les-${lessons.length + 1}`, ...data, createdAt: new Date(), occurrenceCount: 1 };
      lessons.push(les);
      return les;
    },
    findById: async (id) => lessons.find(l => l.id === id),
    findCritical: async (userId, limit) =>
      lessons.filter(l => l.userId === userId && l.severity === 'critical').slice(0, limit),
    search: async (userId, query, opts = {}) => {
      const filtered = lessons.filter(l =>
        l.userId === userId &&
        (l.mistake.includes(query) || l.correction.includes(query))
      );
      return filtered.map(l => ({ ...l, combinedScore: 0.6 })).slice(0, opts.limit || 10);
    },
    recordOccurrence: async (id) => {
      const lesson = lessons.find(l => l.id === id);
      if (lesson) lesson.occurrenceCount++;
      return lesson;
    },
    getStats: async () => ({ total: lessons.length }),
    _lessons: lessons,
  };
}

function createMockEmbedder() {
  return {
    embed: async (text) => {
      // Simple deterministic embedding based on text hash
      const embedding = new Array(1536).fill(0).map((_, i) =>
        Math.sin(text.charCodeAt(i % text.length) + i) * 0.5
      );
      return embedding;
    },
  };
}

// ============================================================================
// CONSTRUCTOR TESTS
// ============================================================================

describe('MemoryRetriever Constructor', () => {
  it('requires a pool', () => {
    assert.throws(
      () => new MemoryRetriever({}),
      /requires a database pool/
    );
  });

  it('creates with pool only', () => {
    const pool = createMockPool();
    const retriever = new MemoryRetriever({ pool });

    assert.ok(retriever);
    assert.equal(retriever.embedder, null);
    assert.ok(retriever.memories);
    assert.ok(retriever.decisions);
    assert.ok(retriever.lessons);
  });

  it('creates with pool and embedder', () => {
    const pool = createMockPool();
    const embedder = createMockEmbedder();
    const retriever = new MemoryRetriever({ pool, embedder });

    assert.ok(retriever);
    assert.equal(retriever.embedder, embedder);
  });

  it('has φ constants', () => {
    const pool = createMockPool();
    const retriever = new MemoryRetriever({ pool });

    assert.equal(retriever.PHI_FTS, 0.381966011250105);
    assert.equal(retriever.PHI_VECTOR, 0.618033988749895);
    assert.equal(retriever.MIN_RELEVANCE, 0.236067977499790);
  });
});

describe('createMemoryRetriever factory', () => {
  it('creates MemoryRetriever instance', () => {
    const pool = createMockPool();
    const retriever = createMemoryRetriever({ pool });

    assert.ok(retriever instanceof MemoryRetriever);
  });
});

// ============================================================================
// MEMORY OPERATIONS TESTS (with mocked repos)
// ============================================================================

describe('MemoryRetriever with mock repositories', () => {
  let retriever;
  let mockMemories;
  let mockDecisions;
  let mockLessons;
  let mockEmbedder;

  beforeEach(() => {
    const pool = createMockPool();
    mockMemories = createMockMemoriesRepo();
    mockDecisions = createMockDecisionsRepo();
    mockLessons = createMockLessonsRepo();
    mockEmbedder = createMockEmbedder();

    retriever = new MemoryRetriever({ pool, embedder: mockEmbedder });

    // Replace real repositories with mocks
    retriever.memories = mockMemories;
    retriever.decisions = mockDecisions;
    retriever.lessons = mockLessons;
  });

  describe('rememberConversation', () => {
    it('stores a conversation memory', async () => {
      const memory = await retriever.rememberConversation('user1', MemoryType.KEY_MOMENT, 'Test content', {
        sessionId: 'session1',
        importance: 0.8,
        context: { topic: 'testing' },
      });

      assert.ok(memory);
      assert.equal(memory.userId, 'user1');
      assert.equal(memory.memoryType, MemoryType.KEY_MOMENT);
      assert.equal(memory.content, 'Test content');
      assert.equal(memory.importance, 0.8);
      assert.ok(memory.embedding); // Should have embedding from mock embedder
    });

    it('defaults importance to 0.5', async () => {
      const memory = await retriever.rememberConversation('user1', MemoryType.SUMMARY, 'Summary content');

      assert.equal(memory.importance, 0.5);
    });
  });

  describe('rememberDecision', () => {
    it('stores an architectural decision', async () => {
      const decision = await retriever.rememberDecision('user1', {
        projectPath: '/test/project',
        decisionType: DecisionType.PATTERN,
        title: 'Use Repository Pattern',
        description: 'Data access layer abstraction',
        rationale: 'Better testability',
      });

      assert.ok(decision);
      assert.equal(decision.userId, 'user1');
      assert.equal(decision.decisionType, DecisionType.PATTERN);
      assert.equal(decision.title, 'Use Repository Pattern');
    });
  });

  describe('rememberLesson', () => {
    it('stores a lesson learned', async () => {
      const lesson = await retriever.rememberLesson('user1', {
        category: LessonCategory.BUG,
        mistake: 'Forgot null check',
        correction: 'Always validate input',
        prevention: 'Add input validation utility',
        severity: LessonSeverity.HIGH,
      });

      assert.ok(lesson);
      assert.equal(lesson.userId, 'user1');
      assert.equal(lesson.category, LessonCategory.BUG);
      assert.equal(lesson.severity, LessonSeverity.HIGH);
    });

    it('defaults severity to medium', async () => {
      const lesson = await retriever.rememberLesson('user1', {
        mistake: 'Simple mistake',
        correction: 'Simple fix',
      });

      assert.equal(lesson.severity, 'medium');
    });
  });

  describe('search', () => {
    it('searches all sources by default', async () => {
      // Seed some data
      await retriever.rememberConversation('user1', MemoryType.SUMMARY, 'Authentication flow');
      await retriever.rememberDecision('user1', {
        title: 'JWT Authentication',
        description: 'Use JWT for auth',
      });
      await retriever.rememberLesson('user1', {
        mistake: 'Authentication token exposed',
        correction: 'Store in httpOnly cookie',
      });

      const results = await retriever.search('user1', 'Authentication');

      assert.ok(results);
      assert.equal(results.query, 'Authentication');
      assert.ok(results.timestamp);
      assert.ok(results.sources);
      assert.ok(Array.isArray(results.sources.memories));
      assert.ok(Array.isArray(results.sources.decisions));
      assert.ok(Array.isArray(results.sources.lessons));
    });

    it('searches specific sources', async () => {
      await retriever.rememberConversation('user1', MemoryType.KEY_MOMENT, 'Test memory');

      const results = await retriever.search('user1', 'Test', {
        sources: ['memories'],
      });

      assert.ok(results.sources.memories);
      assert.equal(results.sources.decisions, undefined);
      assert.equal(results.sources.lessons, undefined);
    });

    it('respects limit option', async () => {
      for (let i = 0; i < 5; i++) {
        await retriever.rememberConversation('user1', MemoryType.SUMMARY, `Memory ${i}`);
      }

      const results = await retriever.search('user1', 'Memory', {
        sources: ['memories'],
        limit: 3,
      });

      assert.ok(results.sources.memories.length <= 3);
    });
  });

  describe('getRelevantContext', () => {
    it('retrieves context for current task', async () => {
      // Seed data
      await retriever.rememberConversation('user1', MemoryType.KEY_MOMENT, 'Important testing insight', {
        importance: 0.9,
      });
      await retriever.rememberDecision('user1', {
        projectPath: '/test/project',
        title: 'Test Decision',
        description: 'Testing strategy',
      });
      await retriever.rememberLesson('user1', {
        category: LessonCategory.BUG,
        mistake: 'Critical testing bug',
        correction: 'Always test edge cases',
        severity: LessonSeverity.CRITICAL,
      });

      const context = await retriever.getRelevantContext('user1', {
        projectPath: '/test/project',
        currentTask: 'Write tests',
        recentTopics: ['testing', 'coverage'],
      });

      assert.ok(context);
      assert.ok(Array.isArray(context.memories));
      assert.ok(Array.isArray(context.decisions));
      assert.ok(Array.isArray(context.lessons));
      assert.ok(Array.isArray(context.criticalLessons));
      assert.ok(Array.isArray(context.importantMemories));
      assert.ok(context.timestamp);
    });

    it('returns empty arrays without query', async () => {
      const context = await retriever.getRelevantContext('user1', {});

      assert.ok(context);
      assert.deepEqual(context.memories, []);
    });
  });

  describe('checkForMistakes', () => {
    it('returns no warning when no similar lessons', async () => {
      const result = await retriever.checkForMistakes('user1', 'new action');

      assert.equal(result.warning, false);
    });

    it('returns warning when similar lesson found', async () => {
      await retriever.rememberLesson('user1', {
        category: LessonCategory.BUG,
        mistake: 'Forgot null check on user input',
        correction: 'Add validation',
        severity: LessonSeverity.HIGH,
      });

      const result = await retriever.checkForMistakes('user1', 'user input');

      assert.equal(result.warning, true);
      assert.ok(result.lessons);
      assert.ok(result.message);
      assert.ok(result.message.includes('GROWL'));
    });
  });

  describe('getStats', () => {
    it('returns statistics for all memory types', async () => {
      await retriever.rememberConversation('user1', MemoryType.SUMMARY, 'Test');
      await retriever.rememberDecision('user1', { title: 'Test', description: 'Test' });
      await retriever.rememberLesson('user1', { mistake: 'Test', correction: 'Test' });

      const stats = await retriever.getStats();

      assert.ok(stats);
      assert.ok(stats.memories);
      assert.ok(stats.decisions);
      assert.ok(stats.lessons);
      assert.ok(stats.totals);
      assert.equal(stats.totals.combined, 3);
      assert.equal(stats.hasEmbedder, true);
      assert.ok(stats.phiConstants);
      assert.equal(stats.phiConstants.ftsWeight, 0.381966011250105);
    });
  });
});

// ============================================================================
// φ CONSTANTS TESTS
// ============================================================================

describe('φ Constants Verification', () => {
  it('FTS + Vector weights sum to 1', () => {
    const pool = createMockPool();
    const retriever = new MemoryRetriever({ pool });

    const sum = retriever.PHI_FTS + retriever.PHI_VECTOR;
    assert.ok(Math.abs(sum - 1) < 0.0001, `FTS + Vector should equal 1, got ${sum}`);
  });

  it('weights follow golden ratio', () => {
    const pool = createMockPool();
    const retriever = new MemoryRetriever({ pool });

    // φ⁻¹ ≈ 0.618
    // φ⁻² ≈ 0.382
    assert.ok(Math.abs(retriever.PHI_VECTOR - 0.618) < 0.001);
    assert.ok(Math.abs(retriever.PHI_FTS - 0.382) < 0.001);
  });

  it('MIN_RELEVANCE is φ⁻³', () => {
    const pool = createMockPool();
    const retriever = new MemoryRetriever({ pool });

    // φ⁻³ ≈ 0.236
    assert.ok(Math.abs(retriever.MIN_RELEVANCE - 0.236) < 0.001);
  });
});
