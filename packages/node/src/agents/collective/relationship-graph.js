/**
 * @cynic/node - RelationshipGraph
 *
 * CYNIC learns agent relationships through observation.
 * Rather than hardcoding structure, CYNIC observes:
 * - Which agents interact frequently
 * - Which collaborations produce positive outcomes
 * - Which pairs have natural synergy
 *
 * The graph evolves based on evidence, with Sefirot as initial seed.
 *
 * @module @cynic/node/agents/collective/relationship-graph
 */

'use strict';

import { CYNIC_CONSTANTS } from './constants.js';
import { SEFIROT_TEMPLATE } from './sefirot.js';

/**
 * RelationshipGraph - CYNIC learns agent relationships through observation
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
    const PHI_INV_2 = CYNIC_CONSTANTS.OVERRIDE_THRESHOLD; // φ⁻² = 38.2%

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

  /**
   * Get weight between two agents (simplified accessor for router)
   * D1: Closes learning -> routing feedback loop
   * @param {string} from - Source agent name
   * @param {string} to - Target agent name
   * @returns {number} Weight (-1 to 1), or 0 if not found
   */
  getWeight(from, to) {
    const rel = this.getRelationship(from, to);
    return rel ? rel.weight : 0;
  }

  /**
   * Set weight between two agents (used by router to apply learned weights)
   * D1: Accepts Q-Learning/DPO/Thompson weights and stores them
   * @param {string} from - Source agent name
   * @param {string} to - Target agent name
   * @param {number} weight - New weight value (-1 to 1)
   */
  setWeight(from, to, weight) {
    let rel = this.getRelationship(from, to);

    if (!rel) {
      rel = {
        weight: 0,
        source: 'learned',
        interactions: 0,
        positiveOutcomes: 0,
        negativeOutcomes: 0,
        createdAt: Date.now(),
        lastUpdated: Date.now(),
      };
    }

    rel.weight = Math.max(-1, Math.min(1, weight));
    rel.lastUpdated = Date.now();
    if (rel.source !== 'sefirot_template') {
      rel.source = 'learned';
    }

    this._setRelationship(from, to, rel);
  }

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
