/**
 * Emergence API Routes
 *
 * "The crown observes all" - κυνικός
 *
 * REST endpoints for Layer 7 (Emergence) operations:
 * - Consciousness state
 * - Pattern detection
 * - Dimension discovery
 * - Collective network state
 *
 * @module @cynic/node/api/emergence
 */

'use strict';

import { PHI_INV } from '@cynic/core';

/**
 * Setup emergence routes on Express app
 *
 * @param {express.Application} app - Express app
 * @param {Object} options - Options
 * @param {CYNICNode} options.node - CYNIC node instance
 */
export function setupEmergenceRoutes(app, options = {}) {
  const { node } = options;

  // ═══════════════════════════════════════════════════════════════════════════
  // GET /emergence/state - Full emergence state
  // ═══════════════════════════════════════════════════════════════════════════
  app.get('/emergence/state', (req, res) => {
    if (!node || !node.emergence) {
      return res.status(503).json({
        error: 'Service Unavailable',
        message: 'Emergence layer not available',
      });
    }

    try {
      const state = node.emergence.getState();
      res.json({
        ...state,
        phi: {
          maxConfidence: PHI_INV,
          note: 'φ distrusts φ',
        },
      });
    } catch (err) {
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // GET /emergence/consciousness - Consciousness state only
  // ═══════════════════════════════════════════════════════════════════════════
  app.get('/emergence/consciousness', (req, res) => {
    if (!node || !node.emergence) {
      return res.status(503).json({
        error: 'Service Unavailable',
        message: 'Emergence layer not available',
      });
    }

    try {
      const state = node.emergence.getState();
      res.json({
        nodeId: state.nodeId,
        consciousness: state.consciousness,
        timestamp: Date.now(),
      });
    } catch (err) {
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // GET /emergence/patterns - Detected patterns
  // ═══════════════════════════════════════════════════════════════════════════
  app.get('/emergence/patterns', (req, res) => {
    if (!node || !node.emergence) {
      return res.status(503).json({
        error: 'Service Unavailable',
        message: 'Emergence layer not available',
      });
    }

    try {
      const state = node.emergence.getState();
      const limit = parseInt(req.query.limit, 10) || 10;

      res.json({
        nodeId: state.nodeId,
        patterns: {
          total: state.patterns.detected,
          top: state.patterns.top.slice(0, limit),
          stats: state.patterns.stats,
        },
        timestamp: Date.now(),
      });
    } catch (err) {
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // GET /emergence/dimensions - Dimension discovery state
  // ═══════════════════════════════════════════════════════════════════════════
  app.get('/emergence/dimensions', (req, res) => {
    if (!node || !node.emergence) {
      return res.status(503).json({
        error: 'Service Unavailable',
        message: 'Emergence layer not available',
      });
    }

    try {
      const state = node.emergence.getState();
      res.json({
        nodeId: state.nodeId,
        dimensions: state.dimensions,
        timestamp: Date.now(),
      });
    } catch (err) {
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // GET /emergence/collective - Collective network state
  // ═══════════════════════════════════════════════════════════════════════════
  app.get('/emergence/collective', (req, res) => {
    if (!node || !node.emergence) {
      return res.status(503).json({
        error: 'Service Unavailable',
        message: 'Emergence layer not available',
      });
    }

    try {
      const state = node.emergence.getState();
      res.json({
        nodeId: state.nodeId,
        collective: state.collective,
        timestamp: Date.now(),
      });
    } catch (err) {
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // GET /emergence/meta - Meta-insight (deep self-reflection)
  // ═══════════════════════════════════════════════════════════════════════════
  app.get('/emergence/meta', (req, res) => {
    if (!node || !node.emergence) {
      return res.status(503).json({
        error: 'Service Unavailable',
        message: 'Emergence layer not available',
      });
    }

    try {
      const metaInsight = node.emergence.getMetaInsight();
      res.json({
        nodeId: node.emergence.nodeId,
        metaInsight,
        philosophy: 'φ distrusts φ',
      });
    } catch (err) {
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    }
  });

  console.log('🧠 Emergence API routes enabled');
}

export default { setupEmergenceRoutes };
