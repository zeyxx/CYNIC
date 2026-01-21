/**
 * Reputation Graph - Trust Relationships Between Nodes
 *
 * "Trust is a web, not a chain" - κυνικός
 *
 * Tracks trust relationships between CYNIC nodes:
 * - Direct trust (explicit vouching)
 * - Indirect trust (transitive via trusted nodes)
 * - Distrust (explicit warnings)
 *
 * Trust is weighted by E-Score - high reputation nodes'
 * trust carries more weight.
 *
 * @module @cynic/identity/reputation
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Trust levels
 */
export const TrustLevel = {
  DISTRUST: -1,     // Explicit distrust (warning)
  UNKNOWN: 0,       // No relationship
  WEAK: 0.382,      // φ⁻² - Weak trust
  MODERATE: 0.618,  // φ⁻¹ - Moderate trust
  STRONG: 1.0,      // Full trust (verified vouching)
};

/**
 * Trust decay rate per day (φ⁻¹ = 61.8% retained)
 */
export const TRUST_DECAY_RATE = PHI_INV;

/**
 * Maximum trust propagation depth
 */
export const MAX_PROPAGATION_DEPTH = 3;

/**
 * Trust relationship
 * @typedef {Object} TrustRelation
 * @property {string} from - Trusting node ID
 * @property {string} to - Trusted node ID
 * @property {number} level - Trust level
 * @property {number} timestamp - When relationship was established
 * @property {string} [reason] - Reason for trust/distrust
 * @property {string} [signature] - Signature from trusting node
 */

/**
 * Reputation Graph
 *
 * Manages a web of trust between CYNIC nodes.
 *
 * @example
 * ```javascript
 * const graph = new ReputationGraph();
 *
 * // Node A trusts Node B
 * graph.setTrust('node_a', 'node_b', TrustLevel.STRONG, 'Consistent good judgments');
 *
 * // Node B trusts Node C
 * graph.setTrust('node_b', 'node_c', TrustLevel.MODERATE);
 *
 * // Check trust from A to C (transitive)
 * const trust = graph.getTrust('node_a', 'node_c');
 * console.log(trust); // ~0.382 (propagated with decay)
 *
 * // Get reputation of Node C from perspective of Node A
 * const rep = graph.getReputation('node_c', 'node_a');
 * ```
 */
export class ReputationGraph {
  /**
   * @param {Object} options - Configuration
   * @param {number} [options.decayRate] - Trust decay per day
   * @param {number} [options.maxDepth] - Max propagation depth
   * @param {Function} [options.getEScore] - Function to get node E-Score
   */
  constructor(options = {}) {
    this.decayRate = options.decayRate ?? TRUST_DECAY_RATE;
    this.maxDepth = options.maxDepth ?? MAX_PROPAGATION_DEPTH;
    this.getEScore = options.getEScore || (() => 50); // Default E-Score

    // Adjacency list: nodeId -> Map<targetId, TrustRelation>
    this.graph = new Map();

    // Reverse index: nodeId -> Set<nodesWhoTrustMe>
    this.reversIndex = new Map();

    // Stats
    this.stats = {
      totalRelations: 0,
      trustRelations: 0,
      distrustRelations: 0,
    };
  }

  /**
   * Set trust relationship
   *
   * @param {string} from - Trusting node ID
   * @param {string} to - Trusted node ID
   * @param {number} level - Trust level
   * @param {string} [reason] - Reason for trust
   * @param {string} [signature] - Signature from trusting node
   * @returns {TrustRelation} The relationship
   */
  setTrust(from, to, level, reason = null, signature = null) {
    // Normalize level
    level = Math.max(-1, Math.min(1, level));

    // Get or create adjacency map for 'from'
    if (!this.graph.has(from)) {
      this.graph.set(from, new Map());
    }

    // Get or create reverse index for 'to'
    if (!this.reversIndex.has(to)) {
      this.reversIndex.set(to, new Set());
    }

    const relations = this.graph.get(from);
    const existing = relations.get(to);

    const relation = {
      from,
      to,
      level,
      timestamp: Date.now(),
      reason,
      signature,
    };

    relations.set(to, relation);
    this.reversIndex.get(to).add(from);

    // Update stats
    if (!existing) {
      this.stats.totalRelations++;
    }

    if (level > 0) {
      this.stats.trustRelations++;
      if (existing && existing.level < 0) {
        this.stats.distrustRelations--;
      }
    } else if (level < 0) {
      this.stats.distrustRelations++;
      if (existing && existing.level > 0) {
        this.stats.trustRelations--;
      }
    }

    return relation;
  }

  /**
   * Remove trust relationship
   *
   * @param {string} from - Trusting node ID
   * @param {string} to - Trusted node ID
   * @returns {boolean} True if removed
   */
  removeTrust(from, to) {
    const relations = this.graph.get(from);
    if (!relations) return false;

    const existing = relations.get(to);
    if (!existing) return false;

    relations.delete(to);
    this.reversIndex.get(to)?.delete(from);

    // Update stats
    this.stats.totalRelations--;
    if (existing.level > 0) {
      this.stats.trustRelations--;
    } else if (existing.level < 0) {
      this.stats.distrustRelations--;
    }

    return true;
  }

  /**
   * Get direct trust level
   *
   * @param {string} from - Trusting node ID
   * @param {string} to - Trusted node ID
   * @returns {number} Trust level (or 0 if unknown)
   */
  getDirectTrust(from, to) {
    const relation = this.graph.get(from)?.get(to);
    if (!relation) return TrustLevel.UNKNOWN;

    // Apply time decay
    const ageInDays = (Date.now() - relation.timestamp) / (24 * 60 * 60 * 1000);
    const decayFactor = Math.pow(this.decayRate, ageInDays);

    return relation.level * decayFactor;
  }

  /**
   * Get trust level (including transitive trust)
   *
   * Uses BFS to find trust paths and combines them.
   *
   * @param {string} from - Trusting node ID
   * @param {string} to - Trusted node ID
   * @param {number} [maxDepth] - Maximum path depth
   * @returns {number} Combined trust level
   */
  getTrust(from, to, maxDepth = this.maxDepth) {
    if (from === to) return TrustLevel.STRONG; // Trust yourself

    // Check direct trust first
    const direct = this.getDirectTrust(from, to);
    if (direct !== TrustLevel.UNKNOWN) {
      return direct;
    }

    if (maxDepth <= 1) return TrustLevel.UNKNOWN;

    // BFS for transitive trust
    const visited = new Set([from]);
    const queue = [{ nodeId: from, trust: 1.0, depth: 0 }];
    let maxFoundTrust = TrustLevel.UNKNOWN;

    while (queue.length > 0) {
      const { nodeId, trust, depth } = queue.shift();

      if (depth >= maxDepth) continue;

      const relations = this.graph.get(nodeId);
      if (!relations) continue;

      for (const [targetId, relation] of relations) {
        if (visited.has(targetId)) continue;

        // Calculate propagated trust
        // Trust decays by φ⁻¹ per hop
        const directTrust = this.getDirectTrust(nodeId, targetId);
        if (directTrust <= 0) continue; // Don't propagate through distrust

        const propagatedTrust = trust * directTrust * PHI_INV;

        if (targetId === to) {
          maxFoundTrust = Math.max(maxFoundTrust, propagatedTrust);
        } else if (propagatedTrust > 0.01) { // Threshold for continuing
          visited.add(targetId);
          queue.push({
            nodeId: targetId,
            trust: propagatedTrust,
            depth: depth + 1,
          });
        }
      }
    }

    return maxFoundTrust;
  }

  /**
   * Get reputation of a node from a perspective
   *
   * Combines:
   * - Direct trust from perspective node
   * - Weighted trust from nodes that perspective trusts
   * - Node's E-Score as base
   *
   * @param {string} nodeId - Node to get reputation for
   * @param {string} [perspectiveId] - Perspective node (or global if null)
   * @returns {Object} Reputation info
   */
  getReputation(nodeId, perspectiveId = null) {
    const baseEScore = this.getEScore(nodeId);

    if (!perspectiveId) {
      // Global reputation = E-Score + average trust from all
      const trusters = this.reversIndex.get(nodeId);
      if (!trusters || trusters.size === 0) {
        return {
          score: baseEScore,
          trust: TrustLevel.UNKNOWN,
          trusters: 0,
          source: 'escore_only',
        };
      }

      let totalTrust = 0;
      let weightedTrust = 0;

      for (const trusterId of trusters) {
        const trust = this.getDirectTrust(trusterId, nodeId);
        const trusterEScore = this.getEScore(trusterId);

        totalTrust += 1;
        weightedTrust += trust * (trusterEScore / 100);
      }

      const avgTrust = weightedTrust / totalTrust;

      return {
        score: Math.min(100, baseEScore * (1 + avgTrust * 0.5)),
        trust: avgTrust,
        trusters: trusters.size,
        source: 'global',
      };
    }

    // Perspective-specific reputation
    const directTrust = this.getTrust(perspectiveId, nodeId);

    return {
      score: baseEScore,
      trust: directTrust,
      perspective: perspectiveId,
      source: directTrust !== 0 ? 'trust' : 'escore_only',
    };
  }

  /**
   * Get nodes trusted by a node
   *
   * @param {string} nodeId - Node ID
   * @param {number} [minTrust=0] - Minimum trust level
   * @returns {Array<{nodeId: string, trust: number}>}
   */
  getTrustedBy(nodeId, minTrust = 0) {
    const relations = this.graph.get(nodeId);
    if (!relations) return [];

    const result = [];
    for (const [targetId, relation] of relations) {
      const trust = this.getDirectTrust(nodeId, targetId);
      if (trust >= minTrust) {
        result.push({ nodeId: targetId, trust, relation });
      }
    }

    return result.sort((a, b) => b.trust - a.trust);
  }

  /**
   * Get nodes that trust a node
   *
   * @param {string} nodeId - Node ID
   * @param {number} [minTrust=0] - Minimum trust level
   * @returns {Array<{nodeId: string, trust: number}>}
   */
  getTrusters(nodeId, minTrust = 0) {
    const trusters = this.reversIndex.get(nodeId);
    if (!trusters) return [];

    const result = [];
    for (const trusterId of trusters) {
      const trust = this.getDirectTrust(trusterId, nodeId);
      if (trust >= minTrust) {
        result.push({ nodeId: trusterId, trust });
      }
    }

    return result.sort((a, b) => b.trust - a.trust);
  }

  /**
   * Find trust path between nodes
   *
   * @param {string} from - Start node
   * @param {string} to - End node
   * @param {number} [maxDepth] - Maximum path length
   * @returns {Array<string>|null} Path or null if not found
   */
  findTrustPath(from, to, maxDepth = this.maxDepth) {
    if (from === to) return [from];

    const visited = new Set([from]);
    const queue = [[from]];

    while (queue.length > 0) {
      const path = queue.shift();

      if (path.length > maxDepth) continue;

      const current = path[path.length - 1];
      const relations = this.graph.get(current);

      if (!relations) continue;

      for (const [targetId, relation] of relations) {
        if (visited.has(targetId)) continue;
        if (relation.level <= 0) continue; // Only positive trust

        const newPath = [...path, targetId];

        if (targetId === to) {
          return newPath;
        }

        visited.add(targetId);
        queue.push(newPath);
      }
    }

    return null;
  }

  /**
   * Get graph statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      nodes: this.graph.size,
      avgRelationsPerNode: this.stats.totalRelations / Math.max(1, this.graph.size),
    };
  }

  /**
   * Export graph for persistence
   * @returns {Object}
   */
  export() {
    const relations = [];

    for (const [from, targets] of this.graph) {
      for (const [to, relation] of targets) {
        relations.push(relation);
      }
    }

    return {
      relations,
      exportedAt: Date.now(),
    };
  }

  /**
   * Import graph from persistence
   * @param {Object} data - Exported data
   */
  import(data) {
    if (!data.relations) return;

    for (const relation of data.relations) {
      this.setTrust(
        relation.from,
        relation.to,
        relation.level,
        relation.reason,
        relation.signature
      );

      // Restore original timestamp
      const rel = this.graph.get(relation.from)?.get(relation.to);
      if (rel) {
        rel.timestamp = relation.timestamp;
      }
    }
  }

  /**
   * Clear all relationships
   */
  clear() {
    this.graph.clear();
    this.reversIndex.clear();
    this.stats = {
      totalRelations: 0,
      trustRelations: 0,
      distrustRelations: 0,
    };
  }
}

/**
 * Create a ReputationGraph instance
 * @param {Object} [options] - Configuration
 * @returns {ReputationGraph}
 */
export function createReputationGraph(options = {}) {
  return new ReputationGraph(options);
}

export default {
  ReputationGraph,
  createReputationGraph,
  TrustLevel,
  TRUST_DECAY_RATE,
  MAX_PROPAGATION_DEPTH,
};
