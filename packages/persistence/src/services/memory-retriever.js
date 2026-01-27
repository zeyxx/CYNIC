/**
 * Memory Retriever Service
 *
 * Hybrid search combining FTS + vector similarity using φ-weighted scoring.
 * The core service for CYNIC's Total Memory system.
 *
 * φ-weighted: 0.382 FTS + 0.618 vector (golden ratio)
 *
 * @module @cynic/persistence/services/memory-retriever
 */

'use strict';

import { ConversationMemoriesRepository, MemoryType } from '../postgres/repositories/conversation-memories.js';
import { ArchitecturalDecisionsRepository, DecisionType, DecisionStatus } from '../postgres/repositories/architectural-decisions.js';
import { LessonsLearnedRepository, LessonCategory, LessonSeverity } from '../postgres/repositories/lessons-learned.js';

// Re-export types for convenience
export { MemoryType, DecisionType, DecisionStatus, LessonCategory, LessonSeverity };

/**
 * φ constants for search weighting
 */
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;     // Vector weight
const PHI_INV_2 = 0.381966011250105;   // FTS weight
const PHI_INV_3 = 0.236067977499790;   // Min relevance threshold

/**
 * Memory Retriever Service
 *
 * Provides unified interface for searching and managing CYNIC's memories.
 */
export class MemoryRetriever {
  /**
   * Create a new MemoryRetriever
   *
   * @param {Object} options - Options
   * @param {Object} options.pool - PostgreSQL connection pool
   * @param {Object} [options.embedder] - Embedding service (optional)
   */
  constructor(options = {}) {
    if (!options.pool) {
      throw new Error('MemoryRetriever requires a database pool');
    }

    this.pool = options.pool;
    this.embedder = options.embedder || null;

    // Initialize repositories
    this.memories = new ConversationMemoriesRepository(this.pool);
    this.decisions = new ArchitecturalDecisionsRepository(this.pool);
    this.lessons = new LessonsLearnedRepository(this.pool);

    // φ constants
    this.PHI_FTS = PHI_INV_2;
    this.PHI_VECTOR = PHI_INV;
    this.MIN_RELEVANCE = PHI_INV_3;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // UNIFIED SEARCH
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Search all memory types with unified results
   *
   * @param {string} userId - User ID
   * @param {string} query - Search query
   * @param {Object} [options] - Search options
   * @param {string[]} [options.sources] - Sources to search: 'memories', 'decisions', 'lessons'
   * @param {number} [options.limit=10] - Results per source
   * @param {boolean} [options.useVector=true] - Whether to use vector search
   * @returns {Promise<Object>} Results from all sources
   */
  async search(userId, query, options = {}) {
    const {
      sources = ['memories', 'decisions', 'lessons'],
      limit = 10,
      useVector = true,
    } = options;

    // Generate embedding if embedder available and vector search requested
    let embedding = null;
    if (useVector && this.embedder) {
      try {
        embedding = await this.embedder.embed(query);
      } catch (err) {
        console.warn('[MemoryRetriever] Embedding failed, falling back to FTS:', err.message);
      }
    }

    const results = {
      query,
      timestamp: Date.now(),
      sources: {},
    };

    // Search each source in parallel
    const searches = [];

    if (sources.includes('memories')) {
      searches.push(
        this.memories.search(userId, query, {
          embedding,
          limit,
          minRelevance: this.MIN_RELEVANCE,
        }).then(r => { results.sources.memories = r; })
      );
    }

    if (sources.includes('decisions')) {
      searches.push(
        this.decisions.search(userId, query, {
          embedding,
          limit,
        }).then(r => { results.sources.decisions = r; })
      );
    }

    if (sources.includes('lessons')) {
      searches.push(
        this.lessons.search(userId, query, {
          embedding,
          limit,
        }).then(r => { results.sources.lessons = r; })
      );
    }

    await Promise.all(searches);

    // Record access to retrieved memories
    const memoryIds = (results.sources.memories || []).map(m => m.id);
    if (memoryIds.length > 0) {
      await this.memories.recordAccess(memoryIds);
    }

    return results;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CONTEXT RETRIEVAL
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get relevant context for current task
   *
   * Retrieves memories, decisions, and lessons relevant to the current context.
   * Called at session start or when context changes.
   *
   * @param {string} userId - User ID
   * @param {Object} context - Current context
   * @param {string} [context.projectPath] - Current project path
   * @param {string[]} [context.recentTopics] - Recent conversation topics
   * @param {string} [context.currentTask] - Description of current task
   * @returns {Promise<Object>} Relevant context
   */
  async getRelevantContext(userId, context = {}) {
    const { projectPath, recentTopics = [], currentTask } = context;

    const result = {
      memories: [],
      decisions: [],
      lessons: [],
      timestamp: Date.now(),
    };

    // Build search query from context
    const queryParts = [];
    if (currentTask) queryParts.push(currentTask);
    if (recentTopics.length > 0) queryParts.push(recentTopics.join(' '));

    const query = queryParts.join(' ').trim();

    // Get relevant memories if we have a query
    if (query) {
      const searchResults = await this.search(userId, query, {
        sources: ['memories', 'lessons'],
        limit: 5,
        useVector: true,
      });
      result.memories = searchResults.sources.memories || [];
      result.lessons = searchResults.sources.lessons || [];
    }

    // Always get project-specific decisions
    if (projectPath) {
      result.decisions = await this.decisions.findByProject(userId, projectPath, {
        status: 'active',
        limit: 10,
      });
    }

    // Get critical lessons (should never repeat)
    const criticalLessons = await this.lessons.findCritical(userId, 5);
    result.criticalLessons = criticalLessons;

    // Get important memories
    const importantMemories = await this.memories.findImportant(userId, 0.7, 5);
    result.importantMemories = importantMemories;

    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MEMORY OPERATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Store a conversation memory
   *
   * @param {string} userId - User ID
   * @param {string} memoryType - Type of memory
   * @param {string} content - Memory content
   * @param {Object} [options] - Additional options
   * @returns {Promise<Object>} Created memory
   */
  async rememberConversation(userId, memoryType, content, options = {}) {
    const { sessionId, importance = 0.5, context = {} } = options;

    // Generate embedding if available
    let embedding = null;
    if (this.embedder) {
      try {
        embedding = await this.embedder.embed(content);
      } catch (err) {
        console.warn('[MemoryRetriever] Embedding failed:', err.message);
      }
    }

    return this.memories.create({
      userId,
      sessionId,
      memoryType,
      content,
      embedding,
      importance,
      context,
    });
  }

  /**
   * Store an architectural decision
   *
   * @param {string} userId - User ID
   * @param {Object} decision - Decision data
   * @returns {Promise<Object>} Created decision
   */
  async rememberDecision(userId, decision) {
    // Generate embedding if available
    let embedding = null;
    if (this.embedder) {
      try {
        const text = `${decision.title} ${decision.description} ${decision.rationale || ''}`;
        embedding = await this.embedder.embed(text);
      } catch (err) {
        console.warn('[MemoryRetriever] Embedding failed:', err.message);
      }
    }

    return this.decisions.create({
      userId,
      projectPath: decision.projectPath,
      decisionType: decision.decisionType || 'other',
      title: decision.title,
      description: decision.description,
      rationale: decision.rationale,
      alternatives: decision.alternatives,
      consequences: decision.consequences,
      embedding,
      status: 'active',
    });
  }

  /**
   * Store a lesson learned
   *
   * @param {string} userId - User ID
   * @param {Object} lesson - Lesson data
   * @returns {Promise<Object>} Created lesson
   */
  async rememberLesson(userId, lesson) {
    // Generate embedding if available
    let embedding = null;
    if (this.embedder) {
      try {
        const text = `${lesson.mistake} ${lesson.correction} ${lesson.prevention || ''}`;
        embedding = await this.embedder.embed(text);
      } catch (err) {
        console.warn('[MemoryRetriever] Embedding failed:', err.message);
      }
    }

    return this.lessons.create({
      userId,
      category: lesson.category || 'other',
      mistake: lesson.mistake,
      correction: lesson.correction,
      prevention: lesson.prevention,
      severity: lesson.severity || 'medium',
      embedding,
      sourceJudgmentId: lesson.sourceJudgmentId,
      sourceSessionId: lesson.sourceSessionId,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SELF-CORRECTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if we've made similar mistakes before
   *
   * Called before taking an action to prevent repeating mistakes.
   *
   * @param {string} userId - User ID
   * @param {string} context - Current context/action description
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Warning if similar mistake found
   */
  async checkForMistakes(userId, context, options = {}) {
    const { limit = 3 } = options;

    // Search for similar lessons
    let embedding = null;
    if (this.embedder) {
      try {
        embedding = await this.embedder.embed(context);
      } catch (err) {
        // Continue without embedding
      }
    }

    const similarLessons = await this.lessons.search(userId, context, {
      embedding,
      limit,
      severities: ['critical', 'high', 'medium'],
    });

    if (similarLessons.length === 0) {
      return { warning: false };
    }

    // Check relevance threshold
    const relevant = similarLessons.filter(l =>
      (l.combinedScore || 0) > this.MIN_RELEVANCE ||
      l.severity === 'critical' ||
      l.severity === 'high'
    );

    if (relevant.length === 0) {
      return { warning: false };
    }

    // Record occurrence of the most relevant lesson
    if (relevant[0].combinedScore > 0.5) {
      await this.lessons.recordOccurrence(relevant[0].id);
    }

    return {
      warning: true,
      lessons: relevant,
      message: this._formatWarning(relevant),
    };
  }

  /**
   * Format warning message from lessons
   * @private
   */
  _formatWarning(lessons) {
    if (lessons.length === 0) return '';

    const topLesson = lessons[0];
    let message = `*GROWL* Similar mistake detected:\n`;
    message += `Mistake: ${topLesson.mistake}\n`;
    message += `Prevention: ${topLesson.prevention || topLesson.correction}`;

    if (topLesson.severity === 'critical') {
      message = `*GROWL* CRITICAL: ${message}`;
    }

    return message;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get memory statistics
   *
   * @param {string} [userId] - Optional user ID filter
   * @returns {Promise<Object>} Statistics
   */
  async getStats(userId = null) {
    const [memoryStats, decisionStats, lessonStats] = await Promise.all([
      this.memories.getStats(userId),
      this.decisions.getStats(userId),
      this.lessons.getStats(userId),
    ]);

    return {
      memories: memoryStats,
      decisions: decisionStats,
      lessons: lessonStats,
      totals: {
        memories: memoryStats.total,
        decisions: decisionStats.total,
        lessons: lessonStats.total,
        combined: memoryStats.total + decisionStats.total + lessonStats.total,
      },
      hasEmbedder: !!this.embedder,
      phiConstants: {
        ftsWeight: this.PHI_FTS,
        vectorWeight: this.PHI_VECTOR,
        minRelevance: this.MIN_RELEVANCE,
      },
    };
  }
}

/**
 * Create a MemoryRetriever instance
 *
 * @param {Object} options - Options
 * @returns {MemoryRetriever}
 */
export function createMemoryRetriever(options) {
  return new MemoryRetriever(options);
}

export default MemoryRetriever;
