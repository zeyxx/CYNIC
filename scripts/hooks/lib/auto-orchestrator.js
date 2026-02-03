/**
 * CYNIC Auto-Orchestrator
 *
 * Automatic Dog consultation and consensus for hooks.
 * No more ad-hoc LLM decisions - the collective decides.
 *
 * "Le collectif décide" - κυνικός
 *
 * Architecture:
 * ```
 *   Hook Event ──→ AutoOrchestrator ──→ CollectivePack ──→ KabbalisticRouter
 *                         │                    │
 *                         ▼                    ▼
 *                  NeuronalConsensus    11 Dogs (Sefirot)
 *                         │                    │
 *                         └────── Decision ────┘
 * ```
 *
 * @module scripts/hooks/lib/auto-orchestrator
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/** Auto-orchestrator configuration */
const CONFIG = {
  /** Enable neuronal consensus for high-risk decisions */
  USE_NEURONAL_CONSENSUS: true,

  /** Timeout for synchronous checks (ms) */
  SYNC_TIMEOUT_MS: 2000,

  /** Timeout for async analysis (ms) */
  ASYNC_TIMEOUT_MS: 5000,

  /** Risk threshold for full consensus */
  HIGH_RISK_THRESHOLD: 0.6,

  /** Cache TTL for repeated commands (ms) */
  CACHE_TTL_MS: 30000,
};

/**
 * Tools that should always trigger full consensus
 */
const HIGH_RISK_TOOLS = new Set([
  'bash', 'write', 'edit', 'notebookedit',
  'mcp__github__push_files',
  'mcp__github__create_pull_request',
  'mcp__github__merge_pull_request',
  'mcp__render__create_web_service',
  'mcp__render__update_web_service',
]);

/**
 * Tools that can skip consensus (read-only)
 */
const SAFE_TOOLS = new Set([
  'read', 'glob', 'grep', 'ls', 'websearch', 'webfetch',
  'tasklist', 'taskget',
]);

// ═══════════════════════════════════════════════════════════════════════════
// AUTO-ORCHESTRATOR
// ═══════════════════════════════════════════════════════════════════════════

/**
 * AutoOrchestrator - Automatic Dog consultation for hooks
 *
 * Usage:
 * ```javascript
 * const orchestrator = await getAutoOrchestrator();
 *
 * // In PreToolUse hook:
 * const result = await orchestrator.preCheck(event);
 * if (result.blocked) return { decision: 'block', message: result.message };
 *
 * // In PostToolUse hook:
 * await orchestrator.postAnalyze(event);
 * ```
 */
class AutoOrchestrator {
  constructor() {
    this._collectivePack = null;
    this._neuronalConsensus = null;
    this._swarmConsensus = null;
    this._planningGate = null;
    this._initialized = false;
    this._initPromise = null;
    this._decisionCache = new Map();

    // Stats
    this.stats = {
      preChecks: 0,
      postAnalyzes: 0,
      consensusRequests: 0,
      planningTriggered: 0,
      planningPaused: 0,
      blocksIssued: 0,
      cacheHits: 0,
      errors: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // INITIALIZATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize the orchestrator (lazy, singleton)
   * @returns {Promise<boolean>} Success
   */
  async initialize() {
    if (this._initialized) return true;
    if (this._initPromise) return this._initPromise;

    this._initPromise = this._doInitialize();
    return this._initPromise;
  }

  /**
   * Actual initialization logic
   * @private
   */
  async _doInitialize() {
    try {
      // ═══════════════════════════════════════════════════════════════════════════
      // USE SINGLETON - "One pack, one truth"
      // This ensures hooks and MCP server share the same Dogs and Memory
      // ═══════════════════════════════════════════════════════════════════════════
      const { getCollectivePack } = await import('@cynic/node');

      // Try to get persistence for storage
      let persistence = null;
      try {
        const { getPersistence } = await import('@cynic/persistence');
        persistence = await getPersistence();
      } catch {
        // Persistence optional
      }

      // Get CollectivePack SINGLETON (same instance as MCP server!)
      this._collectivePack = getCollectivePack({
        persistence,
        onDogDecision: (decision) => this._handleDogDecision(decision),
      });

      // Import NeuronalConsensus if available
      if (CONFIG.USE_NEURONAL_CONSENSUS) {
        try {
          const { createSecurityNeuron } = await import('@cynic/node/agents/neuronal-consensus');
          this._neuronalConsensus = createSecurityNeuron();
        } catch {
          // Neuronal consensus optional
        }
      }

      // Import SwarmConsensus if available
      try {
        const { getSwarmConsensus } = await import('@cynic/node/agents/swarm-consensus');
        this._swarmConsensus = getSwarmConsensus();
      } catch {
        // Swarm consensus optional
      }

      // Import PlanningGate for meta-cognition
      try {
        const { getPlanningGate } = await import('@cynic/node/orchestration');
        this._planningGate = getPlanningGate();
      } catch {
        // Planning gate optional
      }

      // Import ChaosGenerator for chaos engineering
      // "Un système qui survit au hasard survit à tout"
      try {
        const { getChaosGenerator } = await import('@cynic/node/chaos');
        this._chaosGenerator = getChaosGenerator();
        // Enable chaos only in dev/test or when explicitly requested
        if (process.env.CYNIC_CHAOS_ENABLED === 'true') {
          this._chaosGenerator.enable();
        }
      } catch {
        // Chaos generator optional
      }

      this._initialized = true;
      return true;
    } catch (error) {
      console.error('[AutoOrchestrator] Init error:', error.message);
      this.stats.errors++;
      return false;
    }
  }

  /**
   * Check if initialized
   * @returns {boolean}
   */
  isInitialized() {
    return this._initialized;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRE-CHECK (PreToolUse)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Pre-check before tool execution
   *
   * @param {Object} event - Hook event
   * @param {string} event.tool - Tool name
   * @param {Object} event.input - Tool input
   * @param {string} [event.userId] - User ID
   * @param {string} [event.sessionId] - Session ID
   * @returns {Promise<PreCheckResult>} Pre-check result
   */
  async preCheck(event) {
    this.stats.preChecks++;

    // Ensure initialized
    if (!this._initialized) {
      await this.initialize();
    }

    const { tool = '', input = {} } = event;
    const toolLower = tool.toLowerCase();

    // Fast path: safe tools skip consensus
    if (SAFE_TOOLS.has(toolLower)) {
      return {
        blocked: false,
        confidence: PHI_INV,
        reason: 'Safe tool - consensus skipped',
        fromCache: false,
      };
    }

    // Check cache for repeated commands
    const cacheKey = this._getCacheKey(tool, input);
    const cached = this._checkCache(cacheKey);
    if (cached) {
      this.stats.cacheHits++;
      return { ...cached, fromCache: true };
    }

    // Determine risk level
    const isHighRisk = HIGH_RISK_TOOLS.has(toolLower) ||
      this._assessRisk(tool, input) > CONFIG.HIGH_RISK_THRESHOLD;

    // ═══════════════════════════════════════════════════════════════════════════
    // CHAOS ENGINEERING: Random planning triggers to test robustness
    // "Un système qui survit au hasard survit à tout"
    // ═══════════════════════════════════════════════════════════════════════════
    let chaosResult = null;
    let forcePlanningFromChaos = false;

    if (this._chaosGenerator && this._chaosGenerator.enabled) {
      try {
        // Check if chaos should force planning on this task
        chaosResult = this._chaosGenerator.shouldForcePlanning({
          content: `${tool}: ${JSON.stringify(input).slice(0, 200)}`,
          context: { tool, isHighRisk },
        });

        if (chaosResult.injected) {
          forcePlanningFromChaos = true;
          this.stats.chaosInjected = (this.stats.chaosInjected || 0) + 1;
        }
      } catch (e) {
        // Chaos failure should not block operation
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PLANNING GATE: Meta-cognition layer - think before acting
    // "Un système qui pense avant d'agir"
    // ═══════════════════════════════════════════════════════════════════════════
    const shouldCheckPlanning = isHighRisk || forcePlanningFromChaos;
    if (this._planningGate && shouldCheckPlanning) {
      try {
        // Create minimal decision event for planning gate
        const planningEvent = {
          id: `hook-${Date.now()}`,
          content: `${tool}: ${JSON.stringify(input).slice(0, 200)}`,
          context: {
            tool,
            input,
            isHighRisk,
            skipPlanning: false,
            chaosInjected: forcePlanningFromChaos,
            chaosId: chaosResult?.id,
          },
          userContext: {
            userId: event.userId,
            trustLevel: 'BUILDER', // Default, could be enhanced
          },
          routing: { risk: isHighRisk ? 'high' : 'medium' },
          judgment: null,
          setPlanning: function(p) { this.planning = p; },
          recordError: function() {},
        };

        // If chaos forced planning, add chaos_test trigger
        const chaosContext = forcePlanningFromChaos ? { forcedByChaos: true } : {};
        const planningResult = this._planningGate.shouldPlan(planningEvent, {
          complexity: isHighRisk ? 0.3 : (forcePlanningFromChaos ? 0.4 : 0.7),
          confidence: 0.5,
          ...chaosContext,
        });

        if (planningResult.needed || forcePlanningFromChaos) {
          this.stats.planningTriggered++;

          // Add chaos trigger if applicable
          if (forcePlanningFromChaos && !planningResult.triggers?.includes('chaos_test')) {
            planningResult.triggers = [...(planningResult.triggers || []), 'chaos_test'];
          }

          // Generate plan with alternatives
          await this._planningGate.generatePlan(planningEvent, planningResult);

          // If PAUSE decision, return for human approval
          if (planningResult.decision === 'pause' || forcePlanningFromChaos) {
            this.stats.planningPaused++;

            return {
              blocked: false,
              needsPlanning: true,
              planningDecision: 'pause',
              triggers: planningResult.triggers,
              alternatives: planningResult.alternatives,
              plan: planningResult.plan,
              confidence: planningResult.confidence,
              reason: `Planning triggered: ${planningResult.triggers.join(', ')}`,
              isHighRisk,
              chaosId: chaosResult?.id,  // For tracking chaos results
              chaosInjected: forcePlanningFromChaos,
            };
          }
        }
      } catch (planErr) {
        // DEFENSIVE: Planning gate failure should NOT block execution
        console.error('[AutoOrchestrator] Planning gate error (non-blocking):', planErr.message);
      }
    }

    // Route through CollectivePack (KabbalisticRouter)
    try {
      const result = await this._withTimeout(
        this._collectivePack?.receiveHookEvent({
          hookType: 'PreToolUse',
          payload: { tool, input, isHighRisk },
          userId: event.userId,
          sessionId: event.sessionId,
        }),
        CONFIG.SYNC_TIMEOUT_MS
      );

      // If high-risk and not blocked, run neuronal consensus
      if (isHighRisk && !result?.blocked && this._neuronalConsensus) {
        const consensusResult = await this._runNeuronalConsensus(result);
        if (consensusResult.shouldBlock) {
          result.blocked = true;
          result.blockedBy = 'neuronal_consensus';
          result.blockMessage = consensusResult.reason;
        }
      }

      const decision = {
        blocked: result?.blocked || false,
        blockedBy: result?.blockedBy || null,
        message: result?.blockMessage || null,
        confidence: result?.synthesis?.confidence || PHI_INV,
        reason: result?.synthesis?.reason || 'Approved by collective',
        agentResults: result?.agentResults || [],
        isHighRisk,
      };

      // Cache the decision
      this._setCache(cacheKey, decision);

      if (decision.blocked) {
        this.stats.blocksIssued++;
      }

      return decision;
    } catch (error) {
      this.stats.errors++;
      // Fail open on error (don't block user)
      return {
        blocked: false,
        confidence: 0.3,
        reason: `Orchestration error: ${error.message}`,
        error: error.message,
      };
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // POST-ANALYZE (PostToolUse)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Analyze after tool execution (non-blocking)
   *
   * @param {Object} event - Hook event
   * @param {string} event.tool - Tool name
   * @param {Object} event.input - Tool input
   * @param {Object} event.output - Tool output
   * @param {number} event.duration - Execution duration (ms)
   * @param {boolean} event.success - Success status
   * @param {string} [event.userId] - User ID
   * @param {string} [event.sessionId] - Session ID
   * @returns {Promise<PostAnalyzeResult>} Analysis result
   */
  async postAnalyze(event) {
    this.stats.postAnalyzes++;

    // Ensure initialized
    if (!this._initialized) {
      await this.initialize();
    }

    const { tool = '', input = {}, output = {}, duration = 0, success = true } = event;

    // Route through CollectivePack (non-blocking)
    try {
      const result = await this._withTimeout(
        this._collectivePack?.receiveHookEvent({
          hookType: 'PostToolUse',
          payload: { tool, input, output, duration, success },
          userId: event.userId,
          sessionId: event.sessionId,
        }),
        CONFIG.ASYNC_TIMEOUT_MS
      );

      return {
        analyzed: true,
        agentResults: result?.agentResults || [],
        patterns: this._extractPatterns(result),
        anomalies: this._extractAnomalies(result),
      };
    } catch (error) {
      this.stats.errors++;
      return {
        analyzed: false,
        error: error.message,
      };
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CONSENSUS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Request explicit consensus on a decision
   *
   * @param {Object} params - Consensus parameters
   * @param {string} params.topic - Decision topic
   * @param {string[]} params.options - Available options
   * @param {Object} params.context - Decision context
   * @returns {Promise<ConsensusResult>} Consensus result
   */
  async requestConsensus(params) {
    this.stats.consensusRequests++;

    // Ensure initialized
    if (!this._initialized) {
      await this.initialize();
    }

    const { topic, options = ['APPROVE', 'REJECT'], context = {} } = params;

    // Collect votes from Dogs via event bus
    const votes = [];

    // Use SwarmConsensus if available
    if (this._swarmConsensus) {
      try {
        const swarmResult = await this._swarmConsensus.requestConsensus(topic, {
          options,
          context,
          timeout: CONFIG.SYNC_TIMEOUT_MS,
        });

        if (swarmResult?.hasConsensus) {
          return {
            hasConsensus: true,
            decision: swarmResult.decision,
            confidence: swarmResult.confidence,
            votes: swarmResult.votes,
            source: 'swarm',
          };
        }

        votes.push(...(swarmResult?.votes || []));
      } catch {
        // Swarm failed, continue with neuronal
      }
    }

    // Use NeuronalConsensus for final decision
    if (this._neuronalConsensus && votes.length > 0) {
      const neuronResult = this._neuronalConsensus.processVotes(
        votes.map(v => ({
          dogId: v.dogId || v.agent,
          vote: v.vote || v.decision,
          confidence: v.confidence || PHI_INV,
          weight: v.weight || 1.0,
        }))
      );

      return {
        hasConsensus: neuronResult.consensus,
        decision: neuronResult.decision,
        confidence: neuronResult.confidence || PHI_INV,
        votes,
        potential: neuronResult.potential,
        source: 'neuronal',
      };
    }

    // Fallback: simple majority
    return {
      hasConsensus: false,
      decision: null,
      confidence: 0.3,
      votes,
      source: 'none',
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Assess risk level of a tool+input combination
   * @private
   */
  _assessRisk(tool, input) {
    const toolLower = tool.toLowerCase();
    let risk = 0.3; // Base risk

    // Destructive commands
    if (toolLower === 'bash') {
      const command = (input.command || '').toLowerCase();
      if (command.includes('rm ') || command.includes('del ')) risk = 0.9;
      else if (command.includes('git push') || command.includes('git reset')) risk = 0.8;
      else if (command.includes('npm publish') || command.includes('docker push')) risk = 0.85;
      else if (command.includes('chmod') || command.includes('chown')) risk = 0.7;
    }

    // File modifications
    if (toolLower === 'write' || toolLower === 'edit') {
      const path = (input.file_path || input.path || '').toLowerCase();
      if (path.includes('.env') || path.includes('secret')) risk = 0.9;
      else if (path.includes('package.json') || path.includes('tsconfig')) risk = 0.6;
      else risk = 0.5;
    }

    return risk;
  }

  /**
   * Run neuronal consensus on agent results
   * @private
   */
  async _runNeuronalConsensus(routerResult) {
    if (!this._neuronalConsensus) {
      return { shouldBlock: false };
    }

    const agentResults = routerResult?.agentResults || [];
    if (agentResults.length === 0) {
      return { shouldBlock: false };
    }

    // Convert agent results to votes
    const votes = agentResults
      .filter(r => !r.skipped)
      .map(r => ({
        dogId: r.agent,
        vote: r.response === 'block' ? 'REJECT'
          : r.response === 'warn' ? 'ABSTAIN'
          : 'APPROVE',
        confidence: r.confidence || PHI_INV,
        weight: 1.0,
      }));

    const result = this._neuronalConsensus.processVotes(votes);

    return {
      shouldBlock: result.consensus && result.decision === 'REJECT',
      reason: result.consensus
        ? `Neuronal consensus: ${result.decision}`
        : 'No neuronal consensus reached',
      potential: result.potential,
      fired: result.consensus,
    };
  }

  /**
   * Handle dog decision (callback from CollectivePack)
   * @private
   */
  _handleDogDecision(decision) {
    // Could emit to SSE, log, etc.
    // For now, just track for debugging
    if (process.env.CYNIC_DEBUG) {
      console.log('[AutoOrchestrator] Dog decision:', decision.dog, decision.response);
    }
  }

  /**
   * Extract patterns from analysis result
   * @private
   */
  _extractPatterns(result) {
    const patterns = [];

    for (const agentResult of result?.agentResults || []) {
      if (agentResult.patterns) {
        patterns.push(...agentResult.patterns);
      }
    }

    return patterns;
  }

  /**
   * Extract anomalies from analysis result
   * @private
   */
  _extractAnomalies(result) {
    const anomalies = [];

    for (const agentResult of result?.agentResults || []) {
      if (agentResult.anomalies) {
        anomalies.push(...agentResult.anomalies);
      }
      if (agentResult.response === 'warn' || agentResult.response === 'block') {
        anomalies.push({
          agent: agentResult.agent,
          type: agentResult.response,
          message: agentResult.message,
        });
      }
    }

    return anomalies;
  }

  /**
   * Get cache key for a tool+input
   * @private
   */
  _getCacheKey(tool, input) {
    const inputStr = typeof input === 'string'
      ? input
      : JSON.stringify(input).slice(0, 200);
    return `${tool}:${inputStr}`;
  }

  /**
   * Check cache for decision
   * @private
   */
  _checkCache(key) {
    const entry = this._decisionCache.get(key);
    if (!entry) return null;

    if (Date.now() - entry.timestamp > CONFIG.CACHE_TTL_MS) {
      this._decisionCache.delete(key);
      return null;
    }

    return entry.decision;
  }

  /**
   * Set cache entry
   * @private
   */
  _setCache(key, decision) {
    this._decisionCache.set(key, {
      decision,
      timestamp: Date.now(),
    });

    // Prune old entries
    if (this._decisionCache.size > 100) {
      const now = Date.now();
      for (const [k, v] of this._decisionCache) {
        if (now - v.timestamp > CONFIG.CACHE_TTL_MS) {
          this._decisionCache.delete(k);
        }
      }
    }
  }

  /**
   * Wrap promise with timeout
   * @private
   */
  async _withTimeout(promise, timeoutMs) {
    if (!promise) return null;

    let timeoutId;
    const timeoutPromise = new Promise((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`Operation timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    });

    try {
      const result = await Promise.race([promise, timeoutPromise]);
      clearTimeout(timeoutId);
      return result;
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
    }
  }

  /**
   * Get orchestrator stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      cacheSize: this._decisionCache.size,
      hasCollectivePack: !!this._collectivePack,
      hasNeuronalConsensus: !!this._neuronalConsensus,
      hasSwarmConsensus: !!this._swarmConsensus,
      hasPlanningGate: !!this._planningGate,
      planningGateStats: this._planningGate?.getStats() || null,
      hasChaosGenerator: !!this._chaosGenerator,
      chaosEnabled: this._chaosGenerator?.enabled || false,
      chaosStats: this._chaosGenerator?.getStats() || null,
    };
  }

  /**
   * Get CollectivePack (for advanced usage)
   * @returns {Object|null}
   */
  getCollectivePack() {
    return this._collectivePack;
  }

  /**
   * Get PlanningGate (for advanced usage)
   * @returns {Object|null}
   */
  getPlanningGate() {
    return this._planningGate;
  }

  /**
   * Get ChaosGenerator (for chaos engineering)
   * "Un système qui survit au hasard survit à tout"
   * @returns {Object|null}
   */
  getChaosGenerator() {
    return this._chaosGenerator;
  }

  /**
   * Record chaos result (for learning)
   * @param {string} chaosId - Chaos event ID
   * @param {Object} result - Outcome
   * @param {boolean} result.survived - Whether system survived
   * @param {string} [result.error] - Error message if failed
   */
  recordChaosResult(chaosId, result) {
    if (this._chaosGenerator && chaosId) {
      this._chaosGenerator.recordChaosResult(chaosId, result);
    }
  }

  /**
   * Clear cache and reset
   */
  clear() {
    this._decisionCache.clear();
    this.stats = {
      preChecks: 0,
      postAnalyzes: 0,
      consensusRequests: 0,
      blocksIssued: 0,
      cacheHits: 0,
      errors: 0,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

/** Singleton instance */
let _instance = null;

/**
 * Get AutoOrchestrator singleton instance
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {Promise<AutoOrchestrator>} Initialized orchestrator
 */
export async function getAutoOrchestrator(options = {}) {
  if (!_instance) {
    _instance = new AutoOrchestrator();
  }

  await _instance.initialize();
  return _instance;
}

/**
 * Get AutoOrchestrator synchronously (may not be initialized)
 * @returns {AutoOrchestrator}
 */
export function getAutoOrchestratorSync() {
  if (!_instance) {
    _instance = new AutoOrchestrator();
  }
  return _instance;
}

/**
 * @typedef {Object} PreCheckResult
 * @property {boolean} blocked - Whether to block the tool
 * @property {string|null} blockedBy - Agent that blocked (if any)
 * @property {string|null} message - Block message
 * @property {number} confidence - Decision confidence (0-1)
 * @property {string} reason - Decision reason
 * @property {Object[]} agentResults - Results from each agent
 * @property {boolean} isHighRisk - Whether tool was high-risk
 * @property {boolean} fromCache - Whether result was cached
 */

/**
 * @typedef {Object} PostAnalyzeResult
 * @property {boolean} analyzed - Whether analysis completed
 * @property {Object[]} agentResults - Results from each agent
 * @property {Object[]} patterns - Detected patterns
 * @property {Object[]} anomalies - Detected anomalies
 * @property {string} [error] - Error message if failed
 */

/**
 * @typedef {Object} ConsensusResult
 * @property {boolean} hasConsensus - Whether consensus was reached
 * @property {string|null} decision - Final decision
 * @property {number} confidence - Consensus confidence
 * @property {Object[]} votes - Individual votes
 * @property {number} [potential] - Neuronal potential (if neuronal)
 * @property {string} source - Consensus source (swarm/neuronal/none)
 */

export { AutoOrchestrator, CONFIG };
export default AutoOrchestrator;
