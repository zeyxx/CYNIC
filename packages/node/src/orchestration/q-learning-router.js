/**
 * CYNIC Q-Learning Router
 *
 * @deprecated Use QLearningService from './learning-service.js' instead.
 * This module's features have been extracted to QLearningService for
 * integration with KabbalisticRouter. This file is kept for backward
 * compatibility only.
 *
 * Replaces rule-based routing with learned routing.
 * Tracks dog success/failure per task type and updates routing weights.
 *
 * "Le chien apprend qui appeler" - CYNIC learns who to call
 *
 * Inspired by Claude-flow's Q-Learning Router (84.8% SWE-Bench accuracy).
 *
 * @module @cynic/node/orchestration/q-learning-router
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// CONFIGURATION
// =============================================================================

/**
 * Q-Learning hyperparameters (φ-aligned)
 */
export const Q_CONFIG = {
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
 * State features for Q-table
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

/**
 * Available actions (dogs)
 */
export const Actions = {
  GUARDIAN: 'GUARDIAN',
  ANALYST: 'ANALYST',
  ARCHITECT: 'ARCHITECT',
  SCOUT: 'SCOUT',
  SCHOLAR: 'SCHOLAR',
  SAGE: 'SAGE',
  ORACLE: 'ORACLE',
  JANITOR: 'JANITOR',
  DEPLOYER: 'DEPLOYER',
  CARTOGRAPHER: 'CARTOGRAPHER',
  CYNIC: 'CYNIC',
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
    // Stored as: { stateKey: { action: qValue } }
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
// Q-LEARNING ROUTER
// =============================================================================

/**
 * Q-Learning Router - Learned dog selection
 */
export class QLearningRouter {
  /**
   * @param {Object} options
   * @param {Object} [options.config] - Q-Learning configuration
   * @param {QTable} [options.qTable] - Existing Q-table
   * @param {Object} [options.persistence] - Persistence layer
   */
  constructor(options = {}) {
    this.config = { ...Q_CONFIG, ...options.config };
    this.qTable = options.qTable || new QTable();
    this.persistence = options.persistence || null;

    // Current exploration rate (decays over time)
    this.explorationRate = this.config.explorationRate;

    // History for episode tracking
    this.currentEpisode = null;
    this.episodeHistory = [];

    // Stats
    this.stats = {
      routingDecisions: 0,
      explorations: 0,
      exploitations: 0,
      correctPredictions: 0,
      totalFeedback: 0,
    };
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
  // ACTION SELECTION
  // ===========================================================================

  /**
   * Select best action (dog) for given state
   * Uses ε-greedy policy with softmax
   */
  selectAction(features, availableActions = null) {
    const actions = availableActions || Object.values(Actions);
    this.stats.routingDecisions++;

    // Exploration: random action
    if (Math.random() < this.explorationRate) {
      this.stats.explorations++;
      const randomIndex = Math.floor(Math.random() * actions.length);
      return {
        action: actions[randomIndex],
        method: 'exploration',
        confidence: this.explorationRate,
      };
    }

    // Exploitation: select best action using softmax
    this.stats.exploitations++;
    const qValues = actions.map(a => ({
      action: a,
      q: this.qTable.get(features, a),
      visits: this.qTable.getVisits(features, a),
    }));

    // Softmax selection
    const temperature = this.config.temperature;
    const maxQ = Math.max(...qValues.map(v => v.q));
    const expValues = qValues.map(v => ({
      ...v,
      exp: Math.exp((v.q - maxQ) / temperature),
    }));
    const sumExp = expValues.reduce((s, v) => s + v.exp, 0);
    const probs = expValues.map(v => ({
      ...v,
      prob: v.exp / sumExp,
    }));

    // Select based on probability
    const rand = Math.random();
    let cumProb = 0;
    for (const p of probs) {
      cumProb += p.prob;
      if (rand <= cumProb) {
        return {
          action: p.action,
          method: 'exploitation',
          qValue: p.q,
          confidence: p.prob,
          visits: p.visits,
        };
      }
    }

    // Fallback to highest Q-value
    const best = qValues.reduce((a, b) => (a.q > b.q ? a : b));
    return {
      action: best.action,
      method: 'exploitation',
      qValue: best.q,
      confidence: PHI_INV, // φ⁻¹: Even best Q-value has uncertainty
      visits: best.visits,
    };
  }

  /**
   * Get top-k actions for a state
   */
  getTopActions(features, k = 3) {
    const actions = Object.values(Actions);
    const qValues = actions.map(a => ({
      action: a,
      q: this.qTable.get(features, a),
      visits: this.qTable.getVisits(features, a),
    }));

    qValues.sort((a, b) => b.q - a.q);
    return qValues.slice(0, k);
  }

  // ===========================================================================
  // LEARNING
  // ===========================================================================

  /**
   * Start a new routing episode
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
   * Record an action taken in current episode
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
   * End episode and update Q-values based on outcome
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

    // Update Q-values for each action in episode
    for (let i = 0; i < episode.actions.length; i++) {
      const action = episode.actions[i].action;
      const isLast = i === episode.actions.length - 1;

      // Q-learning update: Q(s,a) += α * (r + γ * max(Q(s',a')) - Q(s,a))
      const currentQ = this.qTable.get(episode.features, action);

      // For last action, no future state
      // For intermediate actions, use discounted future reward
      const futureQ = isLast ? 0 :
        Math.max(...Object.values(Actions).map(a =>
          this.qTable.get(episode.features, a)
        ));

      const target = reward + this.config.discountFactor * futureQ;
      const newQ = currentQ + this.config.learningRate * (target - currentQ);

      this.qTable.set(episode.features, action, newQ);
    }

    // Decay exploration rate
    this.explorationRate = Math.max(
      this.config.minExplorationRate,
      this.explorationRate * this.config.explorationDecay
    );

    // Update stats
    this.stats.totalFeedback++;
    if (outcome.success || outcome.type === 'success') {
      this.stats.correctPredictions++;
    }

    // Store episode in history
    this.episodeHistory.push(episode);
    if (this.episodeHistory.length > 1000) {
      this.episodeHistory.shift(); // Keep last 1000 episodes
    }

    // Clear current episode
    this.currentEpisode = null;

    // Persist if available
    this._persist();

    return episode;
  }

  /**
   * Calculate reward from outcome
   * @private
   */
  _calculateReward(outcome) {
    const rewards = this.config.rewards;

    if (outcome.blocked) {
      return rewards.blocked; // Blocking dangerous operations is good
    }
    if (outcome.success === true || outcome.type === 'success') {
      return rewards.success;
    }
    if (outcome.success === false || outcome.type === 'failure') {
      return rewards.failure;
    }
    if (outcome.timeout || outcome.type === 'timeout') {
      return rewards.timeout;
    }
    if (outcome.partial || outcome.type === 'partial') {
      return rewards.partialSuccess;
    }

    // Use score if available
    if (typeof outcome.score === 'number') {
      return (outcome.score - 50) / 50; // Normalize 0-100 to -1 to 1
    }

    return 0;
  }

  // ===========================================================================
  // ROUTING API
  // ===========================================================================

  /**
   * Route a task to the best dog(s)
   * Main API for routing decisions
   */
  route(context, options = {}) {
    const {
      count = 1,                    // Number of dogs to return
      mustInclude = [],             // Dogs that must be included
      exclude = [],                 // Dogs to exclude
    } = options;

    // Extract features
    const features = this.extractFeatures(context);

    // Get available actions
    let availableActions = Object.values(Actions).filter(a => !exclude.includes(a));

    // Start episode
    this.startEpisode(context);

    // Select actions
    const selections = [];

    // Include required dogs first
    for (const dog of mustInclude) {
      if (availableActions.includes(dog)) {
        selections.push({
          action: dog,
          method: 'required',
          confidence: PHI_INV, // φ⁻¹: Required ≠ perfect
        });
        availableActions = availableActions.filter(a => a !== dog);
      }
    }

    // Select remaining dogs
    while (selections.length < count && availableActions.length > 0) {
      const selection = this.selectAction(features, availableActions);
      selections.push(selection);
      this.recordAction(selection.action, { method: selection.method });
      availableActions = availableActions.filter(a => a !== selection.action);
    }

    return {
      episodeId: this.currentEpisode?.id,
      features,
      selections,
      dogs: selections.map(s => s.action),
      primaryDog: selections[0]?.action,
      confidence: selections[0]?.confidence || 0,
      explorationRate: this.explorationRate,
    };
  }

  /**
   * Provide feedback for a routing decision
   */
  feedback(episodeId, outcome) {
    // If no current episode, try to find it
    if (!this.currentEpisode || this.currentEpisode.id !== episodeId) {
      // Can't update past episodes without more infrastructure
      return null;
    }

    return this.endEpisode(outcome);
  }

  // ===========================================================================
  // PERSISTENCE
  // ===========================================================================

  /**
   * Persist Q-table
   * @private
   */
  async _persist() {
    if (!this.persistence?.query) return;

    try {
      const data = {
        qTable: this.qTable.toJSON(),
        explorationRate: this.explorationRate,
        stats: this.stats,
        timestamp: Date.now(),
      };

      await this.persistence.query(`
        INSERT INTO q_learning_router (id, data, updated_at)
        VALUES ('default', $1, NOW())
        ON CONFLICT (id) DO UPDATE SET
          data = $1,
          updated_at = NOW()
      `, [JSON.stringify(data)]);
    } catch (e) {
      // Persistence failed - continue without
    }
  }

  /**
   * Load Q-table from persistence
   */
  async load() {
    if (!this.persistence?.query) return false;

    try {
      const result = await this.persistence.query(
        'SELECT data FROM q_learning_router WHERE id = $1',
        ['default']
      );

      if (result.rows?.[0]?.data) {
        const data = result.rows[0].data;
        this.qTable = QTable.fromJSON(data.qTable);
        this.explorationRate = data.explorationRate || this.config.explorationRate;
        this.stats = { ...this.stats, ...data.stats };
        return true;
      }
    } catch (e) {
      // Load failed - start fresh
    }

    return false;
  }

  // ===========================================================================
  // STATS & DEBUGGING
  // ===========================================================================

  /**
   * Get router statistics
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
   * Get Q-value heatmap for debugging
   */
  getQHeatmap() {
    const heatmap = {};
    const features = Object.values(StateFeatures);
    const actions = Object.values(Actions);

    for (const feature of features) {
      heatmap[feature] = {};
      for (const action of actions) {
        heatmap[feature][action] = this.qTable.get([feature], action);
      }
    }

    return heatmap;
  }

  /**
   * Get best dogs per task type
   */
  getBestDogsPerTask() {
    const taskFeatures = Object.values(StateFeatures).filter(f => f.startsWith('task:'));
    const result = {};

    for (const feature of taskFeatures) {
      const taskType = feature.replace('task:', '');
      const topActions = this.getTopActions([feature], 3);
      result[taskType] = topActions.map(a => ({
        dog: a.action,
        qValue: Math.round(a.q * 100) / 100,
        visits: a.visits,
      }));
    }

    return result;
  }
}

/**
 * Create a Q-Learning Router instance
 */
export function createQLearningRouter(options = {}) {
  return new QLearningRouter(options);
}

// Singleton instance
let _router = null;

/**
 * Get the global Q-Learning Router instance
 */
export function getQLearningRouter() {
  if (!_router) {
    _router = createQLearningRouter();
  }
  return _router;
}

export default {
  QLearningRouter,
  QTable,
  Q_CONFIG,
  StateFeatures,
  Actions,
  createQLearningRouter,
  getQLearningRouter,
};
