/**
 * CYNIC Kabbalistic Router
 *
 * Implements the "Lightning Flash" (Seder Hishtalshelut) - decisions flow
 * through the Tree of Life, not through random independent hooks.
 *
 * "L'arbre vit" - The tree lives.
 *
 *                    Keter (CYNIC)
 *                   /      |      \
 *           Binah     Daat      Chochmah
 *         (Analyst) (Scholar)   (Sage)
 *                   \      |      /
 *           Gevurah   Tiferet   Chesed
 *         (Guardian)  (Oracle) (Architect)
 *                   \      |      /
 *             Hod      Yesod     Netzach
 *          (Deployer) (Janitor)  (Scout)
 *                   \      |      /
 *                    Malkhut
 *                 (Cartographer)
 *
 * @module @cynic/node/orchestration/kabbalistic-router
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { SEFIROT_TEMPLATE } from '../agents/collective/sefirot.js';
import { RelationshipGraph } from '../agents/collective/relationship-graph.js';
import { CONSULTATION_MATRIX, getConsultants, shouldConsult } from '@cynic/core/orchestration';

// Optional integrations (lazy loaded)
let LearningService = null;
let CostOptimizer = null;

// =============================================================================
// CONSTANTS
// =============================================================================

/**
 * φ-aligned thresholds
 */
const THRESHOLDS = {
  CONSENSUS: PHI_INV,          // 61.8% - agreement needed for consensus
  ESCALATION: PHI_INV_2,       // 38.2% - confidence below this triggers escalation
  CERTAINTY: PHI_INV * 0.9,    // 55.6% - high confidence threshold
};

/**
 * Circuit breaker limits
 */
const CIRCUIT_BREAKER = {
  MAX_DEPTH: 3,                // Max levels of consultation
  MAX_CONSULTATIONS: 5,        // Max total consultations per request
  COOLDOWN_MS: 5000,           // Cooldown between same-agent consultations
};

/**
 * Task type to entry Sefirah mapping
 *
 * The Lightning Flash enters at different Sefirot based on task domain:
 * - Protection tasks → Gevurah (Guardian)
 * - Design tasks → Chesed (Architect)
 * - Analysis tasks → Binah (Analyst)
 * - Knowledge tasks → Daat (Scholar)
 * - etc.
 */
const TASK_ENTRY_POINTS = {
  // Hook-based entry points (PreToolUse, PostToolUse)
  protection: 'guardian',      // Gevurah - security, blocking
  analysis: 'analyst',         // Binah - pattern detection, observation

  // Domain-based entry points
  design: 'architect',         // Chesed - architecture, structure
  wisdom: 'sage',              // Chochmah - guidance, teaching
  knowledge: 'scholar',        // Daat - extraction, documentation
  visualization: 'oracle',     // Tiferet - insights, dashboards
  exploration: 'scout',        // Netzach - discovery, search
  cleanup: 'janitor',          // Yesod - hygiene, quality
  deployment: 'deployer',      // Hod - CI/CD, infrastructure
  mapping: 'cartographer',     // Malkhut - reality, ecosystem

  // Default for unknown tasks
  default: 'guardian',         // Gevurah - safety first
};

/**
 * Lightning Flash paths - top-down traversal based on task type
 *
 * Each path defines the sequence of Sefirot for a task category.
 * Agents process in order; each can:
 * - process: handle and continue
 * - delegate_down: pass to next in path
 * - escalate_up: go back up the path
 * - consult_peer: use CONSULTATION_MATRIX
 */
const LIGHTNING_PATHS = {
  // PreToolUse: Security-first path
  PreToolUse: ['guardian', 'architect', 'analyst'],

  // PostToolUse: Analysis path
  PostToolUse: ['analyst', 'oracle', 'scholar'],

  // SessionStart: Full initialization path
  SessionStart: ['cynic', 'sage', 'scholar', 'cartographer'],

  // SessionEnd: Cleanup path
  SessionEnd: ['janitor', 'oracle', 'cynic'],

  // Design: Architect-led path
  design: ['architect', 'guardian', 'analyst', 'janitor'],

  // Security: Guardian-led path with escalation to Oracle
  security: ['guardian', 'architect', 'oracle'],

  // Exploration: Scout-led path
  exploration: ['scout', 'cartographer', 'analyst'],

  // Deployment: Deployer-led with safety checks
  deployment: ['deployer', 'guardian', 'architect', 'janitor'],

  // Default: Safety-first path
  default: ['guardian', 'analyst', 'oracle'],
};

// =============================================================================
// KABBALISTIC ROUTER
// =============================================================================

/**
 * KabbalisticRouter - Routes decisions through the Tree of Life
 *
 * @example
 * const router = new KabbalisticRouter({ collectivePack, persistence });
 * const result = await router.route({
 *   taskType: 'PreToolUse',
 *   payload: { tool: 'Bash', input: 'rm -rf /' },
 * });
 */
export class KabbalisticRouter {
  /**
   * @param {Object} options
   * @param {Object} options.collectivePack - CollectivePack instance with all dogs
   * @param {Object} [options.persistence] - Persistence manager
   * @param {Object} [options.relationshipGraph] - RelationshipGraph for learned weights
   * @param {Object} [options.learningService] - QLearningService for Q-learning feedback
   * @param {Object} [options.costOptimizer] - CostOptimizer for tier selection
   */
  constructor(options = {}) {
    const {
      collectivePack,
      persistence = null,
      relationshipGraph = null,
      learningService = null,
      costOptimizer = null,
    } = options;

    if (!collectivePack) {
      throw new Error('collectivePack is required');
    }

    this.pack = collectivePack;
    this.persistence = persistence;
    this.relationshipGraph = relationshipGraph || new RelationshipGraph();

    // Optional integrations
    this.learningService = learningService;
    this.costOptimizer = costOptimizer;
    this._currentEpisodeId = null;

    // Track consultation history for circuit breaker
    this.consultationHistory = new Map();

    // Stats
    this.stats = {
      routesProcessed: 0,
      consultationsTriggered: 0,
      escalationsTriggered: 0,
      consensusReached: 0,
      blocksIssued: 0,
      localResolutions: 0,
      costSaved: 0,
    };
  }

  // ===========================================================================
  // MAIN ROUTING
  // ===========================================================================

  /**
   * Route a task through the Tree of Life
   *
   * @param {Object} task - Task to route
   * @param {string} task.taskType - Type of task (PreToolUse, PostToolUse, design, etc.)
   * @param {Object} task.payload - Task payload
   * @param {string} [task.userId] - User ID
   * @param {string} [task.sessionId] - Session ID
   * @returns {Promise<Object>} Routing result with decisions and synthesis
   */
  async route(task) {
    const { taskType, payload = {}, userId, sessionId } = task;
    const startTime = Date.now();

    this.stats.routesProcessed++;

    // 0. Cost optimization check (if enabled)
    let costOptimization = null;
    if (this.costOptimizer) {
      costOptimization = this.costOptimizer.optimize({
        content: payload.input || payload.content || '',
        type: taskType,
        context: { complexity: payload.complexity, risk: payload.risk },
      });

      // LOCAL tier = skip full routing
      if (!costOptimization.shouldRoute) {
        this.stats.localResolutions++;
        this.stats.costSaved += costOptimization.cost;

        return {
          success: true,
          taskType,
          path: [],
          entrySefirah: null,
          decisions: [],
          consultations: [],
          escalations: [],
          blocked: false,
          blockedBy: null,
          blockMessage: null,
          synthesis: {
            hasConsensus: true,
            consensusResponse: 'allow',
            confidence: PHI_INV,
            reason: 'Local resolution (no LLM needed)',
          },
          tier: costOptimization.tier,
          costOptimization,
          durationMs: Date.now() - startTime,
          error: null,
        };
      }
    }

    // 1. Start learning episode (if enabled)
    if (this.learningService) {
      this._currentEpisodeId = this.learningService.startEpisode({
        taskType,
        tool: payload.tool,
        inputLength: (payload.input || '').length,
      });
    }

    // 2. Determine entry point and path
    const path = this.getPath(taskType);
    const entrySefirah = path[0];

    // 3. Create context for path traversal
    const context = {
      taskType,
      payload,
      userId,
      sessionId,
      path,
      currentIndex: 0,
      decisions: [],
      consultations: [],
      escalations: [],
      blocked: false,
      blockedBy: null,
      blockMessage: null,
      depth: 0,
      totalConsultations: 0,
      costOptimization,
    };

    // 4. Execute Lightning Flash traversal
    try {
      await this.traversePath(context);
    } catch (error) {
      context.error = error.message;
    }

    // 5. Synthesize at Keter (CYNIC)
    const synthesis = await this.synthesize(context);

    const durationMs = Date.now() - startTime;

    // 6. Record stats
    if (context.consultations.length > 0) this.stats.consultationsTriggered++;
    if (context.escalations.length > 0) this.stats.escalationsTriggered++;
    if (synthesis.hasConsensus) this.stats.consensusReached++;
    if (context.blocked) this.stats.blocksIssued++;

    // 7. End learning episode (if enabled)
    if (this.learningService && this._currentEpisodeId) {
      const reward = this._calculateReward(synthesis, context, durationMs);
      this.learningService.endEpisode(this._currentEpisodeId, reward);
      this.applyLearnedWeights(); // D1: Close feedback loop — learned weights flow back to routing
      this._currentEpisodeId = null;
    }

    // 8. Record cost outcome (if enabled)
    if (this.costOptimizer && costOptimization) {
      this.costOptimizer.recordOutcome(
        costOptimization.tier,
        !context.error,
        durationMs
      );
    }

    return {
      success: !context.error,
      taskType,
      path,
      entrySefirah,
      // Decisions from each Sefirah
      decisions: context.decisions,
      // Consultations that occurred
      consultations: context.consultations,
      // Escalations that occurred
      escalations: context.escalations,
      // Block status
      blocked: context.blocked,
      blockedBy: context.blockedBy,
      blockMessage: context.blockMessage,
      // Keter synthesis
      synthesis,
      // Cost tier (if optimized)
      tier: costOptimization?.tier,
      costOptimization,
      // Timing
      durationMs,
      // Error if any
      error: context.error,
    };
  }

  /**
   * Calculate reward for learning service
   * @private
   */
  _calculateReward(synthesis, context, durationMs) {
    let reward = 0;

    // Base reward for successful completion
    if (synthesis.hasConsensus) reward += 0.5;

    // Bonus for high confidence
    if (synthesis.confidence >= THRESHOLDS.CERTAINTY) reward += 0.2;

    // Bonus for efficiency (fewer consultations)
    if (context.consultations.length === 0) reward += 0.2;
    else if (context.consultations.length <= 2) reward += 0.1;

    // Penalty for excessive latency
    if (durationMs > 5000) reward -= 0.1;
    if (durationMs > 10000) reward -= 0.2;

    // Penalty for errors
    if (context.error) reward -= 0.5;

    // Penalty for unnecessary escalations
    if (context.escalations.length > 2) reward -= 0.1;

    return Math.max(-1, Math.min(1, reward));
  }

  // ===========================================================================
  // PATH TRAVERSAL
  // ===========================================================================

  /**
   * Get the Lightning Flash path for a task type
   *
   * @param {string} taskType - Task type
   * @returns {string[]} Path of agent names
   */
  getPath(taskType) {
    return LIGHTNING_PATHS[taskType] || LIGHTNING_PATHS.default;
  }

  /**
   * Traverse the Lightning Flash path
   *
   * @param {Object} context - Routing context
   */
  async traversePath(context) {
    const { path, payload, taskType, userId, sessionId } = context;

    for (let i = 0; i < path.length && !context.blocked; i++) {
      context.currentIndex = i;
      const agentName = path[i];

      // Get agent from pack
      const agent = this.getAgent(agentName);
      if (!agent) {
        context.decisions.push({
          agent: agentName,
          skipped: true,
          reason: 'Agent not found in pack',
        });
        continue;
      }

      // Create event for agent processing
      const event = {
        type: taskType,
        tool: payload.tool,
        input: payload.input,
        output: payload.output,
        duration: payload.duration,
        success: payload.success !== false,
        userId,
        sessionId,
        timestamp: Date.now(),
      };

      // Check if agent should trigger
      if (!agent.shouldTrigger?.(event, { hookType: taskType })) {
        context.decisions.push({
          agent: agentName,
          skipped: true,
          reason: 'Agent did not trigger',
        });
        continue;
      }

      // Process with agent
      const decision = await this.processWithAgent(agent, agentName, event, context);
      context.decisions.push(decision);

      // Handle block
      if (decision.response === 'block') {
        context.blocked = true;
        context.blockedBy = agentName;
        context.blockMessage = decision.message;
        break;
      }

      // Handle low confidence → consultation
      if (decision.confidence < THRESHOLDS.ESCALATION) {
        await this.handleLowConfidence(agentName, taskType, decision, context);
      }

      // Handle escalation request
      if (decision.action === 'escalate') {
        await this.handleEscalation(agentName, decision, context);
      }
    }
  }

  /**
   * Process event with a specific agent
   *
   * @param {Object} agent - Agent instance
   * @param {string} agentName - Agent name
   * @param {Object} event - Event to process
   * @param {Object} context - Routing context
   * @returns {Promise<Object>} Agent decision
   */
  async processWithAgent(agent, agentName, event, context) {
    const startTime = Date.now();

    try {
      // Run agent's process method
      const result = await agent.process?.(event, {
        hookType: context.taskType,
        userId: context.userId,
        sessionId: context.sessionId,
      });

      // Extract decision from result
      const response = result?.response || 'allow';
      const confidence = result?.confidence ?? PHI_INV;
      const action = result?.action || 'continue';
      const message = result?.message || '';

      const durationMs = Date.now() - startTime;

      // Record action to learning service
      if (this.learningService && this._currentEpisodeId) {
        this.learningService.recordAction(this._currentEpisodeId, {
          agent: agentName,
          response,
          confidence,
          latency: durationMs,
        });
      }

      return {
        agent: agentName,
        sefirah: SEFIROT_TEMPLATE.mappings[agentName]?.sefira || 'Unknown',
        response,
        confidence,
        action,
        message,
        durationMs,
      };
    } catch (error) {
      const durationMs = Date.now() - startTime;

      // Record error to learning service
      if (this.learningService && this._currentEpisodeId) {
        this.learningService.recordAction(this._currentEpisodeId, {
          agent: agentName,
          response: 'error',
          confidence: 0,
          latency: durationMs,
          error: error.message,
        });
      }

      return {
        agent: agentName,
        sefirah: SEFIROT_TEMPLATE.mappings[agentName]?.sefira || 'Unknown',
        response: 'allow',
        confidence: THRESHOLDS.ESCALATION,
        action: 'continue',
        message: `Error: ${error.message}`,
        error: error.message,
        durationMs,
      };
    }
  }

  // ===========================================================================
  // CONSULTATION
  // ===========================================================================

  /**
   * Handle low confidence by consulting peers
   *
   * @param {string} agentName - Agent with low confidence
   * @param {string} taskType - Task type
   * @param {Object} decision - Agent's decision
   * @param {Object} context - Routing context
   */
  async handleLowConfidence(agentName, taskType, decision, context) {
    // Check circuit breaker
    if (context.depth >= CIRCUIT_BREAKER.MAX_DEPTH) {
      return;
    }
    if (context.totalConsultations >= CIRCUIT_BREAKER.MAX_CONSULTATIONS) {
      return;
    }

    // Get consultants from matrix
    const consultResult = shouldConsult(agentName, taskType, {
      confidence: decision.confidence,
      isHighRisk: decision.response === 'warn' || context.payload?.isHighRisk,
    });

    if (!consultResult.needed) {
      return;
    }

    context.depth++;

    // Consult each recommended agent
    for (const consultantName of consultResult.consultants) {
      if (context.totalConsultations >= CIRCUIT_BREAKER.MAX_CONSULTATIONS) {
        break;
      }

      // Check cooldown
      if (this.isOnCooldown(consultantName)) {
        continue;
      }

      const consultant = this.getAgent(consultantName);
      if (!consultant) continue;

      // Create consultation event
      const consultEvent = {
        type: 'consultation',
        originalTask: context.taskType,
        requestedBy: agentName,
        reason: consultResult.reason,
        originalDecision: decision,
        tool: context.payload.tool,
        input: context.payload.input,
        timestamp: Date.now(),
      };

      // Process consultation
      const consultDecision = await this.processWithAgent(
        consultant,
        consultantName,
        consultEvent,
        context
      );

      context.consultations.push({
        requestedBy: agentName,
        consultant: consultantName,
        reason: consultResult.reason,
        decision: consultDecision,
      });

      context.totalConsultations++;
      this.recordCooldown(consultantName);

      // If consultant blocks, propagate
      if (consultDecision.response === 'block') {
        context.blocked = true;
        context.blockedBy = consultantName;
        context.blockMessage = consultDecision.message;
        break;
      }
    }

    context.depth--;
  }

  /**
   * Handle escalation request
   *
   * @param {string} agentName - Agent requesting escalation
   * @param {Object} decision - Agent's decision
   * @param {Object} context - Routing context
   */
  async handleEscalation(agentName, decision, context) {
    // Check circuit breaker
    if (context.depth >= CIRCUIT_BREAKER.MAX_DEPTH) {
      return;
    }

    // Determine escalation target based on tree geometry
    const currentMapping = SEFIROT_TEMPLATE.mappings[agentName];
    if (!currentMapping) return;

    // Escalate to center column (Oracle/Tiferet) for balance
    let escalationTarget = 'oracle';

    // If already at Oracle, escalate to CYNIC (Keter)
    if (agentName === 'oracle') {
      escalationTarget = 'cynic';
    }

    // If at level 1, escalate to CYNIC directly
    if (currentMapping.level <= 1) {
      escalationTarget = 'cynic';
    }

    const escalationAgent = this.getAgent(escalationTarget);
    if (!escalationAgent) return;

    context.depth++;

    const escalationEvent = {
      type: 'escalation',
      originalTask: context.taskType,
      escalatedBy: agentName,
      reason: decision.message || 'Low confidence escalation',
      originalDecision: decision,
      tool: context.payload.tool,
      input: context.payload.input,
      allDecisions: context.decisions,
      timestamp: Date.now(),
    };

    const escalationDecision = await this.processWithAgent(
      escalationAgent,
      escalationTarget,
      escalationEvent,
      context
    );

    context.escalations.push({
      escalatedBy: agentName,
      escalatedTo: escalationTarget,
      reason: decision.message,
      decision: escalationDecision,
    });

    context.depth--;

    // If escalation overrides, update block status
    if (escalationDecision.response === 'block') {
      context.blocked = true;
      context.blockedBy = escalationTarget;
      context.blockMessage = escalationDecision.message;
    } else if (escalationDecision.action === 'override' && context.blocked) {
      // Escalation can override a block (Oracle/CYNIC override)
      context.blocked = false;
      context.blockedBy = null;
      context.blockMessage = null;
    }
  }

  // ===========================================================================
  // SYNTHESIS (KETER)
  // ===========================================================================

  /**
   * Synthesize all decisions at Keter level
   *
   * @param {Object} context - Routing context
   * @returns {Object} Synthesis result
   */
  async synthesize(context) {
    const { decisions, consultations, escalations } = context;

    // Filter valid decisions (not skipped)
    const validDecisions = decisions.filter(d => !d.skipped && !d.error);

    if (validDecisions.length === 0) {
      return {
        hasConsensus: false,
        consensusResponse: 'allow',
        confidence: THRESHOLDS.ESCALATION,
        reason: 'No valid decisions to synthesize',
      };
    }

    // Calculate weighted consensus
    let totalWeight = 0;
    let weightedScore = 0;
    let blockVotes = 0;
    let warnVotes = 0;
    let allowVotes = 0;

    for (const decision of validDecisions) {
      // Get agent's weight from relationship graph or use default
      const weight = this.getAgentWeight(decision.agent);
      totalWeight += weight;

      // Score: block=0, warn=0.3, allow=1
      const score = decision.response === 'block' ? 0
        : decision.response === 'warn' ? 0.3
        : 1;

      weightedScore += score * weight;

      // Count votes
      if (decision.response === 'block') blockVotes++;
      else if (decision.response === 'warn') warnVotes++;
      else allowVotes++;
    }

    const consensusScore = totalWeight > 0 ? weightedScore / totalWeight : 0.5;
    const totalVotes = validDecisions.length;
    const agreementRatio = Math.max(blockVotes, warnVotes, allowVotes) / totalVotes;

    // Determine consensus response
    let consensusResponse = 'allow';
    if (blockVotes > 0 && blockVotes >= totalVotes * THRESHOLDS.ESCALATION) {
      consensusResponse = 'block';
    } else if (warnVotes > 0 && (warnVotes + blockVotes) >= totalVotes * THRESHOLDS.ESCALATION) {
      consensusResponse = 'warn';
    }

    // Calculate final confidence (capped at φ⁻¹)
    const confidence = Math.min(agreementRatio * consensusScore, PHI_INV);

    return {
      hasConsensus: agreementRatio >= THRESHOLDS.CONSENSUS,
      consensusResponse,
      confidence,
      consensusScore,
      agreementRatio,
      votes: {
        block: blockVotes,
        warn: warnVotes,
        allow: allowVotes,
        total: totalVotes,
      },
      consultationCount: consultations.length,
      escalationCount: escalations.length,
      reason: this.generateSynthesisReason(consensusResponse, validDecisions),
    };
  }

  /**
   * Generate human-readable synthesis reason
   *
   * @param {string} response - Consensus response
   * @param {Object[]} decisions - Valid decisions
   * @returns {string} Synthesis reason
   */
  generateSynthesisReason(response, decisions) {
    const agents = decisions.map(d => d.agent).join(', ');

    if (response === 'block') {
      const blockers = decisions.filter(d => d.response === 'block');
      const reasons = blockers.map(d => d.message).filter(Boolean).join('; ');
      return `Blocked by ${blockers.map(d => d.agent).join(', ')}${reasons ? `: ${reasons}` : ''}`;
    }

    if (response === 'warn') {
      const warners = decisions.filter(d => d.response === 'warn' || d.response === 'block');
      const reasons = warners.map(d => d.message).filter(Boolean).join('; ');
      return `Warning from ${warners.map(d => d.agent).join(', ')}${reasons ? `: ${reasons}` : ''}`;
    }

    return `Approved by consensus (${agents})`;
  }

  // ===========================================================================
  // HELPERS
  // ===========================================================================

  /**
   * Get agent from pack by name
   *
   * @param {string} name - Agent name (lowercase)
   * @returns {Object|null} Agent instance
   */
  getAgent(name) {
    // Try direct property access
    const agent = this.pack[name] || this.pack.agents?.[name];
    if (agent) return agent;

    // Try with 'Dog' suffix
    const dogName = `${name}Dog`;
    return this.pack[dogName] || this.pack.agents?.[dogName] || null;
  }

  /**
   * Get agent weight from relationship graph or default
   *
   * @param {string} agentName - Agent name
   * @returns {number} Weight (0-1)
   */
  getAgentWeight(agentName) {
    // Try relationship graph first
    if (this.relationshipGraph?.getWeight) {
      const learned = this.relationshipGraph.getWeight('cynic', agentName);
      if (learned > 0) return learned;
    }

    // Fall back to Sefirot template geometry
    return SEFIROT_TEMPLATE.calculateWeight('cynic', agentName) || PHI_INV_2;
  }

  /**
   * Check if agent is on cooldown
   *
   * @param {string} agentName - Agent name
   * @returns {boolean} True if on cooldown
   */
  isOnCooldown(agentName) {
    const lastConsult = this.consultationHistory.get(agentName);
    if (!lastConsult) return false;
    return Date.now() - lastConsult < CIRCUIT_BREAKER.COOLDOWN_MS;
  }

  /**
   * Record consultation for cooldown tracking
   *
   * @param {string} agentName - Agent name
   */
  recordCooldown(agentName) {
    this.consultationHistory.set(agentName, Date.now());
  }

  /**
   * Get router stats
   *
   * @returns {Object} Stats
   */
  getStats() {
    const stats = { ...this.stats };

    // Add learning stats if available
    if (this.learningService) {
      stats.learning = this.learningService.getStats();
    }

    // Add cost stats if available
    if (this.costOptimizer) {
      stats.cost = this.costOptimizer.getStats();
    }

    return stats;
  }

  /**
   * Reset consultation cooldowns (for testing)
   */
  resetCooldowns() {
    this.consultationHistory.clear();
  }

  /**
   * Set learning service at runtime
   *
   * @param {Object} learningService - LearningService instance
   */
  setLearningService(learningService) {
    this.learningService = learningService;
  }

  /**
   * Set cost optimizer at runtime
   *
   * @param {Object} costOptimizer - CostOptimizer instance
   */
  setCostOptimizer(costOptimizer) {
    this.costOptimizer = costOptimizer;
  }

  /**
   * Get recommended agent weights from learning service
   *
   * @returns {Object|null} Recommended weights or null
   */
  getLearnedWeights() {
    if (!this.learningService) return null;
    return this.learningService.getRecommendedWeights();
  }

  /**
   * Apply learned weights to relationship graph
   */
  applyLearnedWeights() {
    const weights = this.getLearnedWeights();
    if (!weights || !this.relationshipGraph) return false;

    for (const [agent, weight] of Object.entries(weights)) {
      this.relationshipGraph.setWeight?.('cynic', agent, weight);
    }

    return true;
  }
}

// =============================================================================
// FACTORY
// =============================================================================

/**
 * Create a KabbalisticRouter instance
 *
 * @param {Object} options - Router options
 * @returns {KabbalisticRouter} Router instance
 */
export function createKabbalisticRouter(options) {
  return new KabbalisticRouter(options);
}

export default KabbalisticRouter;
