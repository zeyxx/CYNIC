/**
 * @cynic/node - Collective Agents
 *
 * The Five Dogs (Fib(5)) + CYNIC (Keter): A harmonious collective consciousness.
 *
 * Inspired by Kabbalah's Sefirot, implemented with professional naming:
 *
 *                    ╭─────────────────╮
 *                    │     CYNIC       │  ← The Hidden 6th Dog
 *                    │    (Keter)      │  Meta-consciousness
 *                    │   κυνικός       │  "Loyal to truth"
 *                    ╰────────┬────────╯
 *                             │
 *        ╭────────────────────┼────────────────────╮
 *        │                    │                    │
 *   ╭────▼────╮          ╭────▼────╮          ╭────▼────╮
 *   │  SAGE   │◄────────►│ SCHOLAR │◄────────►│GUARDIAN │
 *   │(Chochmah)│  φ-bus  │ (Daat)  │  φ-bus   │(Gevurah)│
 *   │ Wisdom  │          │Knowledge│          │Strength │
 *   ╰────┬────╯          ╰────┬────╯          ╰────┬────╯
 *        │                    │                    │
 *        ╰────────────────────┼────────────────────╯
 *                             │
 *   ╭─────────────────────────┼─────────────────────────╮
 *   │                         │                         │
 *╭──▼──────╮            ╭─────▼─────╮            ╭──────▼───╮
 *│ ANALYST │◄──────────►│ EVENT BUS │◄──────────►│ARCHITECT │
 *│ (Binah) │            │  φ-aligned │            │ (Chesed) │
 *│Understand│            │  987 events│            │ Kindness │
 *╰─────────╯            ╰───────────╯            ╰──────────╯
 *
 * CYNIC observes ALL events and orchestrates the collective.
 * φ⁻¹ (61.8%) = consensus threshold, max confidence
 * φ⁻² (38.2%) = veto threshold, override threshold
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective
 */

'use strict';

import { PHI_INV, createLogger } from '@cynic/core';

const log = createLogger('CollectivePack');
import { AgentEventBus } from '../event-bus.js';
import { AgentEvent, AgentEventMessage, AgentId, ConsensusVote, EventPriority } from '../events.js';
import { ProfileCalculator, ProfileLevel } from '../../profile/calculator.js';
import { OrganicSignals } from '../../profile/organic-signals.js';
import { LocalStore } from '../../privacy/local-store.js';

// Import collective agents (The Five Dogs)
import { CollectiveGuardian, RiskLevel, RiskCategory } from './guardian.js';
import { CollectiveAnalyst, PatternCategory, AnomalyType } from './analyst.js';
import { CollectiveScholar, KnowledgeType } from './scholar.js';
import { CollectiveArchitect, ReviewCategory, FeedbackType } from './architect.js';
import { CollectiveSage, WisdomType } from './sage.js';

// Import additional dogs (completing the Sefirot tree)
import { CollectiveJanitor, JANITOR_CONSTANTS, QualitySeverity, IssueType } from './janitor.js';
import { CollectiveScout, SCOUT_CONSTANTS, DiscoveryType, OpportunityType } from './scout.js';
import { CollectiveCartographer, CARTOGRAPHER_CONSTANTS, RepoType, ConnectionType, MapIssueType } from './cartographer.js';
import { CollectiveOracle, ORACLE_CONSTANTS, ViewType, MetricType, AlertSeverity } from './oracle.js';
import { CollectiveDeployer, DEPLOYER_CONSTANTS, DeploymentState, DeployTarget, HealthStatus } from './deployer.js';

// Import CYNIC - The Hidden Sixth Dog (Keter)
import {
  CollectiveCynic,
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
} from './cynic.js';

// Re-export agents
export {
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  CollectiveCynic,
  // Additional dogs
  CollectiveJanitor,
  CollectiveScout,
  CollectiveCartographer,
  CollectiveOracle,
  CollectiveDeployer,
};

// Re-export types
export {
  RiskLevel,
  RiskCategory,
  PatternCategory,
  AnomalyType,
  KnowledgeType,
  ReviewCategory,
  FeedbackType,
  WisdomType,
  // CYNIC types
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
  // Janitor types
  JANITOR_CONSTANTS,
  QualitySeverity,
  IssueType,
  // Scout types
  SCOUT_CONSTANTS,
  DiscoveryType,
  OpportunityType,
  // Cartographer types
  CARTOGRAPHER_CONSTANTS,
  RepoType,
  ConnectionType,
  MapIssueType,
  // Oracle types
  ORACLE_CONSTANTS,
  ViewType,
  MetricType,
  AlertSeverity,
  // Deployer types
  DEPLOYER_CONSTANTS,
  DeploymentState,
  DeployTarget,
  HealthStatus,
};

/**
 * φ-aligned constants for collective
 *
 * The 11 Dogs (Sefirot):
 * - Guardian (Gevurah), Analyst (Binah), Scholar (Daat), Architect (Chesed), Sage (Chochmah)
 * - CYNIC (Keter), Janitor (Yesod), Scout (Netzach), Cartographer (Malkhut), Oracle (Tiferet), Deployer (Hod)
 */
export const COLLECTIVE_CONSTANTS = {
  /** Number of Dogs in the Collective (11 Sefirot) */
  DOG_COUNT: 11,

  /** Alias for DOG_COUNT (legacy compatibility) */
  AGENT_COUNT: 11,

  /** Max collective confidence (φ⁻¹) */
  MAX_CONFIDENCE: PHI_INV,

  /** Consensus threshold (φ⁻¹) */
  CONSENSUS_THRESHOLD: PHI_INV,

  /** Default profile level */
  DEFAULT_PROFILE_LEVEL: ProfileLevel.PRACTITIONER,
};

/**
 * Collective Pack - The Five Dogs working as one
 */
export class CollectivePack {
  /**
   * Create the collective
   * @param {Object} [options] - Options
   * @param {Object} [options.judge] - CYNIC judge instance
   * @param {Object} [options.state] - State manager instance
   * @param {number} [options.profileLevel] - Initial profile level
   * @param {Object} [options.localStore] - Local store for privacy
   * @param {Function} [options.onDogDecision] - Callback when dog makes decision (for SSE broadcast)
   * @param {Object} [options.persistence] - Persistence manager for storing observations
   * @param {Object} [options.graphIntegration] - JudgmentGraph integration for relationship tracking
   */
  constructor(options = {}) {
    // Callbacks for external integration
    this.onDogDecision = options.onDogDecision || null;
    this.persistence = options.persistence || null;
    this.graphIntegration = options.graphIntegration || null;

    // Shared infrastructure (pass persistence to EventBus for logging)
    this.eventBus = new AgentEventBus({ persistence: this.persistence });
    this.profileCalculator = new ProfileCalculator();
    this.signalCollector = new OrganicSignals();
    this.localStore = options.localStore || null;

    // Current profile level
    this.profileLevel = options.profileLevel || COLLECTIVE_CONSTANTS.DEFAULT_PROFILE_LEVEL;

    // Register all agents with event bus BEFORE creating them
    // (agents subscribe to events in their constructors)
    this.eventBus.registerAgent(AgentId.GUARDIAN);
    this.eventBus.registerAgent(AgentId.ANALYST);
    this.eventBus.registerAgent(AgentId.SCHOLAR);
    this.eventBus.registerAgent(AgentId.ARCHITECT);
    this.eventBus.registerAgent(AgentId.SAGE);
    this.eventBus.registerAgent(AgentId.CYNIC); // The Hidden Dog (Keter)
    this.eventBus.registerAgent(AgentId.JANITOR); // Foundation (Yesod)
    this.eventBus.registerAgent(AgentId.SCOUT); // Victory (Netzach)
    this.eventBus.registerAgent(AgentId.CARTOGRAPHER); // Kingdom (Malkhut)
    this.eventBus.registerAgent(AgentId.ORACLE); // Beauty (Tiferet)
    this.eventBus.registerAgent(AgentId.DEPLOYER); // Splendor (Hod)
    this.eventBus.registerAgent('collective'); // For pack-level subscriptions

    // Create agents with shared infrastructure
    this.guardian = new CollectiveGuardian({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
    });

    this.analyst = new CollectiveAnalyst({
      eventBus: this.eventBus,
      profileCalculator: this.profileCalculator,
      signalCollector: this.signalCollector,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
    });

    this.scholar = new CollectiveScholar({
      eventBus: this.eventBus,
      localStore: this.localStore,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
    });

    this.architect = new CollectiveArchitect({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
    });

    this.sage = new CollectiveSage({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
    });

    // CYNIC - The Hidden Dog (Keter) - Meta-consciousness
    this.cynic = new CollectiveCynic({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
    });

    // Janitor - Foundation (Yesod) - Code quality & hygiene
    this.janitor = new CollectiveJanitor({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      persistence: this.persistence,
    });

    // Scout - Victory (Netzach) - Discovery & exploration
    this.scout = new CollectiveScout({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      persistence: this.persistence,
    });

    // Cartographer - Kingdom (Malkhut) - Reality mapping
    this.cartographer = new CollectiveCartographer({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      persistence: this.persistence,
    });

    // Oracle - Beauty (Tiferet) - Visualization & monitoring
    this.oracle = new CollectiveOracle({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      persistence: this.persistence,
    });

    // Deployer - Splendor (Hod) - Deployment & infrastructure
    this.deployer = new CollectiveDeployer({
      eventBus: this.eventBus,
      guardian: this.guardian, // For deployment approval
      profileLevel: this.profileLevel,
      persistence: this.persistence,
    });

    // Agent map for lookup (5 original Dogs + CYNIC + Janitor + Scout + Cartographer + Oracle + Deployer)
    this.agents = new Map([
      [AgentId.GUARDIAN, this.guardian],
      [AgentId.ANALYST, this.analyst],
      [AgentId.SCHOLAR, this.scholar],
      [AgentId.ARCHITECT, this.architect],
      [AgentId.SAGE, this.sage],
      [AgentId.CYNIC, this.cynic], // Keter - The Crown
      [AgentId.JANITOR, this.janitor], // Yesod - Foundation
      [AgentId.SCOUT, this.scout], // Netzach - Victory
      [AgentId.CARTOGRAPHER, this.cartographer], // Malkhut - Kingdom
      [AgentId.ORACLE, this.oracle], // Tiferet - Beauty
      [AgentId.DEPLOYER, this.deployer], // Hod - Splendor
    ]);

    // Stats
    this.collectiveStats = {
      created: Date.now(),
      totalProcessed: 0,
      profileUpdates: 0,
      consensusRequests: 0,
    };

    // Subscribe to profile updates
    this._subscribeToProfileUpdates();
  }

  /**
   * Subscribe to profile updates from Analyst
   * @private
   */
  _subscribeToProfileUpdates() {
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      'collective',
      this._handleProfileUpdate.bind(this)
    );
  }

  /**
   * Handle profile update from Analyst
   * @private
   */
  _handleProfileUpdate(event) {
    const { newLevel } = event.payload;

    // Update all agents
    this.profileLevel = newLevel;
    this.guardian.setProfileLevel(newLevel);
    this.scholar.setProfileLevel(newLevel);
    this.architect.setProfileLevel(newLevel);
    this.sage.setProfileLevel(newLevel);
    this.cynic.setProfileLevel(newLevel);
    this.janitor.profileLevel = newLevel; // Janitor uses direct property
    this.scout.setProfileLevel(newLevel);
    this.cartographer.setProfileLevel(newLevel);
    this.oracle.setProfileLevel(newLevel);
    this.deployer.setProfileLevel(newLevel);

    this.collectiveStats.profileUpdates++;
  }

  /**
   * Get agent by ID
   * @param {string} agentId - Agent ID
   * @returns {Object|null} Agent or null
   */
  getAgent(agentId) {
    return this.agents.get(agentId) || null;
  }

  /**
   * Get all agents
   * @returns {Object[]} Array of agents
   */
  getAllAgents() {
    return Array.from(this.agents.values());
  }

  /**
   * Process event through appropriate agents
   * @param {Object} event - Event to process
   * @param {Object} [context] - Context
   * @returns {Promise<Object[]>} Results from all triggered agents
   */
  async processEvent(event, context = {}) {
    const results = [];
    this.collectiveStats.totalProcessed++;

    for (const [id, agent] of this.agents) {
      if (agent.shouldTrigger(event)) {
        try {
          const result = await agent.process(event, context);
          results.push({
            agent: id,
            ...result,
          });
        } catch (err) {
          results.push({
            agent: id,
            error: err.message,
          });
        }
      }
    }

    return results;
  }

  /**
   * Check command safety (Guardian)
   * @param {string} command - Command to check
   * @returns {Promise<Object>} Safety assessment
   */
  async checkCommand(command) {
    return this.guardian.checkCommand(command);
  }

  /**
   * Extract knowledge (Scholar)
   * @param {string} content - Content to extract from
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Extraction result
   */
  async extractKnowledge(content, options = {}) {
    return this.scholar.extract(content, options);
  }

  /**
   * Review code (Architect)
   * @param {string} code - Code to review
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Review result
   */
  async reviewCode(code, options = {}) {
    return this.architect.review(code, options);
  }

  /**
   * Get wisdom (Sage)
   * @param {string} topic - Topic
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Wisdom
   */
  async getWisdom(topic, options = {}) {
    return this.sage.shareWisdom(topic, options);
  }

  /**
   * Review a judgment (Gap #1 - Collective Consciousness Integration)
   *
   * The collective reviews the judgment and may reach consensus on
   * whether it's correct, needs adjustment, or is anomalous.
   *
   * @param {Object} judgment - The judgment to review
   * @param {Object} item - The original item that was judged
   * @returns {Promise<Object>} Collective review result
   */
  async reviewJudgment(judgment, item) {
    // Emit event for all dogs to observe
    await this.eventBus.publish({
      type: 'judgment:review_request',
      source: 'collective',
      target: '*', // All agents
      priority: EventPriority.NORMAL,
      payload: {
        judgmentId: judgment.id,
        verdict: judgment.verdict,
        qScore: judgment.q_score || judgment.global_score,
        confidence: judgment.confidence,
        dimensions: judgment.dimensions,
        item: {
          type: item?.type,
          identifier: item?.identifier || item?.name,
          contentPreview: typeof item?.content === 'string'
            ? item.content.slice(0, 200)
            : null,
        },
      },
    });

    // Collect opinions from key dogs (async, parallel)
    const opinions = await Promise.allSettled([
      this.analyst.assessJudgment?.(judgment, item),
      this.sage.reflectOnJudgment?.(judgment, item),
      this.cynic.scrutinizeJudgment?.(judgment, item),
    ]);

    // Filter successful opinions
    const validOpinions = opinions
      .filter(r => r.status === 'fulfilled' && r.value)
      .map(r => r.value);

    // If we have enough opinions, request consensus
    if (validOpinions.length >= 2) {
      const consensusResult = await this.eventBus.requestConsensus?.('collective', {
        topic: 'judgment_review',
        judgmentId: judgment.id,
        options: ['APPROVE', 'ADJUST', 'REJECT'],
        context: {
          type: 'judgment',
          judgmentId: judgment.id,
          opinions: validOpinions,
        },
      });

      return {
        reviewed: true,
        opinions: validOpinions,
        consensus: consensusResult,
      };
    }

    return {
      reviewed: true,
      opinions: validOpinions,
      consensus: null,
    };
  }

  /**
   * Get current profile
   * @returns {Object} Profile data
   */
  getProfile() {
    return {
      level: this.profileLevel,
      ...this.profileCalculator.getProfile(),
    };
  }

  /**
   * Get collective summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      dogCount: COLLECTIVE_CONSTANTS.DOG_COUNT,
      agentCount: COLLECTIVE_CONSTANTS.AGENT_COUNT,
      profileLevel: this.profileLevel,
      eventBusStats: this.eventBus.getStats(),
      agents: {
        // The Five Dogs
        guardian: this.guardian.getSummary(),
        analyst: this.analyst.getSummary(),
        scholar: this.scholar.getSummary(),
        architect: this.architect.getSummary(),
        sage: this.sage.getSummary(),
        // The Hidden Dog - CYNIC (Keter)
        cynic: this.cynic.getSummary(),
        // Additional Dogs
        janitor: this.janitor.getSummary(),
        scout: this.scout.getSummary(),
        cartographer: this.cartographer.getSummary(),
        oracle: this.oracle.getSummary(),
        deployer: this.deployer.getSummary(),
      },
      collectiveState: this.cynic.getCollectiveState(),
      collectiveStats: this.collectiveStats,
    };
  }

  /**
   * Awaken CYNIC for a new session
   * @param {Object} sessionInfo - Session information
   * @returns {Promise<Object>} Awakening result
   */
  async awakenCynic(sessionInfo = {}) {
    return this.cynic.awaken(sessionInfo);
  }

  /**
   * Get CYNIC's view of the collective state
   * @returns {Object} Collective state
   */
  getCollectiveState() {
    return this.cynic.getCollectiveState();
  }

  /**
   * Issue guidance from CYNIC to the collective
   * @param {Object} guidance - Guidance to issue
   * @returns {Promise<Object>} Result
   */
  async issueGuidance(guidance) {
    return this.cynic.issueGuidance(guidance);
  }

  /**
   * Request introspection from CYNIC
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Result
   */
  async introspect(options = {}) {
    return this.cynic.introspect(options);
  }

  /**
   * Get event bus
   * @returns {Object} Event bus
   */
  getEventBus() {
    return this.eventBus;
  }

  /**
   * Receive hook event from Claude Code
   * This bridges external hooks to the collective:
   * 1. Runs full agent pipeline (shouldTrigger → analyze → decide)
   * 2. Publishes to eventBus for inter-dog communication
   *
   * @param {Object} hookData - Hook event data
   * @param {string} hookData.hookType - Type: SessionStart, SessionEnd, UserPromptSubmit, PreToolUse, PostToolUse, Stop
   * @param {Object} hookData.payload - Hook payload
   * @param {string} [hookData.userId] - User ID
   * @param {string} [hookData.sessionId] - Session ID
   * @returns {Promise<Object>} Result with agent decisions and delivery count
   */
  async receiveHookEvent(hookData) {
    const { hookType, payload = {}, userId, sessionId } = hookData;

    // Map Claude Code hook types to AgentEvent types
    const hookEventMap = {
      SessionStart: AgentEvent.HOOK_SESSION_START,
      SessionEnd: AgentEvent.HOOK_SESSION_STOP, // Both Stop and SessionEnd map to session stop
      UserPromptSubmit: AgentEvent.HOOK_PROMPT_SUBMIT,
      PreToolUse: AgentEvent.HOOK_PRE_TOOL,
      PostToolUse: AgentEvent.HOOK_POST_TOOL,
      Stop: AgentEvent.HOOK_SESSION_STOP,
      pattern: AgentEvent.HOOK_PATTERN,
    };

    const eventType = hookEventMap[hookType];
    if (!eventType) {
      return { success: false, error: `Unknown hook type: ${hookType}` };
    }

    // Create event for full pipeline processing
    const processEvent = {
      type: hookType,
      tool: payload.tool,
      input: payload.input,
      output: payload.output,
      duration: payload.duration,
      success: payload.success !== false,
      userId,
      sessionId,
      timestamp: Date.now(),
    };

    // 🐕 RUN FULL AGENT PIPELINE (shouldTrigger → analyze → decide)
    let agentResults = [];
    let blocked = false;
    let blockedBy = null;
    let blockMessage = null;

    try {
      agentResults = await this.processEvent(processEvent, { hookType, userId, sessionId });

      // Check if any agent blocked the operation
      for (const result of agentResults) {
        if (result.response === 'block') {
          blocked = true;
          blockedBy = result.agent;
          blockMessage = result.message;
          break;
        }
      }
    } catch (err) {
      log.error('processEvent error', { error: err.message });
    }

    // 🐕 Broadcast dog decisions via callback (for SSE)
    if (this.onDogDecision && agentResults.length > 0) {
      for (const result of agentResults) {
        try {
          this.onDogDecision({
            dog: result.agent,
            response: result.response,
            action: result.action,
            message: result.message,
            hookType,
            tool: payload.tool,
            blocked: result.response === 'block',
            timestamp: Date.now(),
          });
        } catch (err) {
          log.warn('onDogDecision callback error', { error: err.message });
        }
      }
    }

    // Create event message for inter-dog communication via eventBus
    const busEvent = new AgentEventMessage(
      eventType,
      'external:hook',
      {
        ...payload,
        hookType,
        userId,
        sessionId,
        receivedAt: Date.now(),
        // Include agent decisions for other dogs to learn from
        agentResults: agentResults.map(r => ({
          agent: r.agent,
          response: r.response,
          action: r.action,
        })),
        blocked,
        blockedBy,
      }
    );

    // Publish to eventBus for inter-dog learning
    let busResult = { delivered: 0, errors: [] };
    try {
      busResult = await this.eventBus.publish(busEvent);
    } catch (error) {
      log.error('eventBus publish error', { error: error.message });
    }

    // 🐕 Record dog decisions in graph (for relationship tracking)
    if (this.graphIntegration?.graph && agentResults.length > 0) {
      try {
        for (const result of agentResults) {
          if (result.response === 'block' || result.response === 'warn') {
            const graph = this.graphIntegration.graph;
            const dogName = result.agent?.toLowerCase() || 'unknown';
            const toolName = (payload.tool || 'unknown').toLowerCase();

            // Ensure dog and tool nodes exist before creating edge
            await graph.ensureNode?.('dog', dogName, { name: result.agent, role: 'agent' });
            await graph.ensureNode?.('tool', toolName, { name: payload.tool || 'unknown' });

            // Record significant dog decisions as graph edges
            await graph.connect?.(
              `dog:${dogName}`,
              `tool:${toolName}`,
              'judged',
              {
                response: result.response,
                hookType,
                action: result.action,
                message: result.message,
                timestamp: Date.now(),
              }
            );
          }
        }
      } catch (err) {
        // Non-critical: graph integration is for analytics only
        // Debug level to avoid log spam - core functionality unaffected
        if (process.env.CYNIC_DEBUG) {
          log.trace('graphIntegration', { error: err.message });
        }
      }
    }

    // Update CYNIC's observation count
    if (this.cynic) {
      this.cynic.stats.eventsObserved++;
    }

    return {
      success: true,
      eventId: busEvent.id,
      eventType,
      // Agent pipeline results
      agentResults,
      blocked,
      blockedBy,
      blockMessage,
      // EventBus delivery stats
      delivered: busResult.delivered,
      errors: busResult.errors?.length || 0,
    };
  }

  /**
   * Persist collective stats to storage
   * @returns {Promise<Object>} Result with success status
   */
  async persistStats() {
    if (!this.persistence) {
      return { success: false, error: 'No persistence available' };
    }

    try {
      const summary = this.getSummary();
      const stats = {
        type: 'collective_stats',
        timestamp: Date.now(),
        agentCount: summary.agentCount,
        collectiveStats: summary.collectiveStats,
        cynicState: summary.cynic?.state,
        // Individual dog stats
        dogs: {},
      };

      // Collect stats from each dog
      for (const agent of this.agents) {
        stats.dogs[agent.name] = {
          invocations: agent.stats?.invocations || 0,
          actions: agent.stats?.actions || 0,
          blocks: agent.stats?.blocks || 0,
          warnings: agent.stats?.warnings || 0,
          lastInvocation: agent.stats?.lastInvocation,
        };
      }

      await this.persistence.storeObservation(stats);
      return { success: true, timestamp: stats.timestamp };
    } catch (err) {
      log.warn('persistStats error', { error: err.message });
      return { success: false, error: err.message };
    }
  }

  /**
   * Clear all agent data
   */
  clear() {
    this.guardian.clear();
    this.analyst.clear();
    this.scholar.clear();
    this.architect.clear();
    this.sage.clear();
    this.cynic.clear();
    this.janitor.clear();
    this.scout.clear();
    this.cartographer.clear();
    this.oracle.clear();
    this.deployer.clear();
    this.eventBus.reset();
  }

  /**
   * Shutdown collective
   */
  async shutdown() {
    // Persist stats before shutdown
    await this.persistStats();
    await this.eventBus.shutdown();
    this.clear();
  }
}

/**
 * Create a collective pack with all Five Dogs
 * @param {Object} [options] - Options
 * @returns {CollectivePack} Collective pack
 */
export function createCollectivePack(options = {}) {
  return new CollectivePack(options);
}

/**
 * Create individual agents (for custom configurations)
 */
export function createGuardian(options = {}) {
  return new CollectiveGuardian(options);
}

export function createAnalyst(options = {}) {
  return new CollectiveAnalyst(options);
}

export function createScholar(options = {}) {
  return new CollectiveScholar(options);
}

export function createArchitect(options = {}) {
  return new CollectiveArchitect(options);
}

export function createSage(options = {}) {
  return new CollectiveSage(options);
}

export function createCynic(options = {}) {
  return new CollectiveCynic(options);
}

export function createJanitor(options = {}) {
  return new CollectiveJanitor(options);
}

export function createScout(options = {}) {
  return new CollectiveScout(options);
}

export function createCartographer(options = {}) {
  return new CollectiveCartographer(options);
}

export function createOracle(options = {}) {
  return new CollectiveOracle(options);
}

export function createDeployer(options = {}) {
  return new CollectiveDeployer(options);
}

export default {
  CollectivePack,
  createCollectivePack,
  // The Five Dogs
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  // The Hidden Dog (Keter)
  CollectiveCynic,
  // Additional Dogs (Sefirot)
  CollectiveJanitor,
  CollectiveScout,
  CollectiveCartographer,
  CollectiveOracle,
  CollectiveDeployer,
  // Factory functions
  createGuardian,
  createAnalyst,
  createScholar,
  createArchitect,
  createSage,
  createCynic,
  createJanitor,
  createScout,
  createCartographer,
  createOracle,
  createDeployer,
  // Constants
  COLLECTIVE_CONSTANTS,
  // Types
  RiskLevel,
  RiskCategory,
  PatternCategory,
  AnomalyType,
  KnowledgeType,
  ReviewCategory,
  FeedbackType,
  WisdomType,
  // CYNIC types
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
  // Janitor types
  JANITOR_CONSTANTS,
  QualitySeverity,
  IssueType,
  // Scout types
  SCOUT_CONSTANTS,
  DiscoveryType,
  OpportunityType,
  // Cartographer types
  CARTOGRAPHER_CONSTANTS,
  RepoType,
  ConnectionType,
  MapIssueType,
  // Oracle types
  ORACLE_CONSTANTS,
  ViewType,
  MetricType,
  AlertSeverity,
  // Deployer types
  DEPLOYER_CONSTANTS,
  DeploymentState,
  DeployTarget,
  HealthStatus,
};
