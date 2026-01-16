/**
 * MCP Persistence Integration
 *
 * UNIFIED persistence layer with automatic fallback:
 * 1. PostgreSQL + Redis (production)
 * 2. File-based (local development)
 * 3. In-memory (tests/ephemeral)
 *
 * Same API regardless of backend - "φ qui se méfie de φ"
 *
 * @module @cynic/mcp/persistence
 */

'use strict';

import fs from 'fs/promises';
import path from 'path';
import {
  PostgresClient,
  RedisClient,
  JudgmentRepository,
  PatternRepository,
  FeedbackRepository,
  KnowledgeRepository,
  PoJBlockRepository,
  SessionStore,
} from '@cynic/persistence';

/**
 * In-memory fallback storage
 * Used when neither PostgreSQL nor file storage is configured
 */
class MemoryStore {
  constructor() {
    this.judgments = [];
    this.patterns = [];
    this.feedback = [];
    this.knowledge = [];
  }

  async storeJudgment(judgment) {
    const id = `jdg_${Date.now().toString(36)}`;
    const stored = { ...judgment, judgment_id: id, created_at: new Date() };
    this.judgments.push(stored);
    // Keep bounded
    if (this.judgments.length > 1000) this.judgments.shift();
    return stored;
  }

  async searchJudgments(query, options = {}) {
    const { limit = 10 } = options;
    if (!query) return this.judgments.slice(-limit);
    const q = query.toLowerCase();
    return this.judgments
      .filter(j => JSON.stringify(j).toLowerCase().includes(q))
      .slice(-limit);
  }

  async findRecentJudgments(limit = 10) {
    return this.judgments.slice(-limit);
  }

  async getJudgmentStats() {
    const total = this.judgments.length;
    if (total === 0) return { total: 0, avgScore: 0, avgConfidence: 0, verdicts: {} };

    const avgScore = this.judgments.reduce((s, j) => s + (j.q_score || 0), 0) / total;
    const avgConfidence = this.judgments.reduce((s, j) => s + (j.confidence || 0), 0) / total;
    const verdicts = this.judgments.reduce((v, j) => {
      v[j.verdict] = (v[j.verdict] || 0) + 1;
      return v;
    }, {});

    return { total, avgScore, avgConfidence, verdicts };
  }

  async storeFeedback(fb) {
    const id = `fb_${Date.now().toString(36)}`;
    const stored = { ...fb, feedback_id: id, created_at: new Date() };
    this.feedback.push(stored);
    return stored;
  }

  async storeKnowledge(k) {
    const id = `kn_${Date.now().toString(36)}`;
    const stored = { ...k, knowledge_id: id, created_at: new Date() };
    this.knowledge.push(stored);
    return stored;
  }

  async searchKnowledge(query, options = {}) {
    const { limit = 10 } = options;
    if (!query) return this.knowledge.slice(-limit);
    const q = query.toLowerCase();
    // Search in summary, content, and insights
    return this.knowledge
      .filter(k => {
        const summary = (k.summary || '').toLowerCase();
        const content = (k.content || '').toLowerCase();
        const insights = JSON.stringify(k.insights || []).toLowerCase();
        return summary.includes(q) || content.includes(q) || insights.includes(q);
      })
      .slice(-limit);
  }

  async upsertPattern(pattern) {
    const existing = this.patterns.find(p => p.name === pattern.name);
    if (existing) {
      Object.assign(existing, pattern, { updated_at: new Date() });
      return existing;
    }
    const id = `pat_${Date.now().toString(36)}`;
    const stored = { ...pattern, pattern_id: id, created_at: new Date() };
    this.patterns.push(stored);
    return stored;
  }

  async getPatterns(options = {}) {
    const { category, limit = 10 } = options;
    let result = this.patterns;
    if (category) {
      result = result.filter(p => p.category === category);
    }
    return result.slice(-limit);
  }

  async export() {
    return {
      judgments: this.judgments,
      patterns: this.patterns,
      feedback: this.feedback,
      knowledge: this.knowledge,
    };
  }

  async import(data) {
    if (data.judgments) this.judgments = data.judgments;
    if (data.patterns) this.patterns = data.patterns;
    if (data.feedback) this.feedback = data.feedback;
    if (data.knowledge) this.knowledge = data.knowledge;
  }
}

/**
 * File-based storage adapter
 * Uses MemoryStore + periodic file sync
 */
class FileStore extends MemoryStore {
  constructor(dataDir) {
    super();
    this.dataDir = dataDir;
    this.filePath = path.join(dataDir, 'cynic-state.json');
    this._dirty = false;
  }

  async initialize() {
    try {
      await fs.mkdir(this.dataDir, { recursive: true });
      const data = await fs.readFile(this.filePath, 'utf-8');
      await this.import(JSON.parse(data));
      console.error(`   File storage: loaded from ${this.filePath}`);
    } catch (err) {
      if (err.code !== 'ENOENT') {
        console.error(`   File storage: ${err.message}`);
      } else {
        console.error(`   File storage: fresh start at ${this.filePath}`);
      }
    }
  }

  async storeJudgment(judgment) {
    const result = await super.storeJudgment(judgment);
    this._dirty = true;
    return result;
  }

  async storeFeedback(fb) {
    const result = await super.storeFeedback(fb);
    this._dirty = true;
    return result;
  }

  async storeKnowledge(k) {
    const result = await super.storeKnowledge(k);
    this._dirty = true;
    return result;
  }

  async upsertPattern(pattern) {
    const result = await super.upsertPattern(pattern);
    this._dirty = true;
    return result;
  }

  async save() {
    if (!this._dirty) return;
    try {
      const data = await this.export();
      await fs.writeFile(this.filePath, JSON.stringify(data, null, 2));
      this._dirty = false;
    } catch (err) {
      console.error('Error saving state:', err.message);
    }
  }
}

/**
 * Unified Persistence Manager for MCP server
 *
 * Automatic fallback chain:
 * 1. PostgreSQL + Redis (if CYNIC_DATABASE_URL set)
 * 2. File-based (if dataDir provided)
 * 3. In-memory (always available)
 */
export class PersistenceManager {
  /**
   * @param {Object} [options] - Configuration options
   * @param {string} [options.dataDir] - Directory for file-based fallback
   */
  constructor(options = {}) {
    this.dataDir = options.dataDir || null;

    // PostgreSQL + Redis (primary)
    this.postgres = null;
    this.redis = null;
    this.sessionStore = null;

    // Repositories (PostgreSQL)
    this.judgments = null;
    this.patterns = null;
    this.feedback = null;
    this.knowledge = null;
    this.pojBlocks = null;

    // Fallback store (file or memory)
    this._fallback = null;
    this._backend = 'none'; // 'postgres', 'file', 'memory'

    this._initialized = false;
  }

  /**
   * Initialize all persistence connections
   * Fallback chain: PostgreSQL → File → Memory
   * @returns {Promise<PersistenceManager>}
   */
  async initialize() {
    if (this._initialized) return this;

    const hasPostgres = !!process.env.CYNIC_DATABASE_URL;
    const hasRedis = !!process.env.CYNIC_REDIS_URL;

    // Try PostgreSQL first (production)
    if (hasPostgres) {
      try {
        this.postgres = new PostgresClient();
        await this.postgres.connect();

        // Initialize repositories
        this.judgments = new JudgmentRepository(this.postgres);
        this.patterns = new PatternRepository(this.postgres);
        this.feedback = new FeedbackRepository(this.postgres);
        this.knowledge = new KnowledgeRepository(this.postgres);
        this.pojBlocks = new PoJBlockRepository(this.postgres);

        this._backend = 'postgres';
        console.error('   PostgreSQL: connected');
      } catch (err) {
        console.error(`   PostgreSQL: ${err.message}`);
        this.postgres = null;
      }
    } else {
      console.error('   PostgreSQL: not configured (CYNIC_DATABASE_URL not set)');
    }

    // Initialize Redis (optional, for caching/sessions)
    if (hasRedis) {
      try {
        this.redis = new RedisClient();
        await this.redis.connect();
        this.sessionStore = new SessionStore(this.redis);
        console.error('   Redis: connected');
      } catch (err) {
        console.error(`   Redis: ${err.message}`);
      }
    } else {
      console.error('   Redis: not configured (CYNIC_REDIS_URL not set)');
    }

    // Set up fallback storage if PostgreSQL not available
    if (!this.postgres) {
      if (this.dataDir) {
        // File-based fallback (persistent)
        this._fallback = new FileStore(this.dataDir);
        await this._fallback.initialize();
        this._backend = 'file';
      } else {
        // In-memory fallback (ephemeral)
        this._fallback = new MemoryStore();
        this._backend = 'memory';
        console.error('   Storage: in-memory (ephemeral)');
      }
    }

    this._initialized = true;
    return this;
  }

  /**
   * Store a judgment (PostgreSQL or fallback)
   */
  async storeJudgment(judgment) {
    // Use PostgreSQL if available
    if (this.judgments) {
      try {
        return await this.judgments.create(judgment);
      } catch (err) {
        console.error('Error storing judgment to PostgreSQL:', err.message);
      }
    }
    // Fallback to file/memory
    if (this._fallback) {
      return await this._fallback.storeJudgment(judgment);
    }
    return null;
  }

  /**
   * Search judgments (PostgreSQL or fallback)
   */
  async searchJudgments(query, options = {}) {
    if (this.judgments) {
      try {
        return await this.judgments.search(query, options);
      } catch (err) {
        console.error('Error searching judgments:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.searchJudgments(query, options);
    }
    return [];
  }

  /**
   * Get recent judgments (PostgreSQL or fallback)
   */
  async getRecentJudgments(limit = 10) {
    if (this.judgments) {
      try {
        return await this.judgments.findRecent(limit);
      } catch (err) {
        console.error('Error getting recent judgments:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.findRecentJudgments(limit);
    }
    return [];
  }

  /**
   * Get judgment statistics (PostgreSQL or fallback)
   */
  async getJudgmentStats(options = {}) {
    if (this.judgments) {
      try {
        return await this.judgments.getStats(options);
      } catch (err) {
        console.error('Error getting judgment stats:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getJudgmentStats();
    }
    return { total: 0, avgScore: 0, avgConfidence: 0, verdicts: {} };
  }

  /**
   * Store feedback (PostgreSQL or fallback)
   */
  async storeFeedback(feedback) {
    if (this.feedback) {
      try {
        return await this.feedback.create(feedback);
      } catch (err) {
        console.error('Error storing feedback:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.storeFeedback(feedback);
    }
    return null;
  }

  /**
   * Store knowledge (PostgreSQL or fallback)
   */
  async storeKnowledge(knowledge) {
    if (this.knowledge) {
      try {
        return await this.knowledge.create(knowledge);
      } catch (err) {
        console.error('Error storing knowledge:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.storeKnowledge(knowledge);
    }
    return null;
  }

  /**
   * Search knowledge (PostgreSQL or fallback)
   */
  async searchKnowledge(query, options = {}) {
    if (this.knowledge) {
      try {
        // Try FTS search first
        return await this.knowledge.search(query, options);
      } catch (err) {
        // If FTS fails (migration not run), try fallback search
        if (err.message.includes('search_vector') || err.message.includes('websearch_to_tsquery')) {
          console.error('FTS not available, using fallback search');
          try {
            return await this.knowledge.searchFallback(query, options);
          } catch (fallbackErr) {
            console.error('Error in fallback search:', fallbackErr.message);
          }
        } else {
          console.error('Error searching knowledge:', err.message);
        }
      }
    }
    if (this._fallback) {
      return await this._fallback.searchKnowledge(query, options);
    }
    return [];
  }

  /**
   * Upsert pattern (PostgreSQL or fallback)
   */
  async upsertPattern(pattern) {
    if (this.patterns) {
      try {
        return await this.patterns.upsert(pattern);
      } catch (err) {
        console.error('Error upserting pattern:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.upsertPattern(pattern);
    }
    return null;
  }

  /**
   * Get patterns (PostgreSQL or fallback)
   */
  async getPatterns(options = {}) {
    if (this.patterns) {
      const { category, limit = 10 } = options;
      try {
        if (category) {
          return await this.patterns.findByCategory(category, limit);
        }
        return await this.patterns.getTopPatterns(limit);
      } catch (err) {
        console.error('Error getting patterns:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getPatterns(options);
    }
    return [];
  }

  // ===========================================================================
  // PoJ CHAIN METHODS
  // ===========================================================================

  /**
   * Store a PoJ block (PostgreSQL only - no fallback for blockchain integrity)
   * @param {Object} block - Block to store
   * @returns {Promise<Object|null>} Stored block or null
   */
  async storePoJBlock(block) {
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.create(block);
      } catch (err) {
        console.error('Error storing PoJ block:', err.message);
      }
    }
    // No fallback for PoJ chain - requires PostgreSQL for integrity
    return null;
  }

  /**
   * Get the head (latest) PoJ block
   * @returns {Promise<Object|null>} Head block or null
   */
  async getPoJHead() {
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.getHead();
      } catch (err) {
        console.error('Error getting PoJ head:', err.message);
      }
    }
    return null;
  }

  /**
   * Get PoJ chain statistics
   * @returns {Promise<Object>} Chain statistics
   */
  async getPoJStats() {
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.getStats();
      } catch (err) {
        console.error('Error getting PoJ stats:', err.message);
      }
    }
    return { totalBlocks: 0, headSlot: 0, totalJudgments: 0 };
  }

  /**
   * Get recent PoJ blocks
   * @param {number} [limit=10] - Number of blocks
   * @returns {Promise<Object[]>} Recent blocks
   */
  async getRecentPoJBlocks(limit = 10) {
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.findRecent(limit);
      } catch (err) {
        console.error('Error getting recent PoJ blocks:', err.message);
      }
    }
    return [];
  }

  /**
   * Verify PoJ chain integrity
   * @returns {Promise<Object>} Verification result
   */
  async verifyPoJChain() {
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.verifyIntegrity();
      } catch (err) {
        console.error('Error verifying PoJ chain:', err.message);
      }
    }
    return { valid: true, blocksChecked: 0, errors: [] };
  }

  /**
   * Get health status
   */
  async health() {
    const status = {
      postgres: { status: 'not_configured' },
      redis: { status: 'not_configured' },
    };

    if (this.postgres) {
      try {
        const health = await this.postgres.health();
        status.postgres = health;
      } catch (err) {
        status.postgres = { status: 'unhealthy', error: err.message };
      }
    }

    if (this.redis) {
      try {
        const health = await this.redis.health();
        status.redis = health;
      } catch (err) {
        status.redis = { status: 'unhealthy', error: err.message };
      }
    }

    return status;
  }

  /**
   * Close all connections
   */
  async close() {
    // Save file-based fallback if active
    if (this._fallback?.save) {
      try {
        await this._fallback.save();
      } catch (err) {
        console.error('Error saving file storage:', err.message);
      }
    }

    if (this.postgres) {
      try {
        await this.postgres.close();
      } catch (err) {
        console.error('Error closing PostgreSQL:', err.message);
      }
    }

    if (this.redis) {
      try {
        await this.redis.close();
      } catch (err) {
        console.error('Error closing Redis:', err.message);
      }
    }

    this._initialized = false;
  }

  /**
   * Check if persistence is available
   */
  get isAvailable() {
    return this._initialized && (this.postgres || this.redis || this._fallback);
  }

  /**
   * Check if specific features are available
   */
  get capabilities() {
    // With fallback, all features are always available
    const hasFallback = !!this._fallback;
    return {
      judgments: !!this.judgments || hasFallback,
      patterns: !!this.patterns || hasFallback,
      feedback: !!this.feedback || hasFallback,
      knowledge: !!this.knowledge || hasFallback,
      pojChain: !!this.pojBlocks, // No fallback - requires PostgreSQL
      sessions: !!this.sessionStore,
      cache: !!this.redis,
    };
  }
}

export default PersistenceManager;
