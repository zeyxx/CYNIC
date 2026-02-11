/**
 * CrossScaleRouter — φ-bounded cross-domain feedback routing
 *
 * When one domain acts, other domains may be affected.
 * The router propagates feedback signals between domains via the event bus,
 * weighted by the influence matrix (φ⁻¹ bounded).
 *
 * Example: solana:action → affects CODE (implementation), MARKET (price), COSMOS (ecosystem)
 *
 * The influence matrix defines:
 *   source domain × target domain → weight (0..φ⁻¹)
 *
 * Router listens on `{domain}:action` events, propagates `feedback:{target}` events
 * with attenuated signals — no domain ever dominates another beyond φ⁻¹.
 *
 * "Les fils invisibles entre les mondes" — κυνικός
 *
 * @module @cynic/node/services/cross-scale-router
 */

'use strict';

import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

const log = createLogger('CrossScaleRouter');

// =============================================================================
// DEFAULT INFLUENCE MATRIX
// =============================================================================

/**
 * Default cross-domain influence weights.
 * Each entry: [source, target, weight, reason]
 * Weight capped at φ⁻¹ = 0.618
 *
 * Read as: "When SOURCE acts, TARGET is affected with WEIGHT influence."
 */
const DEFAULT_INFLUENCES = [
  // CODE affects...
  ['code', 'solana', 0.4, 'code changes may affect on-chain programs'],
  ['code', 'cosmos', 0.3, 'code changes affect ecosystem coherence'],
  ['code', 'cynic', 0.2, 'code changes alter self-state'],

  // SOLANA affects...
  ['solana', 'code', 0.5, 'blockchain state informs code decisions'],
  ['solana', 'market', 0.618, 'on-chain activity directly impacts markets'],
  ['solana', 'cosmos', 0.3, 'blockchain state affects ecosystem'],

  // MARKET affects...
  ['market', 'solana', 0.4, 'market conditions inform transaction decisions'],
  ['market', 'social', 0.5, 'price movements drive social activity'],
  ['market', 'human', 0.3, 'market stress affects user psychology'],

  // SOCIAL affects...
  ['social', 'market', 0.3, 'social sentiment influences market'],
  ['social', 'human', 0.4, 'community engagement affects user state'],
  ['social', 'cosmos', 0.2, 'social signals enrich ecosystem view'],

  // HUMAN affects...
  ['human', 'code', 0.3, 'user energy affects code quality decisions'],
  ['human', 'cynic', 0.5, 'user state shapes CYNIC behavior'],
  ['human', 'cosmos', 0.2, 'user patterns inform ecosystem'],

  // CYNIC affects...
  ['cynic', 'code', 0.3, 'self-state influences code decisions'],
  ['cynic', 'human', 0.4, 'CYNIC behavior affects user experience'],
  ['cynic', 'cosmos', 0.3, 'self-evolution propagates to ecosystem'],

  // COSMOS affects...
  ['cosmos', 'code', 0.4, 'ecosystem insights inform code changes'],
  ['cosmos', 'solana', 0.3, 'ecosystem coherence affects blockchain actions'],
  ['cosmos', 'cynic', 0.3, 'ecosystem state shapes self-awareness'],
];

// =============================================================================
// ROUTER
// =============================================================================

/**
 * Create a CrossScaleRouter.
 *
 * @param {Object} [options]
 * @param {Array} [options.influences] - Custom influence tuples [source, target, weight, reason]
 * @param {number} [options.minWeight] - Minimum weight to propagate (default: 0.1)
 * @param {number} [options.maxPropagationDepth] - Max chain depth (default: 2)
 * @param {number} [options.cooldownMs] - Min time between feedback to same target (default: 5000)
 * @returns {Object} Router with start(), stop(), getStats(), getMatrix()
 */
export function createCrossScaleRouter(options = {}) {
  const {
    influences = DEFAULT_INFLUENCES,
    minWeight = 0.1,
    maxPropagationDepth = 2,
    cooldownMs = 5000,
  } = options;

  // Build influence matrix: source → [{ target, weight, reason }]
  const _matrix = buildMatrix(influences);
  const _unsubscribers = [];
  let _started = false;

  // Cooldown tracking: `${source}→${target}` → last propagation timestamp
  const _lastPropagation = new Map();

  // Stats
  const _stats = {
    feedbacksPropagated: 0,
    feedbacksBlocked: 0, // below minWeight
    feedbacksCooledDown: 0, // too recent
    chainsPropagated: 0,
    startedAt: null,
  };

  // Per-route stats
  const _routeStats = new Map();

  return {
    /**
     * Start listening for domain actions and propagating feedback.
     *
     * @param {string[]} [domains] - Domains to listen on (default: all sources in matrix)
     */
    start(domains) {
      if (_started) return;
      _started = true;
      _stats.startedAt = Date.now();

      const sourceDomains = domains || [..._matrix.keys()];

      for (const source of sourceDomains) {
        // Listen on domain:action events
        const unsub = globalEventBus.subscribe(`${source}:action`, (event) => {
          propagateFromSource(source, event, 0);
        });
        _unsubscribers.push(unsub);

        // Also listen on domain:judgment for early feedback (lighter weight)
        const unsubJ = globalEventBus.subscribe(`${source}:judgment`, (event) => {
          propagateFromSource(source, event, 0, 0.5); // Half-weight for judgments
        });
        _unsubscribers.push(unsubJ);
      }

      log.debug('CrossScaleRouter started', { domains: sourceDomains.length });
    },

    /**
     * Stop the router.
     */
    stop() {
      for (const unsub of _unsubscribers) {
        if (typeof unsub === 'function') unsub();
      }
      _unsubscribers.length = 0;
      _started = false;
    },

    /**
     * Get the influence matrix.
     *
     * @returns {Map} source → [{ target, weight, reason }]
     */
    getMatrix() {
      return new Map(_matrix);
    },

    /**
     * Get the influence weight between two domains.
     *
     * @param {string} source
     * @param {string} target
     * @returns {number} Weight (0 if no influence)
     */
    getInfluence(source, target) {
      const targets = _matrix.get(source);
      if (!targets) return 0;
      const entry = targets.find(t => t.target === target);
      return entry ? entry.weight : 0;
    },

    /**
     * Update a single influence weight (for learning).
     *
     * @param {string} source
     * @param {string} target
     * @param {number} newWeight - New weight (capped at φ⁻¹)
     */
    updateInfluence(source, target, newWeight) {
      const capped = Math.min(PHI_INV, Math.max(0, newWeight));
      if (!_matrix.has(source)) _matrix.set(source, []);
      const targets = _matrix.get(source);
      const existing = targets.find(t => t.target === target);
      if (existing) {
        existing.weight = capped;
      } else {
        targets.push({ target, weight: capped, reason: 'learned' });
      }
    },

    /**
     * Reinforce a route based on positive outcome.
     * Increases weight by φ⁻² * (1 - current_weight) — diminishing returns.
     *
     * @param {string} source
     * @param {string} target
     */
    reinforce(source, target) {
      const current = this.getInfluence(source, target);
      const boost = PHI_INV_2 * (PHI_INV - current); // Diminishing: approaches φ⁻¹ asymptotically
      this.updateInfluence(source, target, current + boost);

      const routeKey = `${source}→${target}`;
      const rs = _routeStats.get(routeKey) || { propagations: 0, reinforcements: 0, decays: 0 };
      rs.reinforcements++;
      _routeStats.set(routeKey, rs);
    },

    /**
     * Decay a route based on negative outcome.
     * Decreases weight by φ⁻² * current_weight — diminishing returns.
     *
     * @param {string} source
     * @param {string} target
     */
    decay(source, target) {
      const current = this.getInfluence(source, target);
      const loss = PHI_INV_2 * current;
      this.updateInfluence(source, target, current - loss);

      const routeKey = `${source}→${target}`;
      const rs = _routeStats.get(routeKey) || { propagations: 0, reinforcements: 0, decays: 0 };
      rs.decays++;
      _routeStats.set(routeKey, rs);
    },

    /**
     * Get stats.
     */
    getStats() {
      return {
        ..._stats,
        started: _started,
        routeCount: countRoutes(_matrix),
        routeStats: Object.fromEntries(_routeStats),
      };
    },

    /**
     * Get compact influence summary.
     *
     * @returns {Object[]} Routes sorted by weight (highest first)
     */
    getSummary() {
      const routes = [];
      for (const [source, targets] of _matrix) {
        for (const { target, weight, reason } of targets) {
          const routeKey = `${source}→${target}`;
          const rs = _routeStats.get(routeKey) || { propagations: 0, reinforcements: 0, decays: 0 };
          routes.push({ source, target, weight, reason, ...rs });
        }
      }
      return routes.sort((a, b) => b.weight - a.weight);
    },

    /**
     * Check if started.
     */
    isStarted() {
      return _started;
    },

    /**
     * Reset all stats and cooldowns.
     */
    reset() {
      _stats.feedbacksPropagated = 0;
      _stats.feedbacksBlocked = 0;
      _stats.feedbacksCooledDown = 0;
      _stats.chainsPropagated = 0;
      _lastPropagation.clear();
      _routeStats.clear();
    },
  };

  // ═══════════════════════════════════════════════════════════════
  // INTERNAL
  // ═══════════════════════════════════════════════════════════════

  function propagateFromSource(source, event, depth, weightMultiplier = 1.0) {
    if (depth >= maxPropagationDepth) return;

    const targets = _matrix.get(source);
    if (!targets) return;

    const payload = event.payload || event;
    const now = Date.now();

    for (const { target, weight } of targets) {
      const effectiveWeight = weight * weightMultiplier;

      // Skip below threshold
      if (effectiveWeight < minWeight) {
        _stats.feedbacksBlocked++;
        continue;
      }

      // Cooldown check
      const routeKey = `${source}→${target}`;
      const lastTime = _lastPropagation.get(routeKey) || 0;
      if (now - lastTime < cooldownMs) {
        _stats.feedbacksCooledDown++;
        continue;
      }

      _lastPropagation.set(routeKey, now);

      // Propagate feedback
      globalEventBus.publish(`feedback:${target}`, {
        fromDomain: source,
        toDomain: target,
        weight: effectiveWeight,
        signal: payload,
        depth,
        originalEvent: event.type || source,
      }, { source: 'cross-scale-router' });

      _stats.feedbacksPropagated++;

      // Track per-route
      const rs = _routeStats.get(routeKey) || { propagations: 0, reinforcements: 0, decays: 0 };
      rs.propagations++;
      _routeStats.set(routeKey, rs);

      // Chain propagation (attenuated by φ⁻¹)
      if (depth + 1 < maxPropagationDepth) {
        _stats.chainsPropagated++;
        propagateFromSource(target, event, depth + 1, effectiveWeight * PHI_INV);
      }
    }
  }
}

// =============================================================================
// HELPERS
// =============================================================================

function buildMatrix(influences) {
  const matrix = new Map();
  for (const [source, target, weight, reason] of influences) {
    if (!matrix.has(source)) matrix.set(source, []);
    matrix.get(source).push({
      target,
      weight: Math.min(PHI_INV, weight),
      reason,
    });
  }
  return matrix;
}

function countRoutes(matrix) {
  let count = 0;
  for (const targets of matrix.values()) {
    count += targets.length;
  }
  return count;
}

export { DEFAULT_INFLUENCES };
