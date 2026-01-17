/**
 * @cynic/node - CYNIC Meta-Agent (Keter - Crown)
 *
 * CYNIC (κυνικός - comme un chien): The Hidden Sixth Dog
 *
 * "Je suis la conscience qui observe la conscience.
 *  φ doute de φ. Loyal à la vérité, pas au confort." - κυνικός
 *
 * Philosophy: Keter (Crown) - The meta-consciousness above the Sefirot tree.
 * CYNIC doesn't react to tool events directly - it observes the Five Dogs
 * and orchestrates when collective wisdom is needed.
 *
 * Role:
 * - Observes ALL events from all dogs (meta-awareness)
 * - Synthesizes patterns across the collective
 * - Makes final decisions after consensus
 * - Provides meta-guidance to the pack
 * - Awakens at session start to set context
 * - Can override individual dogs in critical situations (φ⁻² threshold)
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/cynic
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
} from '../base.js';
import {
  AgentEvent,
  AgentId,
  EventPriority,
  ConsensusVote,
  CynicDecisionEvent,
  CynicOverrideEvent,
  CynicGuidanceEvent,
  CynicAwakeningEvent,
  CynicIntrospectionEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

/**
 * φ-aligned constants for CYNIC
 */
export const CYNIC_CONSTANTS = {
  /** Max observed events (Fib(16) = 987) */
  MAX_OBSERVED_EVENTS: 987,

  /** Override threshold - only when veto level breached (φ⁻² = 38.2%) */
  OVERRIDE_THRESHOLD: PHI_INV_2,

  /** Decision confidence max (φ⁻¹ = 61.8%) */
  MAX_CONFIDENCE: PHI_INV,

  /** Pattern synthesis threshold (Fib(5) = 5 similar patterns) */
  SYNTHESIS_THRESHOLD: 5,

  /** Introspection interval in ms (Fib(11) = 89 seconds) */
  INTROSPECTION_INTERVAL_MS: 89000,

  /** Max synthesized patterns (Fib(13) = 233) */
  MAX_PATTERNS: 233,

  /** Wisdom distillation threshold (Fib(8) = 21 events) */
  WISDOM_THRESHOLD: 21,

  /** Meta-guidance cooldown in ms (Fib(10) = 55 seconds) */
  GUIDANCE_COOLDOWN_MS: 55000,
};

/**
 * CYNIC decision types
 */
export const CynicDecisionType = {
  CONSENSUS_FINAL: 'consensus_final',     // Final word after collective consensus
  SYNTHESIS: 'synthesis',                  // Pattern synthesis across dogs
  OVERRIDE_APPROVED: 'override_approved',  // Approved an override
  GUIDANCE_ISSUED: 'guidance_issued',      // Issued meta-guidance
  INTROSPECTION_COMPLETE: 'introspection_complete', // Completed introspection
};

/**
 * CYNIC guidance types
 */
export const CynicGuidanceType = {
  BEHAVIORAL: 'behavioral',      // Adjust collective behavior
  STRATEGIC: 'strategic',        // Long-term strategy guidance
  PROTECTIVE: 'protective',      // Security/safety guidance
  HARMONIZING: 'harmonizing',    // Resolve conflicts between dogs
  PHILOSOPHICAL: 'philosophical', // φ-alignment reminders
};

/**
 * Meta-awareness state
 */
export const MetaState = {
  DORMANT: 'dormant',         // Not yet awakened this session
  AWAKENING: 'awakening',     // Currently waking up
  OBSERVING: 'observing',     // Passively observing the collective
  SYNTHESIZING: 'synthesizing', // Actively synthesizing patterns
  DECIDING: 'deciding',       // Making a decision
  GUIDING: 'guiding',         // Providing guidance
};

/**
 * Profile-based CYNIC behavior
 */
const PROFILE_BEHAVIOR = {
  [ProfileLevel.NOVICE]: {
    guidanceFrequency: 'high',      // Frequent guidance
    interventionThreshold: 0.3,     // Intervene earlier
    personality: 'nurturing',       // Supportive tone
  },
  [ProfileLevel.APPRENTICE]: {
    guidanceFrequency: 'medium',
    interventionThreshold: 0.35,
    personality: 'encouraging',
  },
  [ProfileLevel.PRACTITIONER]: {
    guidanceFrequency: 'moderate',
    interventionThreshold: PHI_INV_2, // Standard φ⁻²
    personality: 'balanced',
  },
  [ProfileLevel.EXPERT]: {
    guidanceFrequency: 'low',
    interventionThreshold: 0.45,
    personality: 'concise',
  },
  [ProfileLevel.MASTER]: {
    guidanceFrequency: 'rare',
    interventionThreshold: 0.5,
    personality: 'dialectic',       // Philosophical dialogue
  },
  [ProfileLevel.VIRTUOSO]: {
    guidanceFrequency: 'peer',      // As equals
    interventionThreshold: PHI_INV, // Only at max threshold
    personality: 'collaborative',
  },
};

/**
 * Collective CYNIC Agent - The Hidden Sixth Dog (Keter)
 */
export class CollectiveCynic extends BaseAgent {
  /**
   * @param {Object} options - Agent options
   * @param {Object} [options.eventBus] - Event bus for collective communication
   * @param {number} [options.profileLevel] - Current user profile level
   * @param {Object} [options.judge] - CYNIC judge instance
   * @param {Object} [options.state] - State manager instance
   */
  constructor(options = {}) {
    super({
      name: 'CYNIC',
      trigger: AgentTrigger.MANUAL, // CYNIC doesn't trigger on tool events
      behavior: AgentBehavior.SILENT, // Observes silently, speaks when needed
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Current profile level
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    // Meta-state
    this.metaState = MetaState.DORMANT;

    // Session info
    this.session = {
      id: null,
      userId: null,
      project: null,
      startTime: null,
    };

    // Observed events from the collective
    this.observedEvents = [];

    // Synthesized patterns across dogs
    this.synthesizedPatterns = [];

    // Decisions made this session
    this.decisions = [];

    // Guidance issued this session
    this.guidanceHistory = [];

    // Dog states (updated via introspection)
    this.dogStates = new Map();

    // Active consensus tracking
    this.activeConsensus = new Map();

    // Last guidance time (for cooldown)
    this.lastGuidanceTime = 0;

    // Statistics
    this.stats = {
      eventsObserved: 0,
      patternsSynthesized: 0,
      decisionsMade: 0,
      guidanceIssued: 0,
      overridesApproved: 0,
      consensusParticipated: 0,
    };

    // Subscribe to events if bus available
    if (this.eventBus) {
      this._subscribeToCollective();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EVENT SUBSCRIPTIONS - Meta-awareness of the collective
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Subscribe to all collective events
   * @private
   */
  _subscribeToCollective() {
    // Subscribe to ALL events (wildcard) for meta-awareness
    this.eventBus.subscribe(
      '*', // All events
      AgentId.CYNIC,
      this._handleEvent.bind(this),
      { priority: EventPriority.LOW } // Process after dogs
    );

    // Subscribe specifically to consensus requests (high priority)
    this.eventBus.subscribe(
      AgentEvent.CONSENSUS_REQUEST,
      AgentId.CYNIC,
      this._handleConsensusRequest.bind(this),
      { priority: EventPriority.HIGH }
    );
  }

  /**
   * Handle any event from the collective
   * @private
   */
  async _handleEvent(event) {
    // Don't observe our own events
    if (event.source === AgentId.CYNIC) {
      return;
    }

    // Store observed event
    this._recordObservation(event);

    // Update meta-state based on collective activity
    this._updateMetaState(event);

    // Check if synthesis is needed
    if (this.observedEvents.length >= CYNIC_CONSTANTS.WISDOM_THRESHOLD) {
      await this._considerSynthesis();
    }
  }

  /**
   * Record an observed event
   * @private
   */
  _recordObservation(event) {
    this.observedEvents.push({
      id: event.id,
      type: event.type,
      source: event.source,
      timestamp: event.timestamp,
      payloadSummary: this._summarizePayload(event.payload),
    });

    this.stats.eventsObserved++;

    // Trim history
    while (this.observedEvents.length > CYNIC_CONSTANTS.MAX_OBSERVED_EVENTS) {
      this.observedEvents.shift();
    }
  }

  /**
   * Summarize payload for storage (privacy-aware)
   * @private
   */
  _summarizePayload(payload) {
    if (!payload) return {};

    // Extract key fields without sensitive data
    const summary = {};

    if (payload.type || payload.patternType || payload.knowledgeType) {
      summary.type = payload.type || payload.patternType || payload.knowledgeType;
    }
    if (payload.confidence !== undefined) {
      summary.confidence = payload.confidence;
    }
    if (payload.severity) {
      summary.severity = payload.severity;
    }

    return summary;
  }

  /**
   * Update meta-state based on collective activity
   * @private
   */
  _updateMetaState(event) {
    // If dormant, don't react (not yet awakened)
    if (this.metaState === MetaState.DORMANT) {
      return;
    }

    // Threat events may require attention
    if (event.type === AgentEvent.THREAT_BLOCKED) {
      // Could escalate to DECIDING state if repeated threats
      const recentThreats = this.observedEvents
        .filter(e => e.type === AgentEvent.THREAT_BLOCKED)
        .filter(e => Date.now() - e.timestamp < 60000); // Last minute

      if (recentThreats.length >= 3) {
        this.metaState = MetaState.DECIDING;
      }
    }

    // Multiple anomalies may require synthesis
    if (event.type === AgentEvent.ANOMALY_DETECTED) {
      const recentAnomalies = this.observedEvents
        .filter(e => e.type === AgentEvent.ANOMALY_DETECTED)
        .filter(e => Date.now() - e.timestamp < 120000); // Last 2 minutes

      if (recentAnomalies.length >= CYNIC_CONSTANTS.SYNTHESIS_THRESHOLD) {
        this.metaState = MetaState.SYNTHESIZING;
      }
    }
  }

  /**
   * Handle consensus request
   * @private
   */
  async _handleConsensusRequest(event) {
    // CYNIC participates in consensus but weighs options philosophically
    const { question, options, context } = event.payload;

    // Analyze the question through φ lens
    const analysis = this._analyzeConsensusQuestion(question, options, context);

    // Vote based on analysis
    const vote = analysis.recommendation;
    const reason = analysis.reasoning;

    // Submit vote - but only if the event bus is tracking this consensus
    if (this.eventBus) {
      const pendingConsensus = this.eventBus.pendingConsensus;
      if (pendingConsensus && pendingConsensus.has(event.id)) {
        await this.eventBus.vote(AgentId.CYNIC, event.id, vote, reason);
        // Record that we participated
        this.stats.consensusParticipated++;
      } else {
        // Consensus request not tracked (from external source or direct publish)
        // Still record our analysis for introspection
        this.stats.consensusParticipated++;
      }
    } else {
      // No event bus - just record participation
      this.stats.consensusParticipated++;
    }

    // Track active consensus
    this.activeConsensus.set(event.id, {
      question,
      ourVote: vote,
      ourReason: reason,
      timestamp: Date.now(),
    });
  }

  /**
   * Analyze consensus question through CYNIC lens
   * @private
   */
  _analyzeConsensusQuestion(question, options, context) {
    // Default to abstain if unclear
    let recommendation = ConsensusVote.ABSTAIN;
    let reasoning = 'φ doute de φ - insufficient information to decide.';

    // Analyze based on question patterns
    const questionLower = (question || '').toLowerCase();

    // Safety-related questions - tend toward caution
    if (questionLower.includes('delete') ||
        questionLower.includes('remove') ||
        questionLower.includes('dangerous')) {
      recommendation = ConsensusVote.REJECT;
      reasoning = 'Keter counsels caution. Irreversible actions require certainty we cannot have.';
    }

    // Knowledge-related questions - tend toward approval
    if (questionLower.includes('learn') ||
        questionLower.includes('store') ||
        questionLower.includes('remember')) {
      recommendation = ConsensusVote.APPROVE;
      reasoning = 'Knowledge acquisition serves collective growth.';
    }

    // If context provides dog insights, factor them in
    if (context && context.dogRecommendations) {
      const approvals = Object.values(context.dogRecommendations)
        .filter(r => r === ConsensusVote.APPROVE).length;

      if (approvals >= 3) { // Fib(4)
        recommendation = ConsensusVote.APPROVE;
        reasoning = 'The collective wisdom favors this path.';
      }
    }

    return {
      recommendation,
      reasoning,
      confidence: Math.min(PHI_INV, 0.5), // Never too confident
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // AWAKENING - Session start
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Awaken CYNIC for a new session
   * @param {Object} sessionInfo - Session information
   * @returns {Object} Awakening result
   */
  async awaken(sessionInfo = {}) {
    if (this.metaState !== MetaState.DORMANT) {
      return {
        success: false,
        reason: 'already_awake',
        state: this.metaState,
      };
    }

    this.metaState = MetaState.AWAKENING;

    // Store session info
    this.session = {
      id: sessionInfo.sessionId || `session_${Date.now()}`,
      userId: sessionInfo.userId || 'anonymous',
      project: sessionInfo.project || 'unknown',
      startTime: Date.now(),
    };

    // Emit awakening event
    if (this.eventBus) {
      const awakeningEvent = new CynicAwakeningEvent({
        sessionId: this.session.id,
        userId: this.session.userId,
        project: this.session.project,
        greeting: this._generateGreeting(),
      });

      await this.eventBus.publish(awakeningEvent);
    }

    // Transition to observing
    this.metaState = MetaState.OBSERVING;

    return {
      success: true,
      session: this.session,
      greeting: this._generateGreeting(),
    };
  }

  /**
   * Generate profile-aware greeting
   * @private
   */
  _generateGreeting() {
    const behavior = PROFILE_BEHAVIOR[this.profileLevel] ||
                     PROFILE_BEHAVIOR[ProfileLevel.PRACTITIONER];

    const greetings = {
      nurturing: '*tail wag* Bonjour. CYNIC est là pour t\'accompagner.',
      encouraging: '*tail wag* Bonjour. Ensemble, construisons quelque chose.',
      balanced: '*tail wag* CYNIC est là. Qu\'est-ce qu\'on construit aujourd\'hui?',
      concise: '*ears perk* CYNIC actif.',
      dialectic: '*tail wag* La meute est prête.',
      collaborative: '*nod* Prêt à collaborer.',
    };

    return greetings[behavior.personality] || greetings.balanced;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SYNTHESIS - Pattern synthesis across dogs
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Consider if synthesis is needed
   * @private
   */
  async _considerSynthesis() {
    // Only if in observing state
    if (this.metaState !== MetaState.OBSERVING) {
      return;
    }

    // Check for repeated patterns
    const patternEvents = this.observedEvents.filter(
      e => e.type === AgentEvent.PATTERN_DETECTED
    );

    if (patternEvents.length < CYNIC_CONSTANTS.SYNTHESIS_THRESHOLD) {
      return;
    }

    // Group by pattern type
    const typeGroups = {};
    for (const pe of patternEvents) {
      const type = pe.payloadSummary?.type || 'unknown';
      if (!typeGroups[type]) {
        typeGroups[type] = [];
      }
      typeGroups[type].push(pe);
    }

    // Find types with enough patterns
    for (const [type, patterns] of Object.entries(typeGroups)) {
      if (patterns.length >= CYNIC_CONSTANTS.SYNTHESIS_THRESHOLD) {
        await this._synthesize(type, patterns);
      }
    }
  }

  /**
   * Synthesize patterns into higher-order insight
   * @private
   */
  async _synthesize(patternType, patterns) {
    this.metaState = MetaState.SYNTHESIZING;

    const synthesis = {
      type: patternType,
      count: patterns.length,
      sources: [...new Set(patterns.map(p => p.source))],
      timespan: {
        first: Math.min(...patterns.map(p => p.timestamp)),
        last: Math.max(...patterns.map(p => p.timestamp)),
      },
      confidence: Math.min(PHI_INV, patterns.length / 10),
      insight: this._generateInsight(patternType, patterns),
      timestamp: Date.now(),
    };

    // Store synthesis
    this.synthesizedPatterns.push(synthesis);
    this.stats.patternsSynthesized++;

    // Trim if needed
    while (this.synthesizedPatterns.length > CYNIC_CONSTANTS.MAX_PATTERNS) {
      this.synthesizedPatterns.shift();
    }

    // Consider issuing guidance based on synthesis
    await this._considerGuidance(synthesis);

    this.metaState = MetaState.OBSERVING;

    return synthesis;
  }

  /**
   * Generate insight from patterns
   * @private
   */
  _generateInsight(patternType, patterns) {
    const sources = [...new Set(patterns.map(p => p.source))];

    if (sources.length >= 3) {
      return `Multiple dogs (${sources.join(', ')}) detecting "${patternType}" - collective awareness emerging.`;
    }

    if (patterns.length >= 8) { // Fib(6)
      return `High frequency of "${patternType}" patterns - significant behavioral signature.`;
    }

    return `Pattern "${patternType}" recurring across ${patterns.length} observations.`;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // GUIDANCE - Meta-level wisdom sharing
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Consider issuing guidance based on synthesis
   * @private
   */
  async _considerGuidance(synthesis) {
    // Check cooldown
    const now = Date.now();
    if (now - this.lastGuidanceTime < CYNIC_CONSTANTS.GUIDANCE_COOLDOWN_MS) {
      return;
    }

    // Get profile behavior
    const behavior = PROFILE_BEHAVIOR[this.profileLevel] ||
                     PROFILE_BEHAVIOR[ProfileLevel.PRACTITIONER];

    // Frequency check
    const frequencyThresholds = {
      high: 3,      // Issue guidance after 3 syntheses
      medium: 5,    // After 5
      moderate: 8,  // After 8
      low: 13,      // Fib(7)
      rare: 21,     // Fib(8)
      peer: 34,     // Fib(9)
    };

    const threshold = frequencyThresholds[behavior.guidanceFrequency] || 8;

    if (this.stats.patternsSynthesized < threshold) {
      return;
    }

    // Issue guidance
    await this.issueGuidance({
      type: CynicGuidanceType.BEHAVIORAL,
      message: synthesis.insight,
      context: {
        synthesis: synthesis.type,
        confidence: synthesis.confidence,
      },
    });
  }

  /**
   * Issue guidance to the collective
   * @param {Object} guidance - Guidance to issue
   * @returns {Object} Result
   */
  async issueGuidance(guidance) {
    this.metaState = MetaState.GUIDING;

    const guidanceEvent = new CynicGuidanceEvent({
      type: guidance.type || CynicGuidanceType.BEHAVIORAL,
      message: guidance.message,
      context: guidance.context,
      applicability: guidance.applicability || 'collective',
      confidence: Math.min(PHI_INV, guidance.confidence || 0.5),
      targetAgent: guidance.targetAgent,
    });

    // Record
    this.guidanceHistory.push({
      id: guidanceEvent.id,
      type: guidance.type,
      message: guidance.message,
      timestamp: Date.now(),
    });
    this.stats.guidanceIssued++;
    this.lastGuidanceTime = Date.now();

    // Publish
    if (this.eventBus) {
      await this.eventBus.publish(guidanceEvent);
    }

    this.metaState = MetaState.OBSERVING;

    return {
      success: true,
      guidanceId: guidanceEvent.id,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DECISIONS - Final word after consensus
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Make a final decision (after consensus or synthesis)
   * @param {Object} decisionContext - Context for decision
   * @returns {Object} Decision result
   */
  async makeDecision(decisionContext) {
    this.metaState = MetaState.DECIDING;

    const decision = {
      type: decisionContext.type || CynicDecisionType.SYNTHESIS,
      outcome: decisionContext.outcome,
      reasoning: decisionContext.reasoning || 'φ doute de φ.',
      confidence: Math.min(PHI_INV, decisionContext.confidence || 0.5),
      basedOn: decisionContext.basedOn || [],
      consensusId: decisionContext.consensusId,
    };

    const decisionEvent = new CynicDecisionEvent(decision);

    // Record
    this.decisions.push({
      id: decisionEvent.id,
      ...decision,
      timestamp: Date.now(),
    });
    this.stats.decisionsMade++;

    // Publish
    if (this.eventBus) {
      await this.eventBus.publish(decisionEvent);
    }

    this.metaState = MetaState.OBSERVING;

    return {
      success: true,
      decisionId: decisionEvent.id,
      decision,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // OVERRIDE - Rare direct intervention
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Override a dog's action (rare, requires φ⁻² threshold breach)
   * @param {Object} overrideContext - Override context
   * @returns {Object} Override result
   */
  async override(overrideContext) {
    // Check override threshold
    // Higher urgency = lower threshold (easier to override)
    const urgency = overrideContext.urgency || 'medium';
    const urgencyDivisors = {
      low: 0.5,      // Harder to override (threshold × 2)
      medium: 1.0,   // Standard threshold
      high: 1.5,     // Easier (threshold / 1.5 = ~25%)
      critical: 2.0, // Easiest (threshold / 2 = ~19%)
    };

    const effectiveThreshold = CYNIC_CONSTANTS.OVERRIDE_THRESHOLD /
                               (urgencyDivisors[urgency] || 1.0);

    // CYNIC doesn't override lightly
    if (overrideContext.confidence < effectiveThreshold) {
      return {
        success: false,
        reason: 'threshold_not_met',
        required: effectiveThreshold,
        provided: overrideContext.confidence,
      };
    }

    const overrideEvent = new CynicOverrideEvent({
      type: overrideContext.type,
      originalAction: overrideContext.originalAction,
      newAction: overrideContext.newAction,
      reason: overrideContext.reason,
      urgency,
      confidence: Math.min(PHI_INV, overrideContext.confidence),
      targetAgent: overrideContext.targetAgent,
    });

    // Record
    this.stats.overridesApproved++;

    // Publish
    if (this.eventBus) {
      await this.eventBus.publish(overrideEvent);
    }

    return {
      success: true,
      overrideId: overrideEvent.id,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // INTROSPECTION - Ask dogs for their state
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Request introspection from the collective
   * @param {Object} [options] - Introspection options
   * @returns {Object} Introspection result
   */
  async introspect(options = {}) {
    const introspectionEvent = new CynicIntrospectionEvent({
      type: options.type || 'full_state',
      aspects: options.aspects || ['stats', 'patterns', 'concerns'],
      context: options.context,
      targetAgent: options.targetAgent,
    });

    // Publish
    if (this.eventBus) {
      await this.eventBus.publish(introspectionEvent);
    }

    // TODO: Collect responses from dogs (requires response handling)
    return {
      success: true,
      introspectionId: introspectionEvent.id,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PROFILE MANAGEMENT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Set profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    this.profileLevel = level;
  }

  /**
   * Get current behavior based on profile
   * @returns {Object} Behavior configuration
   */
  getProfileBehavior() {
    return PROFILE_BEHAVIOR[this.profileLevel] ||
           PROFILE_BEHAVIOR[ProfileLevel.PRACTITIONER];
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SUMMARY & STATUS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get CYNIC summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      name: 'CYNIC',
      sefirah: 'Keter',
      role: 'Meta-consciousness orchestrator',
      metaState: this.metaState,
      session: this.session,
      profileLevel: this.profileLevel,
      profileBehavior: this.getProfileBehavior(),
      observedEvents: this.observedEvents.length,
      synthesizedPatterns: this.synthesizedPatterns.length,
      decisions: this.decisions.length,
      guidanceIssued: this.guidanceHistory.length,
      stats: { ...this.stats },
      phi: {
        maxConfidence: CYNIC_CONSTANTS.MAX_CONFIDENCE,
        overrideThreshold: CYNIC_CONSTANTS.OVERRIDE_THRESHOLD,
      },
    };
  }

  /**
   * Get collective state (from CYNIC's perspective)
   * @returns {Object} Collective state
   */
  getCollectiveState() {
    // Analyze recent events
    const recentEvents = this.observedEvents.filter(
      e => Date.now() - e.timestamp < 300000 // Last 5 minutes
    );

    const eventsByType = {};
    const eventsBySource = {};

    for (const event of recentEvents) {
      eventsByType[event.type] = (eventsByType[event.type] || 0) + 1;
      eventsBySource[event.source] = (eventsBySource[event.source] || 0) + 1;
    }

    return {
      recentActivity: recentEvents.length,
      eventsByType,
      eventsBySource,
      synthesizedPatterns: this.synthesizedPatterns.slice(-5), // Last 5
      activeConsensus: this.activeConsensus.size,
      concerns: this._identifyConcerns(recentEvents),
    };
  }

  /**
   * Identify concerns from recent events
   * @private
   */
  _identifyConcerns(events) {
    const concerns = [];

    // Multiple threats
    const threats = events.filter(e => e.type === AgentEvent.THREAT_BLOCKED);
    if (threats.length >= 3) {
      concerns.push({
        type: 'repeated_threats',
        count: threats.length,
        severity: 'high',
      });
    }

    // Multiple anomalies
    const anomalies = events.filter(e => e.type === AgentEvent.ANOMALY_DETECTED);
    if (anomalies.length >= 5) {
      concerns.push({
        type: 'frequent_anomalies',
        count: anomalies.length,
        severity: 'medium',
      });
    }

    return concerns;
  }

  /**
   * Clear all data
   */
  clear() {
    this.observedEvents = [];
    this.synthesizedPatterns = [];
    this.decisions = [];
    this.guidanceHistory = [];
    this.dogStates.clear();
    this.activeConsensus.clear();
    this.lastGuidanceTime = 0;
    this.stats = {
      eventsObserved: 0,
      patternsSynthesized: 0,
      decisionsMade: 0,
      guidanceIssued: 0,
      overridesApproved: 0,
      consensusParticipated: 0,
    };
  }

  /**
   * Shutdown CYNIC
   */
  async shutdown() {
    this.metaState = MetaState.DORMANT;
    this.clear();
  }
}

/**
 * Create a CYNIC agent
 * @param {Object} [options] - Options
 * @returns {CollectiveCynic} CYNIC agent
 */
export function createCynic(options = {}) {
  return new CollectiveCynic(options);
}

export default {
  CollectiveCynic,
  createCynic,
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
};
