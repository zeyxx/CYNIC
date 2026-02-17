/**
 * CYNIC Persistence Integration
 * 
 * Connect Learning → Persistence:
 * - Learning events → PostgreSQL (learning_cycles, patterns tables)
 * - Patterns → VectorStore (HNSW) + PostgreSQL (patterns table)
 * - Proof → MerkleDAG / PoJChain
 *
 * NOW WITH REAL POSTGRESQL INTEGRATION:
 * - Uses LearningCyclesRepository for learning events
 * - Uses PatternRepository for learned patterns
 * - Uses VectorStore for semantic search
 *
 * @module @cynic/llm/persistence-integration
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { getVectorStore, PostgresClient, getPool } from '@cynic/persistence';
import { LearningCyclesRepository, PatternRepository } from '@cynic/persistence';

const log = createLogger('PersistenceIntegration');

/**
 * Learning Persistence - connects LearningEngine to storage
 * 
 * NOW WITH REAL POSTGRESQL CONNECTIONS:
 * - LearningCyclesRepository for learning cycle records
 * - PatternRepository for learned patterns
 * - VectorStore for semantic pattern search
 */
export class LearningPersistence {
  constructor(options = {}) {
    this.vectorStore = options.vectorStore || null;
    this.db = options.db || null; // PostgresClient
    this.pojChain = options.pojChain || null;
    
    // Repositories (initialized after db connection)
    this.learningCycles = null;
    this.patterns = null;
    
    // Config
    this.batchSize = options.batchSize || 100;
    this.flushInterval = options.flushInterval || 60000; // 1 minute
    
    // Buffer for learning events
    this.buffer = [];
    this.lastFlush = Date.now();
  }

  /**
   * Initialize connections to PostgreSQL and VectorStore
   */
  async initialize() {
    log.info('Initializing persistence connections');
    
    // Initialize VectorStore for semantic search
    try {
      this.vectorStore = getVectorStore({
        embedder: 'auto', // Auto-detect Ollama or OpenAI
        dimensions: 768,
      });
      log.info('VectorStore connected');
    } catch (e) {
      log.warn('VectorStore not available', { error: e.message });
    }

    // Initialize PostgreSQL connection and repositories
    try {
      // Try to get existing pool or create new one
      this.db = getPool();
      
      // Initialize repositories
      this.learningCycles = new LearningCyclesRepository(this.db);
      this.patterns = new PatternRepository(this.db);
      
      // Test connection
      await this.db.query('SELECT 1');
      log.info('PostgreSQL connected and repositories initialized');
    } catch (e) {
      log.warn('PostgreSQL not available', { error: e.message });
      // Fall back to in-memory if no PostgreSQL
      this.db = null;
      this.learningCycles = null;
      this.patterns = null;
    }
  }

  /**
   * Save learning event to persistence
   * 
   * NOW SAVES TO REAL POSTGRESQL:
   * - Learning cycle record
   * - Pattern record (if applicable)
   * - VectorStore index for semantic search
   */
  async saveEvent(event) {
    this.buffer.push(event);
    
    // Flush if buffer full
    if (this.buffer.length >= this.batchSize) {
      await this.flush();
    }
    
    // Flush if interval exceeded
    if (Date.now() - this.lastFlush > this.flushInterval) {
      await this.flush();
    }
    
    // Index in VectorStore if available
    if (this.vectorStore && event.data?.prompt) {
      await this._indexEvent(event);
    }
  }

  /**
   * Index event in VectorStore for semantic search
   * @private
   */
  async _indexEvent(event) {
    try {
      // Generate embedding using VectorStore's embedder
      const embedding = await this.vectorStore._getEmbedding(event.data.prompt);
      
      await this.vectorStore.store(event.id, event.data.prompt, {
        type: 'learning_event',
        adapter: event.adapter,
        quality: event.data.quality || 0,
        timestamp: event.timestamp,
      });
      
      log.debug('Event indexed in VectorStore', { id: event.id, adapter: event.adapter });
    } catch (e) {
      log.warn('Failed to index event', { error: e.message });
    }
  }

  /**
   * Flush buffer to PostgreSQL database
   * NOW USES REAL LearningCyclesRepository
   */
  async flush() {
    if (this.buffer.length === 0) return;
    
    const events = this.buffer.splice(0, this.buffer.length);
    this.lastFlush = Date.now();
    
    log.info('Flushing learning events', { count: events.length });
    
    // Save to PostgreSQL via LearningCyclesRepository
    if (this.learningCycles) {
      try {
        for (const event of events) {
          await this.learningCycles.record({
            cycleId: event.id,
            feedbackProcessed: event.data?.feedbackProcessed || 1,
            patternsUpdated: event.data?.patternsUpdated || 0,
            patternsMerged: event.data?.patternsMerged || 0,
            weightsAdjusted: event.data?.weightsAdjusted || 0,
            thresholdsAdjusted: event.data?.thresholdsAdjusted || 0,
            avgWeightDelta: event.data?.avgWeightDelta,
            avgThresholdDelta: event.data?.avgThresholdDelta,
            durationMs: event.duration,
          });
        }
        log.debug('Events saved to PostgreSQL', { count: events.length });
      } catch (e) {
        log.error('Failed to save events to PostgreSQL', { error: e.message });
        // Put back in buffer on failure
        this.buffer.unshift(...events);
      }
    } else {
      log.warn('No PostgreSQL repository available, events not persisted');
    }
  }

  /**
   * Save a learned pattern to PostgreSQL
   * 
   * @param {Object} pattern - Pattern data
   * @returns {Promise<Object>} Saved pattern
   */
  async savePattern(pattern) {
    if (!this.patterns) {
      log.warn('Pattern repository not available');
      return null;
    }
    
    try {
      const saved = await this.patterns.upsert({
        category: pattern.category || 'general',
        name: pattern.name,
        description: pattern.description,
        confidence: pattern.confidence,
        frequency: pattern.frequency || 1,
        sourceJudgments: pattern.sourceJudgments || [],
        sourceCount: pattern.sourceCount || 1,
        tags: pattern.tags || [],
        data: pattern.data || {},
      });
      
      // Also index in VectorStore for semantic search
      if (this.vectorStore && pattern.name) {
        await this.vectorStore.store(
          saved.pattern_id,
          `${pattern.name}: ${pattern.description || ''}`,
          {
            type: 'pattern',
            category: pattern.category,
            confidence: pattern.confidence,
          }
        );
      }
      
      log.info('Pattern saved', { 
        id: saved.pattern_id, 
        category: saved.category,
        confidence: saved.confidence,
      });
      
      return saved;
    } catch (e) {
      log.error('Failed to save pattern', { error: e.message });
      return null;
    }
  }

  /**
   * Search similar patterns by query
   * 
   * Uses VectorStore for semantic similarity search
   */
  async searchSimilar(query, options = {}) {
    if (!this.vectorStore) {
      log.warn('VectorStore not available for pattern search');
      return [];
    }
    
    try {
      const results = await this.vectorStore.search(query, options.k || 5, {
        minScore: options.minScore || 0.3,
        filter: (meta) => meta.type === 'pattern',
      });
      
      return results;
    } catch (e) {
      log.error('Failed to search patterns', { error: e.message });
      return [];
    }
  }

  /**
   * Get patterns by category from PostgreSQL
   * 
   * @param {string} category - Pattern category
   * @param {number} limit - Max results
   * @returns {Promise<Array>} Patterns
   */
  async getPatternsByCategory(category, limit = 10) {
    if (!this.patterns) {
      return [];
    }
    
    try {
      return await this.patterns.findByCategory(category, limit);
    } catch (e) {
      log.error('Failed to get patterns by category', { error: e.message });
      return [];
    }
  }

  /**
   * Get recent learning cycles from PostgreSQL
   * 
   * @param {number} limit - Max results
   * @returns {Promise<Array>} Learning cycles
   */
  async getRecentCycles(limit = 10) {
    if (!this.learningCycles) {
      return [];
    }
    
    try {
      return await this.learningCycles.getRecent(limit);
    } catch (e) {
      log.error('Failed to get recent cycles', { error: e.message });
      return [];
    }
  }

  /**
   * Get learning statistics from PostgreSQL
   * 
   * @param {number} days - Days to look back
   * @returns {Promise<Object>} Statistics
   */
  async getStats(days = 7) {
    if (!this.learningCycles) {
      return null;
    }
    
    try {
      return await this.learningCycles.getStats(days);
    } catch (e) {
      log.error('Failed to get learning stats', { error: e.message });
      return null;
    }
  }

  /**
   * Create proof of learning event (PoJ)
   */
  async createProof(event) {
    if (!this.pojChain) {
      log.warn('PoJChain not available');
      return null;
    }
    
    try {
      const block = await this.pojChain.createBlock({
        type: 'learning_event',
        data: event,
      });
      
      log.info('Learning proof created', { block: block.hash });
      return block;
    } catch (e) {
      log.error('Failed to create proof', { error: e.message });
      return null;
    }
  }

  /**
   * Close connections
   */
  async close() {
    await this.flush();
    
    // Close PostgreSQL connection
    if (this.db) {
      try {
        await this.db.close();
        log.info('PostgreSQL connection closed');
      } catch (e) {
        log.warn('Error closing PostgreSQL', { error: e.message });
      }
    }
    
    log.info('Persistence connections closed');
  }
}

/**
 * Create LearningPersistence instance with automatic initialization
 */
export async function createLearningPersistence(options = {}) {
  const persistence = new LearningPersistence(options);
  await persistence.initialize();
  return persistence;
}

/**
 * @deprecated Use createLearningPersistence instead
 */
export function createLearningPersistenceSync(options) {
  return new LearningPersistence(options);
}

export default LearningPersistence;
