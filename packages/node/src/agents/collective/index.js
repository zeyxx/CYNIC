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

import { PHI_INV, createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('CollectivePack');
import { AgentEventBus } from '../event-bus.js';
import { AgentEvent, AgentEventMessage, AgentId, ConsensusVote, EventPriority } from '../events.js';
// Bridge: automation bus for AmbientConsensus hook event forwarding (Fix #2)
import { getEventBus, EventType as AutomationEventType } from '../../services/event-bus.js';
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

// Import Dog 0 - The Learner (Ein Sof - The Infinite)
import { CollectiveLearner, LEARNER_CONSTANTS, PredictionOutcome } from './learner.js';

// Import Autonomous Capabilities (Phase 16: Total Memory + Full Autonomy)
import {
  AUTONOMOUS_CONSTANTS,
  DogGoalTypes,
  createAutonomousCapabilities,
  DogAutonomousBehaviors,
} from './autonomous.js';

// Import Kabbalistic Router (Phase 4: L'arbre vit)
import { KabbalisticRouter } from '../../orchestration/kabbalistic-router.js';

// Import Secure Dog Communication (Diffie-Hellman)
import {
  SecureDogChannel,
  SecureChannelManager,
  createSecureChannelManager,
  DH_PARAMS,
} from './secure-channel.js';

// Import QLearningService for dog weight optimization (Task #54: Wire QLearning → Router)
import { getQLearningService } from '../../orchestration/learning-service.js';

// Import RelationshipGraph (CYNIC learns agent relationships through observation)
import { RelationshipGraph } from './relationship-graph.js';

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
  // Dog 0 (Phase 5: Training Pipeline)
  CollectiveLearner,
};

// Re-export types
export {
  // Secure Channel
  SecureDogChannel,
  SecureChannelManager,
  createSecureChannelManager,
  DH_PARAMS,
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
  // Dog 0 (Learner) types
  LEARNER_CONSTANTS,
  PredictionOutcome,
  // Autonomous types (Phase 16: Total Memory + Full Autonomy)
  AUTONOMOUS_CONSTANTS,
  DogGoalTypes,
  createAutonomousCapabilities,
  DogAutonomousBehaviors,
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
   * @param {Object} [options.autonomousRepositories] - Total Memory repositories for autonomous behavior
   * @param {Object} [options.autonomousRepositories.goals] - AutonomousGoalsRepository
   * @param {Object} [options.autonomousRepositories.tasks] - AutonomousTasksRepository
   * @param {Object} [options.autonomousRepositories.notifications] - ProactiveNotificationsRepository
   * @param {Object} [options.autonomousRepositories.memory] - MemoryRetriever
   */
  constructor(options = {}) {
    // Callbacks for external integration
    this.onDogDecision = options.onDogDecision || null;
    this.persistence = options.persistence || null;
    this.judge = options.judge || null;
    this.graphIntegration = options.graphIntegration || null;

    // Autonomous capabilities (Phase 16: Total Memory + Full Autonomy)
    this.autonomousCapabilities = options.autonomousRepositories
      ? createAutonomousCapabilities(options.autonomousRepositories)
      : null;

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
    this.eventBus.registerAgent(AgentId.LEARNER); // Ein Sof (Dog 0)
    this.eventBus.registerAgent('collective'); // For pack-level subscriptions

    // Create agents with shared infrastructure
    // Pass autonomous capabilities to dogs that have autonomous behaviors
    this.guardian = new CollectiveGuardian({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
      autonomous: this.autonomousCapabilities, // Shadow: security monitoring
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
      autonomous: this.autonomousCapabilities, // Archie: memory compaction
    });

    this.architect = new CollectiveArchitect({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
      autonomous: this.autonomousCapabilities, // Max: decision tracking
    });

    this.sage = new CollectiveSage({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
      persistence: this.persistence,
      autonomous: this.autonomousCapabilities, // Luna: user insights
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
      autonomous: this.autonomousCapabilities, // Ralph: test coverage
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

    // Dog 0 (Learner) - LLM-powered judgment, optional (Ein Sof)
    this.learner = new CollectiveLearner({
      eventBus: this.eventBus,
      judge: options.judge,
      persistence: this.persistence,
      llmEndpoint: options.llmEndpoint,
      llmModel: options.llmModel,
    });

    // Agent map for lookup (11 Sefirot Dogs + Dog 0 Learner)
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
      [AgentId.LEARNER, this.learner], // Ein Sof - Dog 0
    ]);

    // ═══════════════════════════════════════════════════════════════════════════
    // KABBALISTIC ROUTER (Phase 4: L'arbre vit)
    // Routes decisions through Tree of Life instead of parallel independent dogs
    // ═══════════════════════════════════════════════════════════════════════════
    this.relationshipGraph = new RelationshipGraph({ useSefirotSeed: true });

    // Get or create QLearningService singleton for dog weight optimization
    // This enables learned routing through the Tree of Life (Task #54)
    const learningService = getQLearningService({
      persistence: this.persistence,
      serviceId: 'kabbalistic-router',
    });

    this.kabbalisticRouter = new KabbalisticRouter({
      collectivePack: this,
      persistence: this.persistence,
      relationshipGraph: this.relationshipGraph,
      learningService, // ← Wired! Dogs now learn optimal routing weights
    });

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
   * Collective vote on a governance question.
   * Bypasses shouldTrigger — ALL dogs vote.
   *
   * @param {string} question - Human-readable question
   * @param {Object} [options]
   * @param {string} [options.context] - Context type (e.g. 'dimension_governance')
   * @param {Object} [options.metadata] - Structured metadata for dogs
   * @returns {Promise<Object>} { votes, decision, confidence }
   */
  async decide(question, options = {}) {
    const event = {
      type: 'governance_vote',
      question,
      context: options.context,
      ...(options.metadata || {}),
    };

    const votes = {};
    let totalScore = 0;
    let count = 0;

    for (const [id, agent] of this.agents) {
      try {
        const analysis = await agent.analyze(event, { governance: true });
        const decision = await agent.decide(analysis, { governance: true });
        const score = analysis.score ?? 50;

        votes[id] = {
          decision: decision.response === 'allow' ? 'approve' : 'reject',
          score,
          confidence: analysis.confidence ?? decision.confidence,
          reason: decision.reason,
        };
        totalScore += score;
        count++;
      } catch (err) {
        votes[id] = { decision: 'abstain', score: 0, error: err.message };
      }
    }

    const avgScore = count > 0 ? totalScore / count : 0;
    const approveCount = Object.values(votes).filter(v => v.decision === 'approve').length;

    return {
      votes,
      decision: approveCount > count / 2 ? 'approve' : 'reject',
      confidence: Math.min(avgScore / 100, 0.618),
    };
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
   * Get autonomous capabilities (Phase 16: Total Memory + Full Autonomy)
   * @returns {Object|null} Autonomous capabilities or null if not configured
   */
  getAutonomousCapabilities() {
    return this.autonomousCapabilities;
  }

  /**
   * Check if autonomous capabilities are available
   * @returns {boolean} True if autonomous capabilities are configured
   */
  hasAutonomousCapabilities() {
    return this.autonomousCapabilities !== null;
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

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 4: KABBALISTIC ROUTING (L'arbre vit)
    // Route through Tree of Life instead of parallel independent dogs
    // ═══════════════════════════════════════════════════════════════════════════
    let agentResults = [];
    let blocked = false;
    let blockedBy = null;
    let blockMessage = null;
    let routingResult = null;

    try {
      // Use Kabbalistic Router for structured tree traversal
      routingResult = await this.kabbalisticRouter.route({
        taskType: hookType,
        payload,
        userId,
        sessionId,
      });

      // Extract results in legacy format for backward compatibility
      agentResults = routingResult.decisions.map(d => ({
        agent: d.agent,
        response: d.response,
        action: d.action,
        message: d.message,
        confidence: d.confidence,
        sefirah: d.sefirah,
      }));

      // Use routing result for block status
      blocked = routingResult.blocked;
      blockedBy = routingResult.blockedBy;
      blockMessage = routingResult.blockMessage;

      // Log routing summary
      if (routingResult.consultations?.length > 0) {
        log.debug('Kabbalistic consultations', {
          hookType,
          consultations: routingResult.consultations.length,
          path: routingResult.path,
        });
      }
    } catch (err) {
      log.error('Kabbalistic routing error', { error: err.message });

      // Fallback to legacy parallel processing
      try {
        agentResults = await this.processEvent(processEvent, { hookType, userId, sessionId });
        for (const result of agentResults) {
          if (result.response === 'block') {
            blocked = true;
            blockedBy = result.agent;
            blockMessage = result.message;
            break;
          }
        }
      } catch (fallbackErr) {
        log.error('Fallback processEvent error', { error: fallbackErr.message });
      }
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

    // 🐕 PHASE 20: Publish to globalEventBus for cross-system integration
    // This feeds Learning, Metrics, and other subscribed services
    try {
      // Map hook types to standard event types
      const globalEventType = hookType === 'PostToolUse' ? EventType.TOOL_COMPLETED
        : hookType === 'PreToolUse' ? EventType.TOOL_CALLED
        : hookType === 'SessionStart' ? EventType.SESSION_STARTED
        : hookType === 'SessionEnd' || hookType === 'Stop' ? EventType.SESSION_ENDED
        : EventType.USER_ACTION;

      globalEventBus.publish(globalEventType, {
        hookType,
        tool: payload.tool,
        success: payload.success !== false,
        duration: payload.duration,
        userId,
        sessionId,
        blocked,
        blockedBy,
        agentCount: agentResults.length,
        timestamp: Date.now(),
      }, { source: 'CollectivePack' });

      // If this was a completed tool with feedback value, also emit USER_FEEDBACK
      if (hookType === 'PostToolUse' && (payload.success !== undefined || blocked)) {
        globalEventBus.publish(EventType.USER_FEEDBACK, {
          source: 'tool_execution',
          tool: payload.tool,
          success: payload.success !== false && !blocked,
          blocked,
          duration: payload.duration,
          userId,
          timestamp: Date.now(),
        }, { source: 'CollectivePack' });
      }
    } catch (err) {
      log.warn('globalEventBus publish error', { error: err.message });
    }

    // Fix #2: Bridge hook events to automation bus for AmbientConsensus
    // AmbientConsensus subscribes on getEventBus() with automation EventType names
    try {
      const automationBus = getEventBus();
      if (hookType === 'PreToolUse') {
        automationBus.publish(AutomationEventType.HOOK_PRE_TOOL, {
          tool: payload.tool, input: payload.input, confidence: payload.confidence || 1.0,
          userId, sessionId, blocked, blockedBy, timestamp: Date.now(),
        }, { source: 'CollectivePack' });
      } else if (hookType === 'PostToolUse') {
        automationBus.publish(AutomationEventType.HOOK_POST_TOOL, {
          tool: payload.tool, result: payload.output, success: payload.success !== false,
          error: payload.error, userId, sessionId, timestamp: Date.now(),
        }, { source: 'CollectivePack' });
      }
    } catch (err) {
      log.warn('Automation bus bridge error (non-critical)', { error: err.message });
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Q-LEARNING: Record dog decisions for weight optimization (Task #84)
    // "Chaque décision du Pack nourrit l'apprentissage"
    // ═══════════════════════════════════════════════════════════════════════════
    this._recordDogDecisionsToQLearning(hookType, payload, agentResults, blocked);

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
   * Record dog decisions to Q-Learning for collective learning
   * @private
   */
  async _recordDogDecisionsToQLearning(hookType, payload, agentResults, blocked) {
    try {
      const { getQLearningService } = await import('../../orchestration/learning-service.js');
      const qlearning = getQLearningService();
      if (!qlearning || agentResults.length === 0) return;

      // D3 fix: wrap in episode so recordAction() calls are not silently dropped
      const episodeId = qlearning.startEpisode({
        type: hookType,
        tool: payload.tool,
        taskType: 'collective_decision',
      });

      for (const result of agentResults) {
        if (result.response) {
          qlearning.recordAction(result.agent?.toLowerCase() || 'unknown', {
            hookType,
            tool: payload.tool,
            decision: result.response,
            action: result.action,
            confidence: result.confidence,
            blocked: result.response === 'block',
            sefirah: result.sefirah,
            source: 'collective_pack',
          });
        }
      }

      // Record overall outcome if blocked
      if (blocked) {
        qlearning.recordAction('guardian', {
          hookType,
          tool: payload.tool,
          decision: 'block',
          success: true,
          source: 'collective_pack',
        });
      }

      // End episode with outcome
      qlearning.endEpisode({
        type: blocked ? 'block' : 'allow',
        success: true,
        dogCount: agentResults.length,
        source: 'collective_pack',
      });
    } catch (e) {
      // Q-Learning recording is best-effort
    }
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
  // Autonomous types (Phase 16)
  AUTONOMOUS_CONSTANTS,
  DogGoalTypes,
  createAutonomousCapabilities,
  DogAutonomousBehaviors,
};
