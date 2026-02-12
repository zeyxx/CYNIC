/**
 * CYNIC Learning Service
 *
 * Extracted from QLearningRouter - provides learning capabilities
 * that feed into KabbalisticRouter for weight optimization.
 *
 * "Le chien apprend qui appeler" - CYNIC learns who to call
 *
 * Features preserved:
 * - Q-Table algorithm
 * - Reward system (φ-aligned)
 * - Feature extraction
 * - Episode management
 * - Exploration/exploitation
 *
 * @module @cynic/node/orchestration/learning-service
 */

'use strict';

import { PHI_INV, PHI_INV_2, globalEventBus, EventType } from '@cynic/core';

// =============================================================================
// CONFIGURATION (φ-aligned)
// =============================================================================

/**
 * Learning hyperparameters
 */
export const LEARNING_CONFIG = {
  // Learning rate - how much new info overrides old (φ⁻¹)
  learningRate: PHI_INV,          // 0.618

  // Discount factor - importance of future rewards (φ⁻²)
  discountFactor: PHI_INV_2,      // 0.382

  // Exploration rate - probability of random action
  explorationRate: 0.1,           // 10% exploration

  // Exploration decay - reduce exploration over time
  explorationDecay: 0.995,

  // Minimum exploration rate
  minExplorationRate: 0.01,

  // Temperature for softmax selection (φ-aligned)
  temperature: PHI_INV,

  // Reward scaling
  rewards: {
    success: 1.0,
    partialSuccess: 0.5,
    failure: -0.5,
    timeout: -0.3,
    blocked: 0.8,                 // Blocking is good (safety)
  },
};

/**
 * State features for learning
 */
export const StateFeatures = {
  // Task type features
  TASK_SECURITY: 'task:security',
  TASK_CODE_CHANGE: 'task:code_change',
  TASK_ANALYSIS: 'task:analysis',
  TASK_EXPLORATION: 'task:exploration',
  TASK_DEPLOYMENT: 'task:deployment',
  TASK_CLEANUP: 'task:cleanup',
  TASK_DOCUMENTATION: 'task:documentation',
  TASK_TEST: 'task:test',
  TASK_DEBUG: 'task:debug',
  TASK_DESIGN: 'task:design',

  // Context features
  CONTEXT_ERROR: 'ctx:error',
  CONTEXT_URGENT: 'ctx:urgent',
  CONTEXT_COMPLEX: 'ctx:complex',
  CONTEXT_SIMPLE: 'ctx:simple',

  // Tool features
  TOOL_BASH: 'tool:bash',
  TOOL_WRITE: 'tool:write',
  TOOL_EDIT: 'tool:edit',
  TOOL_READ: 'tool:read',
  TOOL_TASK: 'tool:task',
};

// =============================================================================
// Q-TABLE
// =============================================================================

/**
 * Q-Table for storing state-action values
 */
export class QTable {
  constructor() {
    // Q(s, a) = expected reward for taking action a in state s
    this.table = new Map();

    // Visit counts for each state-action pair
    this.visits = new Map();

    // Stats
    this.stats = {
      updates: 0,
      states: 0,
    };
  }

  /**
   * Get state key from features
   */
  static stateKey(features) {
    return features.sort().join('|');
  }

  /**
   * Get Q-value for state-action pair
   */
  get(features, action) {
    const key = QTable.stateKey(features);
    const stateQ = this.table.get(key);
    if (!stateQ) return 0; // Optimistic initialization
    return stateQ[action] || 0;
  }

  /**
   * Set Q-value for state-action pair
   */
  set(features, action, value) {
    const key = QTable.stateKey(features);

    if (!this.table.has(key)) {
      this.table.set(key, {});
      this.stats.states++;
    }

    this.table.get(key)[action] = value;
    this.stats.updates++;
  }

  /**
   * Get all Q-values for a state
   */
  getAll(features) {
    const key = QTable.stateKey(features);
    return this.table.get(key) || {};
  }

  /**
   * Increment visit count
   */
  visit(features, action) {
    const key = `${QTable.stateKey(features)}:${action}`;
    this.visits.set(key, (this.visits.get(key) || 0) + 1);
  }

  /**
   * Get visit count
   */
  getVisits(features, action) {
    const key = `${QTable.stateKey(features)}:${action}`;
    return this.visits.get(key) || 0;
  }

  /**
   * Export to JSON
   */
  toJSON() {
    return {
      table: Object.fromEntries(this.table),
      visits: Object.fromEntries(this.visits),
      stats: this.stats,
    };
  }

  /**
   * Import from JSON
   */
  static fromJSON(json) {
    const qt = new QTable();
    if (json.table) {
      qt.table = new Map(Object.entries(json.table));
    }
    if (json.visits) {
      qt.visits = new Map(Object.entries(json.visits));
    }
    if (json.stats) {
      qt.stats = json.stats;
    }
    return qt;
  }
}

// =============================================================================
// LEARNING SERVICE
// =============================================================================

/**
 * QLearningService - Provides learning capabilities for KabbalisticRouter
 *
 * Usage:
 * 1. Call startEpisode() before routing
 * 2. Call recordAction() for each dog that processes
 * 3. Call endEpisode() with outcome to update weights
 * 4. Use getRecommendedWeights() to feed into RelationshipGraph
 */
export class QLearningService {
  /**
   * @param {Object} options
   * @param {Object} [options.config] - Learning configuration
   * @param {QTable} [options.qTable] - Existing Q-table
   * @param {Object} [options.persistence] - Persistence layer
   * @param {string} [options.serviceId] - Service identifier for persistence
   */
  constructor(options = {}) {
    this.config = { ...LEARNING_CONFIG, ...options.config };
    this.qTable = options.qTable || new QTable();
    this.persistence = options.persistence || null;
    this.serviceId = options.serviceId || 'default';

    // Current exploration rate (decays over time)
    this.explorationRate = this.config.explorationRate;

    // Episode tracking
    this.currentEpisode = null;
    this.episodeHistory = [];
    this.maxEpisodeHistory = 1000;

    // Stats
    this.stats = {
      episodes: 0,
      updates: 0,
      correctPredictions: 0,
      totalFeedback: 0,
    };

    // Debounced persistence (5s debounce)
    this._persistTimeout = null;
    this._persistDebounceMs = 5000;
    this._initialized = false;
    this._initializing = null;
  }

  // ===========================================================================
  // FEATURE EXTRACTION
  // ===========================================================================

  /**
   * Extract state features from context
   */
  extractFeatures(context) {
    const features = [];

    // Task type features
    const taskType = context.taskType || context.type || '';
    const taskTypeMap = {
      security: StateFeatures.TASK_SECURITY,
      code_change: StateFeatures.TASK_CODE_CHANGE,
      analysis: StateFeatures.TASK_ANALYSIS,
      exploration: StateFeatures.TASK_EXPLORATION,
      deployment: StateFeatures.TASK_DEPLOYMENT,
      cleanup: StateFeatures.TASK_CLEANUP,
      documentation: StateFeatures.TASK_DOCUMENTATION,
      test: StateFeatures.TASK_TEST,
      debug: StateFeatures.TASK_DEBUG,
      design: StateFeatures.TASK_DESIGN,
    };

    if (taskTypeMap[taskType]) {
      features.push(taskTypeMap[taskType]);
    }

    // Detect task type from content
    const content = (context.content || context.prompt || '').toLowerCase();

    if (content.match(/security|vulnerab|credential|auth|permission/)) {
      features.push(StateFeatures.TASK_SECURITY);
    }
    if (content.match(/write|edit|create|modify|change/)) {
      features.push(StateFeatures.TASK_CODE_CHANGE);
    }
    if (content.match(/analyz|review|check|inspect|assess/)) {
      features.push(StateFeatures.TASK_ANALYSIS);
    }
    if (content.match(/find|search|explore|discover|look/)) {
      features.push(StateFeatures.TASK_EXPLORATION);
    }
    if (content.match(/deploy|publish|release|push|ci|cd/)) {
      features.push(StateFeatures.TASK_DEPLOYMENT);
    }
    if (content.match(/clean|refactor|simplify|remove|delete/)) {
      features.push(StateFeatures.TASK_CLEANUP);
    }
    if (content.match(/document|readme|comment|explain/)) {
      features.push(StateFeatures.TASK_DOCUMENTATION);
    }
    if (content.match(/test|spec|assert|expect|verify/)) {
      features.push(StateFeatures.TASK_TEST);
    }
    if (content.match(/debug|fix|error|bug|issue/)) {
      features.push(StateFeatures.TASK_DEBUG);
    }
    if (content.match(/design|architect|structure|plan/)) {
      features.push(StateFeatures.TASK_DESIGN);
    }

    // Context features
    if (context.isError || context.error) {
      features.push(StateFeatures.CONTEXT_ERROR);
    }
    if (context.urgent || context.priority === 'high') {
      features.push(StateFeatures.CONTEXT_URGENT);
    }
    if (context.complexity === 'high' || (content.length > 500)) {
      features.push(StateFeatures.CONTEXT_COMPLEX);
    }
    if (context.complexity === 'low' || (content.length < 100)) {
      features.push(StateFeatures.CONTEXT_SIMPLE);
    }

    // Tool features
    const tool = context.tool || context.toolName || '';
    const toolMap = {
      Bash: StateFeatures.TOOL_BASH,
      Write: StateFeatures.TOOL_WRITE,
      Edit: StateFeatures.TOOL_EDIT,
      Read: StateFeatures.TOOL_READ,
      Task: StateFeatures.TOOL_TASK,
    };
    if (toolMap[tool]) {
      features.push(toolMap[tool]);
    }

    // Ensure unique features
    return [...new Set(features)];
  }

  // ===========================================================================
  // EPISODE MANAGEMENT
  // ===========================================================================

  /**
   * Start a learning episode
   * @param {Object} context - Routing context
   * @returns {string} Episode ID
   */
  startEpisode(context) {
    const features = this.extractFeatures(context);

    this.currentEpisode = {
      id: `ep_${Date.now().toString(36)}`,
      features,
      context: {
        taskType: context.taskType || context.type,
        tool: context.tool || context.toolName,
      },
      actions: [],
      startTime: Date.now(),
    };

    return this.currentEpisode.id;
  }

  /**
   * Record an action (dog) in current episode
   * @param {string} action - Dog name
   * @param {Object} metadata - Additional metadata
   */
  recordAction(action, metadata = {}) {
    if (!this.currentEpisode) return;

    this.currentEpisode.actions.push({
      action,
      metadata,
      timestamp: Date.now(),
    });

    // Record visit
    this.qTable.visit(this.currentEpisode.features, action);
  }

  /**
   * End episode and update Q-values
   * @param {Object} outcome - Episode outcome
   * @returns {Object} Episode summary
   */
  endEpisode(outcome) {
    if (!this.currentEpisode) return null;

    const episode = this.currentEpisode;
    episode.outcome = outcome;
    episode.endTime = Date.now();
    episode.duration = episode.endTime - episode.startTime;

    // Calculate reward
    const reward = this._calculateReward(outcome);
    episode.reward = reward;

    // Q-learning update for each action
    this._updateQValues(episode, reward);

    // Decay exploration rate
    this.explorationRate = Math.max(
      this.config.minExplorationRate,
      this.explorationRate * this.config.explorationDecay
    );

    // Update stats
    this.stats.episodes++;
    this.stats.totalFeedback++;
    if (outcome.success || outcome.type === 'success') {
      this.stats.correctPredictions++;
    }

    // Store in history
    this.episodeHistory.push(episode);
    if (this.episodeHistory.length > this.maxEpisodeHistory) {
      this.episodeHistory.shift();
    }

    // Clear current
    this.currentEpisode = null;

    // Persist Q-Table (debounced) and episode
    this._persist();
    this._persistEpisode(episode).catch(() => {});

    return episode;
  }

  /**
   * Update Q-values using Bellman equation
   * @private
   */
  _updateQValues(episode, reward) {
    for (let i = 0; i < episode.actions.length; i++) {
      const action = episode.actions[i].action;
      const isLast = i === episode.actions.length - 1;

      const currentQ = this.qTable.get(episode.features, action);

      // For last action, no future state
      const futureQ = isLast ? 0 :
        Math.max(...this._getAllActions().map(a =>
          this.qTable.get(episode.features, a)
        ));

      // Q(s,a) += α * (r + γ * max(Q(s',a')) - Q(s,a))
      const target = reward + this.config.discountFactor * futureQ;
      const newQ = currentQ + this.config.learningRate * (target - currentQ);

      this.qTable.set(episode.features, action, newQ);
      this.stats.updates++;

      // A2: Hot-swap routing weights — emit event for live router update
      try {
        globalEventBus.publish(EventType.QLEARNING_WEIGHT_UPDATE, {
          state: episode.features,
          action,
          qValue: newQ,
          delta: newQ - currentQ,
          serviceId: this.serviceId,
        }, { source: 'learning-service' });
      } catch (err) {
        // Non-blocking — router update is optional enhancement
      }
    }
  }

  /**
   * Calculate reward from outcome
   * @private
   */
  _calculateReward(outcome) {
    const rewards = this.config.rewards;

    // FIX: Check nuanced score FIRST — binary success/failure was swallowing
    // the real 0-100 signal from calculateRealReward(). Score converts to [-1, 1].
    if (typeof outcome.score === 'number') {
      return (outcome.score - 50) / 50;
    }

    // Fall back to categorical signals when no nuanced score available
    if (outcome.blocked) {
      return rewards.blocked;
    }
    if (outcome.timeout || outcome.type === 'timeout') {
      return rewards.timeout;
    }
    if (outcome.partial || outcome.type === 'partial') {
      return rewards.partialSuccess;
    }
    if (outcome.success === true || outcome.type === 'success') {
      return rewards.success;
    }
    if (outcome.success === false || outcome.type === 'failure') {
      return rewards.failure;
    }

    return 0;
  }

  /**
   * Get all possible actions (dogs)
   * @private
   */
  _getAllActions() {
    return [
      'guardian', 'analyst', 'architect', 'scout', 'scholar',
      'sage', 'oracle', 'janitor', 'deployer', 'cartographer', 'cynic',
    ];
  }

  // ===========================================================================
  // WEIGHT RECOMMENDATIONS
  // ===========================================================================

  /**
   * Get recommended weights for dogs based on learning
   * Use this to feed into RelationshipGraph
   *
   * @param {string[]} features - Current state features
   * @returns {Object} Dog weights { dogName: weight }
   */
  getRecommendedWeights(features) {
    // D1: Fall back to current episode features or empty array
    const resolvedFeatures = features || this.currentEpisode?.features || [];
    const weights = {};
    const actions = this._getAllActions();

    for (const action of actions) {
      const q = this.qTable.get(resolvedFeatures, action);
      // Normalize Q-value to 0-1 weight (sigmoid-like)
      weights[action] = 1 / (1 + Math.exp(-q));
    }

    return weights;
  }

  /**
   * Get top-k recommended dogs for features
   * @param {string[]} features - State features
   * @param {number} k - Number of dogs to return
   * @returns {Array} Top dogs with scores
   */
  getTopDogs(features, k = 3) {
    const weights = this.getRecommendedWeights(features);
    const sorted = Object.entries(weights)
      .sort((a, b) => b[1] - a[1])
      .slice(0, k);

    return sorted.map(([dog, weight]) => ({
      dog,
      weight: Math.round(weight * 1000) / 1000,
      visits: this.qTable.getVisits(features, dog),
    }));
  }

  // ===========================================================================
  // PERSISTENCE
  // ===========================================================================

  /**
   * Schedule debounced persistence
   * @private
   */
  _schedulePersist() {
    if (this._persistTimeout) {
      clearTimeout(this._persistTimeout);
    }
    this._persistTimeout = setTimeout(() => {
      this._doPersist().catch(() => {});
    }, this._persistDebounceMs);
  }

  /**
   * Persist learning state immediately
   * @private
   */
  async _doPersist() {
    if (!this.persistence?.query) return;

    try {
      const qTableData = this.qTable.toJSON();

      // Use UPSERT to handle both new and existing service_ids
      await this.persistence.query(`
        INSERT INTO qlearning_state (service_id, q_table, exploration_rate, stats, version)
        VALUES ($4, $1, $2, $3, 1)
        ON CONFLICT (service_id) DO UPDATE SET
          q_table = $1,
          exploration_rate = $2,
          stats = $3,
          version = qlearning_state.version + 1,
          updated_at = NOW()
      `, [
        JSON.stringify(qTableData),
        this.explorationRate,
        JSON.stringify(this.stats),
        this.serviceId,
      ]);
    } catch (e) {
      // Log but don't throw - persistence is best-effort
      // console.error('[QLearningService] Persist failed:', e.message);
    }
  }

  /**
   * Persist (debounced)
   * @private
   */
  _persist() {
    this._schedulePersist();
  }

  /**
   * Persist episode to database
   * @private
   */
  async _persistEpisode(episode) {
    if (!this.persistence?.query) return;

    try {
      await this.persistence.query(`
        INSERT INTO qlearning_episodes (
          episode_id, service_id, features, task_type, tool,
          actions, outcome, reward, duration_ms
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (episode_id) DO NOTHING
      `, [
        episode.id,
        this.serviceId,
        episode.features,
        episode.context?.taskType || null,
        episode.context?.tool || null,
        JSON.stringify(episode.actions),
        JSON.stringify(episode.outcome || {}),
        episode.reward,
        episode.duration,
      ]);
    } catch (e) {
      // Episode persistence is optional - don't throw
    }
  }

  /**
   * Initialize from persistence (call once on startup)
   */
  async initialize() {
    if (this._initialized) return true;
    if (this._initializing) return this._initializing;

    this._initializing = this._doInitialize();
    return this._initializing;
  }

  /**
   * @private
   */
  async _doInitialize() {
    const loaded = await this.load();
    this._initialized = true;
    this._initializing = null;
    return loaded;
  }

  /**
   * Load learning state from persistence
   */
  async load() {
    if (!this.persistence?.query) return false;

    try {
      const result = await this.persistence.query(
        'SELECT q_table, exploration_rate, stats FROM qlearning_state WHERE service_id = $1',
        [this.serviceId]
      );

      if (result.rows?.[0]) {
        const row = result.rows[0];

        // Load Q-Table
        if (row.q_table) {
          const qData = typeof row.q_table === 'string'
            ? JSON.parse(row.q_table)
            : row.q_table;
          this.qTable = QTable.fromJSON(qData);
        }

        // Load exploration rate
        if (row.exploration_rate != null) {
          this.explorationRate = parseFloat(row.exploration_rate);
        }

        // Load stats
        if (row.stats) {
          const statsData = typeof row.stats === 'string'
            ? JSON.parse(row.stats)
            : row.stats;
          this.stats = { ...this.stats, ...statsData };
        }

        return true;
      }
    } catch (e) {
      // Load failed - start fresh
      // console.error('[QLearningService] Load failed:', e.message);
    }

    return false;
  }

  /**
   * Force immediate persistence (call on shutdown)
   */
  async flush() {
    if (this._persistTimeout) {
      clearTimeout(this._persistTimeout);
      this._persistTimeout = null;
    }
    await this._doPersist();
  }

  // ===========================================================================
  // STATS & DEBUGGING
  // ===========================================================================

  /**
   * Get service statistics
   */
  getStats() {
    const accuracy = this.stats.totalFeedback > 0
      ? this.stats.correctPredictions / this.stats.totalFeedback
      : 0;

    return {
      ...this.stats,
      accuracy: Math.round(accuracy * 100),
      explorationRate: Math.round(this.explorationRate * 100),
      qTableStats: this.qTable.stats,
      episodesInMemory: this.episodeHistory.length,
    };
  }

  /**
   * Get best dogs per task type (for debugging)
   */
  getBestDogsPerTask() {
    const taskFeatures = Object.values(StateFeatures).filter(f => f.startsWith('task:'));
    const result = {};

    for (const feature of taskFeatures) {
      const taskType = feature.replace('task:', '');
      const topDogs = this.getTopDogs([feature], 3);
      result[taskType] = topDogs;
    }

    return result;
  }
}

/**
 * Create a QLearningService instance
 */
export function createQLearningService(options = {}) {
  return new QLearningService(options);
}

// Singleton instance
let _service = null;
let _serviceInitPromise = null;

/**
 * Get the global QLearningService instance
 * @param {Object} options - Options for creating the service
 * @returns {QLearningService}
 */
export function getQLearningService(options) {
  if (!_service) {
    _service = createQLearningService(options);
  }
  return _service;
}

/**
 * Get the global QLearningService instance with async initialization
 * Ensures persistence is loaded before returning.
 * @param {Object} options - Options for creating the service
 * @returns {Promise<QLearningService>}
 */
export async function getQLearningServiceAsync(options) {
  const service = getQLearningService(options);

  if (!_serviceInitPromise && !service._initialized) {
    _serviceInitPromise = service.initialize().finally(() => {
      _serviceInitPromise = null;
    });
  }

  if (_serviceInitPromise) {
    await _serviceInitPromise;
  }

  return service;
}

/**
 * Reset singleton for testing
 * @private
 */
export function _resetQLearningServiceForTesting() {
  if (_service) {
    if (_service._persistTimeout) {
      clearTimeout(_service._persistTimeout);
    }
    _service = null;
  }
  _serviceInitPromise = null;
}

export default {
  QLearningService,
  QTable,
  LEARNING_CONFIG,
  StateFeatures,
  createQLearningService,
  getQLearningService,
  getQLearningServiceAsync,
  _resetQLearningServiceForTesting,
};
