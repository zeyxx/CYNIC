/**
 * MCP Persistence Integration
 *
 * UNIFIED persistence layer with automatic fallback:
 * 1. PostgreSQL + Redis (production)
 * 2. File-based (local development)
 * 3. In-memory (tests/ephemeral)
 *
 * ISP-Compliant: Exposes domain-specific adapters.
 * DIP-Compliant: Depends on abstractions via adapters.
 *
 * "φ qui se méfie de φ"
 *
 * @module @cynic/mcp/persistence
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('PersistenceManager');

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
  PsychologyRepository,
  SessionRepository,
  SessionStore,
  // Phase 16: Total Memory + Full Autonomy
  AutonomousGoalsRepository,
  AutonomousTasksRepository,
  ProactiveNotificationsRepository,
  createMemoryRetriever,
  // Phase 18: Embedder for vector search
  getEmbedder,
} from '@cynic/persistence';

// ISP: Domain-specific adapters
import { MemoryStore, FileStore } from './persistence/stores.js';
import { JudgmentAdapter } from './persistence/JudgmentAdapter.js';
import { PatternAdapter } from './persistence/PatternAdapter.js';
import { PoJChainAdapter } from './persistence/PoJChainAdapter.js';
import { TriggerAdapter } from './persistence/TriggerAdapter.js';
import { KnowledgeAdapter } from './persistence/KnowledgeAdapter.js';
import { FeedbackAdapter } from './persistence/FeedbackAdapter.js';
import { LibraryCacheAdapter } from './persistence/LibraryCacheAdapter.js';
import { PsychologyAdapter } from './persistence/PsychologyAdapter.js';

/**
 * Unified Persistence Manager for MCP server
 *
 * ISP: Exposes domain-specific adapters via getters
 * DIP: Composes adapters rather than direct repository access
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

    // Repositories (PostgreSQL) - private
    this._judgments = null;
    this._patterns = null;
    this._feedback = null;
    this._knowledge = null;
    this._pojBlocks = null;
    this._libraryCache = null;
    this._triggers = null;
    this._discovery = null;
    this._userLearningProfiles = null;
    this._psychology = null;
    this._sessions = null;

    // Phase 16: Total Memory + Full Autonomy repos
    this._goals = null;
    this._tasks = null;
    this._notifications = null;
    this._memoryRetriever = null;

    // Fallback store (file or memory)
    this._fallback = null;
    this._backend = 'none'; // 'postgres', 'file', 'memory'

    // ISP: Domain adapters (lazy initialized)
    this._judgmentAdapter = null;
    this._patternAdapter = null;
    this._pojChainAdapter = null;
    this._triggerAdapter = null;
    this._knowledgeAdapter = null;
    this._feedbackAdapter = null;
    this._libraryCacheAdapter = null;
    this._psychologyAdapter = null;

    this._initialized = false;
  }

  /**
   * Initialize all persistence connections
   * Fallback chain: PostgreSQL → File → Memory
   * @returns {Promise<PersistenceManager>}
   */
  async initialize() {
    if (this._initialized) return this;

    // Check for PostgreSQL config
    const hasPostgres = !!process.env.CYNIC_DATABASE_URL
      || (!!process.env.CYNIC_DB_HOST && !!process.env.CYNIC_DB_PASSWORD);
    const hasRedis = !!process.env.CYNIC_REDIS_URL;

    // Try PostgreSQL first (production)
    if (hasPostgres) {
      try {
        this.postgres = new PostgresClient();
        await this.postgres.connect();

        // Initialize repositories
        this._judgments = new JudgmentRepository(this.postgres);
        this._patterns = new PatternRepository(this.postgres);
        this._feedback = new FeedbackRepository(this.postgres);
        this._knowledge = new KnowledgeRepository(this.postgres);
        this._pojBlocks = new PoJBlockRepository(this.postgres);
        this._libraryCache = new LibraryCacheRepository(this.postgres);
        this._triggers = new TriggerRepository(this.postgres);
        this._discovery = new DiscoveryRepository(this.postgres);
        this._userLearningProfiles = new UserLearningProfilesRepository(this.postgres);
        this._psychology = new PsychologyRepository(this.postgres);
        this._sessions = new SessionRepository(this.postgres);

        // Phase 16: Total Memory + Full Autonomy repos
        this._goals = new AutonomousGoalsRepository(this.postgres);
        this._tasks = new AutonomousTasksRepository(this.postgres);
        this._notifications = new ProactiveNotificationsRepository(this.postgres);

        // Phase 18: Initialize embedder for vector search
        // Auto-detects OpenAI (if OPENAI_API_KEY set) or falls back to MockEmbedder
        const embedder = getEmbedder();
        this._memoryRetriever = createMemoryRetriever({ pool: this.postgres, embedder });
        log.info('Memory retriever initialized', { embedderType: embedder.type });

        this._backend = 'postgres';
        log.info('PostgreSQL connected');
      } catch (err) {
        log.error('PostgreSQL error', { error: err.message });
        this.postgres = null;
      }
    } else {
      log.debug('PostgreSQL not configured');
    }

    // Initialize Redis (optional, for caching/sessions)
    if (hasRedis) {
      try {
        this.redis = new RedisClient();
        await this.redis.connect();
        this.sessionStore = new SessionStore(this.redis);
        log.info('Redis connected');
      } catch (err) {
        log.error('Redis error', { error: err.message });
      }
    } else {
      log.debug('Redis not configured');
    }

    // Set up fallback storage if PostgreSQL not available
    if (!this.postgres) {
      if (this.dataDir) {
        this._fallback = new FileStore(this.dataDir);
        await this._fallback.initialize();
        this._backend = 'file';
      } else {
        this._fallback = new MemoryStore();
        this._backend = 'memory';
        log.info('Storage: in-memory (ephemeral)');
      }
    }

    // Initialize ISP adapters
    this._initializeAdapters();

    this._initialized = true;
    return this;
  }

  /**
   * Initialize domain-specific adapters (ISP)
   * @private
   */
  _initializeAdapters() {
    this._judgmentAdapter = new JudgmentAdapter(this._judgments, this._fallback);
    this._patternAdapter = new PatternAdapter(this._patterns, this._fallback);
    this._pojChainAdapter = new PoJChainAdapter(this._pojBlocks, this._fallback);
    this._triggerAdapter = new TriggerAdapter(this._triggers, this._fallback);
    this._knowledgeAdapter = new KnowledgeAdapter(this._knowledge, this._fallback);
    this._feedbackAdapter = new FeedbackAdapter(this._feedback, this._fallback, this._userLearningProfiles);
    this._libraryCacheAdapter = new LibraryCacheAdapter(this._libraryCache);
    this._psychologyAdapter = new PsychologyAdapter(this._psychology);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // ISP: DOMAIN ADAPTERS (prefer these over legacy methods)
  // "Clients should not depend on interfaces they don't use"
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {JudgmentAdapter} */
  get judgment() { return this._judgmentAdapter; }

  /** @returns {PatternAdapter} */
  get pattern() { return this._patternAdapter; }

  /** @returns {PoJChainAdapter} */
  get pojChain() { return this._pojChainAdapter; }

  /** @returns {TriggerAdapter} */
  get trigger() { return this._triggerAdapter; }

  /** @returns {KnowledgeAdapter} */
  get knowledge() { return this._knowledgeAdapter; }

  /** @returns {FeedbackAdapter} */
  get feedback() { return this._feedbackAdapter; }

  /** @returns {LibraryCacheAdapter} */
  get libraryCache() { return this._libraryCacheAdapter; }

  /** @returns {PsychologyAdapter} */
  get psychology() { return this._psychologyAdapter; }

  // ═══════════════════════════════════════════════════════════════════════════
  // PHASE 16: Total Memory + Full Autonomy (direct repo access)
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {AutonomousGoalsRepository} */
  get goals() { return this._goals; }

  /** @returns {AutonomousTasksRepository} */
  get tasks() { return this._tasks; }

  /** @returns {ProactiveNotificationsRepository} */
  get notifications() { return this._notifications; }

  /** @returns {MemoryRetriever} */
  get memoryRetriever() { return this._memoryRetriever; }

  // ═══════════════════════════════════════════════════════════════════════════
  // LEGACY API (backward compatibility - delegates to adapters)
  // Will be deprecated in future versions
  // ═══════════════════════════════════════════════════════════════════════════

  // --- Judgments ---
  async storeJudgment(judgment) { return this._judgmentAdapter?.store(judgment) ?? null; }
  async getJudgment(judgmentId) { return this._judgmentAdapter?.getById(judgmentId) ?? null; }
  async searchJudgments(query, options = {}) { return this._judgmentAdapter?.search(query, options) ?? []; }
  async getRecentJudgments(limit = 10) { return this._judgmentAdapter?.getRecent(limit) ?? []; }
  async getJudgmentStats(options = {}) { return this._judgmentAdapter?.getStats(options) ?? {}; }

  // --- Patterns ---
  async upsertPattern(pattern) { return this._patternAdapter.upsert(pattern); }
  async getPatterns(options = {}) { return this._patternAdapter.get(options); }

  // --- PoJ Chain ---
  async storePoJBlock(block) { return this._pojChainAdapter.store(block); }
  async getPoJHead() { return this._pojChainAdapter.getHead(); }
  async getPoJStats() { return this._pojChainAdapter.getStats(); }
  async getRecentPoJBlocks(limit = 10) { return this._pojChainAdapter.getRecent(limit); }
  async getPoJBlockBySlot(slot) { return this._pojChainAdapter.getBySlot(slot); }
  async verifyPoJChain() { return this._pojChainAdapter.verifyIntegrity(); }

  // --- Triggers ---
  async getEnabledTriggers() { return this._triggerAdapter.getEnabled(); }
  async getAllTriggers() { return this._triggerAdapter.getAll(); }
  async createTrigger(trigger) { return this._triggerAdapter.create(trigger); }
  async updateTrigger(triggerId, updates) { return this._triggerAdapter.update(triggerId, updates); }
  async enableTrigger(triggerId) { return this._triggerAdapter.enable(triggerId); }
  async disableTrigger(triggerId) { return this._triggerAdapter.disable(triggerId); }
  async deleteTrigger(triggerId) { return this._triggerAdapter.delete(triggerId); }
  async recordTriggerExecution(execution) { return this._triggerAdapter.recordExecution(execution); }
  async storeTriggerEvent(event) { return this._triggerAdapter.storeEvent(event); }
  async getTriggerStats() { return this._triggerAdapter.getStats(); }
  async getActiveTriggersSummary() { return this._triggerAdapter.getActiveSummary(); }
  async getTriggerExecutionsLastMinute(triggerId) { return this._triggerAdapter.countExecutionsLastMinute(triggerId); }
  async getTriggersState() { return this._triggerAdapter.getState(); }
  async saveTriggersState(state) { return this._triggerAdapter.saveState(state); }

  // --- Knowledge ---
  async storeKnowledge(knowledge) { return this._knowledgeAdapter.store(knowledge); }
  async searchKnowledge(query, options = {}) { return this._knowledgeAdapter.search(query, options); }

  // --- Feedback ---
  async storeFeedback(feedback) { return this._feedbackAdapter.store(feedback); }

  // --- Library Cache ---
  async getLibraryDoc(libraryId, query) { return this._libraryCacheAdapter.get(libraryId, query); }
  async setLibraryDoc(libraryId, query, content, metadata = {}, ttlHours = 24) {
    return this._libraryCacheAdapter.set(libraryId, query, content, metadata, ttlHours);
  }
  async isLibraryDocCached(libraryId, query) { return this._libraryCacheAdapter.isCached(libraryId, query); }
  async cleanExpiredCache() { return this._libraryCacheAdapter.cleanExpired(); }
  async invalidateLibraryCache(libraryId) { return this._libraryCacheAdapter.invalidate(libraryId); }
  async getLibraryCacheStats() { return this._libraryCacheAdapter.getStats(); }
  async getTopCachedLibraries(limit = 10) { return this._libraryCacheAdapter.getTopLibraries(limit); }

  // --- Psychology ---
  async syncPsychology(userId, data) { return this._psychologyAdapter.sync(userId, data); }
  async loadPsychology(userId) { return this._psychologyAdapter.load(userId); }
  async recordIntervention(userId, intervention) { return this._psychologyAdapter.recordIntervention(userId, intervention); }
  async getInterventionEffectiveness(userId) { return this._psychologyAdapter.getInterventionEffectiveness(userId); }
  async recordLearningObservation(userId, observation) { return this._psychologyAdapter.recordLearningObservation(userId, observation); }
  async getCalibrationStats(userId) { return this._psychologyAdapter.getCalibrationStats(userId); }
  async getPsychologyStats() { return this._psychologyAdapter.getStats(); }
  async getTopPerformers(limit = 10) { return this._psychologyAdapter.getTopPerformers(limit); }

  // ═══════════════════════════════════════════════════════════════════════════
  // OBSERVATIONS (lightweight, non-critical)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Store an observation from agents (The Four Dogs)
   * @param {Object} observation
   * @returns {Promise<Object|null>}
   */
  async storeObservation(observation) {
    const obs = {
      ...observation,
      timestamp: observation.timestamp || Date.now(),
      id: `obs_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
    };

    if (this._fallback?.storeObservation) {
      try {
        return await this._fallback.storeObservation(obs);
      } catch {
        // Silently ignore - observations are non-critical
      }
    }

    return obs;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DIRECT REPOSITORY ACCESS (for advanced use cases)
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {JudgmentRepository|null} */
  get judgments() { return this._judgments; }

  /** @returns {PatternRepository|null} */
  get patterns() { return this._patterns; }

  /** @returns {PoJBlockRepository|null} */
  get pojBlocks() { return this._pojBlocks; }

  /** @returns {TriggerRepository|null} */
  get triggers() { return this._triggers; }

  /** @returns {DiscoveryRepository|null} */
  get discovery() { return this._discovery; }

  /** @returns {UserLearningProfilesRepository|null} */
  get userLearningProfiles() { return this._userLearningProfiles; }

  /** @returns {SessionRepository|null} */
  get sessions() { return this._sessions; }

  // ═══════════════════════════════════════════════════════════════════════════
  // HEALTH & LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════════

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
    if (this._fallback?.save) {
      try {
        await this._fallback.save();
      } catch (err) {
        log.error('Error saving file storage', { error: err.message });
      }
    }

    if (this.postgres) {
      try {
        await this.postgres.close();
      } catch (err) {
        log.error('Error closing PostgreSQL', { error: err.message });
      }
    }

    if (this.redis) {
      try {
        await this.redis.close();
      } catch (err) {
        log.error('Error closing Redis', { error: err.message });
      }
    }

    this._initialized = false;
  }

  /**
   * Execute raw SQL query (PostgreSQL only)
   * @param {string} sql
   * @param {Array} params
   * @returns {Promise<{rows: Array}>}
   */
  async query(sql, params = []) {
    if (this.postgres) {
      try {
        return await this.postgres.query(sql, params);
      } catch (err) {
        log.error('Error executing query', { error: err.message });
        return { rows: [] };
      }
    }
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
    return {
      judgments: this._judgmentAdapter?.isAvailable || false,
      patterns: this._patternAdapter?.isAvailable || false,
      pojChain: this._pojChainAdapter?.isAvailable || false,
      triggers: this._triggerAdapter?.isAvailable || false,
      knowledge: this._knowledgeAdapter?.isAvailable || false,
      feedback: this._feedbackAdapter?.isAvailable || false,
      libraryCache: this._libraryCacheAdapter?.isAvailable || false,
      psychology: this._psychologyAdapter?.isAvailable || false,
      sessions: !!this.sessionStore,
      cache: !!this.redis,
    };
  }
}

export default PersistenceManager;
