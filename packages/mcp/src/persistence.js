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
  LibraryCacheRepository,
  TriggerRepository,
  DiscoveryRepository,
  UserLearningProfilesRepository,
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
    this.pojBlocks = [];
    this.triggersState = null;
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

  async getJudgment(judgmentId) {
    return this.judgments.find(j => j.judgment_id === judgmentId) || null;
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
      pojBlocks: this.pojBlocks,
      triggersState: this.triggersState,
    };
  }

  async import(data) {
    if (data.judgments) this.judgments = data.judgments;
    if (data.patterns) this.patterns = data.patterns;
    if (data.feedback) this.feedback = data.feedback;
    if (data.knowledge) this.knowledge = data.knowledge;
    if (data.pojBlocks) this.pojBlocks = data.pojBlocks;
    if (data.triggersState) this.triggersState = data.triggersState;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PoJ CHAIN FALLBACK METHODS
  // "φ distrusts φ" - the chain must exist even without PostgreSQL
  // ═══════════════════════════════════════════════════════════════════════════

  async storePoJBlock(block) {
    // Validate chain integrity before storing
    if (this.pojBlocks.length > 0) {
      const head = this.pojBlocks[this.pojBlocks.length - 1];
      const expectedPrevHash = head.hash || head.block_hash;
      if (block.prev_hash !== expectedPrevHash) {
        throw new Error(`Chain integrity violation: expected prev_hash ${expectedPrevHash}, got ${block.prev_hash}`);
      }
      if (block.slot !== head.slot + 1) {
        throw new Error(`Slot mismatch: expected ${head.slot + 1}, got ${block.slot}`);
      }
    }

    const stored = {
      ...block,
      block_hash: block.hash,
      judgment_count: block.judgments?.length || 0,
      judgment_ids: block.judgments?.map(j => j.judgment_id) || [],
      created_at: new Date(),
    };
    this.pojBlocks.push(stored);

    // Keep bounded (but generous for chain integrity)
    if (this.pojBlocks.length > 10000) {
      // Archive old blocks - for memory, just remove oldest
      this.pojBlocks = this.pojBlocks.slice(-5000);
    }

    return stored;
  }

  async getPoJHead() {
    if (this.pojBlocks.length === 0) return null;
    return this.pojBlocks[this.pojBlocks.length - 1];
  }

  async getPoJStats() {
    const total = this.pojBlocks.length;
    if (total === 0) return { totalBlocks: 0, headSlot: 0, totalJudgments: 0 };

    const head = this.pojBlocks[total - 1];
    const totalJudgments = this.pojBlocks.reduce((sum, b) => sum + (b.judgment_count || 0), 0);

    return {
      totalBlocks: total,
      headSlot: head.slot,
      totalJudgments,
    };
  }

  async getRecentPoJBlocks(limit = 10) {
    return this.pojBlocks.slice(-limit).reverse();
  }

  async getPoJBlockBySlot(slot) {
    return this.pojBlocks.find(b => b.slot === slot) || null;
  }

  async verifyPoJChain() {
    const errors = [];

    for (let i = 1; i < this.pojBlocks.length; i++) {
      const block = this.pojBlocks[i];
      const prevBlock = this.pojBlocks[i - 1];
      const expectedPrevHash = prevBlock.hash || prevBlock.block_hash;

      if (block.prev_hash !== expectedPrevHash) {
        errors.push({
          slot: block.slot,
          error: `Invalid prev_hash: expected ${expectedPrevHash}, got ${block.prev_hash}`,
        });
      }

      if (block.slot !== prevBlock.slot + 1) {
        errors.push({
          slot: block.slot,
          error: `Slot gap: expected ${prevBlock.slot + 1}, got ${block.slot}`,
        });
      }
    }

    return {
      valid: errors.length === 0,
      blocksChecked: this.pojBlocks.length,
      errors,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRIGGERS STATE FALLBACK METHODS
  // "φ distrusts φ" - watchdogs must persist their rules
  // ═══════════════════════════════════════════════════════════════════════════

  async getTriggersState() {
    return this.triggersState;
  }

  async saveTriggersState(state) {
    this.triggersState = state;
    return state;
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

  async storePoJBlock(block) {
    const result = await super.storePoJBlock(block);
    this._dirty = true;
    // Force immediate save for PoJ chain integrity
    await this.save();
    return result;
  }

  async saveTriggersState(state) {
    const result = await super.saveTriggersState(state);
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
    this.libraryCache = null;
    this.triggers = null;
    this.discovery = null;
    this.userLearningProfiles = null;

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

    // Check for PostgreSQL config: URL or component env vars (host + password required)
    const hasPostgres = !!process.env.CYNIC_DATABASE_URL
      || (!!process.env.CYNIC_DB_HOST && !!process.env.CYNIC_DB_PASSWORD);
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
        this.libraryCache = new LibraryCacheRepository(this.postgres);
        this.triggers = new TriggerRepository(this.postgres);
        this.discovery = new DiscoveryRepository(this.postgres);
        this.userLearningProfiles = new UserLearningProfilesRepository(this.postgres);

        this._backend = 'postgres';
        console.error('   PostgreSQL: connected');
      } catch (err) {
        console.error(`   PostgreSQL: ${err.message}`);
        this.postgres = null;
      }
    } else {
      console.error('   PostgreSQL: not configured (set CYNIC_DATABASE_URL or CYNIC_DB_HOST + CYNIC_DB_PASSWORD)');
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
   * Get a judgment by ID (PostgreSQL or fallback)
   */
  async getJudgment(judgmentId) {
    // Use PostgreSQL if available
    if (this.judgments?.findById) {
      try {
        return await this.judgments.findById(judgmentId);
      } catch (err) {
        console.error('Error getting judgment from PostgreSQL:', err.message);
      }
    }
    // Fallback to file/memory
    if (this._fallback) {
      return await this._fallback.getJudgment(judgmentId);
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
  // "φ distrusts φ" - the chain must exist for CYNIC to verify itself
  // Fallback chain: PostgreSQL → File → Memory
  // ===========================================================================

  /**
   * Store a PoJ block (PostgreSQL or fallback)
   * @param {Object} block - Block to store
   * @returns {Promise<Object|null>} Stored block or null
   */
  async storePoJBlock(block) {
    // Use PostgreSQL if available
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.create(block);
      } catch (err) {
        console.error('Error storing PoJ block to PostgreSQL:', err.message);
      }
    }
    // Fallback to file/memory
    if (this._fallback) {
      try {
        return await this._fallback.storePoJBlock(block);
      } catch (err) {
        console.error('Error storing PoJ block to fallback:', err.message);
      }
    }
    return null;
  }

  /**
   * Get the head (latest) PoJ block (PostgreSQL or fallback)
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
    if (this._fallback) {
      return await this._fallback.getPoJHead();
    }
    return null;
  }

  /**
   * Get PoJ chain statistics (PostgreSQL or fallback)
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
    if (this._fallback) {
      return await this._fallback.getPoJStats();
    }
    return { totalBlocks: 0, headSlot: 0, totalJudgments: 0 };
  }

  /**
   * Get recent PoJ blocks (PostgreSQL or fallback)
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
    if (this._fallback) {
      return await this._fallback.getRecentPoJBlocks(limit);
    }
    return [];
  }

  /**
   * Get PoJ block by slot number (PostgreSQL or fallback)
   * @param {number} slot - Slot number
   * @returns {Promise<Object|null>} Block or null
   */
  async getPoJBlockBySlot(slot) {
    if (this.pojBlocks) {
      try {
        return await this.pojBlocks.findByNumber(slot);
      } catch (err) {
        console.error('Error getting PoJ block by slot:', err.message);
      }
    }
    if (this._fallback) {
      return await this._fallback.getPoJBlockBySlot(slot);
    }
    return null;
  }

  /**
   * Verify PoJ chain integrity (PostgreSQL or fallback)
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
    if (this._fallback) {
      return await this._fallback.verifyPoJChain();
    }
    return { valid: true, blocksChecked: 0, errors: [] };
  }

  // ===========================================================================
  // TRIGGER METHODS
  // "φ distrusts φ" - watchdogs must persist their rules
  // Primary: PostgreSQL → Fallback: File → Memory
  // ===========================================================================

  /**
   * Get all enabled triggers from DB
   * @returns {Promise<Object[]>} Enabled triggers
   */
  async getEnabledTriggers() {
    if (this.triggers) {
      try {
        return await this.triggers.findEnabled();
      } catch (err) {
        console.error('Error getting enabled triggers:', err.message);
      }
    }
    // Fallback: return empty array or from memory
    if (this._fallback?.triggersState?.triggers) {
      return this._fallback.triggersState.triggers.filter(t => t.enabled);
    }
    return [];
  }

  /**
   * Get all triggers from DB
   * @returns {Promise<Object[]>} All triggers
   */
  async getAllTriggers() {
    if (this.triggers) {
      try {
        return await this.triggers.findAll();
      } catch (err) {
        console.error('Error getting all triggers:', err.message);
      }
    }
    if (this._fallback?.triggersState?.triggers) {
      return this._fallback.triggersState.triggers;
    }
    return [];
  }

  /**
   * Create a new trigger
   * @param {Object} trigger - Trigger definition
   * @returns {Promise<Object|null>} Created trigger
   */
  async createTrigger(trigger) {
    if (this.triggers) {
      try {
        return await this.triggers.create(trigger);
      } catch (err) {
        console.error('Error creating trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Update a trigger
   * @param {string} triggerId - Trigger ID
   * @param {Object} updates - Updates to apply
   * @returns {Promise<Object|null>} Updated trigger
   */
  async updateTrigger(triggerId, updates) {
    if (this.triggers) {
      try {
        return await this.triggers.update(triggerId, updates);
      } catch (err) {
        console.error('Error updating trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Enable a trigger
   * @param {string} triggerId - Trigger ID
   * @returns {Promise<Object|null>} Updated trigger
   */
  async enableTrigger(triggerId) {
    if (this.triggers) {
      try {
        return await this.triggers.enable(triggerId);
      } catch (err) {
        console.error('Error enabling trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Disable a trigger
   * @param {string} triggerId - Trigger ID
   * @returns {Promise<Object|null>} Updated trigger
   */
  async disableTrigger(triggerId) {
    if (this.triggers) {
      try {
        return await this.triggers.disable(triggerId);
      } catch (err) {
        console.error('Error disabling trigger:', err.message);
      }
    }
    return null;
  }

  /**
   * Delete a trigger
   * @param {string} triggerId - Trigger ID
   * @returns {Promise<boolean>} Success
   */
  async deleteTrigger(triggerId) {
    if (this.triggers) {
      try {
        return await this.triggers.delete(triggerId);
      } catch (err) {
        console.error('Error deleting trigger:', err.message);
      }
    }
    return false;
  }

  /**
   * Record a trigger execution
   * @param {Object} execution - Execution details
   * @returns {Promise<Object|null>} Stored execution
   */
  async recordTriggerExecution(execution) {
    if (this.triggers) {
      try {
        return await this.triggers.recordExecution(execution);
      } catch (err) {
        console.error('Error recording trigger execution:', err.message);
      }
    }
    return null;
  }

  /**
   * Store a trigger event for pattern matching
   * @param {Object} event - Event data
   * @returns {Promise<Object|null>} Stored event
   */
  async storeTriggerEvent(event) {
    if (this.triggers) {
      try {
        return await this.triggers.storeEvent(event);
      } catch (err) {
        console.error('Error storing trigger event:', err.message);
      }
    }
    return null;
  }

  /**
   * Get trigger statistics
   * @returns {Promise<Object>} Trigger stats
   */
  async getTriggerStats() {
    if (this.triggers) {
      try {
        return await this.triggers.getStats();
      } catch (err) {
        console.error('Error getting trigger stats:', err.message);
      }
    }
    return { totalTriggers: 0, enabledTriggers: 0 };
  }

  /**
   * Get active triggers summary
   * @returns {Promise<Object[]>} Active triggers summary
   */
  async getActiveTriggersSummary() {
    if (this.triggers) {
      try {
        return await this.triggers.getActiveSummary();
      } catch (err) {
        console.error('Error getting active triggers summary:', err.message);
      }
    }
    return [];
  }

  /**
   * Check rate limit for a trigger
   * @param {string} triggerId - Trigger ID
   * @returns {Promise<number>} Executions in last minute
   */
  async getTriggerExecutionsLastMinute(triggerId) {
    if (this.triggers) {
      try {
        return await this.triggers.countExecutionsLastMinute(triggerId);
      } catch (err) {
        console.error('Error checking trigger rate limit:', err.message);
      }
    }
    return 0;
  }

  /**
   * Legacy: Get triggers state (for backward compatibility)
   * @deprecated Use getEnabledTriggers() instead
   * @returns {Promise<Object|null>} Triggers state
   */
  async getTriggersState() {
    // For backward compatibility with file/memory fallback
    if (this._fallback) {
      return await this._fallback.getTriggersState();
    }
    return null;
  }

  /**
   * Legacy: Save triggers state (for backward compatibility)
   * @deprecated Use createTrigger() or updateTrigger() instead
   * @param {Object} state - Triggers state
   * @returns {Promise<Object|null>} Saved state
   */
  async saveTriggersState(state) {
    // For backward compatibility with file/memory fallback
    if (this._fallback) {
      return await this._fallback.saveTriggersState(state);
    }
    return null;
  }

  // ===========================================================================
  // LIBRARY CACHE METHODS
  // ===========================================================================

  /**
   * Get cached library documentation
   * @param {string} libraryId - Library ID
   * @param {string} query - Search query
   * @returns {Promise<Object|null>} Cached content or null
   */
  async getLibraryDoc(libraryId, query) {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.get(libraryId, query);
      } catch (err) {
        console.error('Error getting library doc:', err.message);
      }
    }
    return null;
  }

  /**
   * Store library documentation in cache
   * @param {string} libraryId - Library ID
   * @param {string} query - Search query
   * @param {string} content - Documentation content
   * @param {Object} [metadata] - Additional metadata
   * @param {number} [ttlHours] - TTL in hours
   */
  async setLibraryDoc(libraryId, query, content, metadata = {}, ttlHours = 24) {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.set(libraryId, query, content, metadata, ttlHours);
      } catch (err) {
        console.error('Error setting library doc:', err.message);
      }
    }
    return null;
  }

  /**
   * Check if library documentation is cached
   * @param {string} libraryId - Library ID
   * @param {string} query - Search query
   * @returns {Promise<boolean>} True if cached
   */
  async isLibraryDocCached(libraryId, query) {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.isCached(libraryId, query);
      } catch (err) {
        console.error('Error checking library cache:', err.message);
      }
    }
    return false;
  }

  /**
   * Clean expired library cache entries
   * @returns {Promise<number>} Number of cleaned entries
   */
  async cleanExpiredCache() {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.cleanExpired();
      } catch (err) {
        console.error('Error cleaning cache:', err.message);
      }
    }
    return 0;
  }

  /**
   * Invalidate library cache
   * @param {string} libraryId - Library ID
   * @returns {Promise<number>} Number of invalidated entries
   */
  async invalidateLibraryCache(libraryId) {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.invalidate(libraryId);
      } catch (err) {
        console.error('Error invalidating cache:', err.message);
      }
    }
    return 0;
  }

  /**
   * Get library cache statistics
   * @returns {Promise<Object>} Cache statistics
   */
  async getLibraryCacheStats() {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.getStats();
      } catch (err) {
        console.error('Error getting cache stats:', err.message);
      }
    }
    return { totalEntries: 0, activeEntries: 0, uniqueLibraries: 0 };
  }

  /**
   * Get top cached libraries
   * @param {number} [limit=10] - Max libraries
   * @returns {Promise<Object[]>} Top libraries
   */
  async getTopCachedLibraries(limit = 10) {
    if (this.libraryCache) {
      try {
        return await this.libraryCache.getTopLibraries(limit);
      } catch (err) {
        console.error('Error getting top libraries:', err.message);
      }
    }
    return [];
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
   * Execute raw SQL query (PostgreSQL only)
   * Used by emergence detection and other analytical queries.
   * @param {string} sql - SQL query
   * @param {Array} [params] - Query parameters
   * @returns {Promise<{rows: Array}>} Query result
   */
  async query(sql, params = []) {
    if (this.postgres) {
      try {
        return await this.postgres.query(sql, params);
      } catch (err) {
        console.error('Error executing query:', err.message);
        return { rows: [] };
      }
    }
    // No fallback for raw SQL queries
    return { rows: [] };
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
    // With fallback, most features are always available
    const hasFallback = !!this._fallback;
    return {
      judgments: !!this.judgments || hasFallback,
      patterns: !!this.patterns || hasFallback,
      feedback: !!this.feedback || hasFallback,
      knowledge: !!this.knowledge || hasFallback,
      pojChain: !!this.pojBlocks || hasFallback, // Now has fallback - CYNIC must verify itself
      triggers: !!this.triggers || hasFallback, // PostgreSQL triggers repository with fallback
      libraryCache: !!this.libraryCache, // No fallback - requires PostgreSQL
      sessions: !!this.sessionStore,
      cache: !!this.redis,
    };
  }
}

export default PersistenceManager;
