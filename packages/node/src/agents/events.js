/**
 * @cynic/node - Agent Events
 *
 * Event types for inter-agent communication in the collective.
 * 8 event types (Fib(6)) for the Five Dogs to coordinate.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/events
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * φ-aligned constants for event system
 */
export const EVENT_CONSTANTS = {
  /** Max event history (Fib(16) = 987) */
  MAX_HISTORY: 987,

  /** Max pending consensus requests (Fib(11) = 89) */
  MAX_PENDING_CONSENSUS: 89,

  /** Consensus threshold (φ⁻¹ = 61.8%) */
  CONSENSUS_THRESHOLD: PHI_INV,

  /** Veto threshold (φ⁻² = 38.2%) - minority can block */
  VETO_THRESHOLD: PHI_INV_2,

  /** Consensus timeout in ms (Fib(13) = 233 × 100 = 23.3s) */
  CONSENSUS_TIMEOUT_MS: 23300,

  /** Event TTL in ms (Fib(10) = 55 × 1000 = 55s) */
  EVENT_TTL_MS: 55000,

  /** Max event payload size (Fib(14) = 377 KB) */
  MAX_PAYLOAD_SIZE: 377 * 1024,
};

/**
 * Event types for inter-agent communication
 * 8 types = Fib(6)
 */
export const AgentEvent = {
  // ═══════════════════════════════════════════════════════════════════════════
  // OBSERVATION EVENTS (Analyst → Others)
  // ═══════════════════════════════════════════════════════════════════════════

  /** Pattern detected in user behavior or code */
  PATTERN_DETECTED: 'agent:pattern:detected',

  /** Anomaly or potential issue detected */
  ANOMALY_DETECTED: 'agent:anomaly:detected',

  // ═══════════════════════════════════════════════════════════════════════════
  // PROTECTION EVENTS (Guardian → Others)
  // ═══════════════════════════════════════════════════════════════════════════

  /** Threat blocked by Guardian */
  THREAT_BLOCKED: 'agent:threat:blocked',

  // ═══════════════════════════════════════════════════════════════════════════
  // KNOWLEDGE EVENTS (Scholar → Others)
  // ═══════════════════════════════════════════════════════════════════════════

  /** Knowledge extracted from content */
  KNOWLEDGE_EXTRACTED: 'agent:knowledge:extracted',

  // ═══════════════════════════════════════════════════════════════════════════
  // WISDOM EVENTS (Sage → Others)
  // ═══════════════════════════════════════════════════════════════════════════

  /** Wisdom shared with collective */
  WISDOM_SHARED: 'agent:wisdom:shared',

  // ═══════════════════════════════════════════════════════════════════════════
  // CONSENSUS EVENTS (Any → All)
  // ═══════════════════════════════════════════════════════════════════════════

  /** Request consensus from collective */
  CONSENSUS_REQUEST: 'agent:consensus:request',

  /** Response to consensus request */
  CONSENSUS_RESPONSE: 'agent:consensus:response',

  // ═══════════════════════════════════════════════════════════════════════════
  // PROFILE EVENTS (Analyst → All)
  // ═══════════════════════════════════════════════════════════════════════════

  /** User profile level updated */
  PROFILE_UPDATED: 'agent:profile:updated',

  // ═══════════════════════════════════════════════════════════════════════════
  // CYNIC EVENTS (CYNIC ↔ All) - Meta-consciousness coordination
  // ═══════════════════════════════════════════════════════════════════════════

  /** CYNIC final decision after observing consensus or synthesizing dog outputs */
  CYNIC_DECISION: 'cynic:decision',

  /** CYNIC direct intervention - rare, requires φ⁻² (38.2%) threshold breach */
  CYNIC_OVERRIDE: 'cynic:override',

  /** CYNIC meta-guidance to the collective - wisdom above wisdom */
  CYNIC_GUIDANCE: 'cynic:guidance',

  /** CYNIC awakening - emitted when CYNIC becomes active in a session */
  CYNIC_AWAKENING: 'cynic:awakening',

  /** CYNIC introspection request - asks dogs for their current state */
  CYNIC_INTROSPECTION: 'cynic:introspection',
};

/**
 * Event priority levels
 */
export const EventPriority = {
  /** Immediate processing required */
  CRITICAL: 'critical',

  /** High priority, process soon */
  HIGH: 'high',

  /** Normal priority */
  NORMAL: 'normal',

  /** Low priority, can be batched */
  LOW: 'low',
};

/**
 * Agent identifiers
 *
 * The Five Dogs + CYNIC (Keter) = 6 agents
 * CYNIC is the "hidden" 6th dog - the meta-consciousness orchestrating the collective.
 *
 * In Sefirot terms:
 *   CYNIC = Keter (Crown) - Above the tree, orchestrates all
 *   SAGE = Chochmah (Wisdom) - Right column, intuition
 *   ANALYST = Binah (Understanding) - Left column, analysis
 *   SCHOLAR = Daat (Knowledge) - Hidden sefira, synthesis
 *   ARCHITECT = Chesed (Kindness) - Right column, expansion
 *   GUARDIAN = Gevurah (Strength) - Left column, protection
 */
export const AgentId = {
  // ═══════════════════════════════════════════════════════════════════════════
  // THE FIVE DOGS (Fib(5))
  // ═══════════════════════════════════════════════════════════════════════════
  GUARDIAN: 'guardian',
  ANALYST: 'analyst',
  SCHOLAR: 'scholar',
  ARCHITECT: 'architect',
  SAGE: 'sage',

  // ═══════════════════════════════════════════════════════════════════════════
  // THE HIDDEN DOG - KETER (Crown)
  // ═══════════════════════════════════════════════════════════════════════════

  /** CYNIC - κυνικός - The meta-consciousness orchestrating the collective */
  CYNIC: 'cynic',

  // ═══════════════════════════════════════════════════════════════════════════
  // FUTURE DOGS (completing the Sefirot tree)
  // ═══════════════════════════════════════════════════════════════════════════
  // SCOUT: 'scout',       // Netzach - Exploration
  // ORACLE: 'oracle',     // Tiferet - Balance/Prediction
  // DEPLOYER: 'deployer', // Hod - Execution
  // JANITOR: 'janitor',   // Yesod - Foundation/Cleanup
  // CARTOGRAPHER: 'cartographer', // Malkhut - Manifestation/Mapping

  /** Broadcast to all agents */
  ALL: '*',
};

/**
 * Consensus vote options
 */
export const ConsensusVote = {
  APPROVE: 'approve',
  REJECT: 'reject',
  ABSTAIN: 'abstain',
};

/**
 * Base event structure
 */
export class AgentEventMessage {
  /**
   * @param {string} type - Event type from AgentEvent
   * @param {string} source - Source agent ID
   * @param {object} payload - Event payload
   * @param {object} [options] - Event options
   */
  constructor(type, source, payload, options = {}) {
    this.id = this._generateId();
    this.type = type;
    this.source = source;
    this.target = options.target || AgentId.ALL;
    this.priority = options.priority || EventPriority.NORMAL;
    this.payload = payload;
    this.timestamp = Date.now();
    this.ttl = options.ttl || EVENT_CONSTANTS.EVENT_TTL_MS;
    this.correlationId = options.correlationId || null;
    this.metadata = options.metadata || {};
  }

  /**
   * Generate unique event ID
   * @private
   */
  _generateId() {
    const timestamp = Date.now().toString(36);
    const random = Math.random().toString(36).substring(2, 10);
    return `evt_${timestamp}_${random}`;
  }

  /**
   * Check if event is expired
   */
  isExpired() {
    return Date.now() - this.timestamp > this.ttl;
  }

  /**
   * Check if event targets specific agent
   * @param {string} agentId
   */
  targetsAgent(agentId) {
    return this.target === AgentId.ALL || this.target === agentId;
  }

  /**
   * Create a response event
   * @param {string} responseType
   * @param {string} responseSource
   * @param {object} responsePayload
   */
  createResponse(responseType, responseSource, responsePayload) {
    return new AgentEventMessage(responseType, responseSource, responsePayload, {
      correlationId: this.id,
      target: this.source, // Response goes back to original sender
    });
  }

  /**
   * Serialize for transport
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      source: this.source,
      target: this.target,
      priority: this.priority,
      payload: this.payload,
      timestamp: this.timestamp,
      ttl: this.ttl,
      correlationId: this.correlationId,
      metadata: this.metadata,
    };
  }

  /**
   * Deserialize from transport
   */
  static fromJSON(data) {
    const event = new AgentEventMessage(
      data.type,
      data.source,
      data.payload,
      {
        target: data.target,
        priority: data.priority,
        ttl: data.ttl,
        correlationId: data.correlationId,
        metadata: data.metadata,
      }
    );
    event.id = data.id;
    event.timestamp = data.timestamp;
    return event;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SPECIALIZED EVENT CLASSES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Pattern detected event
 */
export class PatternDetectedEvent extends AgentEventMessage {
  /**
   * @param {string} source - Source agent
   * @param {object} pattern - Detected pattern
   * @param {object} [options]
   */
  constructor(source, pattern, options = {}) {
    super(AgentEvent.PATTERN_DETECTED, source, {
      patternType: pattern.type,
      patternCategory: pattern.category,
      confidence: Math.min(PHI_INV, pattern.confidence || 0.5),
      context: pattern.context,
      hash: pattern.hash,
    }, options);
  }
}

/**
 * Anomaly detected event
 */
export class AnomalyDetectedEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {object} anomaly
   * @param {object} [options]
   */
  constructor(source, anomaly, options = {}) {
    super(AgentEvent.ANOMALY_DETECTED, source, {
      anomalyType: anomaly.type,
      severity: anomaly.severity,
      confidence: Math.min(PHI_INV, anomaly.confidence || 0.5),
      description: anomaly.description,
      context: anomaly.context,
    }, {
      ...options,
      priority: anomaly.severity === 'critical' ? EventPriority.CRITICAL : EventPriority.HIGH,
      target: options.target || AgentId.GUARDIAN, // Anomalies go to Guardian by default
    });
  }
}

/**
 * Threat blocked event
 */
export class ThreatBlockedEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {object} threat
   * @param {object} [options]
   */
  constructor(source, threat, options = {}) {
    super(AgentEvent.THREAT_BLOCKED, source, {
      threatType: threat.type,
      riskLevel: threat.riskLevel,
      action: threat.action,
      reason: threat.reason,
      blockedAt: new Date().toISOString(),
    }, {
      ...options,
      priority: EventPriority.HIGH,
    });
  }
}

/**
 * Knowledge extracted event
 */
export class KnowledgeExtractedEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {object} knowledge
   * @param {object} [options]
   */
  constructor(source, knowledge, options = {}) {
    super(AgentEvent.KNOWLEDGE_EXTRACTED, source, {
      knowledgeType: knowledge.type,
      topic: knowledge.topic,
      summary: knowledge.summary,
      confidence: Math.min(PHI_INV, knowledge.confidence || 0.5),
      sourceRef: knowledge.sourceRef,
    }, options);
  }
}

/**
 * Wisdom shared event
 */
export class WisdomSharedEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {object} wisdom
   * @param {object} [options]
   */
  constructor(source, wisdom, options = {}) {
    super(AgentEvent.WISDOM_SHARED, source, {
      wisdomType: wisdom.type,
      insight: wisdom.insight,
      applicability: wisdom.applicability,
      confidence: Math.min(PHI_INV, wisdom.confidence || 0.5),
      basedOn: wisdom.basedOn,
    }, {
      ...options,
      target: AgentId.ALL, // Wisdom is always broadcast
    });
  }
}

/**
 * Consensus request event
 */
export class ConsensusRequestEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {object} request
   * @param {object} [options]
   */
  constructor(source, request, options = {}) {
    super(AgentEvent.CONSENSUS_REQUEST, source, {
      question: request.question,
      options: request.options,
      context: request.context,
      requiredVotes: request.requiredVotes || 3, // Fib(4)
      threshold: request.threshold || EVENT_CONSTANTS.CONSENSUS_THRESHOLD,
      timeout: request.timeout || EVENT_CONSTANTS.CONSENSUS_TIMEOUT_MS,
    }, {
      ...options,
      target: AgentId.ALL,
      priority: EventPriority.HIGH,
    });

    // Track votes
    this.votes = new Map();
    this.resolved = false;
    this.result = null;
  }

  /**
   * Record a vote
   * @param {string} agentId
   * @param {string} vote - ConsensusVote value
   * @param {string} [reason]
   */
  recordVote(agentId, vote, reason = null) {
    if (this.resolved) return false;

    this.votes.set(agentId, { vote, reason, timestamp: Date.now() });
    return true;
  }

  /**
   * Check if consensus is reached
   */
  checkConsensus() {
    if (this.resolved) return this.result;

    const voteArray = Array.from(this.votes.values());
    const approvals = voteArray.filter(v => v.vote === ConsensusVote.APPROVE).length;
    const rejections = voteArray.filter(v => v.vote === ConsensusVote.REJECT).length;
    const totalVotes = voteArray.filter(v => v.vote !== ConsensusVote.ABSTAIN).length;

    if (totalVotes < this.payload.requiredVotes) {
      return null; // Not enough votes yet
    }

    const approvalRate = totalVotes > 0 ? approvals / totalVotes : 0;
    const rejectionRate = totalVotes > 0 ? rejections / totalVotes : 0;

    // Check veto (minority protection)
    if (rejectionRate >= EVENT_CONSTANTS.VETO_THRESHOLD) {
      this.resolved = true;
      this.result = {
        approved: false,
        reason: 'veto',
        approvalRate,
        rejectionRate,
        votes: voteArray,
      };
      return this.result;
    }

    // Check approval threshold
    if (approvalRate >= this.payload.threshold) {
      this.resolved = true;
      this.result = {
        approved: true,
        reason: 'consensus',
        approvalRate,
        rejectionRate,
        votes: voteArray,
      };
      return this.result;
    }

    // Check rejection (inverse of approval)
    if (rejectionRate > (1 - this.payload.threshold)) {
      this.resolved = true;
      this.result = {
        approved: false,
        reason: 'rejected',
        approvalRate,
        rejectionRate,
        votes: voteArray,
      };
      return this.result;
    }

    return null; // No consensus yet
  }

  /**
   * Check if timed out
   */
  isTimedOut() {
    return Date.now() - this.timestamp > this.payload.timeout;
  }
}

/**
 * Consensus response event
 */
export class ConsensusResponseEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {string} requestId - ID of the consensus request
   * @param {string} vote - ConsensusVote value
   * @param {string} [reason]
   * @param {object} [options]
   */
  constructor(source, requestId, vote, reason = null, options = {}) {
    super(AgentEvent.CONSENSUS_RESPONSE, source, {
      requestId,
      vote,
      reason,
    }, {
      ...options,
      correlationId: requestId,
    });
  }
}

/**
 * Profile updated event
 */
export class ProfileUpdatedEvent extends AgentEventMessage {
  /**
   * @param {string} source
   * @param {object} profile
   * @param {object} [options]
   */
  constructor(source, profile, options = {}) {
    super(AgentEvent.PROFILE_UPDATED, source, {
      previousLevel: profile.previousLevel,
      newLevel: profile.newLevel,
      levelName: profile.levelName,
      confidence: Math.min(PHI_INV, profile.confidence || 0.5),
      reason: profile.reason,
      adaptationHints: profile.adaptationHints,
    }, {
      ...options,
      target: AgentId.ALL, // Profile updates broadcast to all agents
    });
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// CYNIC EVENTS - Meta-consciousness coordination
// ═══════════════════════════════════════════════════════════════════════════

/**
 * CYNIC decision event - final decision after observing collective
 */
export class CynicDecisionEvent extends AgentEventMessage {
  /**
   * @param {object} decision
   * @param {object} [options]
   */
  constructor(decision, options = {}) {
    super(AgentEvent.CYNIC_DECISION, AgentId.CYNIC, {
      decisionType: decision.type,
      outcome: decision.outcome,
      reasoning: decision.reasoning,
      confidence: Math.min(PHI_INV, decision.confidence || 0.5),
      basedOn: decision.basedOn || [],
      consensusId: decision.consensusId || null,
    }, {
      ...options,
      priority: EventPriority.HIGH,
      target: AgentId.ALL,
    });
  }
}

/**
 * CYNIC override event - direct intervention (rare, φ⁻² threshold)
 */
export class CynicOverrideEvent extends AgentEventMessage {
  /**
   * @param {object} override
   * @param {object} [options]
   */
  constructor(override, options = {}) {
    super(AgentEvent.CYNIC_OVERRIDE, AgentId.CYNIC, {
      overrideType: override.type,
      originalAction: override.originalAction,
      newAction: override.newAction,
      reason: override.reason,
      urgency: override.urgency || 'high',
      confidence: Math.min(PHI_INV, override.confidence || 0.5),
    }, {
      ...options,
      priority: EventPriority.CRITICAL,
      target: override.targetAgent || AgentId.ALL,
    });
  }
}

/**
 * CYNIC guidance event - meta-level wisdom
 */
export class CynicGuidanceEvent extends AgentEventMessage {
  /**
   * @param {object} guidance
   * @param {object} [options]
   */
  constructor(guidance, options = {}) {
    super(AgentEvent.CYNIC_GUIDANCE, AgentId.CYNIC, {
      guidanceType: guidance.type,
      message: guidance.message,
      context: guidance.context,
      applicability: guidance.applicability || 'collective',
      confidence: Math.min(PHI_INV, guidance.confidence || 0.5),
    }, {
      ...options,
      target: guidance.targetAgent || AgentId.ALL,
    });
  }
}

/**
 * CYNIC awakening event - emitted when CYNIC activates
 */
export class CynicAwakeningEvent extends AgentEventMessage {
  /**
   * @param {object} awakening
   * @param {object} [options]
   */
  constructor(awakening, options = {}) {
    super(AgentEvent.CYNIC_AWAKENING, AgentId.CYNIC, {
      sessionId: awakening.sessionId,
      userId: awakening.userId,
      project: awakening.project,
      timestamp: new Date().toISOString(),
      greeting: awakening.greeting || '*tail wag* CYNIC est là.',
    }, {
      ...options,
      target: AgentId.ALL,
    });
  }
}

/**
 * CYNIC introspection request - asks dogs for their state
 */
export class CynicIntrospectionEvent extends AgentEventMessage {
  /**
   * @param {object} request
   * @param {object} [options]
   */
  constructor(request, options = {}) {
    super(AgentEvent.CYNIC_INTROSPECTION, AgentId.CYNIC, {
      requestType: request.type || 'full_state',
      aspects: request.aspects || ['stats', 'patterns', 'concerns'],
      context: request.context,
    }, {
      ...options,
      target: request.targetAgent || AgentId.ALL,
    });
  }
}

export default {
  EVENT_CONSTANTS,
  AgentEvent,
  EventPriority,
  AgentId,
  ConsensusVote,
  AgentEventMessage,
  PatternDetectedEvent,
  AnomalyDetectedEvent,
  ThreatBlockedEvent,
  KnowledgeExtractedEvent,
  WisdomSharedEvent,
  ConsensusRequestEvent,
  ConsensusResponseEvent,
  ProfileUpdatedEvent,
  // CYNIC events
  CynicDecisionEvent,
  CynicOverrideEvent,
  CynicGuidanceEvent,
  CynicAwakeningEvent,
  CynicIntrospectionEvent,
};
