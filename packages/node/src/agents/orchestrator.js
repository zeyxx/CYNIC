/**
 * Dog Orchestrator
 *
 * Spawns 11 dogs as parallel subagents with hybrid context injection.
 * Implements the 6-layer memory architecture for fresh execution + shared memory.
 *
 * @module @cynic/node/agents/orchestrator
 */

'use strict';

import { PHI_INV } from '@cynic/core';
import { AgentId } from './events.js';
import { SwarmConsensus, createSwarmConsensus } from './swarm-consensus.js';
import { getEventBus } from '../services/event-bus.js';

/**
 * Dog execution modes
 */
export const DogMode = {
  PARALLEL: 'parallel',       // All dogs run simultaneously
  SEQUENTIAL: 'sequential',   // Dogs run one after another
  CRITICAL_ONLY: 'critical',  // Only Guardian + CYNIC run
  FAST: 'fast',               // Only haiku models run
};

/**
 * Dog model selection
 */
export const DogModel = {
  HAIKU: 'haiku',     // Fast, cheap - most dogs
  SONNET: 'sonnet',   // Balanced - Guardian, complex tasks
  OPUS: 'opus',       // Powerful - CYNIC meta-consciousness
};

/**
 * Dog configuration with model and capabilities
 */
export const DOG_CONFIG = {
  SAGE:         { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.SAGE },
  ANALYST:      { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.ANALYST },
  GUARDIAN:     { model: DogModel.SONNET, blocking: true,  timeout: 10000, agentId: AgentId.GUARDIAN },
  SCHOLAR:      { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.SCHOLAR },
  ARCHITECT:    { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.ARCHITECT },
  JANITOR:      { model: DogModel.HAIKU,  blocking: false, timeout: 3000,  agentId: AgentId.JANITOR },
  SCOUT:        { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.SCOUT },
  CARTOGRAPHER: { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.CARTOGRAPHER },
  ORACLE:       { model: DogModel.HAIKU,  blocking: false, timeout: 5000,  agentId: AgentId.ORACLE },
  DEPLOYER:     { model: DogModel.SONNET, blocking: true,  timeout: 10000, agentId: AgentId.DEPLOYER },
  CYNIC:        { model: DogModel.OPUS,   blocking: false, timeout: 15000, agentId: AgentId.CYNIC },
};

/**
 * φ-aligned axioms injected into every dog
 */
const PHI_AXIOMS = {
  PHI_INV: 0.618,
  MAX_CONFIDENCE: 0.618,
  MIN_DOUBT: 0.382,
  CONSENSUS_THRESHOLD: 0.618,
  MOTTO: 'φ distrusts φ',
};

/**
 * Dog Orchestrator - Spawns and coordinates parallel dog subagents
 */
export class DogOrchestrator {
  /**
   * @param {Object} options
   * @param {Object} options.collectivePack - Existing CollectivePack instance
   * @param {Object} options.sharedMemory - SharedMemory instance
   * @param {Object} [options.userLab] - UserLab instance (optional)
   * @param {string} [options.mode] - Execution mode
   * @param {number} [options.consensusThreshold] - Custom threshold (default: φ⁻¹)
   * @param {boolean} [options.useSwarmConsensus] - Use P2.4 swarm consensus (default: true)
   * @param {SwarmConsensus} [options.swarmConsensus] - Custom swarm consensus instance
   */
  constructor(options = {}) {
    this.collectivePack = options.collectivePack;
    this.sharedMemory = options.sharedMemory;
    this.userLab = options.userLab;
    this.mode = options.mode || DogMode.PARALLEL;
    this.consensusThreshold = options.consensusThreshold || PHI_INV;

    // P2.4: Swarm consensus (enabled by default)
    this.useSwarmConsensus = options.useSwarmConsensus !== false;
    if (this.useSwarmConsensus) {
      this.swarmConsensus = options.swarmConsensus || createSwarmConsensus({ orchestrator: this });
    }

    // Custom spawner (for future Claude API integration)
    this.spawner = options.spawner || null;

    // Event bus for visibility
    this.eventBus = options.eventBus || getEventBus();

    // Stats
    this.stats = {
      judgments: 0,
      consensusReached: 0,
      blockedByGuardian: 0,
      blockedByDeployer: 0,
      averageLatency: 0,
      dogVotes: {},
    };

    // Initialize dog vote stats
    for (const dog of Object.keys(DOG_CONFIG)) {
      this.stats.dogVotes[dog] = { total: 0, allow: 0, block: 0, errors: 0 };
    }
  }

  /**
   * Judge an item using parallel dogs with hybrid context
   * @param {Object} item - Item to judge
   * @param {Object} [context] - Additional context
   * @returns {Promise<Object>} Judgment result
   */
  async judge(item, context = {}) {
    const startTime = Date.now();
    this.stats.judgments++;

    // Build injected context from all memory layers
    const injectedContext = await this._buildInjectedContext(item, context);

    // Determine which dogs to run based on mode
    const dogsToRun = this._selectDogs();

    // Spawn dogs and collect votes
    let votes;
    if (this.mode === DogMode.SEQUENTIAL) {
      votes = await this._runSequential(item, dogsToRun, injectedContext);
    } else {
      votes = await this._runParallel(item, dogsToRun, injectedContext);
    }

    // Calculate consensus (P2.4: use swarm consensus if available)
    const consensus = this.useSwarmConsensus && this.swarmConsensus
      ? this.swarmConsensus.calculateConsensus(votes, { ...context, itemType: item?.type })
      : this._calculateConsensus(votes);

    // Emit vote complete and consensus reached events
    this.eventBus?.publish('dog:vote:complete', {
      voteCount: votes.filter(v => v.success).length,
      verdict: consensus.verdict,
    }, { source: 'DogOrchestrator' });

    this.eventBus?.publish('consensus:reached', {
      score: consensus.score,
      verdict: consensus.verdict,
      agreementRatio: consensus.agreementRatio,
      blocked: consensus.blocked,
      strategy: consensus.strategy,
    }, { source: 'DogOrchestrator' });

    // Check for blocking votes
    if (consensus.blocked) {
      if (consensus.blockedBy === 'GUARDIAN') this.stats.blockedByGuardian++;
      if (consensus.blockedBy === 'DEPLOYER') this.stats.blockedByDeployer++;

      // Emit danger blocked event for visibility
      this.eventBus?.publish('danger:blocked', {
        reason: consensus.blockReason,
        blockedBy: consensus.blockedBy,
        severity: 'high',
      }, { source: 'DogOrchestrator' });

      return {
        blocked: true,
        blockedBy: consensus.blockedBy,
        reason: consensus.blockReason,
        votes,
        latency: Date.now() - startTime,
      };
    }

    // Build final judgment
    const judgment = this._buildJudgment(item, votes, consensus);

    // Record latency
    const latency = Date.now() - startTime;
    this.stats.averageLatency =
      (this.stats.averageLatency * (this.stats.judgments - 1) + latency)
      / this.stats.judgments;

    // Index judgment for future similarity searches
    if (this.sharedMemory) {
      this.sharedMemory.indexJudgment(judgment, item);
    }

    return {
      ...judgment,
      latency,
    };
  }

  /**
   * Build injected context from all 6 memory layers
   * @private
   */
  async _buildInjectedContext(item, requestContext) {
    await this.sharedMemory?.initialize();

    return {
      // ════════════════════════════════════════════════════════════════════
      // Layer 1: DOG IDENTITY (immutable, always injected)
      // ════════════════════════════════════════════════════════════════════
      axioms: PHI_AXIOMS,

      // ════════════════════════════════════════════════════════════════════
      // Layer 2: COLLECTIVE MEMORY (from SharedMemory)
      // ════════════════════════════════════════════════════════════════════
      patterns: this.sharedMemory?.getRelevantPatterns(item, 5) || [],
      dimensionWeights: this.sharedMemory?.getLearnedWeights() || {},
      similarJudgments: this.sharedMemory?.getSimilarJudgments(item, 3) || [],

      // ════════════════════════════════════════════════════════════════════
      // Layer 3: PROCEDURAL MEMORY (how-to for this item type)
      // ════════════════════════════════════════════════════════════════════
      procedure: this.sharedMemory?.getForItemType(item?.type) || null,
      scoringRules: this.sharedMemory?.getScoringRules(item?.type) || {},

      // ════════════════════════════════════════════════════════════════════
      // Layer 4: USER LAB (personal context)
      // ════════════════════════════════════════════════════════════════════
      userPreferences: this.userLab?.getPreferences?.() || {},
      projectPatterns: this.userLab?.getProjectPatterns?.() || [],
      recentUserFeedback: this.userLab?.getRecentFeedback?.(3) || [],

      // ════════════════════════════════════════════════════════════════════
      // Request-specific context
      // ════════════════════════════════════════════════════════════════════
      ...requestContext,

      // ════════════════════════════════════════════════════════════════════
      // Metadata
      // ════════════════════════════════════════════════════════════════════
      _meta: {
        timestamp: Date.now(),
        mode: this.mode,
        itemType: item?.type || 'unknown',
      },
    };
  }

  /**
   * Select dogs based on execution mode
   * @private
   */
  _selectDogs() {
    const allDogs = Object.keys(DOG_CONFIG);

    switch (this.mode) {
      case DogMode.CRITICAL_ONLY:
        return ['GUARDIAN', 'CYNIC'];

      case DogMode.FAST:
        return allDogs.filter(dog => DOG_CONFIG[dog].model === DogModel.HAIKU);

      default:
        return allDogs;
    }
  }

  /**
   * Run dogs in parallel
   * @private
   */
  async _runParallel(item, dogs, injectedContext) {
    // Emit vote start event for visibility
    this.eventBus?.publish('dog:vote:start', {
      dogs: dogs.map(d => d.toLowerCase()),
      mode: 'parallel',
    }, { source: 'DogOrchestrator' });

    const promises = dogs.map(dog =>
      this._invokeDog(dog, item, injectedContext)
        .then(result => ({ dog, ...result, success: true }))
        .catch(err => ({ dog, error: err.message, success: false }))
    );

    const results = await Promise.allSettled(promises);

    return results.map(r => {
      const vote = r.status === 'fulfilled' ? r.value : { dog: 'unknown', error: r.reason?.message, success: false };
      this._recordVote(vote);

      // Emit individual vote event for visibility
      this.eventBus?.publish('dog:vote:cast', {
        dogId: vote.dog?.toLowerCase(),
        score: vote.score,
        response: vote.response,
        confidence: vote.confidence,
        success: vote.success,
      }, { source: 'DogOrchestrator' });

      return vote;
    });
  }

  /**
   * Run dogs sequentially (with early exit on block)
   * @private
   */
  async _runSequential(item, dogs, injectedContext) {
    // Emit vote start event for visibility
    this.eventBus?.publish('dog:vote:start', {
      dogs: dogs.map(d => d.toLowerCase()),
      mode: 'sequential',
    }, { source: 'DogOrchestrator' });

    const votes = [];

    for (const dog of dogs) {
      try {
        const result = await this._invokeDog(dog, item, injectedContext);
        const vote = { dog, ...result, success: true };
        this._recordVote(vote);
        votes.push(vote);

        // Emit individual vote event for visibility
        this.eventBus?.publish('dog:vote:cast', {
          dogId: dog.toLowerCase(),
          score: vote.score,
          response: vote.response,
          confidence: vote.confidence,
          success: true,
        }, { source: 'DogOrchestrator' });

        // Early exit on block
        if (vote.response === 'block' && DOG_CONFIG[dog]?.blocking) {
          break;
        }
      } catch (err) {
        const vote = { dog, error: err.message, success: false };
        this._recordVote(vote);
        votes.push(vote);

        // Emit error vote event
        this.eventBus?.publish('dog:vote:cast', {
          dogId: dog.toLowerCase(),
          success: false,
          error: err.message,
        }, { source: 'DogOrchestrator' });
      }
    }

    return votes;
  }

  /**
   * Invoke a single dog
   * @private
   */
  async _invokeDog(dogName, item, injectedContext) {
    const config = DOG_CONFIG[dogName];
    if (!config) {
      throw new Error(`Unknown dog: ${dogName}`);
    }

    // If we have a custom spawner (Claude API), use it
    if (this.spawner) {
      return this.spawner({
        dog: dogName,
        model: config.model,
        timeout: config.timeout,
        item,
        context: injectedContext,
      });
    }

    // Otherwise, use the existing CollectivePack
    if (this.collectivePack) {
      const agent = this._getAgentFromPack(dogName);
      if (agent) {
        // Call the agent's process method with injected context
        const result = await Promise.race([
          this._processWithAgent(agent, dogName, item, injectedContext),
          this._timeout(config.timeout),
        ]);
        return result;
      }
    }

    // Fallback mock response
    return this._mockDogResponse(dogName, item);
  }

  /**
   * Get agent from CollectivePack by name
   * @private
   */
  _getAgentFromPack(dogName) {
    const nameToAgent = {
      SAGE: this.collectivePack?.sage,
      ANALYST: this.collectivePack?.analyst,
      GUARDIAN: this.collectivePack?.guardian,
      SCHOLAR: this.collectivePack?.scholar,
      ARCHITECT: this.collectivePack?.architect,
      JANITOR: this.collectivePack?.janitor,
      SCOUT: this.collectivePack?.scout,
      CARTOGRAPHER: this.collectivePack?.cartographer,
      ORACLE: this.collectivePack?.oracle,
      DEPLOYER: this.collectivePack?.deployer,
      CYNIC: this.collectivePack?.cynic,
    };
    return nameToAgent[dogName];
  }

  /**
   * Process item with an agent
   * @private
   *
   * Agent method mapping (based on actual implementations):
   * - Guardian, Analyst, Sage, Architect, Scholar: analyze(event, context)
   * - Janitor, Scout, Cartographer, Oracle, Deployer: process(event, context)
   * - CYNIC: makeDecision(context) for judgment requests
   */
  async _processWithAgent(agent, dogName, item, injectedContext) {
    // Prepare event-like input for the agent
    const event = {
      type: 'orchestrator:judge_request',
      dog: dogName,
      item,
      context: injectedContext,
      timestamp: Date.now(),
    };

    // Dogs with analyze() method: Guardian, Analyst, Sage, Architect, Scholar
    const dogsWithAnalyze = ['GUARDIAN', 'ANALYST', 'SAGE', 'ARCHITECT', 'SCHOLAR'];
    if (dogsWithAnalyze.includes(dogName) && agent.analyze) {
      const result = await agent.analyze(event, injectedContext);
      return this._normalizeAgentResult(result, dogName);
    }

    // CYNIC uses makeDecision for judgment requests
    if (dogName === 'CYNIC' && agent.makeDecision) {
      const result = await agent.makeDecision({
        type: 'judgment',
        item,
        context: injectedContext,
      });
      return this._normalizeAgentResult(result, dogName);
    }

    // Dogs with process() method: Janitor, Scout, Cartographer, Oracle, Deployer
    if (agent.process) {
      const result = await agent.process(event, injectedContext);
      return this._normalizeAgentResult(result, dogName);
    }

    // If no method found, return mock
    return this._mockDogResponse(dogName, item);
  }

  /**
   * Normalize agent result to standard vote format
   * @private
   */
  _normalizeAgentResult(result, dogName) {
    if (!result) {
      return this._mockDogResponse(dogName, null);
    }

    // Handle different result formats
    const score = result.score ?? result.riskScore ?? result.qualityScore ?? 50;
    const verdict = result.verdict ?? this._scoreToVerdict(score);
    const blocked = result.blocked ?? result.shouldBlock ?? false;

    return {
      score,
      verdict,
      response: blocked ? 'block' : 'allow',
      reason: result.reason || result.summary || null,
      dimensions: result.dimensions || result.scores || {},
      weight: DOG_CONFIG[dogName]?.blocking ? 1.5 : 1,
      insights: result.insights || result.recommendations || [],
    };
  }

  /**
   * Convert score to verdict
   * @private
   */
  _scoreToVerdict(score) {
    if (score >= 85) return 'HOWL';
    if (score >= 62) return 'WAG';
    if (score >= 38) return 'GROWL';
    return 'BARK';
  }

  /**
   * Mock response for dogs without real implementation
   * @private
   */
  _mockDogResponse(dogName, item) {
    const baseScore = 50 + Math.random() * 30;
    return {
      score: baseScore,
      verdict: this._scoreToVerdict(baseScore),
      response: 'allow',
      weight: 1,
      dimensions: {},
      insights: [`[${dogName}] Mock evaluation`],
    };
  }

  /**
   * Timeout promise
   * @private
   */
  _timeout(ms) {
    return new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`Timeout after ${ms}ms`)), ms)
    );
  }

  /**
   * Record vote in stats
   * @private
   */
  _recordVote(vote) {
    const dogStats = this.stats.dogVotes[vote.dog];
    if (!dogStats) return;

    dogStats.total++;
    if (!vote.success) {
      dogStats.errors++;
    } else if (vote.response === 'block') {
      dogStats.block++;
    } else {
      dogStats.allow++;
    }
  }

  /**
   * Calculate φ-consensus from votes
   * @private
   */
  _calculateConsensus(votes) {
    const successfulVotes = votes.filter(v => v.success);
    const totalWeight = successfulVotes.reduce((sum, v) => sum + (v.weight || 1), 0);

    // Check for blocking votes
    const blockingVote = successfulVotes.find(v =>
      v.response === 'block' && DOG_CONFIG[v.dog]?.blocking
    );

    if (blockingVote) {
      return {
        blocked: true,
        blockedBy: blockingVote.dog,
        blockReason: blockingVote.reason,
        ratio: 0,
      };
    }

    // Calculate weighted approval ratio
    const approvalWeight = successfulVotes
      .filter(v => v.response === 'allow' || v.response === 'approve')
      .reduce((sum, v) => sum + (v.weight || 1), 0);

    const ratio = totalWeight > 0 ? approvalWeight / totalWeight : 0;

    // Track consensus reached
    if (ratio >= this.consensusThreshold) {
      this.stats.consensusReached++;
    }

    return {
      blocked: false,
      ratio,
      reached: ratio >= this.consensusThreshold,
      votes: successfulVotes.length,
      totalDogs: Object.keys(DOG_CONFIG).length,
      threshold: this.consensusThreshold,
    };
  }

  /**
   * Build final judgment from votes
   * @private
   */
  _buildJudgment(item, votes, consensus) {
    const successfulVotes = votes.filter(v => v.success);

    // Aggregate dimension scores (weighted average)
    const dimensions = {};
    const weights = {};

    for (const vote of successfulVotes) {
      if (vote.dimensions) {
        for (const [dim, score] of Object.entries(vote.dimensions)) {
          const weight = vote.weight || 1;
          dimensions[dim] = (dimensions[dim] || 0) + score * weight;
          weights[dim] = (weights[dim] || 0) + weight;
        }
      }
    }

    // Normalize dimensions
    for (const dim of Object.keys(dimensions)) {
      dimensions[dim] = dimensions[dim] / weights[dim];
    }

    // Calculate global score
    const scores = successfulVotes.map(v => v.score).filter(s => typeof s === 'number');
    const globalScore = scores.length > 0
      ? scores.reduce((a, b) => a + b, 0) / scores.length
      : 50;

    // Determine verdict
    const verdict = this._scoreToVerdict(globalScore);

    // Collect all insights
    const allInsights = successfulVotes
      .flatMap(v => v.insights || [])
      .filter(Boolean);

    return {
      id: `jdg_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      global_score: globalScore,
      verdict,
      dimensions,
      consensus: {
        ratio: consensus.ratio,
        reached: consensus.reached,
        threshold: consensus.threshold,
        votingDogs: successfulVotes.length,
      },
      votes: successfulVotes.map(v => ({
        dog: v.dog,
        score: v.score,
        verdict: v.verdict,
        weight: v.weight || 1,
      })),
      insights: allInsights.slice(0, 10), // Top 10 insights
      timestamp: Date.now(),
      itemType: item?.type,
    };
  }

  /**
   * Process feedback and update SharedMemory
   * @param {string} judgmentId - Judgment ID
   * @param {string} outcome - 'correct' | 'incorrect'
   * @param {Object} [details] - Additional feedback details
   */
  async processFeedback(judgmentId, outcome, details = {}) {
    const wasCorrect = outcome === 'correct';

    // P2.4: Record feedback in swarm consensus for learning
    if (this.swarmConsensus) {
      this.swarmConsensus.recordFeedback(judgmentId, wasCorrect, details);
    }

    if (!this.sharedMemory) return;

    // Record feedback
    this.sharedMemory.recordFeedback({
      judgmentId,
      outcome,
      ...details,
    });

    // Adjust weights based on feedback
    if (details.dimensions) {
      const delta = wasCorrect ? 0.1 : -0.1;
      for (const dim of Object.keys(details.dimensions)) {
        this.sharedMemory.adjustWeight(dim, delta, 'user_feedback');
      }
    }

    // Save updated memory
    await this.sharedMemory.save();
  }

  /**
   * Get orchestrator stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      mode: this.mode,
      consensusThreshold: this.consensusThreshold,
      memoryStats: this.sharedMemory?.getStats() || null,
      swarmStats: this.swarmConsensus?.getStats() || null,
    };
  }

  /**
   * Export swarm consensus state for persistence
   * @returns {Object|null} State or null if not using swarm
   */
  exportSwarmState() {
    return this.swarmConsensus?.exportState() || null;
  }

  /**
   * Import swarm consensus state from persistence
   * @param {Object} state - State to import
   */
  importSwarmState(state) {
    if (this.swarmConsensus && state) {
      this.swarmConsensus.importState(state);
    }
  }
}

// =============================================================================
// DOG CHAIN (Phase 3: Stream Chaining)
// =============================================================================

/**
 * Pre-defined dog chains for common workflows
 */
export const DOG_CHAINS = {
  // Security review: Guardian verifies, then Analyst assesses
  SECURITY_REVIEW: ['GUARDIAN', 'ANALYST'],

  // Architecture: Scout explores, Architect designs, Guardian validates
  ARCHITECTURE: ['SCOUT', 'ARCHITECT', 'GUARDIAN'],

  // Code quality: Analyst reviews, Janitor simplifies, Guardian checks
  CODE_QUALITY: ['ANALYST', 'JANITOR', 'GUARDIAN'],

  // Learning: Scholar researches, Sage interprets, Oracle predicts
  RESEARCH: ['SCHOLAR', 'SAGE', 'ORACLE'],

  // Deployment: Cartographer maps, Deployer executes, Guardian monitors
  DEPLOYMENT: ['CARTOGRAPHER', 'DEPLOYER', 'GUARDIAN'],

  // Full consensus: All critical dogs in sequence
  FULL_REVIEW: ['SCOUT', 'ANALYST', 'ARCHITECT', 'GUARDIAN', 'CYNIC'],
};

/**
 * Dog Chain - Sequential dog execution with accumulated context
 *
 * Unlike parallel/sequential mode in DogOrchestrator, DogChain:
 * - Accumulates insights between dogs
 * - Each dog receives context from all previous dogs
 * - Can transform/filter context between steps
 * - Supports early exit and conditional branching
 *
 * "La chaîne des chiens - each dog adds to the pack's wisdom"
 */
export class DogChain {
  /**
   * @param {Object} options
   * @param {DogOrchestrator} options.orchestrator - The dog orchestrator
   * @param {string[]} [options.chain] - Dog names in order, or use a preset
   * @param {string} [options.preset] - Preset chain name from DOG_CHAINS
   * @param {Function} [options.contextTransformer] - Transform context between dogs
   * @param {Function} [options.shouldContinue] - Decide if chain should continue
   */
  constructor(options = {}) {
    this.orchestrator = options.orchestrator;

    // Use preset or custom chain
    if (options.preset && DOG_CHAINS[options.preset]) {
      this.chain = [...DOG_CHAINS[options.preset]];
    } else {
      this.chain = options.chain || ['SCOUT', 'ANALYST', 'GUARDIAN'];
    }

    // Context transformer between dogs (optional)
    this.contextTransformer = options.contextTransformer || ((ctx, result) => ({
      ...ctx,
      previousResults: [...(ctx.previousResults || []), result],
      accumulatedInsights: [
        ...(ctx.accumulatedInsights || []),
        ...(result.insights || []),
      ],
      chainProgress: (ctx.chainProgress || 0) + 1,
    }));

    // Early exit condition (optional)
    this.shouldContinue = options.shouldContinue || ((result, ctx) => {
      // Stop if Guardian blocks
      if (result.dog === 'GUARDIAN' && result.response === 'block') {
        return false;
      }
      // Stop if critical error
      if (!result.success && DOG_CONFIG[result.dog]?.blocking) {
        return false;
      }
      return true;
    });

    // Execution stats
    this.stats = {
      runs: 0,
      completed: 0,
      aborted: 0,
      averageChainLength: 0,
    };
  }

  /**
   * Execute the dog chain
   * @param {*} item - Item to process
   * @param {Object} [initialContext] - Initial context
   * @returns {Promise<Object>} Chain result with all dog outputs
   */
  async execute(item, initialContext = {}) {
    this.stats.runs++;

    const context = {
      ...initialContext,
      chainId: `chain_${Date.now()}`,
      chainDogs: this.chain,
      previousResults: [],
      accumulatedInsights: [],
      chainProgress: 0,
    };

    const results = [];
    let finalResult = null;
    let aborted = false;

    for (const dog of this.chain) {
      try {
        // Build context for this dog
        const dogContext = this._buildDogContext(dog, context, results);

        // Invoke the dog
        const result = await this.orchestrator._invokeDog(dog, item, dogContext);
        const enrichedResult = {
          dog,
          ...result,
          success: true,
          chainStep: context.chainProgress + 1,
          totalSteps: this.chain.length,
        };

        results.push(enrichedResult);
        this.orchestrator._recordVote(enrichedResult);

        // Check if we should continue
        if (!this.shouldContinue(enrichedResult, context)) {
          aborted = true;
          finalResult = enrichedResult;
          break;
        }

        // Transform context for next dog
        Object.assign(context, this.contextTransformer(context, enrichedResult));

      } catch (err) {
        const errorResult = {
          dog,
          error: err.message,
          success: false,
          chainStep: context.chainProgress + 1,
          totalSteps: this.chain.length,
        };
        results.push(errorResult);

        // Check if blocking dog failed
        if (DOG_CONFIG[dog]?.blocking) {
          aborted = true;
          finalResult = errorResult;
          break;
        }
      }
    }

    // Update stats
    if (aborted) {
      this.stats.aborted++;
    } else {
      this.stats.completed++;
    }
    this.stats.averageChainLength =
      (this.stats.averageChainLength * (this.stats.runs - 1) + results.length) /
      this.stats.runs;

    // Build final chain result
    return {
      chainId: context.chainId,
      chain: this.chain,
      results,
      completed: !aborted,
      abortedBy: aborted ? finalResult?.dog : null,
      abortReason: aborted ? (finalResult?.reason || finalResult?.error) : null,
      insights: context.accumulatedInsights,
      finalScore: this._calculateChainScore(results),
      timestamp: Date.now(),
    };
  }

  /**
   * Build context for a specific dog in the chain
   * @private
   */
  _buildDogContext(dog, chainContext, previousResults) {
    const context = {
      // Chain metadata
      chainId: chainContext.chainId,
      chainPosition: chainContext.chainProgress + 1,
      chainTotal: this.chain.length,
      previousDogs: previousResults.map(r => r.dog),

      // Accumulated insights from previous dogs
      insights: chainContext.accumulatedInsights,

      // Summary of previous results
      previousSummary: previousResults.length > 0
        ? this._summarizePreviousResults(previousResults)
        : null,

      // Dog-specific instructions
      chainRole: this._getChainRole(dog, chainContext.chainProgress),
    };

    return context;
  }

  /**
   * Summarize previous results for next dog
   * @private
   */
  _summarizePreviousResults(results) {
    const successful = results.filter(r => r.success);
    const blocked = results.find(r => r.response === 'block');

    return {
      dogsCompleted: successful.length,
      averageScore: successful.length > 0
        ? successful.reduce((sum, r) => sum + (r.score || 50), 0) / successful.length
        : null,
      hasBlock: !!blocked,
      blockReason: blocked?.reason,
      keyInsights: results
        .flatMap(r => r.insights || [])
        .slice(0, 5),
    };
  }

  /**
   * Get role description for dog in chain context
   * @private
   */
  _getChainRole(dog, position) {
    if (position === 0) {
      return `You are the first dog in this chain. Provide initial analysis.`;
    }
    if (position === this.chain.length - 1) {
      return `You are the final dog. Synthesize previous insights and give final verdict.`;
    }
    return `You are dog ${position + 1} of ${this.chain.length}. Build on previous insights.`;
  }

  /**
   * Calculate overall chain score
   * @private
   */
  _calculateChainScore(results) {
    const successful = results.filter(r => r.success && typeof r.score === 'number');
    if (successful.length === 0) return null;

    // Weight later dogs higher (they have more context)
    let weightedSum = 0;
    let totalWeight = 0;

    successful.forEach((r, i) => {
      const weight = 1 + (i * 0.5); // 1, 1.5, 2, 2.5, ...
      weightedSum += r.score * weight;
      totalWeight += weight;
    });

    return Math.round(weightedSum / totalWeight);
  }

  /**
   * Get chain stats
   */
  getStats() {
    return { ...this.stats };
  }
}

// =============================================================================
// P1.4: STREAM CHAIN - Full context preservation between dogs
// "La chaîne coule" - Context flows like water between dogs
// =============================================================================

/**
 * Output transformer types for stream chaining
 */
export const StreamTransformers = {
  /**
   * Pass full output to next dog (100% context preservation)
   */
  FULL: (prevOutput, _context) => ({
    fullPreviousOutput: prevOutput,
    previousScore: prevOutput.score,
    previousVerdict: prevOutput.verdict,
    previousInsights: prevOutput.insights || [],
    previousRecommendations: prevOutput.recommendations || [],
  }),

  /**
   * Pass only insights (filtered context)
   */
  INSIGHTS_ONLY: (prevOutput, _context) => ({
    insights: prevOutput.insights || [],
    score: prevOutput.score,
  }),

  /**
   * Pass summary (compressed context)
   */
  SUMMARY: (prevOutput, context) => ({
    summary: {
      dog: prevOutput.dog,
      score: prevOutput.score,
      verdict: prevOutput.verdict,
      keyInsight: (prevOutput.insights || [])[0],
      chainProgress: context.chainProgress,
    },
  }),

  /**
   * Custom transformer
   */
  custom: (fn) => fn,
};

/**
 * StreamChain - Enhanced DogChain with 100% context preservation
 *
 * Unlike basic DogChain, StreamChain:
 * - Preserves complete output from each dog
 * - Supports custom output transformers
 * - Enables parallel sub-chains
 * - Allows chain composition
 */
export class StreamChain extends DogChain {
  /**
   * @param {Object} options
   * @param {DogOrchestrator} options.orchestrator
   * @param {string[]} [options.chain]
   * @param {string} [options.preset]
   * @param {Function|string} [options.transformer] - Transform output between dogs
   * @param {boolean} [options.preserveFull] - Preserve full output (100% context)
   */
  constructor(options = {}) {
    super(options);

    // Stream-specific options
    this.preserveFull = options.preserveFull ?? true;
    this.outputTransformer = this._resolveTransformer(options.transformer);

    // Stream state
    this.streamHistory = [];

    // Override context transformer for stream behavior
    this.contextTransformer = this._streamContextTransformer.bind(this);
  }

  /**
   * Resolve transformer from options
   * @private
   */
  _resolveTransformer(transformer) {
    if (!transformer) return StreamTransformers.FULL;
    if (typeof transformer === 'function') return transformer;
    if (StreamTransformers[transformer]) return StreamTransformers[transformer];
    return StreamTransformers.FULL;
  }

  /**
   * Stream context transformer - preserves full context
   * @private
   */
  _streamContextTransformer(ctx, result) {
    // Store in stream history
    this.streamHistory.push({
      dog: result.dog,
      output: result,
      timestamp: Date.now(),
    });

    // Apply output transformer
    const transformed = this.outputTransformer(result, ctx);

    return {
      ...ctx,
      previousResults: [...(ctx.previousResults || []), result],
      accumulatedInsights: [
        ...(ctx.accumulatedInsights || []),
        ...(result.insights || []),
      ],
      chainProgress: (ctx.chainProgress || 0) + 1,
      // Stream-specific: transformed output from previous dog
      streamInput: transformed,
      // Full history if preserveFull is true
      ...(this.preserveFull ? { streamHistory: this.streamHistory } : {}),
    };
  }

  /**
   * Execute stream chain with full context preservation
   */
  async execute(item, initialContext = {}) {
    // Reset stream history
    this.streamHistory = [];

    // Execute chain with stream context
    const result = await super.execute(item, {
      ...initialContext,
      streamMode: true,
      preserveFull: this.preserveFull,
    });

    // Add stream history to result
    return {
      ...result,
      streamHistory: this.streamHistory,
      streamStats: {
        totalContextSize: JSON.stringify(this.streamHistory).length,
        dogsExecuted: this.streamHistory.length,
        contextPreserved: this.preserveFull ? '100%' : 'filtered',
      },
    };
  }

  /**
   * Execute multiple chains in parallel and merge results
   * @param {Array<StreamChain>} chains - Chains to execute in parallel
   * @param {*} item - Item to process
   * @param {Object} context - Shared context
   * @returns {Promise<Object>} Merged results
   */
  static async parallel(chains, item, context = {}) {
    const results = await Promise.allSettled(
      chains.map(chain => chain.execute(item, context))
    );

    const successful = results
      .filter(r => r.status === 'fulfilled')
      .map(r => r.value);

    const failed = results
      .filter(r => r.status === 'rejected')
      .map(r => r.reason?.message || 'Unknown error');

    // Merge insights from all chains
    const allInsights = successful.flatMap(r => r.insights || []);
    const uniqueInsights = [...new Set(allInsights.map(i => JSON.stringify(i)))]
      .map(i => JSON.parse(i));

    // Calculate consensus score
    const scores = successful
      .map(r => r.finalScore)
      .filter(s => typeof s === 'number');
    const consensusScore = scores.length > 0
      ? Math.round(scores.reduce((a, b) => a + b, 0) / scores.length)
      : null;

    return {
      mode: 'parallel',
      chains: successful.length,
      failed: failed.length,
      results: successful,
      insights: uniqueInsights,
      consensusScore,
      errors: failed,
      timestamp: Date.now(),
    };
  }

  /**
   * Compose multiple chains into a single chain
   * @param {Array<string[]>} chainDefinitions - Array of dog arrays
   * @param {DogOrchestrator} orchestrator
   * @returns {StreamChain} Composed chain
   */
  static compose(chainDefinitions, orchestrator) {
    const composedChain = chainDefinitions.flat();
    return new StreamChain({
      orchestrator,
      chain: composedChain,
      preserveFull: true,
    });
  }

  /**
   * Create a chain that branches based on intermediate results
   * @param {Object} options
   * @param {string[]} options.initial - Initial dogs
   * @param {Object} options.branches - Condition → chain mapping
   * @param {Function} options.condition - Function to evaluate which branch
   */
  static conditional(options) {
    // Return a factory function
    return async (orchestrator, item, context = {}) => {
      // Execute initial chain
      const initialChain = new StreamChain({
        orchestrator,
        chain: options.initial,
      });
      const initialResult = await initialChain.execute(item, context);

      // Evaluate condition
      const branchKey = options.condition(initialResult, context);
      const branchChain = options.branches[branchKey];

      if (!branchChain) {
        return {
          ...initialResult,
          branch: 'none',
          branchKey,
        };
      }

      // Execute branch chain
      const branch = new StreamChain({
        orchestrator,
        chain: branchChain,
      });
      const branchResult = await branch.execute(item, {
        ...context,
        previousChainResult: initialResult,
      });

      return {
        initial: initialResult,
        branch: branchResult,
        branchKey,
        combined: {
          insights: [...initialResult.insights, ...branchResult.insights],
          finalScore: branchResult.finalScore,
        },
      };
    };
  }
}

export default DogOrchestrator;
