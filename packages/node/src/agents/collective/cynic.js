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
import { SelfSkeptic, createSelfSkeptic } from '../../judge/self-skeptic.js';
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
  IntrospectionResponseEvent,
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

  /** Relationship learning rate (φ⁻³ ≈ 0.236) */
  LEARNING_RATE: PHI_INV_2 * PHI_INV,

  /** Relationship decay rate (φ⁻⁴ ≈ 0.146) */
  DECAY_RATE: PHI_INV_2 * PHI_INV_2,

  /** Minimum interaction count before relationship is considered learned (Fib(5) = 5) */
  MIN_INTERACTIONS: 5,

  /** Structure proposal threshold - suggest changes after Fib(8) = 21 learned patterns */
  STRUCTURE_PROPOSAL_THRESHOLD: 21,
};

// ═══════════════════════════════════════════════════════════════════════════
// SEFIROT TEMPLATE - Initial guidance, not law
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Sefirot relationships - CYNIC's initial intuition, subject to learning
 *
 * φ-aligned relationship weights based on Tree of Life geometry:
 *
 *                    Keter (CYNIC)
 *                        │
 *         ┌──────────────┼──────────────┐
 *         │              │              │
 *      Binah          (Daat)        Chochmah
 *    (Analyst)       (Scholar)       (Sage)
 *         │              │              │
 *         └──────────────┼──────────────┘
 *                        │
 *         ┌──────────────┼──────────────┐
 *         │              │              │
 *      Gevurah       Tiferet        Chesed
 *    (Guardian)      (Oracle)     (Architect)
 *         │              │              │
 *         └──────────────┼──────────────┘
 *                        │
 *         ┌──────────────┼──────────────┐
 *         │              │              │
 *        Hod          Yesod         Netzach
 *    (Deployer)      (Janitor)       (Scout)
 *         │              │              │
 *         └──────────────┴──────────────┘
 *                        │
 *                    Malkhut
 *                 (Cartographer)
 *
 * Weight system (φ-aligned):
 * - φ⁻¹ = 0.618: Direct vertical connections (same pillar)
 * - φ⁻² = 0.382: Horizontal connections (same level, adjacent)
 * - φ⁻³ = 0.236: Diagonal connections (different pillar, adjacent level)
 * - φ⁻⁴ = 0.146: Indirect connections (skip level or far pillar)
 *
 * "φ doute de φ" - even this template should be questioned.
 */
export const SEFIROT_TEMPLATE = {
  // φ ratio weights
  weights: {
    DIRECT: PHI_INV,       // φ⁻¹ = 0.618 - Direct vertical connection
    HORIZONTAL: PHI_INV_2, // φ⁻² = 0.382 - Same level, adjacent pillar
    DIAGONAL: PHI_INV_2 * PHI_INV,  // φ⁻³ ≈ 0.236 - Diagonal connection
    INDIRECT: PHI_INV_2 * PHI_INV_2, // φ⁻⁴ ≈ 0.146 - Distant connection
  },

  // Pillar assignments (for geometric reasoning)
  pillars: {
    left: ['analyst', 'guardian', 'deployer'],    // Binah, Gevurah, Hod
    middle: ['scholar', 'oracle', 'janitor', 'cartographer'], // Daat, Tiferet, Yesod, Malkhut
    right: ['sage', 'architect', 'scout'],        // Chochmah, Chesed, Netzach
  },

  // Level assignments (0 = top, 3 = bottom)
  levels: {
    cynic: 0,       // Keter
    analyst: 1, sage: 1, scholar: 1,  // Level 1
    guardian: 2, oracle: 2, architect: 2,  // Level 2
    deployer: 3, janitor: 3, scout: 3,  // Level 3
    cartographer: 4,  // Malkhut
  },

  // Sefirah mappings (agent -> Sefirah)
  mappings: {
    cynic: { sefira: 'Keter', meaning: 'Crown', role: 'Meta-consciousness', pillar: 'middle', level: 0 },
    analyst: { sefira: 'Binah', meaning: 'Understanding', role: 'Analysis', pillar: 'left', level: 1 },
    sage: { sefira: 'Chochmah', meaning: 'Wisdom', role: 'Guidance', pillar: 'right', level: 1 },
    scholar: { sefira: 'Daat', meaning: 'Knowledge', role: 'Knowledge extraction', pillar: 'middle', level: 1 },
    guardian: { sefira: 'Gevurah', meaning: 'Strength', role: 'Protection', pillar: 'left', level: 2 },
    oracle: { sefira: 'Tiferet', meaning: 'Beauty', role: 'Visualization', pillar: 'middle', level: 2 },
    architect: { sefira: 'Chesed', meaning: 'Kindness', role: 'Design', pillar: 'right', level: 2 },
    deployer: { sefira: 'Hod', meaning: 'Splendor', role: 'Deployment', pillar: 'left', level: 3 },
    janitor: { sefira: 'Yesod', meaning: 'Foundation', role: 'Code hygiene', pillar: 'middle', level: 3 },
    scout: { sefira: 'Netzach', meaning: 'Victory', role: 'Discovery', pillar: 'right', level: 3 },
    cartographer: { sefira: 'Malkhut', meaning: 'Kingdom', role: 'Mapping', pillar: 'middle', level: 4 },
  },

  /**
   * Calculate initial weight between two agents based on Tree geometry
   * @param {string} from - Source agent
   * @param {string} to - Target agent
   * @returns {number} φ-aligned weight
   */
  calculateWeight(from, to) {
    const fromMapping = this.mappings[from];
    const toMapping = this.mappings[to];

    if (!fromMapping || !toMapping) return 0;

    const levelDiff = Math.abs(fromMapping.level - toMapping.level);
    const samePillar = fromMapping.pillar === toMapping.pillar;
    const adjacentPillar = (
      (fromMapping.pillar === 'middle') ||
      (toMapping.pillar === 'middle') ||
      (fromMapping.pillar === 'left' && toMapping.pillar === 'right') ||
      (fromMapping.pillar === 'right' && toMapping.pillar === 'left')
    );

    // Same pillar, adjacent level -> DIRECT (φ⁻¹)
    if (samePillar && levelDiff === 1) {
      return this.weights.DIRECT;
    }

    // Same level, adjacent pillar -> HORIZONTAL (φ⁻²)
    if (levelDiff === 0 && adjacentPillar) {
      return this.weights.HORIZONTAL;
    }

    // Adjacent level, different pillar -> DIAGONAL (φ⁻³)
    if (levelDiff === 1 && !samePillar) {
      return this.weights.DIAGONAL;
    }

    // Everything else -> INDIRECT (φ⁻⁴)
    if (levelDiff <= 2) {
      return this.weights.INDIRECT;
    }

    // Very distant -> no initial connection
    return 0;
  },

  /**
   * Generate all initial affinities based on geometric rules
   * @returns {Object} Affinities map
   */
  generateAffinities() {
    const agents = Object.keys(this.mappings).filter(a => a !== 'cynic');
    const affinities = {};

    for (const from of agents) {
      affinities[from] = {};
      for (const to of agents) {
        if (from !== to) {
          const weight = this.calculateWeight(from, to);
          if (weight > 0) {
            affinities[from][to] = weight;
          }
        }
      }
    }

    return affinities;
  },
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

// ═══════════════════════════════════════════════════════════════════════════
// RELATIONSHIP GRAPH - Dynamic learning of agent relationships
// ═══════════════════════════════════════════════════════════════════════════

/**
 * RelationshipGraph - CYNIC learns agent relationships through observation
 *
 * Rather than hardcoding structure, CYNIC observes:
 * - Which agents interact frequently
 * - Which collaborations produce positive outcomes
 * - Which pairs have natural synergy
 *
 * The graph evolves based on evidence, with Sefirot as initial seed.
 */
export class RelationshipGraph {
  /**
   * @param {Object} [options] - Options
   * @param {boolean} [options.useSefirotSeed=true] - Use Sefirot template as initial values
   */
  constructor(options = {}) {
    // Relationship weights: Map<string, Map<string, RelationshipData>>
    // Key format: "from:to" -> { weight, interactions, outcomes }
    this.relationships = new Map();

    // Interaction history for learning
    this.interactionHistory = [];

    // Structure proposals (when learned patterns diverge from template)
    this.structureProposals = [];

    // Initialize with Sefirot template if requested
    if (options.useSefirotSeed !== false) {
      this._seedFromSefirot();
    }

    // Statistics
    this.stats = {
      totalInteractions: 0,
      learnedRelationships: 0,
      structureProposals: 0,
    };
  }

  /**
   * Seed initial weights from Sefirot template
   * Uses φ-aligned geometric calculations
   * @private
   */
  _seedFromSefirot() {
    // Generate affinities from geometric rules
    const affinities = SEFIROT_TEMPLATE.generateAffinities();

    for (const [from, targets] of Object.entries(affinities)) {
      for (const [to, weight] of Object.entries(targets)) {
        this._setRelationship(from, to, {
          weight,
          source: 'sefirot_template',
          sefirotWeight: weight, // Store original for comparison
          interactions: 0,
          positiveOutcomes: 0,
          negativeOutcomes: 0,
          createdAt: Date.now(),
          lastUpdated: Date.now(),
        });
      }
    }
  }

  /**
   * Get relationship key
   * @private
   */
  _getKey(from, to) {
    return `${from}:${to}`;
  }

  /**
   * Set relationship data
   * @private
   */
  _setRelationship(from, to, data) {
    if (!this.relationships.has(from)) {
      this.relationships.set(from, new Map());
    }
    this.relationships.get(from).set(to, data);
  }

  /**
   * Get relationship data
   * @param {string} from - Source agent
   * @param {string} to - Target agent
   * @returns {Object|null} Relationship data or null
   */
  getRelationship(from, to) {
    return this.relationships.get(from)?.get(to) || null;
  }

  /**
   * Record an interaction between two agents
   * @param {string} from - Source agent
   * @param {string} to - Target agent
   * @param {Object} context - Interaction context
   * @returns {Object} Updated relationship
   */
  recordInteraction(from, to, context = {}) {
    let rel = this.getRelationship(from, to);

    if (!rel) {
      // New relationship discovered through observation
      rel = {
        weight: 0.1, // Start with low weight
        source: 'observed',
        interactions: 0,
        positiveOutcomes: 0,
        negativeOutcomes: 0,
        createdAt: Date.now(),
        lastUpdated: Date.now(),
      };
    }

    // Update interaction count
    rel.interactions++;
    rel.lastUpdated = Date.now();

    // Store interaction for learning
    this.interactionHistory.push({
      from,
      to,
      context,
      timestamp: Date.now(),
    });

    // Trim history
    while (this.interactionHistory.length > CYNIC_CONSTANTS.MAX_OBSERVED_EVENTS) {
      this.interactionHistory.shift();
    }

    this._setRelationship(from, to, rel);
    this.stats.totalInteractions++;

    return rel;
  }

  /**
   * Record outcome of a collaboration
   * @param {string} from - Source agent
   * @param {string} to - Target agent
   * @param {boolean} positive - Whether outcome was positive
   * @param {number} [magnitude=1] - Magnitude of impact (0-1)
   */
  recordOutcome(from, to, positive, magnitude = 1) {
    let rel = this.getRelationship(from, to);

    if (!rel) {
      // Create relationship from outcome
      rel = {
        weight: positive ? 0.1 : -0.1,
        source: 'observed',
        interactions: 1,
        positiveOutcomes: 0,
        negativeOutcomes: 0,
        createdAt: Date.now(),
        lastUpdated: Date.now(),
      };
    }

    // Update outcome counts
    if (positive) {
      rel.positiveOutcomes++;
    } else {
      rel.negativeOutcomes++;
    }

    // Calculate new weight using φ-aligned learning
    const learningRate = CYNIC_CONSTANTS.LEARNING_RATE * magnitude;
    const delta = positive ? learningRate : -learningRate;

    // Adjust weight, bounded by [-1, 1]
    rel.weight = Math.max(-1, Math.min(1, rel.weight + delta));
    rel.lastUpdated = Date.now();

    // Check if relationship is now "learned" (enough evidence)
    if (rel.interactions >= CYNIC_CONSTANTS.MIN_INTERACTIONS) {
      rel.learned = true;
      this.stats.learnedRelationships++;
    }

    this._setRelationship(from, to, rel);

    // Check for structure proposals
    this._checkForStructureProposal(from, to, rel);
  }

  /**
   * Apply time-based decay to all relationships
   * Relationships that aren't reinforced gradually decay toward neutral.
   */
  applyDecay() {
    const decay = CYNIC_CONSTANTS.DECAY_RATE;
    const now = Date.now();

    for (const [from, targets] of this.relationships) {
      for (const [to, rel] of targets) {
        // Only decay observed relationships, not template ones
        if (rel.source === 'observed' && rel.learned) {
          const timeSinceUpdate = now - rel.lastUpdated;
          const decayPeriods = timeSinceUpdate / CYNIC_CONSTANTS.INTROSPECTION_INTERVAL_MS;

          if (decayPeriods >= 1) {
            // Decay toward 0 (neutral)
            const decayAmount = decay * Math.floor(decayPeriods);
            if (rel.weight > 0) {
              rel.weight = Math.max(0, rel.weight - decayAmount);
            } else if (rel.weight < 0) {
              rel.weight = Math.min(0, rel.weight + decayAmount);
            }
            rel.lastUpdated = now;
            this._setRelationship(from, to, rel);
          }
        }
      }
    }
  }

  /**
   * Check if learned patterns diverge from Sefirot template
   * @private
   */
  _checkForStructureProposal(from, to, rel) {
    // Only consider learned relationships
    if (!rel.learned) return;

    // Get Sefirot suggested affinity (stored or calculated)
    const sefirotWeight = rel.sefirotWeight ?? SEFIROT_TEMPLATE.calculateWeight(from, to);

    // Check for significant divergence
    const divergence = Math.abs(rel.weight - sefirotWeight);

    if (divergence >= PHI_INV_2) { // 38.2% divergence threshold
      const proposal = {
        type: rel.weight > sefirotWeight ? 'strengthen' : 'weaken',
        from,
        to,
        currentWeight: rel.weight,
        sefirotWeight,
        divergence,
        evidence: {
          interactions: rel.interactions,
          positiveOutcomes: rel.positiveOutcomes,
          negativeOutcomes: rel.negativeOutcomes,
        },
        timestamp: Date.now(),
      };

      // Avoid duplicate proposals
      const existingIdx = this.structureProposals.findIndex(
        p => p.from === from && p.to === to
      );

      if (existingIdx >= 0) {
        this.structureProposals[existingIdx] = proposal;
      } else {
        this.structureProposals.push(proposal);
        this.stats.structureProposals++;
      }
    }
  }

  /**
   * Get strongest relationships for an agent
   * @param {string} agent - Agent name
   * @param {number} [limit=5] - Max relationships to return
   * @returns {Array} Sorted relationships
   */
  getStrongestRelationships(agent, limit = 5) {
    const targets = this.relationships.get(agent);
    if (!targets) return [];

    return Array.from(targets.entries())
      .map(([to, data]) => ({ to, ...data }))
      .sort((a, b) => b.weight - a.weight)
      .slice(0, limit);
  }

  /**
   * Get all learned relationships
   * @returns {Array} All learned relationships
   */
  getLearnedRelationships() {
    const learned = [];

    for (const [from, targets] of this.relationships) {
      for (const [to, data] of targets) {
        if (data.learned) {
          learned.push({ from, to, ...data });
        }
      }
    }

    return learned.sort((a, b) => b.weight - a.weight);
  }

  /**
   * Get current structure proposals
   * @returns {Array} Structure proposals
   */
  getStructureProposals() {
    return [...this.structureProposals];
  }

  /**
   * Export full graph state
   * @returns {Object} Graph state
   */
  export() {
    const edges = [];

    for (const [from, targets] of this.relationships) {
      for (const [to, data] of targets) {
        edges.push({ from, to, ...data });
      }
    }

    return {
      edges,
      stats: { ...this.stats },
      structureProposals: [...this.structureProposals],
      exportedAt: Date.now(),
    };
  }

  /**
   * Import graph state
   * @param {Object} state - Previously exported state
   */
  import(state) {
    if (!state?.edges) return;

    this.relationships.clear();
    for (const edge of state.edges) {
      const { from, to, ...data } = edge;
      this._setRelationship(from, to, data);
    }

    if (state.stats) {
      this.stats = { ...state.stats };
    }

    if (state.structureProposals) {
      this.structureProposals = [...state.structureProposals];
    }
  }

  /**
   * Get summary statistics
   * @returns {Object} Summary
   */
  getSummary() {
    const edges = [];
    let totalWeight = 0;
    let learnedCount = 0;

    for (const [from, targets] of this.relationships) {
      for (const [to, data] of targets) {
        edges.push({ from, to, weight: data.weight, learned: data.learned });
        totalWeight += data.weight;
        if (data.learned) learnedCount++;
      }
    }

    return {
      totalRelationships: edges.length,
      learnedRelationships: learnedCount,
      averageWeight: edges.length > 0 ? totalWeight / edges.length : 0,
      structureProposals: this.structureProposals.length,
      stats: { ...this.stats },
    };
  }
}

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

    // Pending introspection requests (for response collection)
    this.pendingIntrospections = new Map();

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

    // Relationship graph - CYNIC learns agent relationships
    this.relationshipGraph = new RelationshipGraph({
      useSefirotSeed: options.useSefirotSeed !== false,
    });

    // Self-Skeptic: "φ distrusts φ" - active meta-doubt mechanism
    // CYNIC doubts even its own decisions
    this.selfSkeptic = options.selfSkeptic || createSelfSkeptic();

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

    // Subscribe to introspection responses from dogs
    this.eventBus.subscribe(
      AgentEvent.INTROSPECTION_RESPONSE,
      AgentId.CYNIC,
      this._handleIntrospectionResponse.bind(this),
      { priority: EventPriority.NORMAL }
    );
  }

  /**
   * Handle introspection response from a dog
   * @private
   */
  _handleIntrospectionResponse(event) {
    const { introspectionId, stats, patterns, concerns, state } = event.data || {};
    const source = event.source;

    // Find pending introspection
    const pending = this.pendingIntrospections.get(introspectionId);
    if (!pending) {
      return; // Ignore stale responses
    }

    // Record response
    pending.responses.set(source, {
      stats,
      patterns,
      concerns,
      state,
      timestamp: Date.now(),
    });

    // Update dog states
    this.dogStates.set(source, {
      stats,
      patterns,
      concerns,
      state,
      lastUpdate: Date.now(),
    });

    // Check if all expected responses received
    if (pending.responses.size >= pending.expectedResponses) {
      pending.resolve(this._aggregateIntrospectionResponses(pending));
      this.pendingIntrospections.delete(introspectionId);
    }
  }

  /**
   * Aggregate introspection responses from all dogs
   * @private
   */
  _aggregateIntrospectionResponses(pending) {
    const responses = Array.from(pending.responses.entries());

    // Combine stats from all dogs
    const combinedStats = {};
    const allPatterns = [];
    const allConcerns = [];

    for (const [agentId, response] of responses) {
      // Merge stats
      if (response.stats) {
        combinedStats[agentId] = response.stats;
      }

      // Collect patterns
      if (response.patterns) {
        allPatterns.push(...response.patterns.map(p => ({
          ...p,
          source: agentId,
        })));
      }

      // Collect concerns
      if (response.concerns) {
        allConcerns.push(...response.concerns.map(c => ({
          ...c,
          source: agentId,
        })));
      }
    }

    return {
      success: true,
      respondedAgents: responses.map(([id]) => id),
      combinedStats,
      patterns: allPatterns,
      concerns: allConcerns,
      timestamp: Date.now(),
    };
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

    // Learn from agent interactions
    this._learnFromInteraction(event);

    // Update meta-state based on collective activity
    this._updateMetaState(event);

    // Check if synthesis is needed
    if (this.observedEvents.length >= CYNIC_CONSTANTS.WISDOM_THRESHOLD) {
      await this._considerSynthesis();
    }
  }

  /**
   * Learn from agent interaction - update relationship weights
   * @private
   */
  _learnFromInteraction(event) {
    const source = event.source;
    const payload = event.payload || {};

    // Track agent-to-agent interactions
    if (payload.targetAgent && payload.targetAgent !== source) {
      this.relationshipGraph.recordInteraction(source, payload.targetAgent, {
        eventType: event.type,
        timestamp: event.timestamp,
      });
    }

    // Learn from outcomes
    if (event.type === AgentEvent.CONSENSUS_REACHED) {
      // Consensus success - strengthen relationships between participants
      const participants = payload.participants || [];
      for (let i = 0; i < participants.length; i++) {
        for (let j = i + 1; j < participants.length; j++) {
          this.relationshipGraph.recordOutcome(participants[i], participants[j], true, 0.5);
          this.relationshipGraph.recordOutcome(participants[j], participants[i], true, 0.5);
        }
      }
    }

    if (event.type === AgentEvent.KNOWLEDGE_EXTRACTED) {
      // Knowledge extraction - strengthen scholar connections
      if (source && payload.contributingAgent) {
        this.relationshipGraph.recordOutcome(source, payload.contributingAgent, true, 0.3);
      }
    }

    if (event.type === AgentEvent.THREAT_BLOCKED) {
      // Threat blocked - strengthen guardian-analyst connection (if analyst detected)
      if (payload.detectedBy && payload.detectedBy !== source) {
        this.relationshipGraph.recordOutcome(source, payload.detectedBy, true, 0.7);
        this.relationshipGraph.recordOutcome(payload.detectedBy, source, true, 0.7);
      }
    }

    if (event.type === AgentEvent.PATTERN_DETECTED) {
      // Pattern detection - record the collaborating agents
      if (payload.corroboratedBy) {
        for (const corroborator of payload.corroboratedBy) {
          this.relationshipGraph.recordOutcome(source, corroborator, true, 0.4);
        }
      }
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

    // === HOOK EVENTS (from Claude Code) ===

    // Session start - CYNIC awakens
    if (event.type === AgentEvent.HOOK_SESSION_START) {
      this.metaState = MetaState.OBSERVING;
      this.session = {
        ...this.session,
        hookStartTime: Date.now(),
        userId: event.data?.userId || this.session.userId,
        project: event.data?.project || this.session.project,
      };
    }

    // Session end - CYNIC synthesizes
    if (event.type === AgentEvent.HOOK_SESSION_STOP) {
      this.metaState = MetaState.SYNTHESIZING;
      // Consider synthesis when session ends
      this._considerSynthesis();
    }

    // Pre-tool blocked - Guardian acted, CYNIC observes
    if (event.type === AgentEvent.HOOK_PRE_TOOL && event.data?.blocked) {
      // Track blocked operations from hooks
      const recentBlocks = this.observedEvents
        .filter(e => e.type === AgentEvent.HOOK_PRE_TOOL && e.payloadSummary?.blocked)
        .filter(e => Date.now() - e.timestamp < 60000);

      if (recentBlocks.length >= 3) {
        this.metaState = MetaState.DECIDING;
      }
    }

    // === INTERNAL EVENTS ===

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
   * Applies "φ distrusts φ" - even CYNIC's decisions are doubted
   * @param {Object} decisionContext - Context for decision
   * @returns {Object} Decision result
   */
  async makeDecision(decisionContext) {
    this.metaState = MetaState.DECIDING;

    const rawConfidence = Math.min(PHI_INV, decisionContext.confidence || 0.5);

    const decision = {
      type: decisionContext.type || CynicDecisionType.SYNTHESIS,
      outcome: decisionContext.outcome,
      reasoning: decisionContext.reasoning || 'φ doute de φ.',
      confidence: rawConfidence,
      basedOn: decisionContext.basedOn || [],
      consensusId: decisionContext.consensusId,
    };

    // Apply "φ distrusts φ" - self-skepticism to our own decision
    // Create a pseudo-judgment for the skeptic to analyze
    const pseudoJudgment = {
      id: `decision-${Date.now()}`,
      qScore: rawConfidence * 100, // Convert to 0-100 scale
      confidence: rawConfidence,
      verdict: rawConfidence >= 0.5 ? 'WAG' : 'GROWL',
      dimensions: {},
      metadata: { judgedAt: Date.now() },
      item: {
        type: 'decision',
        decisionType: decision.type,
        outcome: decision.outcome,
      },
    };

    const skepticism = this.selfSkeptic.doubt(pseudoJudgment, {
      previousJudgments: this.decisions.slice(-10).map(d => ({
        qScore: d.confidence * 100,
      })),
    });

    // Update decision with skepticism-adjusted confidence
    decision.confidence = skepticism.adjustedConfidence;
    decision.skepticism = {
      originalConfidence: rawConfidence,
      adjustedConfidence: skepticism.adjustedConfidence,
      doubts: skepticism.doubt.reasons,
      biases: skepticism.biases,
      counterHypotheses: skepticism.counterHypotheses,
      recommendation: skepticism.recommendation,
    };

    // Update reasoning if skepticism found issues
    if (skepticism.doubt.reasons.length > 0) {
      decision.reasoning = `${decision.reasoning} (φ doute: ${skepticism.doubt.reasons.length} concern${skepticism.doubt.reasons.length > 1 ? 's' : ''})`;
    }

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
   * @param {number} [options.timeout] - Timeout in ms (default: 5000)
   * @param {number} [options.expectedResponses] - Expected number of responses (default: 5)
   * @returns {Promise<Object>} Introspection result with collected responses
   */
  async introspect(options = {}) {
    const introspectionEvent = new CynicIntrospectionEvent({
      type: options.type || 'full_state',
      aspects: options.aspects || ['stats', 'patterns', 'concerns'],
      context: options.context,
      targetAgent: options.targetAgent,
    });

    const timeout = options.timeout || 5000; // 5 seconds default
    const expectedResponses = options.expectedResponses || 5; // 5 core dogs

    // Create pending introspection tracker
    let resolvePromise;
    const responsePromise = new Promise((resolve) => {
      resolvePromise = resolve;
    });

    this.pendingIntrospections.set(introspectionEvent.id, {
      event: introspectionEvent,
      responses: new Map(),
      expectedResponses,
      resolve: resolvePromise,
      createdAt: Date.now(),
    });

    // Publish event to request introspection
    if (this.eventBus) {
      await this.eventBus.publish(introspectionEvent);
    }

    // Wait for responses with timeout
    const timeoutPromise = new Promise((resolve) => {
      setTimeout(() => {
        const pending = this.pendingIntrospections.get(introspectionEvent.id);
        if (pending) {
          // Resolve with whatever we have
          resolve(this._aggregateIntrospectionResponses(pending));
          this.pendingIntrospections.delete(introspectionEvent.id);
        } else {
          resolve({
            success: true,
            respondedAgents: [],
            combinedStats: {},
            patterns: [],
            concerns: [],
            timedOut: true,
            timestamp: Date.now(),
          });
        }
      }, timeout);
    });

    // Race between responses and timeout
    const result = await Promise.race([responsePromise, timeoutPromise]);

    return {
      ...result,
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
    const skepticStats = this.selfSkeptic.getStats();

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
      // Self-skepticism summary: "φ distrusts φ"
      selfSkepticism: {
        doubtsApplied: skepticStats.metaDoubtsApplied,
        biasesDetected: skepticStats.biasesDetected,
        recentBiases: skepticStats.recentBiases,
      },
      // Relationship learning summary
      relationships: this.relationshipGraph.getSummary(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RELATIONSHIP LEARNING - Structure that emerges from observation
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get self-doubt patterns - "φ distrusts φ" introspection
   * CYNIC analyzes its own skepticism for meta-biases
   * @returns {Object} Self-doubt patterns and meta-analysis
   */
  getSelfDoubtPatterns() {
    const stats = this.selfSkeptic.getStats();
    const patterns = this.selfSkeptic.getSelfDoubtPatterns();

    return {
      // Current skepticism statistics
      stats: {
        judgmentsDoubled: stats.judgmentsDoubled,
        confidenceReductions: stats.confidenceReductions,
        biasesDetected: stats.biasesDetected,
        counterArgumentsGenerated: stats.counterArgumentsGenerated,
        metaDoubtsApplied: stats.metaDoubtsApplied,
      },
      // Recent biases detected in our own thinking
      recentBiases: stats.recentBiases,
      // Meta-patterns: patterns in our skepticism itself
      metaPatterns: patterns.patterns,
      // The recursive truth: even this analysis is bounded by φ
      phi: {
        note: 'φ distrusts φ: even this self-analysis is bounded',
        maxConfidence: CYNIC_CONSTANTS.MAX_CONFIDENCE,
        minDoubt: 1 - CYNIC_CONSTANTS.MAX_CONFIDENCE,
      },
    };
  }

  /**
   * Get learned relationships between agents
   * @param {string} [agent] - Optional: get relationships for specific agent
   * @returns {Object} Relationship data
   */
  getLearnedRelationships(agent) {
    if (agent) {
      return {
        agent,
        strongest: this.relationshipGraph.getStrongestRelationships(agent),
        sefirotMapping: SEFIROT_TEMPLATE.mappings[agent],
      };
    }

    return {
      all: this.relationshipGraph.getLearnedRelationships(),
      summary: this.relationshipGraph.getSummary(),
    };
  }

  /**
   * Get structure proposals (where learned patterns diverge from Sefirot template)
   * @returns {Array} Structure proposals
   */
  getStructureProposals() {
    const proposals = this.relationshipGraph.getStructureProposals();

    return proposals.map(p => ({
      ...p,
      sefirotFrom: SEFIROT_TEMPLATE.mappings[p.from],
      sefirotTo: SEFIROT_TEMPLATE.mappings[p.to],
      recommendation: this._generateStructureRecommendation(p),
    }));
  }

  /**
   * Generate recommendation from structure proposal
   * @private
   */
  _generateStructureRecommendation(proposal) {
    const fromSefira = SEFIROT_TEMPLATE.mappings[proposal.from]?.sefira || proposal.from;
    const toSefira = SEFIROT_TEMPLATE.mappings[proposal.to]?.sefira || proposal.to;

    if (proposal.type === 'strengthen') {
      return {
        action: 'strengthen',
        message: `*ears perk* Les interactions ${fromSefira}-${toSefira} sont plus fortes que prévu par le template. ` +
                 `Poids observé: ${(proposal.currentWeight * 100).toFixed(1)}% vs Sefirot: ${(proposal.sefirotWeight * 100).toFixed(1)}%. ` +
                 `Basé sur ${proposal.evidence.interactions} interactions, ${proposal.evidence.positiveOutcomes} positifs.`,
        confidence: Math.min(PHI_INV, proposal.evidence.interactions / 20),
      };
    } else {
      return {
        action: 'weaken',
        message: `*sniff* Les interactions ${fromSefira}-${toSefira} sont plus faibles que prévu. ` +
                 `Poids observé: ${(proposal.currentWeight * 100).toFixed(1)}% vs Sefirot: ${(proposal.sefirotWeight * 100).toFixed(1)}%. ` +
                 `${proposal.evidence.negativeOutcomes} outcomes négatifs sur ${proposal.evidence.interactions} interactions.`,
        confidence: Math.min(PHI_INV, proposal.evidence.interactions / 20),
      };
    }
  }

  /**
   * Manually record a collaboration outcome (for external feedback)
   * @param {string} agent1 - First agent
   * @param {string} agent2 - Second agent
   * @param {boolean} success - Whether collaboration was successful
   * @param {number} [magnitude=1] - Impact magnitude (0-1)
   */
  recordCollaboration(agent1, agent2, success, magnitude = 1) {
    this.relationshipGraph.recordOutcome(agent1, agent2, success, magnitude);
    this.relationshipGraph.recordOutcome(agent2, agent1, success, magnitude);
  }

  /**
   * Apply decay to relationships (call periodically)
   */
  applyRelationshipDecay() {
    this.relationshipGraph.applyDecay();
  }

  /**
   * Export relationship graph state (for persistence)
   * @returns {Object} Exportable state
   */
  exportRelationships() {
    return this.relationshipGraph.export();
  }

  /**
   * Import relationship graph state (from persistence)
   * @param {Object} state - Previously exported state
   */
  importRelationships(state) {
    this.relationshipGraph.import(state);
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
   * @param {Object} [options] - Options
   * @param {boolean} [options.keepRelationships=true] - Keep learned relationships
   */
  clear(options = {}) {
    const keepRelationships = options.keepRelationships !== false;

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

    // Optionally reset relationship graph (but keep learned knowledge by default)
    if (!keepRelationships) {
      this.relationshipGraph = new RelationshipGraph({ useSefirotSeed: true });
    }

    // Reset self-skeptic (but keep history for bias detection by default)
    this.selfSkeptic.resetStats();
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
  RelationshipGraph,
  SEFIROT_TEMPLATE,
};
