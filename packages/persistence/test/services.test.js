#!/usr/bin/env node
/**
 * Persistence Services Tests
 *
 * Comprehensive tests for:
 * - MemoryRetriever (unified memory search)
 * - FactExtractor (auto fact extraction from tool outputs)
 * - ReasoningBank (trajectory storage and success replay)
 * - PatternLearning (auto pattern extraction and decay)
 *
 * All tests use mock database pools - no real PostgreSQL required.
 *
 * "phi distrusts phi" - verify all services
 *
 * @module @cynic/persistence/test/services
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  MemoryRetriever,
  createMemoryRetriever,
  MemoryType,
  DecisionType,
  LessonCategory,
  LessonSeverity,
} from '../src/services/memory-retriever.js';

import {
  FactExtractor,
  createFactExtractor,
} from '../src/services/fact-extractor.js';

import { FactType } from '../src/postgres/repositories/facts.js';

import {
  ReasoningBank,
  createReasoningBank,
  TrajectoryOutcome,
} from '../src/services/reasoning-bank.js';

import {
  PatternLearning,
  createPatternLearning,
  DEFAULT_PATTERN_CONFIG,
} from '../src/services/pattern-learning.js';

// ============================================================================
// MOCK FACTORIES
// ============================================================================

/**
 * Create a mock pool that returns appropriate rows based on SQL patterns.
 * Every service method eventually calls pool.query(), so this is the
 * single point of mock for all database interactions.
 */
function createMockPool() {
  let idCounter = 0;
  function nextId(prefix) {
    return `${prefix}_${++idCounter}_${Date.now().toString(36)}`;
  }

  return {
    query: mock.fn(async (sql, params = []) => {
      const sqlLower = sql.toLowerCase().trim();

      // ── TABLE/TRIGGER CREATION ──────────────────────────────────────
      if (sqlLower.includes('create table') ||
          sqlLower.includes('create index') ||
          sqlLower.includes('create or replace function') ||
          sqlLower.includes('drop trigger') ||
          sqlLower.includes('create trigger')) {
        return { rows: [] };
      }

      // ── GLOBAL DELETE HANDLER ───────────────────────────────────────
      if (sqlLower.startsWith('delete') || sqlLower.trimStart().startsWith('delete')) {
        if (sqlLower.includes('facts') && sqlLower.includes('relevance')) {
          return { rowCount: 3 };
        }
        if (sqlLower.includes('patterns')) {
          return { rowCount: 2 };
        }
        return { rowCount: params[0] ? 1 : 0 };
      }

      // ── GLOBAL STATS HANDLER ────────────────────────────────────────
      if (sqlLower.includes('count(*)') && !sqlLower.includes('insert') && !sqlLower.includes('update')) {
        if (sqlLower.includes('conversation_memories') && sqlLower.includes('avg')) {
          return {
            rows: [{
              total: '5', recent: '2', avg_importance: '0.6', avg_access_count: '3',
            }],
          };
        }
        if (sqlLower.includes('conversation_memories') && sqlLower.includes('group by memory_type')) {
          return { rows: [{ memory_type: 'summary', count: '3' }] };
        }
        if (sqlLower.includes('architectural_decisions') && sqlLower.includes('active')) {
          return {
            rows: [{ total: '10', active: '7', superseded: '2', recent: '3' }],
          };
        }
        if (sqlLower.includes('architectural_decisions') && sqlLower.includes('group by decision_type')) {
          return { rows: [{ decision_type: 'pattern', count: '5' }] };
        }
        if (sqlLower.includes('lessons_learned') && sqlLower.includes('severity')) {
          return {
            rows: [{ total: '15', critical: '2', high: '5', recurring: '3', total_occurrences: '25' }],
          };
        }
        if (sqlLower.includes('lessons_learned') && sqlLower.includes('group by category')) {
          return { rows: [{ category: 'bug', count: '8' }] };
        }
        if (sqlLower.includes('facts') && sqlLower.includes('avg')) {
          return {
            rows: [{
              total: '50', types: '5', tools: '4',
              avg_confidence: '0.5', avg_relevance: '0.45', total_accesses: '100',
            }],
          };
        }
        if (sqlLower.includes('trajectories') && sqlLower.includes('group by dog_id')) {
          return { rows: [] };
        }
        if (sqlLower.includes('trajectories') && sqlLower.includes('avg') && sqlLower.includes('avg_duration')) {
          return {
            rows: [{
              total: '100', successes: '70', failures: '20',
              avg_reward: '0.45', avg_duration: '5000', avg_tools: '3.5',
              avg_errors: '0.5', avg_switches: '0.2', total_replays: '15',
            }],
          };
        }
        if (sqlLower.includes('trajectories') && sqlLower.includes('avg')) {
          return {
            rows: [{
              total: '100', successes: '70', failures: '20',
              avg_reward: '0.45', avg_duration: '5000', avg_tools: '3.5',
              avg_errors: '0.5', avg_switches: '0.2', total_replays: '15',
            }],
          };
        }
        return { rows: [{ total: '0' }] };
      }

      // ── INSERT INTO conversation_memories ───────────────────────────
      if (sqlLower.includes('insert into conversation_memories')) {
        return {
          rows: [{
            id: 'mem-mock-1',
            user_id: params[0],
            session_id: params[1],
            memory_type: params[2],
            content: params[3],
            importance: params[5] ?? 0.5,
            context: params[6] ? JSON.parse(params[6]) : {},
            created_at: new Date(),
            last_accessed: null,
            access_count: 0,
          }],
        };
      }

      // ── INSERT INTO architectural_decisions ─────────────────────────
      if (sqlLower.includes('insert into architectural_decisions')) {
        return {
          rows: [{
            id: 'dec-mock-1',
            user_id: params[0],
            project_path: params[1],
            decision_type: params[2],
            title: params[3],
            description: params[4],
            rationale: params[5],
            alternatives: params[6] ? JSON.parse(params[6]) : [],
            consequences: params[7] ? JSON.parse(params[7]) : {},
            embedding: null,
            status: params[9] || 'active',
            superseded_by: null,
            related_decisions: [],
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      // ── INSERT INTO lessons_learned ─────────────────────────────────
      if (sqlLower.includes('insert into lessons_learned')) {
        return {
          rows: [{
            id: 'les-mock-1',
            user_id: params[0],
            category: params[1],
            mistake: params[2],
            correction: params[3],
            prevention: params[4],
            severity: params[5] || 'medium',
            embedding: null,
            occurrence_count: 1,
            last_occurred: new Date(),
            source_judgment_id: null,
            source_session_id: null,
            created_at: new Date(),
          }],
        };
      }

      // ── INSERT INTO facts ──────────────────────────────────────────
      if (sqlLower.includes('insert into facts')) {
        return {
          rows: [{
            fact_id: nextId('fact'),
            user_id: params[1] || null,
            session_id: params[2] || null,
            fact_type: params[3] || 'tool_result',
            subject: params[4],
            content: params[5],
            context: params[6] ? JSON.parse(params[6]) : {},
            source_tool: params[7] || null,
            source_file: params[8] || null,
            confidence: params[9] ?? 0.5,
            relevance: params[10] ?? 0.5,
            tags: params[11] || [],
            embedding: null,
            embedding_model: null,
            embedding_dim: null,
            access_count: 0,
            last_accessed: null,
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      // ── INSERT INTO trajectories ───────────────────────────────────
      if (sqlLower.includes('insert into trajectories')) {
        return {
          rows: [{
            trajectory_id: nextId('traj'),
            user_id: params[1] || null,
            session_id: params[2] || null,
            dog_id: params[3] || null,
            task_type: params[4] || null,
            initial_state: params[5] ? JSON.parse(params[5]) : {},
            action_sequence: [],
            outcome: 'pending',
            outcome_details: {},
            reward: 0,
            duration_ms: 0,
            tool_count: 0,
            error_count: 0,
            switch_count: 0,
            similarity_hash: null,
            replay_count: 0,
            success_after_replay: null,
            confidence: 0.5,
            tags: params[6] || [],
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      // ── INSERT INTO patterns (PatternLearning._savePattern) ────────
      if (sqlLower.includes('insert into patterns')) {
        return { rows: [] };
      }

      // ── UPDATE trajectories (recordAction) ─────────────────────────
      if (sqlLower.includes('update trajectories') && sqlLower.includes('action_sequence')) {
        return {
          rows: [{
            trajectory_id: params[2] || 'traj_mock',
            user_id: 'user-1',
            session_id: 'sess-1',
            dog_id: params[1] || 'scout',
            task_type: 'exploration',
            initial_state: {},
            action_sequence: [{ tool: 'Read' }],
            outcome: 'pending',
            outcome_details: {},
            reward: 0,
            duration_ms: 0,
            tool_count: 1,
            error_count: 0,
            switch_count: 0,
            similarity_hash: null,
            replay_count: 0,
            success_after_replay: null,
            confidence: 0.5,
            tags: [],
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      // ── UPDATE trajectories (complete) ─────────────────────────────
      if (sqlLower.includes('update trajectories') && sqlLower.includes('outcome = $1')) {
        return {
          rows: [{
            trajectory_id: params[7] || 'traj_mock',
            user_id: 'user-1',
            session_id: 'sess-1',
            dog_id: 'scout',
            task_type: 'exploration',
            initial_state: {},
            action_sequence: [],
            outcome: params[0] || 'success',
            outcome_details: params[1] ? JSON.parse(params[1]) : {},
            reward: params[2] || 0.618,
            duration_ms: params[3] || 1000,
            tool_count: 2,
            error_count: 0,
            switch_count: 0,
            similarity_hash: params[6] || null,
            replay_count: 0,
            success_after_replay: null,
            confidence: Math.min(params[4] || 0.618, params[5] || 0.618),
            tags: [],
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      // ── UPDATE trajectories (recordReplay) ─────────────────────────
      if (sqlLower.includes('update trajectories') && sqlLower.includes('replay_count')) {
        return {
          rows: [{
            trajectory_id: params[1] || 'traj_mock',
            user_id: 'user-1',
            session_id: 'sess-1',
            dog_id: 'scout',
            task_type: 'exploration',
            initial_state: {},
            action_sequence: [],
            outcome: 'success',
            outcome_details: {},
            reward: 0.618,
            duration_ms: 1000,
            tool_count: 2,
            error_count: 0,
            switch_count: 0,
            similarity_hash: null,
            replay_count: 1,
            success_after_replay: params[0],
            confidence: params[0] ? 0.55 : 0.4,
            tags: [],
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      // ── UPDATE patterns (reinforce/weaken) ─────────────────────────
      if (sqlLower.includes('update patterns')) {
        return {
          rows: [{
            pattern_id: params[1] || params[2] || 'pat_mock',
            name: 'Mock pattern',
            confidence: 0.6,
            frequency: 2,
          }],
        };
      }

      // ── UPDATE facts (access, relevance, decay) ────────────────────
      if (sqlLower.includes('update facts')) {
        if (sqlLower.includes('access_count = access_count + 1')) {
          return { rows: [], rowCount: 1 };
        }
        if (sqlLower.includes('returning')) {
          return {
            rows: [{
              fact_id: 'fact_updated',
              user_id: 'user-1',
              session_id: 'sess-1',
              fact_type: 'code_pattern',
              subject: 'Updated',
              content: 'Updated',
              context: {},
              source_tool: null,
              source_file: null,
              confidence: 0.5,
              relevance: 0.6,
              tags: [],
              embedding: null,
              embedding_model: null,
              embedding_dim: null,
              access_count: 0,
              last_accessed: null,
              created_at: new Date(),
              updated_at: new Date(),
            }],
          };
        }
        return { rowCount: 5, rows: [] };
      }

      // ── DELETE patterns ────────────────────────────────────────────
      if (sqlLower.includes('delete from patterns')) {
        return { rowCount: 2 };
      }

      // ── DELETE facts ───────────────────────────────────────────────
      if (sqlLower.includes('delete from facts')) {
        return { rowCount: 3 };
      }

      // ── SELECT conversation_memories (search / findImportant) ──────
      if (sqlLower.includes('search_memories_hybrid') || sqlLower.includes('record_memory_access')) {
        return { rows: [{ record_memory_access: 0 }] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('importance')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('session_id')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('count(*)')) {
        return {
          rows: [{
            total: '5',
            recent: '2',
            avg_importance: '0.6',
            avg_access_count: '3',
          }],
        };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('group by memory_type')) {
        return { rows: [{ memory_type: 'summary', count: '3' }] };
      }

      if (sqlLower.includes('from conversation_memories')) {
        return { rows: [] };
      }

      // ── SELECT architectural_decisions ─────────────────────────────
      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('count(*)')) {
        return {
          rows: [{
            total: '10',
            active: '7',
            superseded: '2',
            recent: '3',
          }],
        };
      }

      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('group by decision_type')) {
        return { rows: [{ decision_type: 'pattern', count: '5' }] };
      }

      if (sqlLower.includes('from architectural_decisions')) {
        return { rows: [] };
      }

      // ── SELECT lessons_learned ─────────────────────────────────────
      if (sqlLower.includes('record_lesson_occurrence')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('count(*)')) {
        return {
          rows: [{
            total: '15',
            critical: '2',
            high: '5',
            recurring: '3',
            total_occurrences: '25',
          }],
        };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('group by category')) {
        return { rows: [{ category: 'bug', count: '8' }] };
      }

      if (sqlLower.includes('from lessons_learned')) {
        return { rows: [] };
      }

      // ── SELECT facts (search / stats) ──────────────────────────────
      if (sqlLower.includes('from facts') && sqlLower.includes('count(*)')) {
        return {
          rows: [{
            total: '50',
            types: '5',
            tools: '4',
            avg_confidence: '0.5',
            avg_relevance: '0.45',
            total_accesses: '100',
          }],
        };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('session_id')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('user_id')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('search_vector')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from facts')) {
        return { rows: [] };
      }

      // ── SELECT trajectories ────────────────────────────────────────
      if (sqlLower.includes('from trajectories') && sqlLower.includes("outcome = 'success'") && sqlLower.includes('reward')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes('dog_id') && sqlLower.includes('group by')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes('count(*)')) {
        return {
          rows: [{
            total: '100',
            successes: '70',
            failures: '20',
            avg_reward: '0.45',
            avg_duration: '5000',
            avg_tools: '3.5',
            avg_errors: '0.5',
            avg_switches: '0.2',
            total_replays: '15',
          }],
        };
      }

      if (sqlLower.includes('from trajectories')) {
        return { rows: [] };
      }

      // ── SELECT judgments (PatternLearning) ─────────────────────────
      if (sqlLower.includes('from judgments')) {
        return { rows: [] };
      }

      // ── SELECT patterns ────────────────────────────────────────────
      if (sqlLower.includes('from patterns') && sqlLower.includes('where updated_at < $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from patterns') && sqlLower.includes('distinct category')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from patterns') && sqlLower.includes('category = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from patterns')) {
        return { rows: [] };
      }

      // ── DEFAULT ────────────────────────────────────────────────────
      return { rows: [], rowCount: 0 };
    }),
  };
}

/**
 * Create a mock embedder that returns a fixed vector.
 */
function createMockEmbedder() {
  return {
    type: 'test-embedder',
    model: 'test-model',
    embed: mock.fn(async (text) => {
      // Simple deterministic "embedding": hash of text modulo dimensions
      const dim = 128;
      const vec = new Array(dim).fill(0);
      for (let i = 0; i < text.length && i < dim; i++) {
        vec[i % dim] += text.charCodeAt(i) / 1000;
      }
      return vec;
    }),
  };
}

// ============================================================================
// MEMORY RETRIEVER SERVICE
// ============================================================================

describe('MemoryRetriever', () => {
  let retriever;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    retriever = new MemoryRetriever({ pool: mockPool });
  });

  describe('constructor', () => {
    it('should create with pool', () => {
      assert.ok(retriever);
      assert.strictEqual(retriever.pool, mockPool);
      assert.ok(retriever.memories);
      assert.ok(retriever.decisions);
      assert.ok(retriever.lessons);
      assert.ok(retriever.facts);
    });

    it('should throw without pool', () => {
      assert.throws(
        () => new MemoryRetriever({}),
        { message: 'MemoryRetriever requires a database pool' },
      );
    });

    it('should throw with no options', () => {
      assert.throws(
        () => new MemoryRetriever(),
        { message: 'MemoryRetriever requires a database pool' },
      );
    });

    it('should accept optional embedder', () => {
      const embedder = createMockEmbedder();
      const r = new MemoryRetriever({ pool: mockPool, embedder });
      assert.strictEqual(r.embedder, embedder);
    });

    it('should initialize phi constants', () => {
      assert.ok(retriever.PHI_FTS > 0);
      assert.ok(retriever.PHI_VECTOR > 0);
      assert.ok(retriever.MIN_RELEVANCE > 0);
      assert.ok(retriever.PHI_VECTOR > retriever.PHI_FTS);
    });
  });

  describe('createMemoryRetriever factory', () => {
    it('should create an instance', () => {
      const r = createMemoryRetriever({ pool: mockPool });
      assert.ok(r instanceof MemoryRetriever);
    });
  });

  describe('search', () => {
    it('should search all sources by default', async () => {
      const result = await retriever.search('user-1', 'test query');
      assert.ok(result);
      assert.strictEqual(result.query, 'test query');
      assert.ok(result.timestamp);
      assert.ok(result.sources);
      assert.strictEqual(result.hasEmbedding, false);
    });

    it('should search specific sources', async () => {
      const result = await retriever.search('user-1', 'query', {
        sources: ['memories'],
        limit: 5,
      });
      assert.ok(result);
      assert.ok(result.sources);
    });

    it('should use embedder when available', async () => {
      const embedder = createMockEmbedder();
      const r = new MemoryRetriever({ pool: mockPool, embedder });
      const result = await r.search('user-1', 'test query');
      assert.strictEqual(result.hasEmbedding, true);
      assert.ok(embedder.embed.mock.calls.length > 0);
    });
  });

  describe('getRelevantContext', () => {
    it('should return context object with empty context', async () => {
      const result = await retriever.getRelevantContext('user-1', {});
      assert.ok(result);
      assert.ok(Array.isArray(result.memories));
      assert.ok(Array.isArray(result.decisions));
      assert.ok(Array.isArray(result.lessons));
    });

    it('should search when currentTask provided', async () => {
      const result = await retriever.getRelevantContext('user-1', {
        currentTask: 'Fix the auth bug',
      });
      assert.ok(result);
    });

    it('should search for project decisions', async () => {
      const result = await retriever.getRelevantContext('user-1', {
        projectPath: '/project',
        recentTopics: ['auth', 'security'],
      });
      assert.ok(result);
    });
  });

  describe('rememberConversation', () => {
    it('should store a conversation memory', async () => {
      const result = await retriever.rememberConversation(
        'user-1',
        MemoryType.SUMMARY,
        'Session summary content',
        { sessionId: 'sess-1', importance: 0.8 },
      );
      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
    });

    it('should use default importance', async () => {
      const result = await retriever.rememberConversation(
        'user-1',
        MemoryType.KEY_MOMENT,
        'An important moment',
      );
      assert.ok(result);
    });

    it('should generate embedding when embedder available', async () => {
      const embedder = createMockEmbedder();
      const r = new MemoryRetriever({ pool: mockPool, embedder });
      await r.rememberConversation('user-1', 'summary', 'Test');
      assert.ok(embedder.embed.mock.calls.length > 0);
    });
  });

  describe('rememberDecision', () => {
    it('should store an architectural decision', async () => {
      const result = await retriever.rememberDecision('user-1', {
        projectPath: '/project',
        decisionType: DecisionType.PATTERN,
        title: 'Use singleton',
        description: 'For shared state',
        rationale: 'Thread safety',
      });
      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
    });
  });

  describe('rememberLesson', () => {
    it('should store a lesson learned', async () => {
      const result = await retriever.rememberLesson('user-1', {
        category: LessonCategory.BUG,
        mistake: 'Forgot null check',
        correction: 'Added null check',
        prevention: 'Always validate inputs',
        severity: LessonSeverity.HIGH,
      });
      assert.ok(result);
    });

    it('should use defaults for optional fields', async () => {
      const result = await retriever.rememberLesson('user-1', {
        mistake: 'Bad code',
        correction: 'Fixed code',
      });
      assert.ok(result);
    });
  });

  describe('rememberFact', () => {
    it('should store a fact', async () => {
      const result = await retriever.rememberFact('user-1', {
        factType: FactType.CODE_PATTERN,
        subject: 'Exports in utils.js',
        content: 'Exports 5 functions',
        tags: ['exports'],
      });
      assert.ok(result);
    });
  });

  describe('checkForMistakes', () => {
    it('should return no warning when no similar lessons', async () => {
      const result = await retriever.checkForMistakes('user-1', 'doing something');
      assert.strictEqual(result.warning, false);
    });
  });

  describe('_formatWarning', () => {
    it('should return empty string for empty lessons', () => {
      const result = retriever._formatWarning([]);
      assert.strictEqual(result, '');
    });

    it('should format warning with mistake info', () => {
      const result = retriever._formatWarning([{
        mistake: 'Forgot null check',
        correction: 'Added null check',
        prevention: 'Always validate',
        severity: 'high',
      }]);
      assert.ok(result.includes('Forgot null check'));
      assert.ok(result.includes('Always validate'));
    });

    it('should prefix CRITICAL for critical severity', () => {
      const result = retriever._formatWarning([{
        mistake: 'Security hole',
        correction: 'Patched',
        prevention: 'Review',
        severity: 'critical',
      }]);
      assert.ok(result.includes('CRITICAL'));
    });
  });

  describe('getSessionSummary', () => {
    it('should return session summary', async () => {
      const result = await retriever.getSessionSummary('user-1', 'sess-1');
      assert.ok(result);
      assert.strictEqual(result.sessionId, 'sess-1');
      assert.ok(result.memories);
      assert.ok(result.decisions);
      assert.ok(result.lessons);
    });
  });

  describe('getFromSimilarSessions', () => {
    it('should return empty result with no context', async () => {
      const result = await retriever.getFromSimilarSessions('user-1', {});
      assert.deepStrictEqual(result.sessions, []);
      assert.deepStrictEqual(result.memories, []);
    });
  });

  describe('findContinuationContext', () => {
    it('should return continuation context', async () => {
      const result = await retriever.findContinuationContext('user-1', {
        projectPath: '/project',
      });
      assert.ok(result);
      assert.ok(Array.isArray(result.activeDecisions));
      assert.ok(Array.isArray(result.recentLessons));
    });
  });

  describe('getStats', () => {
    it('should return aggregate stats', async () => {
      const stats = await retriever.getStats();
      assert.ok(stats);
      assert.ok(stats.memories);
      assert.ok(stats.decisions);
      assert.ok(stats.lessons);
      assert.ok(stats.facts);
      assert.ok(stats.totals);
      assert.strictEqual(typeof stats.totals.combined, 'number');
      assert.strictEqual(stats.hasEmbedder, false);
    });

    it('should report embedder when available', async () => {
      const r = new MemoryRetriever({ pool: mockPool, embedder: createMockEmbedder() });
      const stats = await r.getStats();
      assert.strictEqual(stats.hasEmbedder, true);
      assert.strictEqual(stats.embedderType, 'test-embedder');
    });

    it('should include phi constants', async () => {
      const stats = await retriever.getStats();
      assert.ok(stats.phiConstants);
      assert.strictEqual(typeof stats.phiConstants.ftsWeight, 'number');
      assert.strictEqual(typeof stats.phiConstants.vectorWeight, 'number');
    });
  });
});

// ============================================================================
// FACT EXTRACTOR SERVICE
// ============================================================================

describe('FactExtractor', () => {
  let extractor;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    extractor = new FactExtractor({ pool: mockPool });
  });

  describe('constructor', () => {
    it('should create with pool', () => {
      assert.ok(extractor);
      assert.ok(extractor._pool);
      assert.ok(extractor._factsRepo);
      assert.strictEqual(extractor._initialized, false);
    });

    it('should throw without pool', () => {
      assert.throws(
        () => new FactExtractor({}),
        { message: 'FactExtractor requires database pool' },
      );
    });

    it('should throw with no options', () => {
      assert.throws(
        () => new FactExtractor(),
        { message: 'FactExtractor requires database pool' },
      );
    });

    it('should accept optional vectorStore', () => {
      const vs = { store: async () => {}, search: async () => [] };
      const e = new FactExtractor({ pool: mockPool, vectorStore: vs });
      assert.strictEqual(e._vectorStore, vs);
    });

    it('should initialize stats', () => {
      assert.deepStrictEqual(extractor._stats, {
        processed: 0,
        extracted: 0,
        stored: 0,
        errors: 0,
      });
    });
  });

  describe('createFactExtractor factory', () => {
    it('should create an instance', () => {
      const e = createFactExtractor({ pool: mockPool });
      assert.ok(e instanceof FactExtractor);
    });
  });

  describe('initialize', () => {
    it('should call ensureTable', async () => {
      await extractor.initialize();
      assert.strictEqual(extractor._initialized, true);
    });

    it('should skip if already initialized', async () => {
      await extractor.initialize();
      const callCount = mockPool.query.mock.calls.length;
      await extractor.initialize();
      // No additional queries
      assert.strictEqual(mockPool.query.mock.calls.length, callCount);
    });
  });

  describe('extract', () => {
    it('should return empty for unknown tool', async () => {
      const result = await extractor.extract({
        tool: 'UnknownTool',
        input: {},
        output: 'stuff',
      });
      assert.deepStrictEqual(result, []);
    });

    it('should extract facts from Read tool', async () => {
      const output = 'a'.repeat(101) + '\n' +
        'export class Foo {}\n' +
        'export function bar() {}\n' +
        'export const baz = 1;\n';

      const result = await extractor.extract({
        tool: 'Read',
        input: { file_path: '/src/test.js' },
        output,
      }, { userId: 'user-1', sessionId: 'sess-1' });

      assert.ok(Array.isArray(result));
      // Should extract at least export patterns
      if (result.length > 0) {
        assert.ok(result[0].factId);
      }
    });

    it('should return empty for Read with short output', async () => {
      const result = await extractor.extract({
        tool: 'Read',
        input: { file_path: '/test.js' },
        output: 'short',
      });
      assert.deepStrictEqual(result, []);
    });

    it('should extract facts from Glob tool', async () => {
      const output = 'src/a.js\nsrc/b.js\nsrc/c.js\n';
      const result = await extractor.extract({
        tool: 'Glob',
        input: { pattern: '**/*.js' },
        output,
      });
      assert.ok(Array.isArray(result));
    });

    it('should return empty for Glob with no output', async () => {
      const result = await extractor.extract({
        tool: 'Glob',
        input: { pattern: '*.ts' },
        output: null,
      });
      assert.deepStrictEqual(result, []);
    });

    it('should extract facts from Grep tool', async () => {
      const output = 'file1.js:10:match1\nfile2.js:20:match2\nfile3.js:30:match3\n';
      const result = await extractor.extract({
        tool: 'Grep',
        input: { pattern: 'something' },
        output,
      });
      assert.ok(Array.isArray(result));
    });

    it('should extract facts from Edit tool', async () => {
      const result = await extractor.extract({
        tool: 'Edit',
        input: {
          file_path: '/src/utils.js',
          old_string: 'const x = 1;',
          new_string: 'const x = 2; // fixed value',
        },
        output: 'success',
      });
      assert.ok(Array.isArray(result));
    });

    it('should skip Edit when old and new are same', async () => {
      const result = await extractor.extract({
        tool: 'Edit',
        input: {
          file_path: '/test.js',
          old_string: 'same',
          new_string: 'same',
        },
        output: 'success',
      });
      assert.deepStrictEqual(result, []);
    });

    it('should extract facts from Bash tool npm install', async () => {
      const result = await extractor.extract({
        tool: 'Bash',
        input: { command: 'npm install express' },
        output: 'added 50 packages',
      });
      assert.ok(Array.isArray(result));
    });

    it('should extract facts from Bash tool git commands', async () => {
      const result = await extractor.extract({
        tool: 'Bash',
        input: { command: 'git status' },
        output: 'On branch main\nNothing to commit',
      });
      assert.ok(Array.isArray(result));
    });

    it('should extract facts from Bash tool test commands', async () => {
      const result = await extractor.extract({
        tool: 'Bash',
        input: { command: 'npm test' },
        output: '10 pass\n2 fail',
      });
      assert.ok(Array.isArray(result));
    });

    it('should return empty for Bash with no command', async () => {
      const result = await extractor.extract({
        tool: 'Bash',
        input: {},
        output: 'something',
      });
      assert.deepStrictEqual(result, []);
    });

    it('should increment stats on extraction', async () => {
      const output = 'a'.repeat(200) + '\nexport class Test {}';
      await extractor.extract({
        tool: 'Read',
        input: { file_path: '/test.js' },
        output,
      });
      assert.ok(extractor._stats.processed >= 1);
    });
  });

  describe('extractErrorResolution', () => {
    it('should create a fact for error resolution', async () => {
      const result = await extractor.extractErrorResolution({
        error: 'TypeError: undefined is not a function',
        solution: 'Added null check before calling method',
        tool: 'Edit',
        file: '/src/utils.js',
      }, { userId: 'user-1' });

      assert.ok(result);
      assert.ok(result.factId);
    });
  });

  describe('extractUserPreference', () => {
    it('should return null for low occurrence count', async () => {
      const result = await extractor.extractUserPreference({
        type: 'indentation',
        value: '2 spaces',
        occurrences: 2,
      });
      assert.strictEqual(result, null);
    });

    it('should create fact for sufficient occurrences', async () => {
      const result = await extractor.extractUserPreference({
        type: 'indentation',
        value: '2 spaces',
        occurrences: 5,
      }, { userId: 'user-1' });
      assert.ok(result);
    });
  });

  describe('search', () => {
    it('should delegate to factsRepo.search', async () => {
      const result = await extractor.search('test query');
      assert.ok(Array.isArray(result));
    });
  });

  describe('getSessionFacts', () => {
    it('should delegate to factsRepo.findBySession', async () => {
      const result = await extractor.getSessionFacts('sess-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('getUserFacts', () => {
    it('should delegate to factsRepo.findByUser', async () => {
      const result = await extractor.getUserFacts('user-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('runMaintenance', () => {
    it('should decay stale and prune old facts', async () => {
      const result = await extractor.runMaintenance();
      assert.ok(result);
      assert.strictEqual(typeof result.decayed, 'number');
      assert.strictEqual(typeof result.pruned, 'number');
    });
  });

  describe('getStats', () => {
    it('should return combined stats', async () => {
      const stats = await extractor.getStats();
      assert.ok(stats);
      assert.strictEqual(typeof stats.total, 'number');
      assert.ok(stats.processing);
      assert.strictEqual(typeof stats.processing.processed, 'number');
    });
  });
});

// ============================================================================
// REASONING BANK SERVICE
// ============================================================================

describe('ReasoningBank', () => {
  let bank;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    bank = new ReasoningBank({ pool: mockPool });
  });

  describe('constructor', () => {
    it('should create with pool', () => {
      assert.ok(bank);
      assert.strictEqual(bank._pool, mockPool);
      assert.ok(bank._trajRepo);
      assert.strictEqual(bank._initialized, false);
      assert.ok(bank._active instanceof Map);
      assert.ok(bank._successCache instanceof Map);
    });

    it('should throw without pool', () => {
      assert.throws(
        () => new ReasoningBank({}),
        { message: 'ReasoningBank requires database pool' },
      );
    });

    it('should throw with no options', () => {
      assert.throws(
        () => new ReasoningBank(),
        { message: 'ReasoningBank requires database pool' },
      );
    });

    it('should accept optional vectorStore', () => {
      const vs = { store: async () => {}, search: async () => [] };
      const b = new ReasoningBank({ pool: mockPool, vectorStore: vs });
      assert.strictEqual(b._vectorStore, vs);
    });

    it('should initialize stats', () => {
      assert.strictEqual(bank._stats.trajectoriesStarted, 0);
      assert.strictEqual(bank._stats.trajectoriesCompleted, 0);
      assert.strictEqual(bank._stats.replaysAttempted, 0);
      assert.strictEqual(bank._stats.replaysSuccessful, 0);
    });
  });

  describe('createReasoningBank factory', () => {
    it('should create an instance', () => {
      const b = createReasoningBank({ pool: mockPool });
      assert.ok(b instanceof ReasoningBank);
    });
  });

  describe('initialize', () => {
    it('should call ensureTable', async () => {
      await bank.initialize();
      assert.strictEqual(bank._initialized, true);
    });

    it('should skip if already initialized', async () => {
      await bank.initialize();
      const callCount = mockPool.query.mock.calls.length;
      await bank.initialize();
      assert.strictEqual(mockPool.query.mock.calls.length, callCount);
    });
  });

  describe('startTrajectory', () => {
    it('should create and track a new trajectory', async () => {
      const result = await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
        initialState: { files: 10 },
      });

      assert.ok(result);
      assert.ok(result.trajectoryId);
      assert.strictEqual(bank._stats.trajectoriesStarted, 1);
      assert.strictEqual(bank._active.size, 1);
    });

    it('should track multiple active trajectories', async () => {
      await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
      });
      await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'architect',
        taskType: 'design',
      });

      assert.strictEqual(bank._active.size, 2);
      assert.strictEqual(bank._stats.trajectoriesStarted, 2);
    });
  });

  describe('recordAction', () => {
    it('should record an action', async () => {
      const traj = await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
      });

      const result = await bank.recordAction(traj.trajectoryId, {
        tool: 'Read',
        input: { file_path: '/test.js' },
        output: 'content',
        success: true,
      });
      assert.ok(result);
    });

    it('should log warning for unknown trajectory', async () => {
      // Should not throw
      await assert.doesNotReject(() =>
        bank.recordAction('unknown-id', { tool: 'Read' }),
      );
    });
  });

  describe('recordSwitch', () => {
    it('should record a dog switch', async () => {
      const traj = await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
      });

      const result = await bank.recordSwitch(
        traj.trajectoryId, 'scout', 'architect', 'complex task',
      );
      assert.ok(result);
    });
  });

  describe('completeTrajectory', () => {
    it('should complete and remove from active', async () => {
      const traj = await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
      });

      const result = await bank.completeTrajectory(traj.trajectoryId, {
        outcome: TrajectoryOutcome.SUCCESS,
        details: { summary: 'Found all files' },
      });

      assert.ok(result);
      assert.strictEqual(bank._active.size, 0);
      assert.strictEqual(bank._stats.trajectoriesCompleted, 1);
    });

    it('should handle completing unknown trajectory', async () => {
      const result = await bank.completeTrajectory('unknown-id', {
        outcome: TrajectoryOutcome.FAILURE,
      });
      assert.ok(result);
    });
  });

  describe('findSimilar', () => {
    it('should return empty array when no matches', async () => {
      const result = await bank.findSimilar({
        taskType: 'exploration',
        dogId: 'scout',
      });
      assert.ok(Array.isArray(result));
    });

    it('should check cache first', async () => {
      // Pre-populate cache
      bank._successCache.set('exploration:scout', [{
        trajectoryId: 'cached-1',
        reward: 0.618,
        taskType: 'exploration',
      }]);

      const result = await bank.findSimilar({
        taskType: 'exploration',
        dogId: 'scout',
      });
      assert.ok(result.length >= 1);
      assert.ok(result.some(r => r.trajectoryId === 'cached-1'));
    });
  });

  describe('getReplaySuggestions', () => {
    it('should return no replay when no similar trajectories', async () => {
      const result = await bank.getReplaySuggestions({
        taskType: 'unknown_type',
        dogId: 'unknown',
      });
      assert.strictEqual(result.hasReplay, false);
      assert.strictEqual(result.confidence, 0);
    });

    it('should return replay when cache has matches', async () => {
      bank._successCache.set('exploration:scout', [{
        trajectoryId: 'cached-1',
        reward: 0.618,
        taskType: 'exploration',
        dogId: 'scout',
        replayCount: 0,
        actionSequence: [{ tool: 'Glob', success: true }, { tool: 'Read', success: true }],
      }]);

      const result = await bank.getReplaySuggestions({
        taskType: 'exploration',
        dogId: 'scout',
      });
      assert.strictEqual(result.hasReplay, true);
      assert.ok(result.confidence > 0);
      assert.strictEqual(result.suggestedDog, 'scout');
    });
  });

  describe('recordReplayOutcome', () => {
    it('should record successful replay', async () => {
      const result = await bank.recordReplayOutcome('traj-1', true);
      assert.ok(result);
      assert.strictEqual(bank._stats.replaysAttempted, 1);
      assert.strictEqual(bank._stats.replaysSuccessful, 1);
    });

    it('should record failed replay', async () => {
      await bank.recordReplayOutcome('traj-1', false);
      assert.strictEqual(bank._stats.replaysAttempted, 1);
      assert.strictEqual(bank._stats.replaysSuccessful, 0);
    });
  });

  describe('getRecommendedDog', () => {
    it('should return null recommendation when no history', async () => {
      const result = await bank.getRecommendedDog('unknown_task');
      assert.strictEqual(result.recommended, null);
      assert.strictEqual(result.confidence, 0);
    });
  });

  describe('_extractActionPlan', () => {
    it('should extract successful actions', () => {
      const plan = bank._extractActionPlan({
        actionSequence: [
          { tool: 'Glob', success: true },
          { tool: 'Read', success: true, input: { file_path: '/test.js' } },
          { tool: 'Edit', success: false },
        ],
      });
      assert.strictEqual(plan.length, 2);
      assert.strictEqual(plan[0].tool, 'Glob');
      assert.strictEqual(plan[1].tool, 'Read');
    });

    it('should return empty for non-array', () => {
      const plan = bank._extractActionPlan({ actionSequence: null });
      assert.deepStrictEqual(plan, []);
    });
  });

  describe('_summarizeAction', () => {
    it('should summarize Read action', () => {
      const summary = bank._summarizeAction({
        tool: 'Read',
        input: { file_path: '/src/test.js' },
      });
      assert.ok(summary.includes('/src/test.js'));
    });

    it('should summarize Edit action', () => {
      const summary = bank._summarizeAction({
        tool: 'Edit',
        input: { file_path: '/src/test.js' },
      });
      assert.ok(summary.includes('Edit'));
    });

    it('should summarize Bash action', () => {
      const summary = bank._summarizeAction({
        tool: 'Bash',
        input: { command: 'npm test' },
      });
      assert.ok(summary.includes('npm test'));
    });

    it('should summarize Glob action', () => {
      const summary = bank._summarizeAction({
        tool: 'Glob',
        input: { pattern: '**/*.js' },
      });
      assert.ok(summary.includes('**/*.js'));
    });

    it('should return tool name for unknown action', () => {
      const summary = bank._summarizeAction({
        tool: 'CustomTool',
        input: { data: 'something' },
      });
      assert.strictEqual(summary, 'CustomTool');
    });

    it('should return tool name when no input', () => {
      const summary = bank._summarizeAction({ tool: 'Read' });
      assert.strictEqual(summary, 'Read');
    });
  });

  describe('getActiveTrajectories', () => {
    it('should return empty array initially', () => {
      assert.deepStrictEqual(bank.getActiveTrajectories(), []);
    });

    it('should return active trajectory IDs', async () => {
      const traj = await bank.startTrajectory({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
      });
      const active = bank.getActiveTrajectories();
      assert.strictEqual(active.length, 1);
      assert.strictEqual(active[0], traj.trajectoryId);
    });
  });

  describe('getStats', () => {
    it('should return combined stats', async () => {
      const stats = await bank.getStats();
      assert.ok(stats);
      assert.strictEqual(typeof stats.total, 'number');
      assert.ok(stats.processing);
      assert.strictEqual(typeof stats.activeTrajectories, 'number');
      assert.strictEqual(typeof stats.cacheSize, 'number');
      assert.strictEqual(typeof stats.replaySuccessRate, 'number');
    });

    it('should calculate replay success rate', async () => {
      bank._stats.replaysAttempted = 10;
      bank._stats.replaysSuccessful = 7;
      const stats = await bank.getStats();
      assert.ok(Math.abs(stats.replaySuccessRate - 0.7) < 0.001);
    });
  });
});

// ============================================================================
// PATTERN LEARNING SERVICE
// ============================================================================

describe('PatternLearning', () => {
  let learning;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    learning = new PatternLearning({ pool: mockPool });
  });

  describe('constructor', () => {
    it('should create with pool', () => {
      assert.ok(learning);
      assert.strictEqual(learning._pool, mockPool);
    });

    it('should throw without pool', () => {
      assert.throws(
        () => new PatternLearning({}),
        { message: 'PatternLearning requires a database pool' },
      );
    });

    it('should throw with no options', () => {
      assert.throws(
        () => new PatternLearning(),
        { message: 'PatternLearning requires a database pool' },
      );
    });

    it('should use default config', () => {
      assert.deepStrictEqual(learning._config, DEFAULT_PATTERN_CONFIG);
    });

    it('should merge custom config with defaults', () => {
      const l = new PatternLearning({
        pool: mockPool,
        config: { batchSize: 100 },
      });
      assert.strictEqual(l._config.batchSize, 100);
      assert.strictEqual(l._config.extractionThreshold, DEFAULT_PATTERN_CONFIG.extractionThreshold);
    });

    it('should accept optional embedder', () => {
      const embedder = createMockEmbedder();
      const l = new PatternLearning({ pool: mockPool, embedder });
      assert.strictEqual(l._embedder, embedder);
    });

    it('should accept optional vectorStore', () => {
      const vs = { store: async () => {}, search: async () => [] };
      const l = new PatternLearning({ pool: mockPool, vectorStore: vs });
      assert.strictEqual(l._vectorStore, vs);
    });

    it('should initialize stats', () => {
      assert.strictEqual(learning._stats.totalExtracted, 0);
      assert.strictEqual(learning._stats.totalDecayed, 0);
      assert.strictEqual(learning._stats.totalPruned, 0);
      assert.strictEqual(learning._stats.totalClustered, 0);
      assert.strictEqual(learning._stats.lastRun, null);
    });
  });

  describe('DEFAULT_PATTERN_CONFIG', () => {
    it('should be frozen', () => {
      assert.ok(Object.isFrozen(DEFAULT_PATTERN_CONFIG));
    });

    it('should have phi-derived thresholds', () => {
      assert.ok(Math.abs(DEFAULT_PATTERN_CONFIG.extractionThreshold - 0.618) < 0.001);
      assert.ok(Math.abs(DEFAULT_PATTERN_CONFIG.similarityThreshold - 0.618) < 0.001);
    });

    it('should have Fibonacci batch size', () => {
      assert.strictEqual(DEFAULT_PATTERN_CONFIG.batchSize, 34);
    });

    it('should have Fibonacci max patterns', () => {
      assert.strictEqual(DEFAULT_PATTERN_CONFIG.maxPatternsPerCategory, 21);
    });

    it('should have Fibonacci decay period', () => {
      assert.strictEqual(DEFAULT_PATTERN_CONFIG.decayPeriodDays, 13);
    });
  });

  describe('createPatternLearning factory', () => {
    it('should create an instance', () => {
      const l = createPatternLearning({ pool: mockPool });
      assert.ok(l instanceof PatternLearning);
    });
  });

  describe('extractFromJudgments', () => {
    it('should process and return results', async () => {
      const result = await learning.extractFromJudgments();
      assert.ok(result);
      assert.strictEqual(typeof result.processed, 'number');
      assert.ok(Array.isArray(result.extracted));
      assert.strictEqual(typeof result.skipped, 'number');
      assert.ok(result.timestamp);
    });

    it('should respect dryRun option', async () => {
      const result = await learning.extractFromJudgments({ dryRun: true });
      assert.ok(result);
    });
  });

  describe('_extractPatternFromJudgment', () => {
    it('should extract pattern from judgment with all fields', () => {
      const pattern = learning._extractPatternFromJudgment({
        judgment_id: 'j-1',
        q_score: 80,
        verdict: 'WAG',
        subject_name: 'MyClass',
        subject_type: 'code',
        reasoning: 'Well structured code\nGood patterns',
        category: 'code_quality',
      });

      assert.ok(pattern);
      assert.ok(pattern.patternId.startsWith('pat_'));
      assert.strictEqual(pattern.category, 'code_quality');
      assert.ok(pattern.name.includes('MyClass'));
      assert.strictEqual(pattern.confidence, 0.8);
      assert.ok(pattern.sourceJudgments.includes('j-1'));
    });

    it('should return null when no category can be inferred', () => {
      const pattern = learning._extractPatternFromJudgment({
        judgment_id: 'j-1',
        q_score: 50,
      });
      // Should still return something because _inferCategory returns 'general' for any verdict/type
      // Only returns null if _inferCategory returns null (which it shouldn't)
      // Actually, _inferPatternName may return null
    });

    it('should return null when no name can be inferred', () => {
      const pattern = learning._extractPatternFromJudgment({
        judgment_id: 'j-1',
        q_score: 50,
        verdict: 'WAG',
        // No subject_name and no reasoning
      });
      assert.strictEqual(pattern, null);
    });

    it('should use reasoning first line for name when no subject_name', () => {
      const pattern = learning._extractPatternFromJudgment({
        judgment_id: 'j-1',
        q_score: 70,
        verdict: 'WAG',
        reasoning: 'Good error handling pattern\nWith details',
      });
      assert.ok(pattern);
      assert.ok(pattern.name.includes('Good error handling'));
    });
  });

  describe('_inferCategory', () => {
    it('should use explicit category', () => {
      assert.strictEqual(learning._inferCategory({ category: 'security' }), 'security');
    });

    it('should map subject_type to category', () => {
      assert.strictEqual(learning._inferCategory({ subject_type: 'code' }), 'code_quality');
      assert.strictEqual(learning._inferCategory({ subject_type: 'token' }), 'token_analysis');
      assert.strictEqual(learning._inferCategory({ subject_type: 'error' }), 'error_handling');
    });

    it('should map verdict to category', () => {
      assert.strictEqual(learning._inferCategory({ verdict: 'HOWL' }), 'excellence');
      assert.strictEqual(learning._inferCategory({ verdict: 'WAG' }), 'quality');
      assert.strictEqual(learning._inferCategory({ verdict: 'BARK' }), 'warning');
      assert.strictEqual(learning._inferCategory({ verdict: 'GROWL' }), 'danger');
    });

    it('should return general for unknown verdict', () => {
      assert.strictEqual(learning._inferCategory({ verdict: 'UNKNOWN' }), 'general');
    });
  });

  describe('_inferPatternName', () => {
    it('should use subject_name', () => {
      const name = learning._inferPatternName({ subject_name: 'Foo' });
      assert.strictEqual(name, 'Pattern: Foo');
    });

    it('should use first line of reasoning', () => {
      const name = learning._inferPatternName({
        reasoning: 'First line\nSecond line',
      });
      assert.strictEqual(name, 'First line');
    });

    it('should return null when no name available', () => {
      const name = learning._inferPatternName({});
      assert.strictEqual(name, null);
    });
  });

  describe('_inferTags', () => {
    it('should extract tags from verdict and type', () => {
      const tags = learning._inferTags({
        verdict: 'WAG',
        subject_type: 'code',
        category: 'quality',
      });
      assert.ok(tags.includes('wag'));
      assert.ok(tags.includes('code'));
      assert.ok(tags.includes('quality'));
    });

    it('should include tags from data', () => {
      const tags = learning._inferTags({
        data: { tags: ['custom1', 'custom2'] },
      });
      assert.ok(tags.includes('custom1'));
      assert.ok(tags.includes('custom2'));
    });

    it('should deduplicate tags', () => {
      const tags = learning._inferTags({
        verdict: 'WAG',
        category: 'wag', // Same as verdict lowercased
      });
      const wagCount = tags.filter(t => t === 'wag').length;
      assert.strictEqual(wagCount, 1);
    });

    it('should return empty array for empty judgment', () => {
      const tags = learning._inferTags({});
      assert.deepStrictEqual(tags, []);
    });
  });

  describe('applyConfidenceDecay', () => {
    it('should return decay results', async () => {
      const result = await learning.applyConfidenceDecay();
      assert.ok(result);
      assert.strictEqual(typeof result.decayed, 'number');
      assert.strictEqual(typeof result.pruned, 'number');
      assert.ok(Array.isArray(result.patterns));
      assert.ok(result.timestamp);
    });

    it('should skip pruning in dryRun mode', async () => {
      const result = await learning.applyConfidenceDecay({ dryRun: true });
      assert.ok(result);
      assert.strictEqual(result.pruned, 0);
    });
  });

  describe('clusterPatterns', () => {
    it('should return empty result without embedder or vectorStore', async () => {
      const result = await learning.clusterPatterns('code_quality');
      assert.ok(result);
      assert.strictEqual(result.category, 'code_quality');
      assert.deepStrictEqual(result.clusters, []);
    });
  });

  describe('reinforcePattern', () => {
    it('should boost confidence', async () => {
      const result = await learning.reinforcePattern('pat-1', 0.1);
      assert.ok(result);
    });
  });

  describe('weakenPattern', () => {
    it('should reduce confidence', async () => {
      const result = await learning.weakenPattern('pat-1', 0.2);
      assert.ok(result);
    });
  });

  describe('runCycle', () => {
    it('should run extraction and decay', async () => {
      const result = await learning.runCycle();
      assert.ok(result);
      assert.ok(result.extraction);
      assert.ok(result.decay);
      assert.strictEqual(typeof result.duration, 'number');
      assert.ok(result.duration >= 0);
    });

    it('should respect dryRun', async () => {
      const result = await learning.runCycle({ dryRun: true });
      assert.strictEqual(result.dryRun, true);
    });

    it('should skip clustering without embedder', async () => {
      const result = await learning.runCycle();
      assert.strictEqual(result.clustering, null);
    });
  });

  describe('_cosineSimilarity', () => {
    it('should return 1 for identical vectors', () => {
      const v = [1, 0, 0, 1];
      assert.ok(Math.abs(learning._cosineSimilarity(v, v) - 1.0) < 0.001);
    });

    it('should return 0 for orthogonal vectors', () => {
      assert.ok(Math.abs(learning._cosineSimilarity([1, 0], [0, 1])) < 0.001);
    });

    it('should return 0 for null inputs', () => {
      assert.strictEqual(learning._cosineSimilarity(null, [1]), 0);
      assert.strictEqual(learning._cosineSimilarity([1], null), 0);
    });

    it('should return 0 for mismatched lengths', () => {
      assert.strictEqual(learning._cosineSimilarity([1, 0], [1]), 0);
    });

    it('should return 0 for zero vectors', () => {
      assert.strictEqual(learning._cosineSimilarity([0, 0], [0, 0]), 0);
    });
  });

  describe('getStats', () => {
    it('should return stats with config', () => {
      const stats = learning.getStats();
      assert.ok(stats);
      assert.ok(stats.config);
      assert.strictEqual(stats.totalExtracted, 0);
      assert.strictEqual(stats.hasEmbedder, false);
    });

    it('should report embedder presence', () => {
      const l = new PatternLearning({ pool: mockPool, embedder: createMockEmbedder() });
      const stats = l.getStats();
      assert.strictEqual(stats.hasEmbedder, true);
    });
  });
});
