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

import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger } from '@cynic/core';
import { ThompsonSampler } from '../learning/thompson-sampler.js';
import { SEFIROT_TEMPLATE } from '../agents/collective/sefirot.js';
import { RelationshipGraph } from '../agents/collective/relationship-graph.js';
import { CONSULTATION_MATRIX, getConsultants, shouldConsult } from '@cynic/core/orchestration';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

const log = createLogger("KabbalisticRouter");

// Optional integrations (lazy loaded)
let LearningService = null;
let CostOptimizer = null;

// =============================================================================
// TEMPORAL AWARENESS (FFT → Router)
// =============================================================================

/**
 * Load harmonic state from disk (written by observe.js)
 * This bridges the FFT temporal analysis to routing decisions.
 *
 * @returns {Object|null} Harmonic state or null if unavailable/stale
 */
function loadHarmonicState() {
  try {
    const statePath = join(homedir(), '.cynic', 'harmonic', 'state.json');
    if (!existsSync(statePath)) return null;

    const state = JSON.parse(readFileSync(statePath, 'utf8'));

    // Check freshness (5 minutes TTL)
    if (Date.now() - state.timestamp > 5 * 60 * 1000) {
      return null; // Stale
    }

    return state;
  } catch {
    return null;
  }
}

/**
 * Temporal energy levels and their routing implications
 */
const TEMPORAL_ENERGY = {
  HIGH: { multiplier: 1.0, allowComplex: true },
  MEDIUM: { multiplier: 0.85, allowComplex: true },
  LOW: { multiplier: 0.7, allowComplex: false },
};

/**
 * Task types that require high cognitive load
 * Should be suppressed or simplified during LOW energy
 */
const HIGH_COGNITIVE_LOAD_TASKS = ['design', 'deployment', 'security'];

/**
 * Task types classified by risk level
 * B2: Girsanov → Router (risk-aware confidence)
 */
const TASK_RISK_LEVELS = {
  // High risk → use Q_risk (risk-averse measure)
  high: ['deployment', 'security', 'PreToolUse'],
  // Medium risk → use P (neutral measure)
  medium: ['design', 'analysis', 'PostToolUse'],
  // Low risk → use Q_opt (optimistic measure)
  low: ['exploration', 'mapping', 'SessionStart', 'SessionEnd'],
};

/**
 * Simplified paths for LOW energy states
 * Fewer agents = less complexity = less cognitive load
 */
/**
 * Core dogs available when consciousness < AWARE (soft gate)
 * Fix 3: Reduced agent set during low consciousness states
 */
const LOW_CONSCIOUSNESS_DOGS = ["guardian", "scout", "analyst"];

const LOW_ENERGY_PATHS = {
  design: ['guardian', 'analyst'], // Skip architect (complex decisions)
  deployment: ['guardian', 'deployer'], // Skip architect, janitor (less verification)
  security: ['guardian', 'oracle'], // Keep security tight but simpler
};

/**
 * Antifragility behavior modifiers
 * B3: Antifragility → Router (stress-aware behavior)
 */
const ANTIFRAGILITY_BEHAVIOR = {
  // Antifragile (index > 0): System benefits from stress
  antifragile: {
    consultationBonus: 0.2,   // Allow more consultations (learning opportunity)
    escalationPenalty: -0.1,  // Less eager to escalate (confident in handling)
    pathExtension: true,      // Can use longer paths
  },
  // Robust (index ≈ 0): System unaffected by stress
  robust: {
    consultationBonus: 0,
    escalationPenalty: 0,
    pathExtension: false,
  },
  // Fragile (index < 0): System harmed by stress
  fragile: {
    consultationBonus: -0.2,  // Reduce consultations (avoid cascading)
    escalationPenalty: 0.15,  // More eager to escalate (seek help)
    pathExtension: false,     // Use shorter paths
  },
};

/**
 * Agent order sensitivity for non-commutative evaluation
 * B4: Non-commutative → Router (order-optimized evaluation)
 * Agents that should be evaluated early due to high commutator effects
 */
const ORDER_SENSITIVE_AGENTS = {
  // These agents' decisions significantly affect later agents
  highImpact: ['guardian', 'architect'],
  // These agents benefit from seeing other decisions first
  contextDependent: ['oracle', 'sage'],
  // These agents are relatively order-independent
  flexible: ['analyst', 'scout', 'deployer', 'janitor', 'cartographer', 'scholar'],
};

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

    // D1: DPO weight cache (loaded from routing_weights table)
    this._dpoWeights = null;
    this._dpoFisher = null;
    this._dpoWeightsTTL = 5 * 60 * 1000; // 5 minutes
    this._dpoWeightsLastLoad = 0;

    // D-GRAVE-1: Orchestration performance cache (from orchestration_log)
    this._orchPerf = null;
    this._orchPerfLastLoad = 0;

    // P-GAP-5: Burnout awareness cache (from psychology_snapshots)
    this._burnoutStatus = null;
    this._burnoutLastLoad = 0;

    // D1: Thompson Sampler for exploration
    this.thompsonSampler = new ThompsonSampler();
    // Initialize arms for all 11 dogs
    const dogNames = collectivePack.getAllAgents ? collectivePack.getAllAgents().map(d => d.name || d.id) : Object.keys(SEFIROT_TEMPLATE.mappings);
    for (const name of dogNames) {
      this.thompsonSampler.initArm(name);
    }
    // R4: Load persisted Thompson state (survives daemon restarts)
    this._lastThompsonSave = 0;
    this._loadThompsonState();

    // Fix 3: Consciousness state tracking (soft gate)
    this._consciousnessState = "AWARE";
    this._awarenessLevel = 0.5;

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

    // =======================================================================
    // TEMPORAL AWARENESS (B1: FFT → Router)
    // =======================================================================
    const harmonicState = loadHarmonicState();
    const temporal = this._extractTemporalAwareness(harmonicState);

    // If LOW energy and high cognitive load task, suggest deferral
    if (temporal.energy === 'LOW' && HIGH_COGNITIVE_LOAD_TASKS.includes(taskType)) {
      temporal.suggestion = `*sniff* Low energy detected (${temporal.phase}). Consider deferring ${taskType} tasks. ${temporal.recommendation}`;
    }

    // =======================================================================
    // RISK-AWARE CONFIDENCE (B2: Girsanov → Router)
    // =======================================================================
    const girsanov = this._extractGirsanovAwareness(harmonicState, taskType);

    // =======================================================================
    // STRESS-AWARE BEHAVIOR (B3: Antifragility → Router)
    // =======================================================================
    const antifragility = this._extractAntifragilityAwareness(harmonicState);

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

    // 2. Determine entry point and path (temporal-aware)
    let path = this.getPath(taskType, temporal);

    // =======================================================================
    // ORDER-OPTIMIZED EVALUATION (B4: Non-commutative → Router)
    // =======================================================================
    const nonCommutative = this._extractNonCommutativeAwareness(harmonicState, path);

    // If high order sensitivity, use optimized path
    if (nonCommutative.reordered) {
      path = nonCommutative.optimizedPath;
    }

    // =======================================================================
    // D1: LEARNING-INFORMED PATH OPTIMIZATION (Q + DPO + Thompson)
    // =======================================================================
    // Load DPO weights + orchestration learning + burnout status (cached, non-blocking)
    try { await this.loadDPOWeights(); } catch (err) { /* best-effort */ }
    try { await this._loadOrchestrationLearning(); } catch (err) { /* best-effort */ }
    try { await this._loadBurnoutStatus(); } catch (err) { /* best-effort */ }

    // P-GAP-5: Burnout-aware path restriction
    // If burnout is high (>φ⁻¹), restrict to LOW_ENERGY_PATHS (fewer dogs = less cognitive load)
    if (this._burnoutStatus?.level === 'high' || this._burnoutStatus?.level === 'critical') {
      const lowPath = LOW_ENERGY_PATHS[taskType];
      if (lowPath) {
        path = lowPath;
        log.info('Burnout-aware routing: restricted to low-energy path', { taskType, burnoutLevel: this._burnoutStatus.level });
      }
    }

    // D1: Thompson exploration - with phi^-2 (23.6%), let Thompson pick entry
    let thompsonExplored = false;
    if (this.shouldExplore() && path.length > 1) {
      const thompsonPick = this.thompsonSelect(path);
      if (thompsonPick && thompsonPick !== path[0]) {
        path = [thompsonPick, ...path.filter(d => d !== thompsonPick)];
        thompsonExplored = true;
        log.info("Thompson exploration: promoted " + thompsonPick + " to entry", { taskType });
      }
    }

    // D1: Reorder by blended weights (Q + DPO), skip if Thompson explored
    if (!thompsonExplored) {
      const blended = this.getBlendedWeights();
      if (blended && Object.keys(blended).length > 0) {
        const securityFirst = ['PreToolUse', 'security', 'deployment'].includes(taskType);

        path = [...path].sort((a, b) => {
          if (securityFirst) {
            if (a === 'guardian') return -1;
            if (b === 'guardian') return 1;
          }
          const wA = blended[a] || 0.5;
          const wB = blended[b] || 0.5;
          return wB - wA;
        });
      }
    }

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
      // B2: Girsanov risk-aware thresholds
      girsanov,
      // B3: Antifragility stress-aware behavior
      antifragility,
      // B4: Non-commutative order awareness
      nonCommutative,
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
      // D1: Pass proper outcome object (score in 0-100 scale for _calculateReward)
      this.learningService.endEpisode({
        score: (reward + 1) * 50, // Convert [-1,1] → [0,100]
        success: synthesis.hasConsensus && !context.error,
        blocked: context.blocked,
      });
      this.applyLearnedWeights(); // D1: Close feedback loop
      // D1: Update Thompson Sampler with routing outcome
      this.updateThompson(path, synthesis.hasConsensus && !context.error); // D1: learned weights flow back to routing
      this._saveThompsonState(); // R4: Persist Thompson state (debounced)
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
      // B1: Temporal awareness (FFT → Router)
      temporal: {
        energy: temporal.energy,
        phase: temporal.phase,
        cycle: temporal.cycle,
        suggestion: temporal.suggestion || null,
        pathSimplified: temporal.energy === 'LOW' && LOW_ENERGY_PATHS[taskType] !== undefined,
      },
      // B2: Risk-aware confidence (Girsanov → Router)
      girsanov: {
        measure: girsanov.measure,
        riskLevel: girsanov.riskLevel,
        brierScore: girsanov.brierScore,
        adjustedEscalation: girsanov.adjustedEscalation,
      },
      // B3: Stress-aware behavior (Antifragility → Router)
      antifragility: {
        index: antifragility.index,
        trend: antifragility.trend,
        consultationLimit: antifragility.consultationLimit,
      },
      // B4: Order-optimized evaluation (Non-commutative → Router)
      nonCommutative: {
        orderSensitivity: nonCommutative.orderSensitivity,
        topPair: nonCommutative.topPair,
        reordered: nonCommutative.reordered,
      },
      // D1: Learning metadata
      learning: {
        thompsonExplored,
        hasDPOWeights: !!(this._dpoWeights && Object.keys(this._dpoWeights).length > 0),
        hasQLearning: !!this.learningService,
        thompsonStats: this.thompsonSampler.getStats(),
      },
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

    // D-GRAVE-1: Orchestration history alignment
    // If entry dog historically succeeds, slight reward boost (learning from past)
    if (this._orchPerf) {
      const entryDog = context.path?.[0];
      const perf = this._orchPerf[entryDog];
      if (perf && perf.sampleSize >= 5) {
        reward += (perf.successRate - 0.5) * PHI_INV_3; // ±23.6% max adjustment
      }
    }

    return Math.max(-1, Math.min(1, reward));
  }

  // ===========================================================================
  // PATH TRAVERSAL
  // ===========================================================================

  /**
   * Get the Lightning Flash path for a task type
   * Now temporal-aware: uses simplified paths during LOW energy states
   *
   * @param {string} taskType - Task type
   * @param {Object} [temporalContext] - Temporal awareness context
   * @returns {string[]} Path of agent names
   */
  /**
   * Subscribe to consciousness state changes
   * Fix 3: Soft gate - reduce available dogs when consciousness < AWARE
   *
   * @param {Object} eventBus - Event bus to subscribe to
   */
  subscribeConsciousness(eventBus) {
    if (eventBus?.subscribe) {
      eventBus.subscribe("consciousness:changed", (event) => {
        try {
          this._consciousnessState = event.payload?.newState || "AWARE";
          this._awarenessLevel = event.payload?.awarenessLevel || 0.5;
          log.info("Consciousness state updated in router", {
            state: this._consciousnessState,
            level: this._awarenessLevel,
          });
        } catch (e) {
          // Non-blocking
        }
      });
    }
  }

  getPath(taskType, temporalContext = null) {
    // Hardcoded fallback in case LIGHTNING_PATHS isn't loaded due to circular deps
    const hardcodedDefault = ['guardian', 'analyst', 'oracle'];

    // B1: If LOW energy and we have a simplified path, use it
    if (temporalContext?.energy === 'LOW') {
      const simplifiedPath = LOW_ENERGY_PATHS?.[taskType];
      if (Array.isArray(simplifiedPath)) {
        return simplifiedPath;
      }
    }

    // Fix 3: Consciousness soft gate - reduce dogs when consciousness < AWARE
    if (this._consciousnessState === "DORMANT" || this._consciousnessState === "AWAKENING") {
      const fullPath = LIGHTNING_PATHS?.[taskType] || LIGHTNING_PATHS?.default || hardcodedDefault;
      const safePath = Array.isArray(fullPath) ? fullPath : hardcodedDefault;
      const gatedPath = safePath.filter(dog => LOW_CONSCIOUSNESS_DOGS.includes(dog));
      if (gatedPath.length > 0) {
        log.info("Consciousness soft gate active", {
          state: this._consciousnessState,
          originalPath: safePath,
          gatedPath,
        });
        return gatedPath;
      }
    }

    const path = LIGHTNING_PATHS?.[taskType] || LIGHTNING_PATHS?.default || hardcodedDefault;
    return Array.isArray(path) ? path : hardcodedDefault;
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
      // B2: Use Girsanov-adjusted threshold for risk-aware escalation
      const escalationThreshold = context.girsanov?.adjustedEscalation || THRESHOLDS.ESCALATION;
      if (decision.confidence < escalationThreshold) {
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

      // Record action to learning service (action = dog name, metadata = details)
      if (this.learningService && this._currentEpisodeId) {
        this.learningService.recordAction(agentName, {
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

      // Record error to learning service (action = dog name, metadata = details)
      if (this.learningService && this._currentEpisodeId) {
        this.learningService.recordAction(agentName, {
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
    // B3: Use antifragility-adjusted consultation limit
    const maxConsultations = context.antifragility?.consultationLimit || CIRCUIT_BREAKER.MAX_CONSULTATIONS;
    if (context.totalConsultations >= maxConsultations) {
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
      // B3: Use antifragility-adjusted consultation limit
      if (context.totalConsultations >= maxConsultations) {
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
   * Extract temporal awareness from harmonic state
   * B1: FFT Temporal → Router
   *
   * @param {Object|null} harmonicState - Harmonic state from disk
   * @returns {Object} Temporal context { energy, phase, cycle, recommendation }
   */
  _extractTemporalAwareness(harmonicState) {
    // Default: assume MEDIUM energy if no data
    const defaults = {
      energy: 'MEDIUM',
      phase: 'unknown',
      cycle: null,
      recommendation: '',
    };

    if (!harmonicState?.temporal) {
      return defaults;
    }

    const { temporal } = harmonicState;

    return {
      energy: temporal.energy || defaults.energy,
      phase: temporal.phase || defaults.phase,
      cycle: temporal.cycle || defaults.cycle,
      recommendation: temporal.recommendation || defaults.recommendation,
    };
  }

  /**
   * Extract Girsanov risk-aware confidence
   * B2: Girsanov → Router (risk-aware confidence)
   *
   * @param {Object|null} harmonicState - Harmonic state from disk
   * @param {string} taskType - Task type to determine risk level
   * @returns {Object} { measure, brierScore, adjustedThreshold }
   */
  _extractGirsanovAwareness(harmonicState, taskType) {
    // Determine risk level of task
    let riskLevel = 'medium';
    if (TASK_RISK_LEVELS.high.includes(taskType)) riskLevel = 'high';
    else if (TASK_RISK_LEVELS.low.includes(taskType)) riskLevel = 'low';

    // Select measure based on risk level
    const measureMap = {
      high: 'Q_risk',   // Risk-averse for dangerous tasks
      medium: 'P',      // Neutral for standard tasks
      low: 'Q_opt',     // Optimistic for exploration
    };
    const selectedMeasure = measureMap[riskLevel];

    // Default thresholds
    const defaults = {
      measure: selectedMeasure,
      riskLevel,
      brierScore: 0.125, // Default neutral calibration
      adjustedEscalation: THRESHOLDS.ESCALATION,
      adjustedConsensus: THRESHOLDS.CONSENSUS,
    };

    if (!harmonicState?.girsanov?.measures) {
      return defaults;
    }

    const { measures, bestMeasure } = harmonicState.girsanov;
    const measureData = measures[selectedMeasure] || {};

    // Adjust thresholds based on measure calibration
    // Better calibration (lower Brier) = can trust thresholds more
    const calibrationFactor = 1 - (measureData.brierScore || 0.125);

    // For high-risk tasks, RAISE escalation threshold (more conservative)
    // For low-risk tasks, LOWER it (more permissive)
    let adjustedEscalation = THRESHOLDS.ESCALATION;
    if (riskLevel === 'high') {
      // More conservative: escalate more often
      adjustedEscalation = THRESHOLDS.ESCALATION * (1 + (1 - calibrationFactor) * 0.2);
    } else if (riskLevel === 'low') {
      // More permissive: escalate less
      adjustedEscalation = THRESHOLDS.ESCALATION * (1 - calibrationFactor * 0.15);
    }

    return {
      measure: selectedMeasure,
      riskLevel,
      brierScore: measureData.brierScore || 0.125,
      bestMeasure: bestMeasure || selectedMeasure,
      adjustedEscalation: Math.min(adjustedEscalation, PHI_INV), // Cap at φ⁻¹
      adjustedConsensus: THRESHOLDS.CONSENSUS, // Keep consensus threshold stable
    };
  }

  /**
   * Extract antifragility awareness
   * B3: Antifragility → Router (stress-aware behavior)
   *
   * @param {Object|null} harmonicState - Harmonic state from disk
   * @returns {Object} { index, trend, behavior, consultationLimit }
   */
  _extractAntifragilityAwareness(harmonicState) {
    const defaults = {
      index: 0,
      trend: 'robust',
      behavior: ANTIFRAGILITY_BEHAVIOR.robust,
      consultationLimit: CIRCUIT_BREAKER.MAX_CONSULTATIONS,
    };

    if (!harmonicState?.antifragility) {
      return defaults;
    }

    const { antifragility } = harmonicState;
    const index = antifragility.index || 0;

    // Determine trend from index (φ-bounded: -0.618 to +0.618)
    let trend = 'robust';
    if (index > 0.1) trend = 'antifragile';
    else if (index < -0.1) trend = 'fragile';

    const behavior = ANTIFRAGILITY_BEHAVIOR[trend];

    // Adjust consultation limit based on antifragility
    let consultationLimit = CIRCUIT_BREAKER.MAX_CONSULTATIONS;
    if (trend === 'antifragile') {
      // Can handle more consultations (opportunity for learning)
      consultationLimit = Math.min(consultationLimit + 2, 7);
    } else if (trend === 'fragile') {
      // Reduce consultations to avoid cascading stress
      consultationLimit = Math.max(consultationLimit - 2, 2);
    }

    return {
      index,
      trend,
      behavior,
      consultationLimit,
      convexity: antifragility.convexity || 0,
    };
  }

  /**
   * Extract non-commutative evaluation order
   * B4: Non-commutative → Router (order-optimized evaluation)
   *
   * @param {Object|null} harmonicState - Harmonic state from disk
   * @param {string[]} originalPath - Original Lightning Flash path
   * @returns {Object} { optimizedPath, orderSensitivity, topPair }
   */
  _extractNonCommutativeAwareness(harmonicState, originalPath) {
    // Guard against undefined/null path - use hardcoded default if LIGHTNING_PATHS not yet loaded
    const defaultPath = LIGHTNING_PATHS?.default || ['guardian', 'analyst', 'oracle'];
    const safePath = Array.isArray(originalPath) ? originalPath : defaultPath;

    const defaults = {
      optimizedPath: safePath,
      orderSensitivity: 0,
      topPair: null,
      reordered: false,
    };

    if (!harmonicState?.nonCommutative) {
      return defaults;
    }

    const { nonCommutative } = harmonicState;
    const orderSensitivity = nonCommutative.orderSensitivity || 0;

    // Only reorder if order sensitivity is significant (> 0.2)
    if (orderSensitivity < 0.2) {
      return {
        ...defaults,
        orderSensitivity,
        topPair: nonCommutative.topPair || null,
      };
    }

    // Optimize path: high-impact agents first, context-dependent later
    const optimizedPath = [...safePath].sort((a, b) => {
      const aImpact = ORDER_SENSITIVE_AGENTS.highImpact.includes(a) ? 0 :
                      ORDER_SENSITIVE_AGENTS.contextDependent.includes(a) ? 2 : 1;
      const bImpact = ORDER_SENSITIVE_AGENTS.highImpact.includes(b) ? 0 :
                      ORDER_SENSITIVE_AGENTS.contextDependent.includes(b) ? 2 : 1;
      return aImpact - bImpact;
    });

    // Check if path was actually reordered
    const reordered = safePath.some((agent, i) => agent !== optimizedPath[i]);

    return {
      optimizedPath,
      orderSensitivity,
      topPair: nonCommutative.topPair || null,
      reordered,
    };
  }

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

    // D1: Thompson Sampler stats
    stats.thompson = this.thompsonSampler.getStats();

    // D1: DPO cache status
    stats.dpoCache = {
      loaded: !!this._dpoWeights,
      dogs: this._dpoWeights ? Object.keys(this._dpoWeights).length : 0,
      age: this._dpoWeightsLastLoad ? Date.now() - this._dpoWeightsLastLoad : null,
    };

    // D-GRAVE-1: Orchestration performance cache
    stats.orchestrationPerf = {
      loaded: !!this._orchPerf,
      dogs: this._orchPerf ? Object.keys(this._orchPerf).length : 0,
      age: this._orchPerfLastLoad ? Date.now() - this._orchPerfLastLoad : null,
    };

    // P-GAP-5: Burnout awareness
    stats.burnout = this._burnoutStatus || { level: 'unknown' };

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

  // ===========================================================================
  // D1: DPO WEIGHT CONSUMPTION
  // ===========================================================================

  async loadDPOWeights() {
    if (this._dpoWeights && (Date.now() - this._dpoWeightsLastLoad) < this._dpoWeightsTTL) {
      return this._dpoWeights;
    }
    if (!this.persistence || !this.persistence.query) return null;
    try {
      const result = await this.persistence.query(
        'SELECT dog_name, weight, confidence, fisher_score FROM routing_weights WHERE service_id = $1',
        ['default']
      );
      if (result && result.rows && result.rows.length > 0) {
        const agg = {};
        const fisherAgg = {};
        for (const row of result.rows) {
          const dog = (row.dog_name || '').toLowerCase();
          if (!agg[dog]) agg[dog] = { sum: 0, count: 0 };
          agg[dog].sum += parseFloat(row.weight) || 0.5;
          agg[dog].count += 1;
          // Track Fisher scores per dog
          if (!fisherAgg[dog]) fisherAgg[dog] = { sum: 0, count: 0 };
          fisherAgg[dog].sum += parseFloat(row.fisher_score) || 0;
          fisherAgg[dog].count += 1;
        }
        this._dpoWeights = {};
        this._dpoFisher = {};
        for (const [dog, data] of Object.entries(agg)) {
          this._dpoWeights[dog] = data.count > 0 ? data.sum / data.count : 0.5;
        }
        for (const [dog, data] of Object.entries(fisherAgg)) {
          this._dpoFisher[dog] = data.count > 0 ? data.sum / data.count : 0;
        }
        this._dpoWeightsLastLoad = Date.now();
        log.info('DPO weights loaded', { dogs: Object.keys(this._dpoWeights).length, withFisher: Object.keys(this._dpoFisher).length });
        return this._dpoWeights;
      }
    } catch (err) {
      log.debug('DPO weight load failed (non-blocking)', { error: err.message });
    }
    return null;
  }

  // ===========================================================================
  // D-GRAVE-1: ORCHESTRATION LEARNING (historical routing success)
  // ===========================================================================

  /**
   * Load historical orchestration performance per dog (sefirah).
   * Reads orchestration_log to determine which dogs have best track records.
   * Uses same TTL as DPO weights (5 min cache).
   */
  async _loadOrchestrationLearning() {
    if (!this.persistence?.query) return null;
    if (this._orchPerf && (Date.now() - this._orchPerfLastLoad) < this._dpoWeightsTTL) {
      return this._orchPerf;
    }

    try {
      const { rows } = await this.persistence.query(`
        SELECT
          LOWER(sefirah) as dog,
          COUNT(*) as total,
          SUM(CASE WHEN outcome = 'ALLOW' AND skill_success IS NOT FALSE THEN 1 ELSE 0 END)::FLOAT
            / NULLIF(COUNT(*), 0) as success_rate,
          AVG(judgment_qscore) as avg_qscore
        FROM orchestration_log
        WHERE created_at > NOW() - INTERVAL '7 days'
          AND sefirah IS NOT NULL
        GROUP BY LOWER(sefirah)
        HAVING COUNT(*) >= 5
      `);

      this._orchPerf = {};
      for (const row of rows) {
        this._orchPerf[row.dog] = {
          successRate: parseFloat(row.success_rate) || 0.5,
          avgQScore: parseFloat(row.avg_qscore) || 50,
          sampleSize: parseInt(row.total),
        };
      }
      this._orchPerfLastLoad = Date.now();

      if (rows.length > 0) {
        log.info('Orchestration learning loaded', { dogs: rows.length });
      }
      return this._orchPerf;
    } catch (err) {
      log.debug('Orchestration learning load failed (non-blocking)', { error: err.message });
      return null;
    }
  }

  // ===========================================================================
  // P-GAP-5: BURNOUT-AWARE ROUTING
  // ===========================================================================

  /**
   * Load recent burnout status from psychology_snapshots (cached, 5min TTL)
   * Bridges the data grave: table exists but nobody reads for routing decisions.
   *
   * @returns {Promise<Object|null>} { level, score, energy, frustration }
   */
  async _loadBurnoutStatus() {
    if (!this.persistence?.query) return null;
    if (this._burnoutStatus && (Date.now() - this._burnoutLastLoad) < this._dpoWeightsTTL) {
      return this._burnoutStatus;
    }

    try {
      const { rows: [latest] } = await this.persistence.query(`
        SELECT burnout_score, energy, frustration, flow_score
        FROM psychology_snapshots
        WHERE created_at > NOW() - INTERVAL '30 minutes'
        ORDER BY created_at DESC
        LIMIT 1
      `);

      if (!latest) {
        this._burnoutStatus = null;
        this._burnoutLastLoad = Date.now();
        return null;
      }

      const burnout = parseFloat(latest.burnout_score) || 0;
      const energy = parseFloat(latest.energy) || 0.5;

      let level;
      if (burnout >= (1 - PHI_INV_3)) level = 'critical';      // 76.4%
      else if (burnout >= PHI_INV) level = 'high';               // 61.8%
      else if (burnout >= PHI_INV_2) level = 'moderate';         // 38.2%
      else level = 'low';

      this._burnoutStatus = {
        level,
        score: burnout,
        energy,
        frustration: parseFloat(latest.frustration) || 0,
        flow: parseFloat(latest.flow_score) || 0,
      };
      this._burnoutLastLoad = Date.now();

      if (level === 'high' || level === 'critical') {
        log.info('Burnout status loaded: ' + level, { burnout: Math.round(burnout * 100), energy: Math.round(energy * 100) });
      }

      return this._burnoutStatus;
    } catch (err) {
      log.debug('Burnout status load failed (non-blocking)', { error: err.message });
      return null;
    }
  }

  // ===========================================================================
  // D1: BLENDED WEIGHTS (Q-Learning + DPO + Thompson)
  // ===========================================================================

  getBlendedWeights() {
    try {
      const qWeights = this.getLearnedWeights();
      const dpoWeights = this._dpoWeights;
      if (!qWeights && !dpoWeights) return null;
      const allDogs = ['guardian','analyst','architect','scout','scholar','sage','oracle','janitor','deployer','cartographer','cynic'];
      const blended = {};
      for (const dog of allDogs) {
        const qW = qWeights ? (qWeights[dog] || 0.5) : 0.5;
        const dpoW = dpoWeights ? (dpoWeights[dog] || 0.5) : 0.5;
        const learnedAvg = (qW + dpoW) / 2;
        const existingWeight = this.getAgentWeight(dog);
        // Fisher modulates trust in learned weights:
        // fisher=0 → PHI_INV_2 (0.382) trust in learned (default)
        // fisher=1 → PHI_INV (0.618) trust in learned (max)
        const fisher = this._dpoFisher?.[dog] || 0;
        const learnedTrust = PHI_INV_2 + fisher * PHI_INV_3;
        blended[dog] = (1 - learnedTrust) * existingWeight + learnedTrust * learnedAvg;
        // D-GRAVE-1: Orchestration performance bias (±PHI_INV_3 from historical success)
        const orchPerf = this._orchPerf?.[dog];
        if (orchPerf && orchPerf.sampleSize >= 5) {
          blended[dog] += (orchPerf.successRate - 0.5) * PHI_INV_3;
        }
      }
      return blended;
    } catch (err) {
      log.debug('Blended weight calculation failed', { error: err.message });
      return null;
    }
  }

  shouldExplore() {
    return Math.random() < PHI_INV_3;
  }

  thompsonSelect(candidates) {
    if (!candidates || candidates.length === 0) return 'guardian';
    return this.thompsonSampler.selectArm(candidates) || candidates[0];
  }

  updateThompson(dogs, success) {
    try {
      for (const dog of dogs) {
        this.thompsonSampler.update(dog, success);
      }
    } catch (err) {
      log.debug('Thompson update failed', { error: err.message });
    }
  }

  setPersistence(persistence) {
    this.persistence = persistence;
  }

  // ===========================================================================
  // R4: THOMPSON SAMPLER PERSISTENCE
  // ===========================================================================

  /**
   * Load Thompson state from disk (survives daemon restarts).
   * Path: ~/.cynic/thompson/state.json
   */
  _loadThompsonState() {
    try {
      const statePath = join(homedir(), '.cynic', 'thompson', 'state.json');
      if (!existsSync(statePath)) return;

      const state = JSON.parse(readFileSync(statePath, 'utf8'));
      this.thompsonSampler.importState(state);
      log.info('Thompson state loaded from disk', {
        arms: state.arms ? Object.keys(state.arms).length : 0,
        totalPulls: state.totalPulls || 0,
      });
    } catch (err) {
      log.debug('Thompson state load failed (starting fresh)', { error: err.message });
    }
  }

  /**
   * Save Thompson state to disk (debounced: at most every 30s).
   */
  _saveThompsonState() {
    if (Date.now() - this._lastThompsonSave < 30000) return;
    this._lastThompsonSave = Date.now();

    try {
      const dir = join(homedir(), '.cynic', 'thompson');
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }
      const state = this.thompsonSampler.exportState();
      writeFileSync(join(dir, 'state.json'), JSON.stringify(state), 'utf8');
    } catch (err) {
      log.debug('Thompson state save failed (non-blocking)', { error: err.message });
    }
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
