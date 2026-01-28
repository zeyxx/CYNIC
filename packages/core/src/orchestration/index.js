/**
 * CYNIC Background Agent Orchestration
 *
 * "φ distrusts φ" - Multi-agent coordination
 *
 * Orchestration patterns:
 * 1. Parallel execution - Run multiple agents simultaneously
 * 2. Task routing - Pick best agent(s) for a task
 * 3. Result aggregation - Combine results using φ-weighted consensus
 * 4. Work stealing - Idle agents pick up pending tasks
 *
 * @module @cynic/core/orchestration
 * @philosophy Many dogs hunt better than one
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

/**
 * φ-aligned orchestration constants
 */
export const ORCHESTRATION_CONSTANTS = {
  /** Max concurrent tasks (Fib(8) = 21) */
  MAX_CONCURRENT: 21,

  /** Task timeout in ms (Fib(10) = 55 * 1000 = 55s) */
  TASK_TIMEOUT_MS: 55000,

  /** Min confidence for result acceptance */
  MIN_CONFIDENCE: PHI_INV_2, // 38.2%

  /** Consensus threshold */
  CONSENSUS_THRESHOLD: PHI_INV, // 61.8%

  /** Max retries per task (Fib(3) = 2) */
  MAX_RETRIES: 2,

  /** Work stealing check interval (Fib(7) = 13 * 100 = 1.3s) */
  WORK_STEAL_INTERVAL_MS: 1300,
};

/**
 * Task priority levels
 */
export const TaskPriority = {
  CRITICAL: 'critical',  // Immediate, blocking
  HIGH: 'high',          // Soon, may block
  NORMAL: 'normal',      // Standard queue
  LOW: 'low',            // Background, whenever
  IDLE: 'idle',          // Only when nothing else
};

/**
 * Task status
 */
export const TaskStatus = {
  PENDING: 'pending',
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  TIMEOUT: 'timeout',
  CANCELLED: 'cancelled',
};

/**
 * Aggregation strategies
 */
export const AggregationStrategy = {
  FIRST: 'first',           // Return first result
  BEST: 'best',             // Return highest confidence result
  CONSENSUS: 'consensus',   // φ-weighted consensus
  ALL: 'all',               // Return all results
  MERGE: 'merge',           // Deep merge results
};

// =============================================================================
// TASK
// =============================================================================

/**
 * Task representation
 */
export class Task {
  /**
   * @param {Object} options - Task options
   * @param {string} options.type - Task type (e.g., 'judge', 'analyze', 'search')
   * @param {Object} options.payload - Task data
   * @param {string} [options.priority] - Task priority
   * @param {string[]} [options.preferredAgents] - Preferred agent IDs
   * @param {number} [options.timeout] - Custom timeout in ms
   */
  constructor(options) {
    this.id = `task_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
    this.type = options.type;
    this.payload = options.payload || {};
    this.priority = options.priority || TaskPriority.NORMAL;
    this.preferredAgents = options.preferredAgents || [];
    this.timeout = options.timeout || ORCHESTRATION_CONSTANTS.TASK_TIMEOUT_MS;
    this.status = TaskStatus.PENDING;
    this.assignedAgent = null;
    this.result = null;
    this.error = null;
    this.createdAt = Date.now();
    this.startedAt = null;
    this.completedAt = null;
    this.retries = 0;
  }

  /**
   * Check if task is timed out
   */
  isTimedOut() {
    if (!this.startedAt) return false;
    return Date.now() - this.startedAt > this.timeout;
  }

  /**
   * Get task duration
   */
  getDuration() {
    if (!this.startedAt) return 0;
    const end = this.completedAt || Date.now();
    return end - this.startedAt;
  }

  /**
   * Mark as running
   */
  start(agentId) {
    this.status = TaskStatus.RUNNING;
    this.assignedAgent = agentId;
    this.startedAt = Date.now();
  }

  /**
   * Mark as completed
   */
  complete(result) {
    this.status = TaskStatus.COMPLETED;
    this.result = result;
    this.completedAt = Date.now();
  }

  /**
   * Mark as failed
   */
  fail(error) {
    this.status = TaskStatus.FAILED;
    this.error = error;
    this.completedAt = Date.now();
  }

  /**
   * To JSON
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      priority: this.priority,
      status: this.status,
      assignedAgent: this.assignedAgent,
      duration: this.getDuration(),
      retries: this.retries,
    };
  }
}

// =============================================================================
// ORCHESTRATOR
// =============================================================================

/**
 * Background Agent Orchestrator
 *
 * Coordinates multiple agents for parallel task execution.
 */
export class Orchestrator {
  /**
   * @param {Object} options
   * @param {Map<string, Object>} options.agents - Available agents by ID
   * @param {Object} [options.eventBus] - Event bus for coordination
   * @param {number} [options.maxConcurrent] - Max concurrent tasks
   */
  constructor(options = {}) {
    this.agents = options.agents || new Map();
    this.eventBus = options.eventBus || null;
    this.maxConcurrent = options.maxConcurrent || ORCHESTRATION_CONSTANTS.MAX_CONCURRENT;

    /** @type {Task[]} Pending task queue */
    this.queue = [];

    /** @type {Map<string, Task>} Running tasks by ID */
    this.running = new Map();

    /** @type {Task[]} Completed tasks (history) */
    this.completed = [];

    /** @type {Map<string, number>} Agent load (tasks per agent) */
    this.agentLoad = new Map();

    // Stats
    this.stats = {
      tasksCreated: 0,
      tasksCompleted: 0,
      tasksFailed: 0,
      tasksTimedOut: 0,
      totalDuration: 0,
      averageDuration: 0,
    };

    // Work stealing interval
    this._workStealInterval = null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // AGENT MANAGEMENT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register an agent
   * @param {string} agentId - Agent identifier
   * @param {Object} agent - Agent instance
   * @param {Object} [capabilities] - Agent capabilities
   */
  registerAgent(agentId, agent, capabilities = {}) {
    this.agents.set(agentId, {
      agent,
      capabilities,
      available: true,
      tasksCompleted: 0,
      averageTime: 0,
    });
    this.agentLoad.set(agentId, 0);
  }

  /**
   * Unregister an agent
   * @param {string} agentId
   */
  unregisterAgent(agentId) {
    this.agents.delete(agentId);
    this.agentLoad.delete(agentId);
  }

  /**
   * Get available agents for a task type
   * @param {string} taskType
   * @param {string[]} [preferred] - Preferred agent IDs
   */
  getAvailableAgents(taskType, preferred = []) {
    const available = [];

    for (const [agentId, info] of this.agents) {
      if (!info.available) continue;

      // Check if agent can handle task type
      const capabilities = info.capabilities.taskTypes || [];
      if (capabilities.length > 0 && !capabilities.includes(taskType)) {
        continue;
      }

      // Check load
      const load = this.agentLoad.get(agentId) || 0;
      if (load >= (info.capabilities.maxConcurrent || 3)) {
        continue;
      }

      available.push({
        agentId,
        agent: info.agent,
        load,
        isPreferred: preferred.includes(agentId),
        averageTime: info.averageTime,
      });
    }

    // Sort: preferred first, then by load, then by average time
    available.sort((a, b) => {
      if (a.isPreferred !== b.isPreferred) return b.isPreferred - a.isPreferred;
      if (a.load !== b.load) return a.load - b.load;
      return a.averageTime - b.averageTime;
    });

    return available;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TASK ROUTING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Route task to best agent
   * @param {Task} task
   * @returns {Object|null} Selected agent info
   */
  routeTask(task) {
    const available = this.getAvailableAgents(task.type, task.preferredAgents);

    if (available.length === 0) {
      return null;
    }

    // Return best match (first after sorting)
    return available[0];
  }

  /**
   * Route task to multiple agents (for parallel/consensus)
   * @param {Task} task
   * @param {number} count - Number of agents
   * @returns {Object[]} Selected agents
   */
  routeToMultiple(task, count = 3) {
    const available = this.getAvailableAgents(task.type, task.preferredAgents);
    return available.slice(0, Math.min(count, available.length));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TASK EXECUTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Submit a task
   * @param {Object} taskOptions - Task options
   * @returns {Task} Created task
   */
  submit(taskOptions) {
    const task = new Task(taskOptions);
    this.stats.tasksCreated++;

    // Add to queue based on priority
    this._enqueue(task);

    // Try to schedule immediately
    this._scheduleNext();

    return task;
  }

  /**
   * Execute a single task (internal)
   * @private
   */
  async _executeTask(task, agentInfo) {
    const { agentId, agent } = agentInfo;

    task.start(agentId);
    this.running.set(task.id, task);
    this.agentLoad.set(agentId, (this.agentLoad.get(agentId) || 0) + 1);

    try {
      // Execute with timeout
      const result = await Promise.race([
        this._runAgent(agent, task),
        this._timeout(task.timeout),
      ]);

      if (result === 'TIMEOUT') {
        task.status = TaskStatus.TIMEOUT;
        task.error = 'Task timed out';
        this.stats.tasksTimedOut++;
      } else {
        task.complete(result);
        this.stats.tasksCompleted++;
      }
    } catch (error) {
      task.fail(error.message || error);
      this.stats.tasksFailed++;
    } finally {
      // Cleanup
      this.running.delete(task.id);
      this.agentLoad.set(agentId, Math.max(0, (this.agentLoad.get(agentId) || 1) - 1));

      // Update agent stats
      const info = this.agents.get(agentId);
      if (info) {
        info.tasksCompleted++;
        const duration = task.getDuration();
        info.averageTime = (info.averageTime * (info.tasksCompleted - 1) + duration) / info.tasksCompleted;
      }

      // Update global stats
      this.stats.totalDuration += task.getDuration();
      this.stats.averageDuration = this.stats.totalDuration / (this.stats.tasksCompleted + this.stats.tasksFailed);

      // Move to completed
      this._addToCompleted(task);

      // Schedule next
      this._scheduleNext();
    }
  }

  /**
   * Run agent on task
   * @private
   */
  async _runAgent(agent, task) {
    // Check for process method (BaseAgent)
    if (typeof agent.process === 'function') {
      return agent.process({ type: task.type, ...task.payload }, {});
    }

    // Check for execute method
    if (typeof agent.execute === 'function') {
      return agent.execute(task.payload);
    }

    // Check for judge method
    if (typeof agent.judge === 'function') {
      return agent.judge(task.payload);
    }

    // Direct function call
    if (typeof agent === 'function') {
      return agent(task.payload);
    }

    throw new Error(`Agent has no executable method`);
  }

  /**
   * Timeout promise
   * @private
   */
  _timeout(ms) {
    return new Promise(resolve => setTimeout(() => resolve('TIMEOUT'), ms));
  }

  /**
   * Enqueue task by priority
   * @private
   */
  _enqueue(task) {
    const priorityOrder = {
      [TaskPriority.CRITICAL]: 0,
      [TaskPriority.HIGH]: 1,
      [TaskPriority.NORMAL]: 2,
      [TaskPriority.LOW]: 3,
      [TaskPriority.IDLE]: 4,
    };

    // Find insertion point
    let insertIndex = this.queue.length;
    for (let i = 0; i < this.queue.length; i++) {
      if (priorityOrder[task.priority] < priorityOrder[this.queue[i].priority]) {
        insertIndex = i;
        break;
      }
    }

    this.queue.splice(insertIndex, 0, task);
  }

  /**
   * Schedule next task from queue
   * @private
   */
  _scheduleNext() {
    // Check capacity
    if (this.running.size >= this.maxConcurrent) {
      return;
    }

    // Get next task
    const task = this.queue.shift();
    if (!task) {
      return;
    }

    // Route to agent
    const agentInfo = this.routeTask(task);
    if (!agentInfo) {
      // No available agent, re-queue
      this.queue.unshift(task);
      return;
    }

    // Execute
    this._executeTask(task, agentInfo);
  }

  /**
   * Add to completed history
   * @private
   */
  _addToCompleted(task) {
    this.completed.push(task);

    // Trim history (keep last 100)
    while (this.completed.length > 100) {
      this.completed.shift();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PARALLEL EXECUTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Execute task in parallel across multiple agents
   *
   * @param {Object} taskOptions - Task options
   * @param {Object} [options] - Parallel options
   * @param {number} [options.count] - Number of agents
   * @param {string} [options.strategy] - Aggregation strategy
   * @returns {Promise<Object>} Aggregated result
   */
  async parallel(taskOptions, options = {}) {
    const {
      count = 3,
      strategy = AggregationStrategy.CONSENSUS,
    } = options;

    const task = new Task(taskOptions);
    const agents = this.routeToMultiple(task, count);

    if (agents.length === 0) {
      throw new Error('No available agents for parallel execution');
    }

    // Execute in parallel
    const promises = agents.map(agentInfo =>
      this._runAgent(agentInfo.agent, task)
        .then(result => ({ agentId: agentInfo.agentId, result, error: null }))
        .catch(error => ({ agentId: agentInfo.agentId, result: null, error: error.message }))
    );

    const results = await Promise.all(promises);

    // Aggregate results
    return this.aggregate(results, strategy);
  }

  /**
   * Execute multiple different tasks in parallel
   *
   * @param {Object[]} taskOptionsList - Array of task options
   * @returns {Promise<Task[]>} Completed tasks
   */
  async batch(taskOptionsList) {
    const tasks = taskOptionsList.map(opts => new Task(opts));

    // Submit all
    const promises = tasks.map(task => {
      return new Promise((resolve, reject) => {
        const agentInfo = this.routeTask(task);
        if (!agentInfo) {
          task.fail('No available agent');
          resolve(task);
          return;
        }

        this._runAgent(agentInfo.agent, task)
          .then(result => {
            task.complete(result);
            resolve(task);
          })
          .catch(error => {
            task.fail(error.message);
            resolve(task);
          });
      });
    });

    return Promise.all(promises);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RESULT AGGREGATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Aggregate results from multiple agents
   *
   * @param {Object[]} results - Array of { agentId, result, error }
   * @param {string} strategy - Aggregation strategy
   * @returns {Object} Aggregated result
   */
  aggregate(results, strategy = AggregationStrategy.CONSENSUS) {
    // Filter successful results
    const successful = results.filter(r => !r.error && r.result);

    if (successful.length === 0) {
      return {
        success: false,
        error: 'All agents failed',
        agentErrors: results.map(r => ({ agentId: r.agentId, error: r.error })),
      };
    }

    switch (strategy) {
      case AggregationStrategy.FIRST:
        return {
          success: true,
          result: successful[0].result,
          agentId: successful[0].agentId,
          strategy,
        };

      case AggregationStrategy.BEST:
        return this._aggregateBest(successful, strategy);

      case AggregationStrategy.CONSENSUS:
        return this._aggregateConsensus(successful, strategy);

      case AggregationStrategy.ALL:
        return {
          success: true,
          results: successful.map(r => ({ agentId: r.agentId, result: r.result })),
          strategy,
        };

      case AggregationStrategy.MERGE:
        return this._aggregateMerge(successful, strategy);

      default:
        return {
          success: true,
          result: successful[0].result,
          strategy: 'fallback',
        };
    }
  }

  /**
   * Select best result by confidence/score
   * @private
   */
  _aggregateBest(results, strategy) {
    let best = results[0];
    let bestScore = this._getResultScore(best.result);

    for (const r of results.slice(1)) {
      const score = this._getResultScore(r.result);
      if (score > bestScore) {
        best = r;
        bestScore = score;
      }
    }

    return {
      success: true,
      result: best.result,
      agentId: best.agentId,
      score: bestScore,
      strategy,
    };
  }

  /**
   * Consensus aggregation using φ-weighted voting
   * @private
   */
  _aggregateConsensus(results, strategy) {
    // Extract scores/verdicts
    const votes = results.map(r => ({
      agentId: r.agentId,
      score: this._getResultScore(r.result),
      verdict: r.result?.verdict || r.result?.response || 'unknown',
      confidence: r.result?.confidence || PHI_INV_2,
    }));

    // Calculate weighted average score
    let totalWeight = 0;
    let weightedSum = 0;
    const verdictCounts = {};

    for (const vote of votes) {
      const weight = vote.confidence;
      totalWeight += weight;
      weightedSum += vote.score * weight;

      verdictCounts[vote.verdict] = (verdictCounts[vote.verdict] || 0) + 1;
    }

    const consensusScore = totalWeight > 0 ? weightedSum / totalWeight : 0;

    // Find majority verdict
    let majorityVerdict = null;
    let maxCount = 0;
    for (const [verdict, count] of Object.entries(verdictCounts)) {
      if (count > maxCount) {
        majorityVerdict = verdict;
        maxCount = count;
      }
    }

    // Check if we have consensus (>= φ⁻¹ agreement)
    const agreementRatio = maxCount / votes.length;
    const hasConsensus = agreementRatio >= ORCHESTRATION_CONSTANTS.CONSENSUS_THRESHOLD;

    return {
      success: true,
      consensusScore,
      verdict: majorityVerdict,
      hasConsensus,
      agreementRatio,
      votes,
      strategy,
      confidence: Math.min(agreementRatio, PHI_INV), // Cap at φ⁻¹
    };
  }

  /**
   * Merge results (deep merge)
   * @private
   */
  _aggregateMerge(results, strategy) {
    const merged = {};

    for (const r of results) {
      if (typeof r.result === 'object' && r.result !== null) {
        this._deepMerge(merged, r.result);
      }
    }

    return {
      success: true,
      result: merged,
      agentCount: results.length,
      strategy,
    };
  }

  /**
   * Deep merge objects
   * @private
   */
  _deepMerge(target, source) {
    for (const key of Object.keys(source)) {
      if (source[key] instanceof Object && key in target) {
        this._deepMerge(target[key], source[key]);
      } else {
        target[key] = source[key];
      }
    }
  }

  /**
   * Extract score from result
   * @private
   */
  _getResultScore(result) {
    if (typeof result === 'number') return result;
    if (!result) return 0;
    return result.score || result.Q || result.qScore || result.confidence * 100 || 50;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // WORK STEALING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start work stealing (idle agents pick up tasks)
   */
  startWorkStealing() {
    if (this._workStealInterval) return;

    this._workStealInterval = setInterval(() => {
      this._scheduleNext();
    }, ORCHESTRATION_CONSTANTS.WORK_STEAL_INTERVAL_MS);
  }

  /**
   * Stop work stealing
   */
  stopWorkStealing() {
    if (this._workStealInterval) {
      clearInterval(this._workStealInterval);
      this._workStealInterval = null;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATUS & STATS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get orchestrator status
   */
  getStatus() {
    return {
      agents: {
        total: this.agents.size,
        available: [...this.agents.values()].filter(a => a.available).length,
        load: Object.fromEntries(this.agentLoad),
      },
      tasks: {
        queued: this.queue.length,
        running: this.running.size,
        completed: this.completed.length,
      },
      stats: this.stats,
      workStealing: !!this._workStealInterval,
    };
  }

  /**
   * Get task by ID
   */
  getTask(taskId) {
    // Check running
    if (this.running.has(taskId)) {
      return this.running.get(taskId);
    }

    // Check queue
    const queued = this.queue.find(t => t.id === taskId);
    if (queued) return queued;

    // Check completed
    return this.completed.find(t => t.id === taskId) || null;
  }

  /**
   * Cancel a task
   */
  cancel(taskId) {
    // Remove from queue
    const queueIndex = this.queue.findIndex(t => t.id === taskId);
    if (queueIndex !== -1) {
      const task = this.queue.splice(queueIndex, 1)[0];
      task.status = TaskStatus.CANCELLED;
      return true;
    }

    // Can't cancel running tasks (would need kill mechanism)
    return false;
  }

  /**
   * Shutdown orchestrator
   */
  shutdown() {
    this.stopWorkStealing();
    this.queue = [];
    // Note: running tasks will complete or timeout
  }
}

// =============================================================================
// PACK COORDINATION - CONSULTATION MATRIX
// =============================================================================

/**
 * Consultation Matrix
 *
 * Defines which agents should consult which others for specific tasks.
 * Format: { agentRole: { taskType: [agents to consult] } }
 *
 * Rules:
 * 1. Never consult yourself
 * 2. Max 2 consultations per decision
 * 3. Security (Guardian) consulted for dangerous operations
 * 4. Memory (Archivist) consulted for pattern matching
 */
export const CONSULTATION_MATRIX = {
  // Architect consults for design decisions
  architect: {
    design: ['reviewer', 'simplifier'],      // Get review + simplification suggestions
    security: ['guardian'],                   // Security review
    patterns: ['archivist', 'oracle'],        // Historical patterns + insights
  },

  // Reviewer consults for code review
  reviewer: {
    quality: ['simplifier'],                  // Simplification opportunities
    security: ['guardian'],                   // Security issues
    patterns: ['archivist'],                  // Past review patterns
  },

  // Guardian consults for security decisions
  guardian: {
    severity: ['architect'],                  // Architecture impact
    patterns: ['archivist'],                  // Past security incidents
  },

  // Scout consults for exploration
  scout: {
    deep_search: ['cartographer'],            // Map structure
    patterns: ['archivist'],                  // What we found before
  },

  // Tester consults for test analysis
  tester: {
    coverage: ['architect'],                  // What should be tested
    failures: ['reviewer'],                   // Code review for failures
  },

  // Simplifier consults for refactoring
  simplifier: {
    impact: ['architect'],                    // Architecture impact
    tests: ['tester'],                        // Test coverage concerns
  },

  // Deployer consults for deployment
  deployer: {
    safety: ['guardian', 'tester'],           // Security + tests passing
    rollback: ['architect'],                  // Architecture dependencies
  },

  // Oracle consults for insights
  oracle: {
    history: ['archivist'],                   // Historical data
    ecosystem: ['integrator'],                // Cross-project view
  },

  // Integrator consults for ecosystem changes
  integrator: {
    impact: ['architect', 'tester'],          // Design + test impact
    history: ['archivist'],                   // Past integrations
  },

  // Doc consults for documentation
  doc: {
    accuracy: ['reviewer'],                   // Code accuracy
    completeness: ['architect'],              // What should be documented
  },

  // Librarian consults for documentation cache
  librarian: {
    relevance: ['archivist'],                 // What's been useful
  },

  // Cartographer consults for mapping
  cartographer: {
    deep: ['scout'],                          // Detailed exploration
    patterns: ['archivist'],                  // Historical structure
  },

  // Archivist consults for memory
  archivist: {
    relevance: ['oracle'],                    // Pattern insights
  },

  // Solana Expert consults for blockchain
  'solana-expert': {
    security: ['guardian'],                   // Security review
    patterns: ['archivist'],                  // Past solutions
  },
};

/**
 * Get consultants for a given agent and task type
 *
 * @param {string} agentRole - The agent's role
 * @param {string} taskType - The type of task
 * @returns {string[]} List of agent roles to consult
 */
export function getConsultants(agentRole, taskType) {
  const matrix = CONSULTATION_MATRIX[agentRole];
  if (!matrix) return [];

  return matrix[taskType] || matrix['default'] || [];
}

/**
 * Check if consultation is needed
 *
 * @param {string} agentRole - The agent's role
 * @param {string} taskType - The type of task
 * @param {Object} [context] - Additional context
 * @returns {Object} { needed: boolean, consultants: string[], reason: string }
 */
export function shouldConsult(agentRole, taskType, context = {}) {
  const { confidence = 0.5, isHighRisk = false, hasPatternMatch = false } = context;

  // Always consult for high-risk operations
  if (isHighRisk) {
    const securityConsultants = ['guardian'];
    return {
      needed: true,
      consultants: securityConsultants,
      reason: 'High-risk operation requires security review',
    };
  }

  // Consult if confidence is below threshold
  if (confidence < PHI_INV_2) {
    const consultants = getConsultants(agentRole, taskType);
    if (consultants.length > 0) {
      return {
        needed: true,
        consultants,
        reason: `Low confidence (${(confidence * 100).toFixed(1)}%) - seeking second opinion`,
      };
    }
  }

  // Consult archivist if pattern match detected
  if (hasPatternMatch) {
    return {
      needed: true,
      consultants: ['archivist'],
      reason: 'Pattern match detected - checking collective memory',
    };
  }

  return {
    needed: false,
    consultants: [],
    reason: 'No consultation needed',
  };
}

// =============================================================================
// PACK COORDINATION - CIRCUIT BREAKER
// =============================================================================

/**
 * Circuit Breaker Constants
 */
export const CIRCUIT_BREAKER_CONSTANTS = {
  /** Max consultation depth */
  MAX_DEPTH: 3,

  /** Max total consultations per request */
  MAX_CONSULTATIONS: 5,

  /** Token budget for consultations (φ⁻² of context) */
  CONSULTATION_BUDGET_RATIO: PHI_INV_2,

  /** Timeout for single consultation (Fib(8) = 21s) */
  CONSULTATION_TIMEOUT_MS: 21000,

  /** Cooldown between same agent consultations (Fib(5) = 5s) */
  SAME_AGENT_COOLDOWN_MS: 5000,
};

/**
 * Circuit Breaker for Agent Consultation
 *
 * Prevents infinite loops and runaway consultation chains.
 *
 * @example
 * const breaker = new ConsultationCircuitBreaker();
 *
 * // Before consulting
 * if (!breaker.canConsult('architect', 'reviewer')) {
 *   // Skip consultation
 * }
 *
 * // After consulting
 * breaker.recordConsultation('architect', 'reviewer', { tokens: 500 });
 */
export class ConsultationCircuitBreaker {
  constructor(options = {}) {
    this.maxDepth = options.maxDepth || CIRCUIT_BREAKER_CONSTANTS.MAX_DEPTH;
    this.maxConsultations = options.maxConsultations || CIRCUIT_BREAKER_CONSTANTS.MAX_CONSULTATIONS;
    this.tokenBudget = options.tokenBudget || 50000; // Default ~50k tokens for consultations

    // State
    this.visited = new Set();           // Set of "from->to" edges visited
    this.consultationCount = 0;         // Total consultations in this chain
    this.currentDepth = 0;              // Current consultation depth
    this.tokensUsed = 0;                // Tokens used in consultations
    this.lastConsultations = new Map(); // agentId -> timestamp (for cooldown)
    this.chain = [];                    // Consultation chain for debugging
  }

  /**
   * Check if consultation is allowed
   *
   * @param {string} fromAgent - Agent requesting consultation
   * @param {string} toAgent - Agent being consulted
   * @param {Object} [context] - Additional context
   * @returns {Object} { allowed: boolean, reason: string }
   */
  canConsult(fromAgent, toAgent, context = {}) {
    const { estimatedTokens = 0 } = context;

    // Rule 1: No self-consultation
    if (fromAgent === toAgent) {
      return { allowed: false, reason: 'Cannot consult self' };
    }

    // Rule 2: Check max depth
    if (this.currentDepth >= this.maxDepth) {
      return {
        allowed: false,
        reason: `Max consultation depth reached (${this.maxDepth})`,
      };
    }

    // Rule 3: Check max consultations
    if (this.consultationCount >= this.maxConsultations) {
      return {
        allowed: false,
        reason: `Max consultations reached (${this.maxConsultations})`,
      };
    }

    // Rule 4: Check for cycles (already visited this edge)
    const edge = `${fromAgent}->${toAgent}`;
    if (this.visited.has(edge)) {
      return {
        allowed: false,
        reason: `Cycle detected: ${edge} already visited`,
      };
    }

    // Rule 5: Check token budget
    if (this.tokensUsed + estimatedTokens > this.tokenBudget) {
      return {
        allowed: false,
        reason: `Token budget exceeded (${this.tokensUsed}/${this.tokenBudget})`,
      };
    }

    // Rule 6: Check cooldown for same agent
    const lastTime = this.lastConsultations.get(toAgent);
    if (lastTime) {
      const elapsed = Date.now() - lastTime;
      if (elapsed < CIRCUIT_BREAKER_CONSTANTS.SAME_AGENT_COOLDOWN_MS) {
        return {
          allowed: false,
          reason: `Cooldown active for ${toAgent} (${Math.ceil((CIRCUIT_BREAKER_CONSTANTS.SAME_AGENT_COOLDOWN_MS - elapsed) / 1000)}s remaining)`,
        };
      }
    }

    return { allowed: true, reason: 'Consultation permitted' };
  }

  /**
   * Record a consultation (call before consulting)
   *
   * @param {string} fromAgent
   * @param {string} toAgent
   */
  enterConsultation(fromAgent, toAgent) {
    const edge = `${fromAgent}->${toAgent}`;
    this.visited.add(edge);
    this.consultationCount++;
    this.currentDepth++;
    this.chain.push({ from: fromAgent, to: toAgent, depth: this.currentDepth, time: Date.now() });
  }

  /**
   * Record consultation completion
   *
   * @param {string} fromAgent
   * @param {string} toAgent
   * @param {Object} [result] - Consultation result
   */
  exitConsultation(fromAgent, toAgent, result = {}) {
    const { tokensUsed = 0 } = result;
    this.currentDepth = Math.max(0, this.currentDepth - 1);
    this.tokensUsed += tokensUsed;
    this.lastConsultations.set(toAgent, Date.now());
  }

  /**
   * Get circuit breaker status
   */
  getStatus() {
    return {
      depth: this.currentDepth,
      maxDepth: this.maxDepth,
      consultations: this.consultationCount,
      maxConsultations: this.maxConsultations,
      tokensUsed: this.tokensUsed,
      tokenBudget: this.tokenBudget,
      visitedEdges: [...this.visited],
      chain: this.chain,
      isOpen: this.consultationCount >= this.maxConsultations ||
              this.currentDepth >= this.maxDepth ||
              this.tokensUsed >= this.tokenBudget,
    };
  }

  /**
   * Reset circuit breaker (for new request)
   */
  reset() {
    this.visited.clear();
    this.consultationCount = 0;
    this.currentDepth = 0;
    this.tokensUsed = 0;
    this.chain = [];
    // Note: lastConsultations preserved for cooldown across requests
  }

  /**
   * Full reset including cooldowns
   */
  fullReset() {
    this.reset();
    this.lastConsultations.clear();
  }
}

// =============================================================================
// PACK EFFECTIVENESS METRICS
// =============================================================================

/**
 * Calculate Pack Effectiveness
 *
 * E = Quality × Speed × Coherence
 *
 * @param {Object} metrics - Pack metrics
 * @returns {Object} Effectiveness calculation
 */
export function calculatePackEffectiveness(metrics) {
  const {
    avgQScore = 50,           // Average Q-Score of outputs
    avgResponseTime = 10000,  // Average response time in ms
    consensusRate = 0.5,      // Rate of consensus reached
    consultationSuccess = 0.5, // Rate of successful consultations
  } = metrics;

  // Quality: Normalize Q-Score to 0-1
  const quality = avgQScore / 100;

  // Speed: Inverse of response time, normalized (10s = 1.0, 60s = 0.17)
  const targetTime = 10000; // 10 seconds target
  const speed = Math.min(1, targetTime / Math.max(avgResponseTime, 1000));

  // Coherence: Combination of consensus and consultation success
  const coherence = (consensusRate + consultationSuccess) / 2;

  // Effectiveness = geometric mean (forces balance)
  const effectiveness = Math.pow(quality * speed * coherence, 1/3) * 100;

  return {
    E: Math.round(effectiveness * 10) / 10,
    breakdown: {
      quality: Math.round(quality * 100),
      speed: Math.round(speed * 100),
      coherence: Math.round(coherence * 100),
    },
    raw: {
      avgQScore,
      avgResponseTime,
      consensusRate,
      consultationSuccess,
    },
    formula: 'E = ∛(Quality × Speed × Coherence) × 100',
  };
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Constants
  ORCHESTRATION_CONSTANTS,
  TaskPriority,
  TaskStatus,
  AggregationStrategy,
  CONSULTATION_MATRIX,
  CIRCUIT_BREAKER_CONSTANTS,

  // Classes
  Task,
  Orchestrator,
  ConsultationCircuitBreaker,

  // Functions
  getConsultants,
  shouldConsult,
  calculatePackEffectiveness,
};
