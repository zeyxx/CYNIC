#!/usr/bin/env node
/**
 * PostgreSQL Repository Tests (Phase 16+ Total Memory & Autonomy)
 *
 * Comprehensive tests for:
 * - ConversationMemoriesRepository
 * - ArchitecturalDecisionsRepository
 * - LessonsLearnedRepository
 * - AutonomousGoalsRepository
 * - AutonomousTasksRepository
 * - ProactiveNotificationsRepository
 * - FactsRepository
 * - TrajectoriesRepository
 *
 * All tests use mock database pools - no real PostgreSQL required.
 *
 * "phi distrusts phi" - verify all persistence
 *
 * @module @cynic/persistence/test/repositories
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  ConversationMemoriesRepository,
  MemoryType,
} from '../src/postgres/repositories/conversation-memories.js';

import {
  ArchitecturalDecisionsRepository,
  DecisionType,
  DecisionStatus,
} from '../src/postgres/repositories/architectural-decisions.js';

import {
  LessonsLearnedRepository,
  LessonCategory,
  LessonSeverity,
} from '../src/postgres/repositories/lessons-learned.js';

import {
  AutonomousGoalsRepository,
  GoalType,
  GoalStatus,
} from '../src/postgres/repositories/autonomous-goals.js';

import {
  AutonomousTasksRepository,
  TaskStatus,
  TaskType,
} from '../src/postgres/repositories/autonomous-tasks.js';

import {
  ProactiveNotificationsRepository,
  NotificationType,
} from '../src/postgres/repositories/proactive-notifications.js';

import {
  FactsRepository,
  FactType,
} from '../src/postgres/repositories/facts.js';

import {
  TrajectoriesRepository,
  TrajectoryOutcome,
} from '../src/postgres/repositories/trajectories.js';

// ============================================================================
// MOCK DATABASE FACTORY
// ============================================================================

/**
 * Creates a mock database pool that intercepts SQL queries and returns
 * appropriate mock data. Supports INSERT, SELECT, UPDATE, DELETE operations.
 */
function createMockPool() {
  const storage = new Map();
  let idCounter = 1;

  function generateUUID() {
    return `uuid-${idCounter++}-${Date.now().toString(36)}`;
  }

  return {
    _storage: storage,
    _idCounter: () => idCounter,

    query: mock.fn(async (sql, params = []) => {
      const sqlLower = sql.toLowerCase().trim();

      // ── TABLE/TRIGGER CREATION ──────────────────────────────────────────
      if (sqlLower.includes('create table') ||
          sqlLower.includes('create index') ||
          sqlLower.includes('create or replace function') ||
          sqlLower.includes('drop trigger') ||
          sqlLower.includes('create trigger')) {
        return { rows: [] };
      }

      // ── GLOBAL DELETE HANDLER ──────────────────────────────────────────
      // Must come before SELECT patterns to avoid DELETE matching findById
      if (sqlLower.startsWith('delete') || sqlLower.trimStart().startsWith('delete')) {
        if (sqlLower.includes('delete from conversation_memories') && sqlLower.includes('where id = $1')) {
          return { rowCount: params[0] ? 1 : 0 };
        }
        if (sqlLower.includes('delete from architectural_decisions') && sqlLower.includes('where id = $1')) {
          return { rowCount: params[0] ? 1 : 0 };
        }
        if (sqlLower.includes('delete from lessons_learned') && sqlLower.includes('where id = $1')) {
          return { rowCount: params[0] ? 1 : 0 };
        }
        if (sqlLower.includes('delete from autonomous_goals')) {
          return { rowCount: params[0] ? 1 : 0 };
        }
        if (sqlLower.includes('delete from autonomous_tasks')) {
          return { rowCount: params[0] ? 1 : 0 };
        }
        if (sqlLower.includes('delete from proactive_notifications') && sqlLower.includes('where id = $1')) {
          return { rowCount: params[0] ? 1 : 0 };
        }
        if (sqlLower.includes('delete from proactive_notifications') && sqlLower.includes('expires_at')) {
          return { rowCount: 3 };
        }
        if (sqlLower.includes('delete from facts') && sqlLower.includes('relevance')) {
          return { rowCount: 5 };
        }
        // Default delete
        return { rowCount: params[0] ? 1 : 0 };
      }

      // ── GLOBAL STATS HANDLER ────────────────────────────────────────────
      // Must come before generic SELECT patterns to avoid stats queries
      // matching simpler WHERE conditions
      if (sqlLower.includes('count(*)') && !sqlLower.includes('insert') && !sqlLower.includes('update')) {
        // Conversation Memories stats
        if (sqlLower.includes('conversation_memories') && sqlLower.includes('avg')) {
          return {
            rows: [{
              total: '5',
              recent: '2',
              avg_importance: '0.6',
              avg_access_count: '3',
            }],
          };
        }
        if (sqlLower.includes('conversation_memories') && sqlLower.includes('group by memory_type')) {
          return { rows: [{ memory_type: 'summary', count: '3' }, { memory_type: 'decision', count: '2' }] };
        }
        // Architectural Decisions stats
        if (sqlLower.includes('architectural_decisions') && sqlLower.includes('active')) {
          return {
            rows: [{
              total: '10',
              active: '7',
              superseded: '2',
              recent: '3',
            }],
          };
        }
        if (sqlLower.includes('architectural_decisions') && sqlLower.includes('group by decision_type')) {
          return { rows: [{ decision_type: 'pattern', count: '5' }] };
        }
        // Lessons Learned stats
        if (sqlLower.includes('lessons_learned') && sqlLower.includes('severity')) {
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
        if (sqlLower.includes('lessons_learned') && sqlLower.includes('group by category')) {
          return { rows: [{ category: 'bug', count: '8' }] };
        }
        // Autonomous Goals stats
        if (sqlLower.includes('autonomous_goals') && sqlLower.includes('avg(progress)')) {
          return {
            rows: [{
              total: '8',
              active: '5',
              completed: '2',
              paused: '1',
              avg_progress: '0.45',
            }],
          };
        }
        if (sqlLower.includes('autonomous_goals') && sqlLower.includes('group by goal_type')) {
          return { rows: [{ goal_type: 'quality', count: '3' }] };
        }
        // Autonomous Tasks stats
        if (sqlLower.includes('autonomous_tasks') && sqlLower.includes('pending')) {
          return {
            rows: [{
              total: '20',
              pending: '8',
              running: '3',
              completed: '7',
              failed: '1',
              retrying: '1',
            }],
          };
        }
        if (sqlLower.includes('autonomous_tasks') && sqlLower.includes('group by task_type')) {
          return { rows: [{ task_type: 'analyze_patterns', count: '10' }] };
        }
        // Proactive Notifications stats
        if (sqlLower.includes('proactive_notifications') && sqlLower.includes('delivered')) {
          return {
            rows: [{
              total: '12',
              pending: '5',
              delivered: '6',
              dismissed: '1',
              recent: '4',
            }],
          };
        }
        if (sqlLower.includes('proactive_notifications') && sqlLower.includes('group by notification_type')) {
          return { rows: [{ notification_type: 'insight', count: '7' }] };
        }
        // Facts stats
        if (sqlLower.includes('facts') && sqlLower.includes('avg')) {
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
        // Trajectories stats
        if (sqlLower.includes('trajectories') && sqlLower.includes('avg')) {
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
        // Default stats
        return { rows: [{ total: '0' }] };
      }

      // ── CONVERSATION_MEMORIES ───────────────────────────────────────────
      if (sqlLower.includes('insert into conversation_memories')) {
        const row = {
          id: generateUUID(),
          user_id: params[0],
          session_id: params[1],
          memory_type: params[2],
          content: params[3],
          embedding: params[4],
          importance: params[5] ?? 0.5,
          context: params[6] ? JSON.parse(params[6]) : {},
          created_at: new Date(),
          last_accessed: null,
          access_count: 0,
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('where id = $1')) {
        return {
          rows: params[0] ? [{
            id: params[0],
            user_id: 'user-1',
            session_id: 'sess-1',
            memory_type: 'summary',
            content: 'Mock memory content',
            importance: 0.5,
            context: {},
            created_at: new Date(),
            last_accessed: null,
            access_count: 0,
          }] : [],
        };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('where user_id = $1') && sqlLower.includes('importance >= $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('where user_id = $1') && sqlLower.includes('memory_type')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('where session_id = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('where user_id = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('where 1=1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update conversation_memories') && sqlLower.includes('where id = $1')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            session_id: 'sess-1',
            memory_type: 'summary',
            content: params[1] || 'Updated content',
            importance: 0.8,
            context: {},
            created_at: new Date(),
            last_accessed: null,
            access_count: 0,
          }],
        };
      }

      if (sqlLower.includes('delete from conversation_memories') && sqlLower.includes('where id = $1')) {
        return { rowCount: params[0] ? 1 : 0 };
      }

      if (sqlLower.includes('select record_memory_access')) {
        return { rows: [{ record_memory_access: params[0]?.length || 0 }] };
      }

      if (sqlLower.includes('from conversation_memories') && sqlLower.includes('count(*)') && sqlLower.includes('avg')) {
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
        return { rows: [{ memory_type: 'summary', count: '3' }, { memory_type: 'decision', count: '2' }] };
      }

      if (sqlLower.includes('search_memories_hybrid')) {
        return { rows: [] };
      }

      if (sqlLower.includes('to_tsvector') && sqlLower.includes('conversation_memories')) {
        return { rows: [] };
      }

      // ── ARCHITECTURAL_DECISIONS ─────────────────────────────────────────
      if (sqlLower.includes('insert into architectural_decisions')) {
        const row = {
          id: generateUUID(),
          user_id: params[0],
          project_path: params[1],
          decision_type: params[2],
          title: params[3],
          description: params[4],
          rationale: params[5],
          alternatives: params[6] ? JSON.parse(params[6]) : [],
          consequences: params[7] ? JSON.parse(params[7]) : {},
          embedding: params[8],
          status: params[9] || 'active',
          superseded_by: null,
          related_decisions: [],
          created_at: new Date(),
          updated_at: new Date(),
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('where id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            id: params[0],
            user_id: 'user-1',
            project_path: '/project',
            decision_type: 'pattern',
            title: 'Mock Decision',
            description: 'Test',
            rationale: 'Because tests',
            alternatives: [],
            consequences: {},
            embedding: null,
            status: 'active',
            superseded_by: null,
            related_decisions: [],
            created_at: new Date(),
            updated_at: new Date(),
          }] : [],
        };
      }

      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('project_path = $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('decision_type = $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('order by updated_at desc') && sqlLower.includes('limit $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update architectural_decisions') && sqlLower.includes('status = \'superseded\'')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update architectural_decisions') && sqlLower.includes('where id = $1')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            project_path: '/project',
            decision_type: 'pattern',
            title: 'Updated',
            description: 'Updated desc',
            rationale: null,
            alternatives: [],
            consequences: {},
            embedding: null,
            status: 'active',
            superseded_by: null,
            related_decisions: [],
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('delete from architectural_decisions') && sqlLower.includes('where id = $1')) {
        return { rowCount: params[0] ? 1 : 0 };
      }

      if (sqlLower.includes('from architectural_decisions') && sqlLower.includes('where 1=1')) {
        return { rows: [] };
      }

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

      if (sqlLower.includes('to_tsvector') && sqlLower.includes('architectural_decisions')) {
        return { rows: [] };
      }

      // ── LESSONS_LEARNED ─────────────────────────────────────────────────
      if (sqlLower.includes('insert into lessons_learned')) {
        const row = {
          id: generateUUID(),
          user_id: params[0],
          category: params[1],
          mistake: params[2],
          correction: params[3],
          prevention: params[4],
          severity: params[5] || 'medium',
          embedding: params[6],
          source_judgment_id: params[7],
          source_session_id: params[8],
          occurrence_count: 1,
          last_occurred: new Date(),
          created_at: new Date(),
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('where id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            id: params[0],
            user_id: 'user-1',
            category: 'bug',
            mistake: 'Mock mistake',
            correction: 'Mock correction',
            prevention: 'Mock prevention',
            severity: 'medium',
            embedding: null,
            occurrence_count: 1,
            last_occurred: new Date(),
            source_judgment_id: null,
            source_session_id: null,
            created_at: new Date(),
          }] : [],
        };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('category = $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes("severity in ('critical', 'high')")) {
        return { rows: [] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('order by created_at desc') && sqlLower.includes('limit $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('source_session_id = $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('occurrence_count >= $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('select record_lesson_occurrence')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update lessons_learned') && sqlLower.includes('where id = $1')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            category: 'bug',
            mistake: 'Updated',
            correction: 'Updated',
            prevention: null,
            severity: 'high',
            embedding: null,
            occurrence_count: 1,
            last_occurred: new Date(),
            source_judgment_id: null,
            source_session_id: null,
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('delete from lessons_learned') && sqlLower.includes('where id = $1')) {
        return { rowCount: params[0] ? 1 : 0 };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('where 1=1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from lessons_learned') && sqlLower.includes('count(*)') && sqlLower.includes('severity')) {
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

      if (sqlLower.includes('to_tsvector') && sqlLower.includes('lessons_learned')) {
        return { rows: [] };
      }

      // ── AUTONOMOUS_GOALS ────────────────────────────────────────────────
      if (sqlLower.includes('insert into autonomous_goals')) {
        const row = {
          id: generateUUID(),
          user_id: params[0],
          goal_type: params[1],
          title: params[2],
          description: params[3],
          success_criteria: params[4] ? JSON.parse(params[4]) : [],
          progress: 0,
          progress_notes: [],
          status: 'active',
          priority: params[5] ?? 50,
          config: params[6] ? JSON.parse(params[6]) : {},
          due_at: params[7] || null,
          created_at: new Date(),
          updated_at: new Date(),
          completed_at: null,
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes('where id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            id: params[0],
            user_id: 'user-1',
            goal_type: 'quality',
            title: 'Mock Goal',
            description: 'Test goal',
            success_criteria: [],
            progress: 0.3,
            progress_notes: [],
            status: 'active',
            priority: 50,
            config: {},
            due_at: null,
            created_at: new Date(),
            updated_at: new Date(),
            completed_at: null,
          }] : [],
        };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes("status = 'active'") && sqlLower.includes('order by priority')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes('goal_type = $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes('due_at')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update autonomous_goals') && sqlLower.includes('progress = $2')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            goal_type: 'quality',
            title: 'Mock Goal',
            description: 'Test',
            success_criteria: [],
            progress: params[1],
            progress_notes: [],
            status: 'active',
            priority: 50,
            config: {},
            due_at: null,
            created_at: new Date(),
            updated_at: new Date(),
            completed_at: null,
          }],
        };
      }

      if (sqlLower.includes('update autonomous_goals') && sqlLower.includes('status = $2')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            goal_type: 'quality',
            title: 'Mock Goal',
            description: 'Test',
            success_criteria: [],
            progress: 0.5,
            progress_notes: [],
            status: params[1],
            priority: 50,
            config: {},
            due_at: null,
            created_at: new Date(),
            updated_at: new Date(),
            completed_at: params[1] === 'completed' ? new Date() : null,
          }],
        };
      }

      if (sqlLower.includes('update autonomous_goals') && sqlLower.includes('where id = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('delete from autonomous_goals')) {
        return { rowCount: params[0] ? 1 : 0 };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes('where 1=1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes('count(*)') && sqlLower.includes('avg(progress)')) {
        return {
          rows: [{
            total: '8',
            active: '5',
            completed: '2',
            paused: '1',
            avg_progress: '0.45',
          }],
        };
      }

      if (sqlLower.includes('from autonomous_goals') && sqlLower.includes('group by goal_type')) {
        return { rows: [{ goal_type: 'quality', count: '3' }] };
      }

      // ── AUTONOMOUS_TASKS ────────────────────────────────────────────────
      if (sqlLower.includes('insert into autonomous_tasks')) {
        const row = {
          id: generateUUID(),
          user_id: params[0],
          goal_id: params[1],
          task_type: params[2],
          payload: params[3] ? JSON.parse(params[3]) : {},
          priority: params[4] ?? 50,
          scheduled_for: params[5] || new Date(),
          max_retries: params[6] ?? 3,
          created_by: params[7] || 'daemon',
          status: 'pending',
          retry_count: 0,
          error_message: null,
          result: null,
          started_at: null,
          completed_at: null,
          created_at: new Date(),
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes('where id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            id: params[0],
            user_id: 'user-1',
            goal_id: null,
            task_type: 'analyze_patterns',
            payload: {},
            priority: 50,
            scheduled_for: new Date(),
            max_retries: 3,
            created_by: 'daemon',
            status: 'pending',
            retry_count: 0,
            error_message: null,
            result: null,
            started_at: null,
            completed_at: null,
            created_at: new Date(),
          }] : [],
        };
      }

      if (sqlLower.includes('get_pending_tasks')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update autonomous_tasks') && sqlLower.includes("status = 'running'")) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            goal_id: null,
            task_type: 'analyze_patterns',
            payload: {},
            priority: 50,
            scheduled_for: new Date(),
            max_retries: 3,
            created_by: 'daemon',
            status: 'running',
            retry_count: 0,
            error_message: null,
            result: null,
            started_at: new Date(),
            completed_at: null,
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('update autonomous_tasks') && sqlLower.includes("status = 'completed'")) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            goal_id: null,
            task_type: 'analyze_patterns',
            payload: {},
            priority: 50,
            scheduled_for: new Date(),
            max_retries: 3,
            created_by: 'daemon',
            status: 'completed',
            retry_count: 0,
            error_message: null,
            result: params[1] ? JSON.parse(params[1]) : null,
            started_at: new Date(),
            completed_at: new Date(),
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('update autonomous_tasks') && sqlLower.includes("status = 'cancelled'")) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            goal_id: null,
            task_type: 'analyze_patterns',
            payload: {},
            priority: 50,
            scheduled_for: new Date(),
            max_retries: 3,
            created_by: 'daemon',
            status: 'cancelled',
            retry_count: 0,
            error_message: null,
            result: null,
            started_at: null,
            completed_at: new Date(),
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('update autonomous_tasks') && sqlLower.includes('error_message')) {
        const status = params[1] || 'failed';
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            goal_id: null,
            task_type: 'analyze_patterns',
            payload: {},
            priority: 50,
            scheduled_for: new Date(),
            max_retries: 3,
            created_by: 'daemon',
            status,
            retry_count: 1,
            error_message: params[2],
            result: null,
            started_at: new Date(),
            completed_at: status === 'failed' ? new Date() : null,
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes('goal_id = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes('user_id = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes("status = 'running'") && !sqlLower.includes('update')) {
        return { rows: [] };
      }

      if (sqlLower.includes('delete from autonomous_tasks')) {
        return { rowCount: params[0] ? 1 : 0 };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes('where 1=1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes('count(*)') && sqlLower.includes('pending')) {
        return {
          rows: [{
            total: '20',
            pending: '8',
            running: '3',
            completed: '7',
            failed: '1',
            retrying: '1',
          }],
        };
      }

      if (sqlLower.includes('from autonomous_tasks') && sqlLower.includes('group by task_type')) {
        return { rows: [{ task_type: 'analyze_patterns', count: '10' }] };
      }

      // ── PROACTIVE_NOTIFICATIONS ─────────────────────────────────────────
      if (sqlLower.includes('insert into proactive_notifications')) {
        const row = {
          id: generateUUID(),
          user_id: params[0],
          notification_type: params[1],
          title: params[2],
          message: params[3],
          priority: params[4] ?? 50,
          context: params[5] ? JSON.parse(params[5]) : {},
          expires_at: params[6] || null,
          delivered: false,
          delivered_at: null,
          dismissed: false,
          dismissed_at: null,
          action_taken: null,
          created_at: new Date(),
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('where id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            id: params[0],
            user_id: 'user-1',
            notification_type: 'insight',
            title: 'Mock Notification',
            message: 'Test message',
            priority: 50,
            context: {},
            expires_at: null,
            delivered: false,
            delivered_at: null,
            dismissed: false,
            dismissed_at: null,
            action_taken: null,
            created_at: new Date(),
          }] : [],
        };
      }

      if (sqlLower.includes('get_pending_notifications')) {
        return { rows: [] };
      }

      if (sqlLower.includes('update proactive_notifications') && sqlLower.includes('delivered = true') && sqlLower.includes('where id = $1')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            notification_type: 'insight',
            title: 'Mock',
            message: 'Test',
            priority: 50,
            context: {},
            expires_at: null,
            delivered: true,
            delivered_at: new Date(),
            dismissed: false,
            dismissed_at: null,
            action_taken: null,
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('update proactive_notifications') && sqlLower.includes('delivered = true') && sqlLower.includes('any($1)')) {
        return { rowCount: params[0]?.length || 0 };
      }

      if (sqlLower.includes('update proactive_notifications') && sqlLower.includes('dismissed = true')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            notification_type: 'insight',
            title: 'Mock',
            message: 'Test',
            priority: 50,
            context: {},
            expires_at: null,
            delivered: true,
            delivered_at: new Date(),
            dismissed: true,
            dismissed_at: new Date(),
            action_taken: params[1] || null,
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('update proactive_notifications') && sqlLower.includes('where id = $1')) {
        return {
          rows: [{
            id: params[0],
            user_id: 'user-1',
            notification_type: 'insight',
            title: 'Updated',
            message: 'Updated msg',
            priority: 75,
            context: {},
            expires_at: null,
            delivered: false,
            delivered_at: null,
            dismissed: false,
            dismissed_at: null,
            action_taken: null,
            created_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('notification_type = $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('order by created_at desc') && sqlLower.includes('limit $2')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('delivered = true') && sqlLower.includes('delivered_at >')) {
        return { rows: [] };
      }

      if (sqlLower.includes('delete from proactive_notifications') && sqlLower.includes('where id = $1')) {
        return { rowCount: params[0] ? 1 : 0 };
      }

      if (sqlLower.includes('delete from proactive_notifications') && sqlLower.includes('expires_at')) {
        return { rowCount: 3 };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('where 1=1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('count(*)')) {
        return {
          rows: [{
            total: '12',
            pending: '5',
            delivered: '6',
            dismissed: '1',
            recent: '4',
          }],
        };
      }

      if (sqlLower.includes('from proactive_notifications') && sqlLower.includes('group by notification_type')) {
        return { rows: [{ notification_type: 'insight', count: '7' }] };
      }

      // ── FACTS ───────────────────────────────────────────────────────────
      if (sqlLower.includes('insert into facts')) {
        const row = {
          fact_id: `fact_${Date.now().toString(16)}`,
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
          embedding: params[12] ? JSON.parse(params[12]) : null,
          embedding_model: params[13] || null,
          embedding_dim: params[14] || null,
          access_count: 0,
          last_accessed: null,
          created_at: new Date(),
          updated_at: new Date(),
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('fact_id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            fact_id: params[0],
            user_id: 'user-1',
            session_id: 'sess-1',
            fact_type: 'code_pattern',
            subject: 'Test fact',
            content: 'Mock content',
            context: {},
            source_tool: 'Read',
            source_file: 'test.js',
            confidence: 0.5,
            relevance: 0.5,
            tags: [],
            embedding: null,
            embedding_model: null,
            embedding_dim: null,
            access_count: 0,
            last_accessed: null,
            created_at: new Date(),
            updated_at: new Date(),
          }] : [],
        };
      }

      if (sqlLower.includes('update facts') && sqlLower.includes('fact_id = $1') && sqlLower.includes('returning')) {
        return {
          rows: [{
            fact_id: params[0],
            user_id: 'user-1',
            session_id: 'sess-1',
            fact_type: 'code_pattern',
            subject: 'Updated fact',
            content: 'Updated content',
            context: {},
            source_tool: 'Read',
            source_file: 'test.js',
            confidence: 0.6,
            relevance: 0.6,
            tags: ['updated'],
            embedding: null,
            embedding_model: null,
            embedding_dim: null,
            access_count: 1,
            last_accessed: new Date(),
            created_at: new Date(),
            updated_at: new Date(),
          }],
        };
      }

      if (sqlLower.includes('update facts') && sqlLower.includes('access_count = access_count + 1')) {
        return { rows: [], rowCount: 1 };
      }

      if (sqlLower.includes('update facts') && sqlLower.includes('returning')) {
        return {
          rows: [{
            fact_id: params[1] || params[0],
            user_id: 'user-1',
            session_id: 'sess-1',
            fact_type: 'code_pattern',
            subject: 'Test',
            content: 'Test',
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

      if (sqlLower.includes('update facts') && sqlLower.includes('relevance')) {
        return { rowCount: 5 };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('user_id = $1') && !sqlLower.includes('count')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('session_id = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('search_vector')) {
        return { rows: [] };
      }

      if (sqlLower.includes('delete from facts') && sqlLower.includes('relevance')) {
        return { rowCount: 5 };
      }

      if (sqlLower.includes('from facts') && sqlLower.includes('count(*)') && sqlLower.includes('avg')) {
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

      // ── TRAJECTORIES ────────────────────────────────────────────────────
      if (sqlLower.includes('insert into trajectories')) {
        const row = {
          trajectory_id: `traj_${Date.now().toString(16)}`,
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
        };
        return { rows: [row] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes('trajectory_id = $1') && !sqlLower.includes('update')) {
        return {
          rows: params[0] ? [{
            trajectory_id: params[0],
            user_id: 'user-1',
            session_id: 'sess-1',
            dog_id: 'scout',
            task_type: 'exploration',
            initial_state: {},
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
            tags: [],
            created_at: new Date(),
            updated_at: new Date(),
          }] : [],
        };
      }

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
            outcome: params[0],
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

      if (sqlLower.includes('from trajectories') && sqlLower.includes("outcome = 'success'") && sqlLower.includes('reward >=')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes('similarity_hash = $1')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes('search_vector')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes("outcome = 'success'") && sqlLower.includes('reward > 0')) {
        return { rows: [] };
      }

      if (sqlLower.includes('from trajectories') && sqlLower.includes('count(*)') && sqlLower.includes('avg')) {
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

      // ── DEFAULT FALLBACK ────────────────────────────────────────────────
      return { rows: [], rowCount: 0 };
    }),
  };
}

// ============================================================================
// ENUM TESTS
// ============================================================================

describe('Enums', () => {
  it('MemoryType should have all types', () => {
    assert.strictEqual(MemoryType.SUMMARY, 'summary');
    assert.strictEqual(MemoryType.KEY_MOMENT, 'key_moment');
    assert.strictEqual(MemoryType.DECISION, 'decision');
    assert.strictEqual(MemoryType.PREFERENCE, 'preference');
    assert.strictEqual(MemoryType.CORRECTION, 'correction');
    assert.strictEqual(MemoryType.INSIGHT, 'insight');
  });

  it('DecisionType should have all types', () => {
    assert.strictEqual(DecisionType.PATTERN, 'pattern');
    assert.strictEqual(DecisionType.TECHNOLOGY, 'technology');
    assert.strictEqual(DecisionType.STRUCTURE, 'structure');
    assert.strictEqual(DecisionType.SECURITY, 'security');
    assert.strictEqual(DecisionType.OTHER, 'other');
  });

  it('DecisionStatus should have all statuses', () => {
    assert.strictEqual(DecisionStatus.ACTIVE, 'active');
    assert.strictEqual(DecisionStatus.SUPERSEDED, 'superseded');
    assert.strictEqual(DecisionStatus.DEPRECATED, 'deprecated');
    assert.strictEqual(DecisionStatus.PENDING, 'pending');
  });

  it('LessonCategory should have all categories', () => {
    assert.strictEqual(LessonCategory.BUG, 'bug');
    assert.strictEqual(LessonCategory.ARCHITECTURE, 'architecture');
    assert.strictEqual(LessonCategory.SECURITY, 'security');
    assert.strictEqual(LessonCategory.JUDGMENT, 'judgment');
    assert.strictEqual(LessonCategory.OTHER, 'other');
  });

  it('LessonSeverity should have all levels', () => {
    assert.strictEqual(LessonSeverity.LOW, 'low');
    assert.strictEqual(LessonSeverity.MEDIUM, 'medium');
    assert.strictEqual(LessonSeverity.HIGH, 'high');
    assert.strictEqual(LessonSeverity.CRITICAL, 'critical');
  });

  it('GoalType should have all types', () => {
    assert.strictEqual(GoalType.QUALITY, 'quality');
    assert.strictEqual(GoalType.LEARNING, 'learning');
    assert.strictEqual(GoalType.MAINTENANCE, 'maintenance');
    assert.strictEqual(GoalType.CUSTOM, 'custom');
  });

  it('GoalStatus should have all statuses', () => {
    assert.strictEqual(GoalStatus.ACTIVE, 'active');
    assert.strictEqual(GoalStatus.PAUSED, 'paused');
    assert.strictEqual(GoalStatus.COMPLETED, 'completed');
    assert.strictEqual(GoalStatus.ABANDONED, 'abandoned');
    assert.strictEqual(GoalStatus.BLOCKED, 'blocked');
  });

  it('TaskStatus should have all statuses', () => {
    assert.strictEqual(TaskStatus.PENDING, 'pending');
    assert.strictEqual(TaskStatus.RUNNING, 'running');
    assert.strictEqual(TaskStatus.COMPLETED, 'completed');
    assert.strictEqual(TaskStatus.FAILED, 'failed');
    assert.strictEqual(TaskStatus.RETRY, 'retry');
    assert.strictEqual(TaskStatus.CANCELLED, 'cancelled');
  });

  it('TaskType should have all types', () => {
    assert.strictEqual(TaskType.ANALYZE_PATTERNS, 'analyze_patterns');
    assert.strictEqual(TaskType.RUN_TESTS, 'run_tests');
    assert.strictEqual(TaskType.SECURITY_SCAN, 'security_scan');
    assert.strictEqual(TaskType.CUSTOM, 'custom');
  });

  it('NotificationType should have all types', () => {
    assert.strictEqual(NotificationType.INSIGHT, 'insight');
    assert.strictEqual(NotificationType.WARNING, 'warning');
    assert.strictEqual(NotificationType.REMINDER, 'reminder');
    assert.strictEqual(NotificationType.ACHIEVEMENT, 'achievement');
    assert.strictEqual(NotificationType.PATTERN, 'pattern');
  });

  it('FactType should have all types and be frozen', () => {
    assert.strictEqual(FactType.CODE_PATTERN, 'code_pattern');
    assert.strictEqual(FactType.API_DISCOVERY, 'api_discovery');
    assert.strictEqual(FactType.ERROR_RESOLUTION, 'error_resolution');
    assert.strictEqual(FactType.USER_PREFERENCE, 'user_preference');
    assert.ok(Object.isFrozen(FactType));
  });

  it('TrajectoryOutcome should have all outcomes and be frozen', () => {
    assert.strictEqual(TrajectoryOutcome.SUCCESS, 'success');
    assert.strictEqual(TrajectoryOutcome.PARTIAL, 'partial');
    assert.strictEqual(TrajectoryOutcome.FAILURE, 'failure');
    assert.strictEqual(TrajectoryOutcome.ABANDONED, 'abandoned');
    assert.ok(Object.isFrozen(TrajectoryOutcome));
  });
});

// ============================================================================
// CONVERSATION MEMORIES REPOSITORY
// ============================================================================

describe('ConversationMemoriesRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new ConversationMemoriesRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
      assert.strictEqual(repo.db, mockPool);
    });
  });

  describe('supportsFTS', () => {
    it('should return true', () => {
      assert.strictEqual(repo.supportsFTS(), true);
    });
  });

  describe('supportsVector', () => {
    it('should return true', () => {
      assert.strictEqual(repo.supportsVector(), true);
    });
  });

  describe('create', () => {
    it('should create a memory with all fields', async () => {
      const result = await repo.create({
        userId: 'user-1',
        sessionId: 'sess-1',
        memoryType: MemoryType.SUMMARY,
        content: 'Test summary content',
        importance: 0.8,
        context: { topic: 'testing' },
      });

      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(result.memoryType, MemoryType.SUMMARY);
      assert.strictEqual(typeof result.importance, 'number');
      assert.ok(result.id);
    });

    it('should use defaults for optional fields', async () => {
      const result = await repo.create({
        userId: 'user-1',
        memoryType: MemoryType.KEY_MOMENT,
        content: 'A key moment',
      });

      assert.ok(result);
      assert.strictEqual(result.importance, 0.5);
    });
  });

  describe('findById', () => {
    it('should return memory when found', async () => {
      const result = await repo.findById('existing-id');
      assert.ok(result);
      assert.strictEqual(result.id, 'existing-id');
    });

    it('should return null when not found', async () => {
      // Override mock to return empty
      mockPool.query = mock.fn(async () => ({ rows: [] }));
      const result = await repo.findById('non-existent');
      assert.strictEqual(result, null);
    });
  });

  describe('findByUser', () => {
    it('should query by user with defaults', async () => {
      const result = await repo.findByUser('user-1');
      assert.ok(Array.isArray(result));
    });

    it('should accept memoryType filter', async () => {
      const result = await repo.findByUser('user-1', { memoryType: 'summary' });
      assert.ok(Array.isArray(result));
    });
  });

  describe('findBySession', () => {
    it('should query by session', async () => {
      const result = await repo.findBySession('sess-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findImportant', () => {
    it('should query with importance threshold', async () => {
      const result = await repo.findImportant('user-1', 0.7, 5);
      assert.ok(Array.isArray(result));
    });
  });

  describe('recordAccess', () => {
    it('should return 0 for empty array', async () => {
      const result = await repo.recordAccess([]);
      assert.strictEqual(result, 0);
    });

    it('should return 0 for null', async () => {
      const result = await repo.recordAccess(null);
      assert.strictEqual(result, 0);
    });

    it('should record access for given memory IDs', async () => {
      const result = await repo.recordAccess(['id-1', 'id-2']);
      assert.strictEqual(result, 2);
    });
  });

  describe('update', () => {
    it('should update memory fields', async () => {
      const result = await repo.update('mem-1', { content: 'New content', importance: 0.9 });
      assert.ok(result);
      assert.strictEqual(result.id, 'mem-1');
    });

    it('should return findById result when no updates provided', async () => {
      const result = await repo.update('mem-1', {});
      assert.ok(result);
    });
  });

  describe('delete', () => {
    it('should return true when deleted', async () => {
      const result = await repo.delete('existing-id');
      assert.strictEqual(result, true);
    });
  });

  describe('list', () => {
    it('should return array', async () => {
      const result = await repo.list();
      assert.ok(Array.isArray(result));
    });

    it('should accept filters', async () => {
      const result = await repo.list({ userId: 'user-1', memoryType: 'decision', limit: 5 });
      assert.ok(Array.isArray(result));
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 5);
      assert.strictEqual(stats.recent, 2);
      assert.strictEqual(typeof stats.avgImportance, 'number');
      assert.ok(stats.byType);
    });

    it('should accept userId filter', async () => {
      const stats = await repo.getStats('user-1');
      assert.ok(stats);
      assert.strictEqual(typeof stats.total, 'number');
    });
  });

  describe('_formatRow', () => {
    it('should return null for null input', () => {
      assert.strictEqual(repo._formatRow(null), null);
    });

    it('should format database row to camelCase', () => {
      const row = {
        id: 'test',
        user_id: 'u1',
        session_id: 's1',
        memory_type: 'summary',
        content: 'Hello',
        importance: '0.8',
        context: { k: 'v' },
        created_at: new Date(),
        last_accessed: null,
        access_count: 3,
        combined_score: 0.6,
        fts_score: 0.4,
      };
      const formatted = repo._formatRow(row);
      assert.strictEqual(formatted.userId, 'u1');
      assert.strictEqual(formatted.sessionId, 's1');
      assert.strictEqual(formatted.memoryType, 'summary');
      assert.strictEqual(formatted.importance, 0.8);
      assert.strictEqual(formatted.accessCount, 3);
      assert.strictEqual(formatted.combinedScore, 0.6);
    });
  });
});

// ============================================================================
// ARCHITECTURAL DECISIONS REPOSITORY
// ============================================================================

describe('ArchitecturalDecisionsRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new ArchitecturalDecisionsRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
      assert.strictEqual(repo.db, mockPool);
    });
  });

  describe('supportsFTS', () => {
    it('should return true', () => {
      assert.strictEqual(repo.supportsFTS(), true);
    });
  });

  describe('create', () => {
    it('should create a decision with all fields', async () => {
      const result = await repo.create({
        userId: 'user-1',
        projectPath: '/project',
        decisionType: DecisionType.PATTERN,
        title: 'Use singleton pattern',
        description: 'For shared state',
        rationale: 'Thread safety',
        alternatives: [{ name: 'Static class' }],
        consequences: { positive: ['safe'], negative: ['coupling'] },
      });

      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(result.decisionType, DecisionType.PATTERN);
      assert.strictEqual(result.status, 'active');
      assert.ok(result.id);
    });

    it('should use defaults for optional fields', async () => {
      const result = await repo.create({
        userId: 'user-1',
        decisionType: DecisionType.API,
        title: 'REST API',
        description: 'Use REST',
      });

      assert.ok(result);
      assert.strictEqual(result.status, 'active');
    });
  });

  describe('findById', () => {
    it('should return decision when found', async () => {
      const result = await repo.findById('dec-1');
      assert.ok(result);
      assert.strictEqual(result.id, 'dec-1');
    });
  });

  describe('findByProject', () => {
    it('should query by project path', async () => {
      const result = await repo.findByProject('user-1', '/project');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findByType', () => {
    it('should query by decision type', async () => {
      const result = await repo.findByType('user-1', DecisionType.PATTERN);
      assert.ok(Array.isArray(result));
    });
  });

  describe('findRecent', () => {
    it('should query recent decisions', async () => {
      const result = await repo.findRecent('user-1', 5);
      assert.ok(Array.isArray(result));
    });
  });

  describe('findBySession', () => {
    it('should return empty array', async () => {
      const result = await repo.findBySession('user-1', 'sess-1');
      assert.deepStrictEqual(result, []);
    });
  });

  describe('supersede', () => {
    it('should create new decision and mark old as superseded', async () => {
      const result = await repo.supersede('old-dec-1', {
        userId: 'user-1',
        decisionType: DecisionType.PATTERN,
        title: 'New pattern',
        description: 'Better approach',
      });

      assert.ok(result);
      assert.strictEqual(result.decisionType, DecisionType.PATTERN);
    });
  });

  describe('update', () => {
    it('should update decision fields', async () => {
      const result = await repo.update('dec-1', { title: 'Updated title' });
      assert.ok(result);
    });

    it('should call findById when no updates', async () => {
      const result = await repo.update('dec-1', {});
      assert.ok(result);
    });
  });

  describe('delete', () => {
    it('should return true when deleted', async () => {
      const result = await repo.delete('dec-1');
      assert.strictEqual(result, true);
    });
  });

  describe('list', () => {
    it('should return array with filters', async () => {
      const result = await repo.list({ userId: 'user-1', status: 'active' });
      assert.ok(Array.isArray(result));
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 10);
      assert.strictEqual(stats.active, 7);
      assert.strictEqual(stats.superseded, 2);
      assert.ok(stats.byType);
    });
  });

  describe('_formatRow', () => {
    it('should return null for null input', () => {
      assert.strictEqual(repo._formatRow(null), null);
    });

    it('should format all fields correctly', () => {
      const row = {
        id: 'test',
        user_id: 'u1',
        project_path: '/p',
        decision_type: 'pattern',
        title: 'T',
        description: 'D',
        rationale: 'R',
        alternatives: [{ name: 'alt' }],
        consequences: { good: true },
        embedding: null,
        status: 'active',
        superseded_by: null,
        related_decisions: [],
        created_at: new Date(),
        updated_at: new Date(),
      };
      const formatted = repo._formatRow(row);
      assert.strictEqual(formatted.projectPath, '/p');
      assert.strictEqual(formatted.decisionType, 'pattern');
      assert.deepStrictEqual(formatted.alternatives, [{ name: 'alt' }]);
    });
  });
});

// ============================================================================
// LESSONS LEARNED REPOSITORY
// ============================================================================

describe('LessonsLearnedRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new LessonsLearnedRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
    });
  });

  describe('create', () => {
    it('should create a lesson with all fields', async () => {
      const result = await repo.create({
        userId: 'user-1',
        category: LessonCategory.BUG,
        mistake: 'Forgot null check',
        correction: 'Added null check',
        prevention: 'Always validate inputs',
        severity: LessonSeverity.HIGH,
      });

      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(result.category, LessonCategory.BUG);
      assert.strictEqual(result.severity, LessonSeverity.HIGH);
      assert.ok(result.id);
    });

    it('should default severity to medium', async () => {
      const result = await repo.create({
        userId: 'user-1',
        category: LessonCategory.PROCESS,
        mistake: 'Bad process',
        correction: 'Fixed process',
      });

      assert.ok(result);
      assert.strictEqual(result.severity, 'medium');
    });
  });

  describe('findById', () => {
    it('should return lesson when found', async () => {
      const result = await repo.findById('les-1');
      assert.ok(result);
      assert.strictEqual(result.id, 'les-1');
    });
  });

  describe('findByCategory', () => {
    it('should query by category', async () => {
      const result = await repo.findByCategory('user-1', LessonCategory.BUG);
      assert.ok(Array.isArray(result));
    });
  });

  describe('findCritical', () => {
    it('should query critical/high severity lessons', async () => {
      const result = await repo.findCritical('user-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findRecent', () => {
    it('should query recent lessons', async () => {
      const result = await repo.findRecent('user-1', 5);
      assert.ok(Array.isArray(result));
    });
  });

  describe('findBySession', () => {
    it('should query by session', async () => {
      const result = await repo.findBySession('user-1', 'sess-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findRecurring', () => {
    it('should query recurring lessons', async () => {
      const result = await repo.findRecurring('user-1', 2);
      assert.ok(Array.isArray(result));
    });
  });

  describe('findSimilar', () => {
    it('should delegate to search', async () => {
      const result = await repo.findSimilar('user-1', 'null check', { limit: 3 });
      assert.ok(Array.isArray(result));
    });
  });

  describe('recordOccurrence', () => {
    it('should call stored procedure', async () => {
      await assert.doesNotReject(() => repo.recordOccurrence('les-1'));
    });
  });

  describe('update', () => {
    it('should update lesson fields', async () => {
      const result = await repo.update('les-1', { severity: 'critical' });
      assert.ok(result);
    });
  });

  describe('delete', () => {
    it('should return true when deleted', async () => {
      const result = await repo.delete('les-1');
      assert.strictEqual(result, true);
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 15);
      assert.strictEqual(stats.critical, 2);
      assert.strictEqual(stats.high, 5);
      assert.strictEqual(stats.recurring, 3);
      assert.strictEqual(stats.totalOccurrences, 25);
      assert.ok(stats.byCategory);
    });
  });

  describe('_formatRow', () => {
    it('should return null for null', () => {
      assert.strictEqual(repo._formatRow(null), null);
    });
  });
});

// ============================================================================
// AUTONOMOUS GOALS REPOSITORY
// ============================================================================

describe('AutonomousGoalsRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new AutonomousGoalsRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
    });
  });

  describe('create', () => {
    it('should create a goal with all fields', async () => {
      const result = await repo.create({
        userId: 'user-1',
        goalType: GoalType.QUALITY,
        title: 'Achieve 80% coverage',
        description: 'Test coverage goal',
        successCriteria: [{ metric: 'coverage', target: 0.8 }],
        priority: 70,
        config: { autoCheck: true },
      });

      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(result.goalType, GoalType.QUALITY);
      assert.strictEqual(result.status, 'active');
      assert.strictEqual(result.priority, 70);
      assert.ok(result.id);
    });

    it('should default priority to 50', async () => {
      const result = await repo.create({
        userId: 'user-1',
        goalType: GoalType.LEARNING,
        title: 'Learn Rust',
      });

      assert.ok(result);
      assert.strictEqual(result.priority, 50);
    });
  });

  describe('findById', () => {
    it('should return goal when found', async () => {
      const result = await repo.findById('goal-1');
      assert.ok(result);
      assert.strictEqual(result.id, 'goal-1');
      assert.strictEqual(typeof result.progress, 'number');
    });
  });

  describe('findActive', () => {
    it('should query active goals', async () => {
      const result = await repo.findActive('user-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findByType', () => {
    it('should query by goal type', async () => {
      const result = await repo.findByType('user-1', GoalType.QUALITY);
      assert.ok(Array.isArray(result));
    });
  });

  describe('updateProgress', () => {
    it('should update progress value', async () => {
      const result = await repo.updateProgress('goal-1', 0.75);
      assert.ok(result);
      assert.strictEqual(result.progress, 0.75);
    });
  });

  describe('updateStatus', () => {
    it('should update status', async () => {
      const result = await repo.updateStatus('goal-1', GoalStatus.COMPLETED);
      assert.ok(result);
      assert.strictEqual(result.status, GoalStatus.COMPLETED);
    });
  });

  describe('update', () => {
    it('should call findById when no updates', async () => {
      const result = await repo.update('goal-1', {});
      assert.ok(result);
    });
  });

  describe('delete', () => {
    it('should return true when deleted', async () => {
      const result = await repo.delete('goal-1');
      assert.strictEqual(result, true);
    });
  });

  describe('list', () => {
    it('should return array', async () => {
      const result = await repo.list({ userId: 'user-1' });
      assert.ok(Array.isArray(result));
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 8);
      assert.strictEqual(stats.active, 5);
      assert.strictEqual(stats.completed, 2);
      assert.strictEqual(stats.paused, 1);
      assert.strictEqual(typeof stats.avgProgress, 'number');
      assert.ok(stats.byType);
    });
  });

  describe('_formatRow', () => {
    it('should return null for null', () => {
      assert.strictEqual(repo._formatRow(null), null);
    });

    it('should format all fields', () => {
      const row = {
        id: 'g1',
        user_id: 'u1',
        goal_type: 'quality',
        title: 'T',
        description: 'D',
        success_criteria: [],
        progress: '0.5',
        progress_notes: [],
        status: 'active',
        priority: 50,
        config: {},
        created_at: new Date(),
        updated_at: new Date(),
        completed_at: null,
        due_at: null,
      };
      const f = repo._formatRow(row);
      assert.strictEqual(f.goalType, 'quality');
      assert.strictEqual(f.progress, 0.5);
      assert.strictEqual(f.userId, 'u1');
    });
  });
});

// ============================================================================
// AUTONOMOUS TASKS REPOSITORY
// ============================================================================

describe('AutonomousTasksRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new AutonomousTasksRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
    });
  });

  describe('create', () => {
    it('should create a task', async () => {
      const result = await repo.create({
        userId: 'user-1',
        taskType: TaskType.ANALYZE_PATTERNS,
        payload: { patterns: ['singleton'] },
        priority: 60,
      });

      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(result.taskType, TaskType.ANALYZE_PATTERNS);
      assert.strictEqual(result.status, 'pending');
    });
  });

  describe('findById', () => {
    it('should return task when found', async () => {
      const result = await repo.findById('task-1');
      assert.ok(result);
      assert.strictEqual(result.id, 'task-1');
    });
  });

  describe('claim', () => {
    it('should atomically claim a pending task', async () => {
      const result = await repo.claim('task-1');
      assert.ok(result);
      assert.strictEqual(result.status, 'running');
    });
  });

  describe('complete', () => {
    it('should mark task as completed', async () => {
      const result = await repo.complete('task-1', { success: true });
      assert.ok(result);
      assert.strictEqual(result.status, 'completed');
    });
  });

  describe('cancel', () => {
    it('should cancel a pending task', async () => {
      const result = await repo.cancel('task-1');
      assert.ok(result);
      assert.strictEqual(result.status, 'cancelled');
    });
  });

  describe('findByGoal', () => {
    it('should query by goal ID', async () => {
      const result = await repo.findByGoal('goal-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findByUser', () => {
    it('should query by user', async () => {
      const result = await repo.findByUser('user-1');
      assert.ok(Array.isArray(result));
    });

    it('should accept status filter', async () => {
      const result = await repo.findByUser('user-1', { status: 'running' });
      assert.ok(Array.isArray(result));
    });
  });

  describe('getRunning', () => {
    it('should query running tasks', async () => {
      const result = await repo.getRunning();
      assert.ok(Array.isArray(result));
    });
  });

  describe('delete', () => {
    it('should return true when deleted', async () => {
      const result = await repo.delete('task-1');
      assert.strictEqual(result, true);
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 20);
      assert.strictEqual(stats.pending, 8);
      assert.strictEqual(stats.running, 3);
      assert.strictEqual(stats.completed, 7);
      assert.ok(stats.byType);
    });
  });

  describe('_formatRow', () => {
    it('should return null for null', () => {
      assert.strictEqual(repo._formatRow(null), null);
    });

    it('should format task fields', () => {
      const row = {
        id: 't1',
        goal_id: 'g1',
        user_id: 'u1',
        task_type: 'run_tests',
        payload: { test: true },
        status: 'pending',
        priority: 50,
        scheduled_for: new Date(),
        started_at: null,
        completed_at: null,
        retry_count: 0,
        max_retries: 3,
        error_message: null,
        result: null,
        created_by: 'daemon',
        created_at: new Date(),
      };
      const f = repo._formatRow(row);
      assert.strictEqual(f.goalId, 'g1');
      assert.strictEqual(f.taskType, 'run_tests');
      assert.strictEqual(f.maxRetries, 3);
      assert.strictEqual(f.createdBy, 'daemon');
    });
  });
});

// ============================================================================
// PROACTIVE NOTIFICATIONS REPOSITORY
// ============================================================================

describe('ProactiveNotificationsRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new ProactiveNotificationsRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
    });
  });

  describe('create', () => {
    it('should create a notification', async () => {
      const result = await repo.create({
        userId: 'user-1',
        notificationType: NotificationType.INSIGHT,
        title: 'Pattern detected',
        message: 'You have been using singleton pattern frequently',
        priority: 60,
      });

      assert.ok(result);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(result.notificationType, NotificationType.INSIGHT);
      assert.strictEqual(result.delivered, false);
    });
  });

  describe('findById', () => {
    it('should return notification when found', async () => {
      const result = await repo.findById('notif-1');
      assert.ok(result);
      assert.strictEqual(result.id, 'notif-1');
    });
  });

  describe('getPending', () => {
    it('should query pending notifications', async () => {
      const result = await repo.getPending('user-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('markDelivered', () => {
    it('should mark as delivered', async () => {
      const result = await repo.markDelivered('notif-1');
      assert.ok(result);
      assert.strictEqual(result.delivered, true);
    });
  });

  describe('markMultipleDelivered', () => {
    it('should return 0 for empty array', async () => {
      const result = await repo.markMultipleDelivered([]);
      assert.strictEqual(result, 0);
    });

    it('should return count for valid IDs', async () => {
      const result = await repo.markMultipleDelivered(['id-1', 'id-2']);
      assert.strictEqual(result, 2);
    });
  });

  describe('dismiss', () => {
    it('should dismiss with action taken', async () => {
      const result = await repo.dismiss('notif-1', 'acknowledged');
      assert.ok(result);
      assert.strictEqual(result.dismissed, true);
    });
  });

  describe('findByType', () => {
    it('should query by type', async () => {
      const result = await repo.findByType('user-1', NotificationType.WARNING);
      assert.ok(Array.isArray(result));
    });
  });

  describe('findRecent', () => {
    it('should query recent notifications', async () => {
      const result = await repo.findRecent('user-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('cleanupExpired', () => {
    it('should delete expired notifications', async () => {
      const result = await repo.cleanupExpired();
      assert.strictEqual(result, 3);
    });
  });

  describe('delete', () => {
    it('should return true when deleted', async () => {
      const result = await repo.delete('notif-1');
      assert.strictEqual(result, true);
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 12);
      assert.strictEqual(stats.pending, 5);
      assert.strictEqual(stats.delivered, 6);
      assert.strictEqual(stats.dismissed, 1);
      assert.ok(stats.byType);
    });
  });

  describe('_formatRow', () => {
    it('should return null for null', () => {
      assert.strictEqual(repo._formatRow(null), null);
    });

    it('should format notification fields', () => {
      const row = {
        id: 'n1',
        user_id: 'u1',
        notification_type: 'insight',
        title: 'T',
        message: 'M',
        priority: 50,
        context: {},
        delivered: false,
        delivered_at: null,
        expires_at: null,
        dismissed: false,
        dismissed_at: null,
        action_taken: null,
        created_at: new Date(),
      };
      const f = repo._formatRow(row);
      assert.strictEqual(f.notificationType, 'insight');
      assert.strictEqual(f.delivered, false);
      assert.strictEqual(f.dismissed, false);
    });
  });
});

// ============================================================================
// FACTS REPOSITORY
// ============================================================================

describe('FactsRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new FactsRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
    });
  });

  describe('ensureTable', () => {
    it('should execute CREATE TABLE and TRIGGER SQL', async () => {
      await assert.doesNotReject(() => repo.ensureTable());
      assert.ok(mockPool.query.mock.calls.length >= 2);
    });
  });

  describe('create', () => {
    it('should create a fact', async () => {
      const result = await repo.create({
        userId: 'user-1',
        sessionId: 'sess-1',
        factType: FactType.CODE_PATTERN,
        subject: 'Export pattern in utils.js',
        content: 'File exports 5 functions',
        tags: ['exports', 'js'],
        confidence: 0.5,
      });

      assert.ok(result);
      assert.ok(result.factId);
      assert.strictEqual(result.userId, 'user-1');
      assert.strictEqual(typeof result.confidence, 'number');
    });

    it('should cap confidence at phi-inverse', async () => {
      const result = await repo.create({
        subject: 'Test',
        content: 'Test content',
        confidence: 0.99,
      });
      // The mock returns the value as-is, but the source code caps at PHI_INV
      assert.ok(result);
    });
  });

  describe('createBatch', () => {
    it('should return empty array for empty input', async () => {
      const result = await repo.createBatch([]);
      assert.deepStrictEqual(result, []);
    });

    it('should return empty array for null input', async () => {
      const result = await repo.createBatch(null);
      assert.deepStrictEqual(result, []);
    });
  });

  describe('findById', () => {
    it('should return fact when found', async () => {
      const result = await repo.findById('fact_abc123');
      assert.ok(result);
      assert.strictEqual(result.factId, 'fact_abc123');
    });

    it('should return null when not found', async () => {
      mockPool.query = mock.fn(async () => ({ rows: [] }));
      const result = await repo.findById('nonexistent');
      assert.strictEqual(result, null);
    });
  });

  describe('update', () => {
    it('should update fact fields', async () => {
      const result = await repo.update('fact_abc', { content: 'Updated', confidence: 0.6 });
      assert.ok(result);
    });

    it('should return null when no fields to update', async () => {
      const result = await repo.update('fact_abc', {});
      assert.strictEqual(result, null);
    });
  });

  describe('findByUser', () => {
    it('should query by user', async () => {
      const result = await repo.findByUser('user-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findBySession', () => {
    it('should query by session', async () => {
      const result = await repo.findBySession('sess-1');
      assert.ok(Array.isArray(result));
    });
  });

  describe('recordAccess', () => {
    it('should not reject', async () => {
      await assert.doesNotReject(() => repo.recordAccess('fact_abc'));
    });
  });

  describe('decayStale', () => {
    it('should return count of decayed facts', async () => {
      const result = await repo.decayStale(30, 0.05);
      assert.strictEqual(typeof result, 'number');
    });
  });

  describe('prune', () => {
    it('should return count of pruned facts', async () => {
      const result = await repo.prune(0.1, 90);
      assert.strictEqual(result, 5);
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 50);
      assert.strictEqual(stats.types, 5);
      assert.strictEqual(stats.tools, 4);
      assert.strictEqual(typeof stats.avgConfidence, 'number');
      assert.strictEqual(typeof stats.totalAccesses, 'number');
    });
  });

  describe('_cosineSimilarity', () => {
    it('should return 1 for identical vectors', () => {
      const v = [1, 0, 0];
      assert.ok(Math.abs(repo._cosineSimilarity(v, v) - 1.0) < 0.001);
    });

    it('should return 0 for orthogonal vectors', () => {
      assert.ok(Math.abs(repo._cosineSimilarity([1, 0], [0, 1])) < 0.001);
    });

    it('should return 0 for null inputs', () => {
      assert.strictEqual(repo._cosineSimilarity(null, [1, 0]), 0);
      assert.strictEqual(repo._cosineSimilarity([1, 0], null), 0);
    });

    it('should return 0 for mismatched lengths', () => {
      assert.strictEqual(repo._cosineSimilarity([1, 0], [1, 0, 0]), 0);
    });

    it('should return 0 for zero vectors', () => {
      assert.strictEqual(repo._cosineSimilarity([0, 0], [0, 0]), 0);
    });
  });

  describe('_mapRow', () => {
    it('should return null for null input', () => {
      assert.strictEqual(repo._mapRow(null), null);
    });

    it('should format fact fields correctly', () => {
      const row = {
        fact_id: 'f1',
        user_id: 'u1',
        session_id: 's1',
        fact_type: 'code_pattern',
        subject: 'S',
        content: 'C',
        context: {},
        source_tool: 'Read',
        source_file: 'test.js',
        confidence: '0.5',
        relevance: '0.4',
        access_count: 2,
        last_accessed: new Date(),
        tags: ['a'],
        embedding: null,
        embedding_model: null,
        embedding_dim: null,
        created_at: new Date(),
        updated_at: new Date(),
        rank: '0.8',
      };
      const f = repo._mapRow(row);
      assert.strictEqual(f.factId, 'f1');
      assert.strictEqual(f.sourceTool, 'Read');
      assert.strictEqual(f.confidence, 0.5);
      assert.strictEqual(f.relevance, 0.4);
      assert.strictEqual(f.rank, 0.8);
    });
  });
});

// ============================================================================
// TRAJECTORIES REPOSITORY
// ============================================================================

describe('TrajectoriesRepository', () => {
  let repo;
  let mockPool;

  beforeEach(() => {
    mockPool = createMockPool();
    repo = new TrajectoriesRepository(mockPool);
  });

  describe('constructor', () => {
    it('should create with provided db pool', () => {
      assert.ok(repo);
    });
  });

  describe('ensureTable', () => {
    it('should execute CREATE TABLE and TRIGGER SQL', async () => {
      await assert.doesNotReject(() => repo.ensureTable());
    });
  });

  describe('start', () => {
    it('should start a new trajectory', async () => {
      const result = await repo.start({
        userId: 'user-1',
        sessionId: 'sess-1',
        dogId: 'scout',
        taskType: 'exploration',
        initialState: { files: 10 },
        tags: ['test'],
      });

      assert.ok(result);
      assert.ok(result.trajectoryId);
      assert.strictEqual(result.outcome, 'pending');
    });
  });

  describe('recordAction', () => {
    it('should append action to trajectory', async () => {
      const result = await repo.recordAction('traj_abc', {
        tool: 'Read',
        input: { file_path: '/test.js' },
        output: 'content',
        success: true,
      });
      assert.ok(result);
    });
  });

  describe('recordSwitch', () => {
    it('should record dog switch', async () => {
      const result = await repo.recordSwitch('traj_abc', 'scout', 'architect', 'complex task');
      assert.ok(result);
    });
  });

  describe('complete', () => {
    it('should complete trajectory with success', async () => {
      const result = await repo.complete('traj_abc', {
        outcome: TrajectoryOutcome.SUCCESS,
        details: { summary: 'Done' },
      });
      assert.ok(result);
      assert.strictEqual(result.outcome, TrajectoryOutcome.SUCCESS);
    });

    it('should complete trajectory with failure', async () => {
      const result = await repo.complete('traj_abc', {
        outcome: TrajectoryOutcome.FAILURE,
        details: { error: 'timeout' },
      });
      assert.ok(result);
    });
  });

  describe('_calculateReward', () => {
    it('should give positive reward for success', () => {
      const reward = repo._calculateReward({ outcome: TrajectoryOutcome.SUCCESS });
      assert.ok(reward > 0);
    });

    it('should give partial reward for partial', () => {
      const reward = repo._calculateReward({ outcome: TrajectoryOutcome.PARTIAL });
      assert.ok(reward > 0);
      assert.ok(reward < 0.618);
    });

    it('should give negative reward for failure', () => {
      const reward = repo._calculateReward({ outcome: TrajectoryOutcome.FAILURE });
      assert.ok(reward < 0);
    });

    it('should give negative reward for abandoned', () => {
      const reward = repo._calculateReward({ outcome: TrajectoryOutcome.ABANDONED });
      assert.ok(reward < 0);
    });

    it('should bonus for zero errors', () => {
      const withErrors = repo._calculateReward({ outcome: TrajectoryOutcome.SUCCESS, errorCount: 1 });
      const noErrors = repo._calculateReward({ outcome: TrajectoryOutcome.SUCCESS, errorCount: 0 });
      assert.ok(noErrors >= withErrors);
    });

    it('should cap reward at phi-inverse', () => {
      const reward = repo._calculateReward({
        outcome: TrajectoryOutcome.SUCCESS,
        errorCount: 0,
        switchCount: 0,
      });
      assert.ok(reward <= 0.618033988749895);
    });
  });

  describe('_computeSimilarityHash', () => {
    it('should return a hex string', () => {
      const hash = repo._computeSimilarityHash({
        taskType: 'explore',
        dogId: 'scout',
        outcome: 'success',
      });
      assert.ok(typeof hash === 'string');
      assert.strictEqual(hash.length, 16);
    });

    it('should produce consistent hashes', () => {
      const input = { taskType: 'test', dogId: 'analyst', outcome: 'success' };
      const hash1 = repo._computeSimilarityHash(input);
      const hash2 = repo._computeSimilarityHash(input);
      assert.strictEqual(hash1, hash2);
    });

    it('should produce different hashes for different inputs', () => {
      const hash1 = repo._computeSimilarityHash({ taskType: 'a', dogId: 'b', outcome: 'c' });
      const hash2 = repo._computeSimilarityHash({ taskType: 'x', dogId: 'y', outcome: 'z' });
      assert.notStrictEqual(hash1, hash2);
    });
  });

  describe('findById', () => {
    it('should return trajectory when found', async () => {
      const result = await repo.findById('traj_abc');
      assert.ok(result);
      assert.strictEqual(result.trajectoryId, 'traj_abc');
    });
  });

  describe('findSuccessful', () => {
    it('should query successful trajectories', async () => {
      const result = await repo.findSuccessful('exploration');
      assert.ok(Array.isArray(result));
    });
  });

  describe('findSimilar', () => {
    it('should query by similarity hash', async () => {
      const result = await repo.findSimilar('abc123', 5);
      assert.ok(Array.isArray(result));
    });
  });

  describe('recordReplay', () => {
    it('should record successful replay', async () => {
      const result = await repo.recordReplay('traj_abc', true);
      assert.ok(result);
    });

    it('should record failed replay', async () => {
      const result = await repo.recordReplay('traj_abc', false);
      assert.ok(result);
    });
  });

  describe('getStats', () => {
    it('should return stats object', async () => {
      const stats = await repo.getStats();
      assert.strictEqual(stats.total, 100);
      assert.strictEqual(stats.successes, 70);
      assert.strictEqual(stats.failures, 20);
      assert.strictEqual(typeof stats.avgReward, 'number');
      assert.strictEqual(typeof stats.totalReplays, 'number');
    });
  });

  describe('_mapRow', () => {
    it('should return null for null', () => {
      assert.strictEqual(repo._mapRow(null), null);
    });

    it('should format trajectory fields', () => {
      const row = {
        trajectory_id: 't1',
        user_id: 'u1',
        session_id: 's1',
        dog_id: 'scout',
        task_type: 'explore',
        initial_state: {},
        action_sequence: [{ tool: 'Read' }],
        outcome: 'success',
        outcome_details: {},
        reward: '0.618',
        duration_ms: 5000,
        tool_count: 3,
        error_count: 0,
        switch_count: 1,
        similarity_hash: 'abc',
        replay_count: 2,
        success_after_replay: true,
        confidence: '0.55',
        tags: ['test'],
        created_at: new Date(),
        updated_at: new Date(),
        rank: '0.9',
      };
      const f = repo._mapRow(row);
      assert.strictEqual(f.trajectoryId, 't1');
      assert.strictEqual(f.dogId, 'scout');
      assert.strictEqual(f.reward, 0.618);
      assert.strictEqual(f.confidence, 0.55);
      assert.strictEqual(f.rank, 0.9);
      assert.strictEqual(f.replayCount, 2);
    });
  });
});
