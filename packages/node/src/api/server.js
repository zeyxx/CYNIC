/**
 * CYNIC HTTP API Server
 *
 * REST API layer for CYNIC node operations
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/api
 */

'use strict';

import express from 'express';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { setupBurnsRoutes } from './burns-api.js';
import { setupEmergenceRoutes } from './emergence-api.js';
import { setupExplorerRoutes } from './explorer-api.js';
import { setupExplorerUI } from './explorer-ui.js';

/**
 * API Server for CYNIC Node
 */
export class APIServer {
  /**
   * Create API server
   * @param {Object} options - Server options
   * @param {CYNICNode} options.node - CYNIC node instance
   * @param {number} [options.port] - Listen port (default: PORT or CYNIC_API_PORT env, or 3000)
   * @param {string} [options.apiKey] - Optional API key for auth
   */
  constructor(options = {}) {
    this.node = options.node;
    this.port = options.port || parseInt(process.env.PORT || process.env.CYNIC_API_PORT, 10) || 3000;
    this.apiKey = options.apiKey || process.env.CYNIC_API_KEY;
    this.burnsCluster = options.burnsCluster || process.env.SOLANA_CLUSTER || 'https://api.mainnet-beta.solana.com';

    // CORS configuration - whitelist in production, permissive in development
    const corsOrigins = options.corsOrigins || process.env.CYNIC_CORS_ORIGINS;
    this.allowedOrigins = corsOrigins
      ? corsOrigins.split(',').map(o => o.trim())
      : null; // null = allow all (development mode)

    // Express app
    this.app = express();
    this.server = null;
    this.burnsService = null;

    // Configure
    this._configure();
    this._setupBurnsRoutes(); // Burns routes BEFORE main routes (404 handler last)
    this._setupEmergenceRoutes(); // Emergence routes (Layer 7)
    this._setupExplorerRoutes(); // Explorer routes (solscan-style)
    this._setupRoutes();
  }

  /**
   * Configure express middleware
   * @private
   */
  _configure() {
    // JSON parsing
    this.app.use(express.json({ limit: '1mb' }));

    // CORS headers (configurable whitelist in production)
    this.app.use((req, res, next) => {
      const origin = req.headers.origin;
      let allowOrigin = '*';

      if (this.allowedOrigins && this.allowedOrigins.length > 0) {
        // Production: only allow whitelisted origins
        if (origin && this.allowedOrigins.includes(origin)) {
          allowOrigin = origin;
        } else if (origin) {
          // Origin not in whitelist - don't set CORS headers (browser will block)
          return next();
        }
      }

      res.setHeader('Access-Control-Allow-Origin', allowOrigin);
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type, X-API-Key');
      if (req.method === 'OPTIONS') {
        return res.sendStatus(204);
      }
      next();
    });

    // Request timing
    this.app.use((req, res, next) => {
      req.startTime = Date.now();
      res.on('finish', () => {
        const duration = Date.now() - req.startTime;
        if (duration > PHI_INV * 1000) {
          console.warn(`⚠️ Slow request: ${req.method} ${req.path} (${duration}ms)`);
        }
      });
      next();
    });

    // API key auth (if configured)
    if (this.apiKey) {
      this.app.use((req, res, next) => {
        // Skip auth for health endpoint
        if (req.path === '/health' || req.path === '/') {
          return next();
        }

        const providedKey = req.headers['x-api-key'];
        if (!providedKey || providedKey !== this.apiKey) {
          return res.status(401).json({
            error: 'Unauthorized',
            message: 'Invalid or missing API key',
          });
        }
        next();
      });
    }
  }

  /**
   * Setup API routes
   * @private
   */
  _setupRoutes() {
    // Root info
    this.app.get('/', (req, res) => {
      res.json({
        name: 'CYNIC API',
        version: '0.1.0',
        greek: 'κυνικός',
        philosophy: 'φ qui se méfie de φ',
        endpoints: [
          'GET /health',
          'GET /info',
          'GET /consensus/status',
          'POST /judge',
          'GET /merkle/proof/:hash',
          'GET /burns/verify/:signature',
          'GET /burns/stats',
          'POST /burns/verify (batch)',
          'GET /emergence/state',
          'GET /emergence/consciousness',
          'GET /emergence/patterns',
          'GET /emergence/dimensions',
          'GET /emergence/collective',
          'GET /emergence/meta',
          'GET /explorer (overview)',
          'GET /explorer/ui (web interface)',
          'GET /explorer/judgments',
          'GET /explorer/judgment/:id',
          'GET /explorer/blocks',
          'GET /explorer/block/:hashOrSlot',
          'GET /explorer/operators',
          'GET /explorer/operator/:pubkey',
          'GET /explorer/burns',
          'GET /explorer/search?q=',
          'GET /explorer/learning',
          'POST /feedback/:judgmentId',
        ],
      });
    });

    // Health check
    this.app.get('/health', (req, res) => {
      const info = this.node ? this.node.getInfo() : { status: 'NO_NODE' };
      const isHealthy = info.status === 'RUNNING';

      res.status(isHealthy ? 200 : 503).json({
        status: isHealthy ? 'healthy' : 'unhealthy',
        nodeStatus: info.status,
        uptime: info.uptime || 0,
        timestamp: Date.now(),
        phi: {
          maxConfidence: PHI_INV,
          minDoubt: PHI_INV_2,
        },
      });
    });

    // Node info
    this.app.get('/info', (req, res) => {
      if (!this.node) {
        return res.status(503).json({ error: 'Node not available' });
      }
      res.json(this.node.getInfo());
    });

    // Consensus status
    this.app.get('/consensus/status', (req, res) => {
      if (!this.node) {
        return res.status(503).json({ error: 'Node not available' });
      }

      const gossipStats = this.node.gossip.getStats();
      const chainSummary = this.node.state.getSummary();

      res.json({
        height: chainSummary.chain?.height || 0,
        latestBlockHash: chainSummary.chain?.latestHash || null,
        validators: gossipStats.peers || 0,
        pendingJudgments: chainSummary.pendingJudgments || 0,
        consensusThreshold: PHI_INV,
        byzantineTolerance: (1 - PHI_INV).toFixed(3),
      });
    });

    // Submit judgment
    this.app.post('/judge', async (req, res) => {
      if (!this.node) {
        return res.status(503).json({ error: 'Node not available' });
      }

      try {
        const { type, item, context } = req.body;

        // Validate item exists and is a non-null object
        if (!item || typeof item !== 'object' || Array.isArray(item)) {
          return res.status(400).json({
            error: 'Bad Request',
            message: 'Invalid item: must be a non-null object',
          });
        }

        // Validate item has content to judge
        if (Object.keys(item).length === 0) {
          return res.status(400).json({
            error: 'Bad Request',
            message: 'Invalid item: object cannot be empty',
          });
        }

        // Generate request ID
        const requestId = `jdg_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;

        // Submit judgment
        const judgment = await this.node.judge(item, {
          ...context,
          requestId,
          type: type || 'general',
          source: 'api',
        });

        // Build response with optional Final score
        const response = {
          requestId,
          status: 'finalized',
          judgment: {
            globalScore: judgment.global_score,
            qScore: judgment.qScore,
            verdict: judgment.verdict,
            qVerdict: judgment.qVerdict?.verdict,
            confidence: judgment.confidence,
            axiomScores: judgment.axiomScores,
            weaknesses: judgment.weaknesses,
            dimensions: judgment.dimensions,
          },
          height: this.node.state.chain?.height || 0,
          timestamp: Date.now(),
        };

        // Add Final score if K-Score was provided
        if (judgment.finalScore !== undefined) {
          response.judgment.kScore = judgment.kScore;
          response.judgment.finalScore = judgment.finalScore;
          response.judgment.finalVerdict = judgment.finalVerdict?.verdict;
          response.judgment.limiting = judgment.limiting;
        }

        res.status(201).json(response);
      } catch (err) {
        console.error('Judge error:', err);
        res.status(500).json({
          error: 'Internal Server Error',
          message: err.message,
        });
      }
    });

    // K-Score judgment (specialized endpoint)
    this.app.post('/judge/kscore', async (req, res) => {
      if (!this.node) {
        return res.status(503).json({ error: 'Node not available' });
      }

      try {
        const { mint, components } = req.body;

        if (!mint) {
          return res.status(400).json({
            error: 'Bad Request',
            message: 'Missing required field: mint',
          });
        }

        if (!components || typeof components.D !== 'number' || typeof components.O !== 'number' || typeof components.L !== 'number') {
          return res.status(400).json({
            error: 'Bad Request',
            message: 'Missing or invalid components: {D, O, L} required as numbers',
          });
        }

        // Calculate K-Score
        const { D, O, L } = components;
        const kScore = 100 * Math.cbrt(D * O * L);

        // Generate request ID
        const requestId = `ks_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;

        // Submit as judgment
        const judgment = await this.node.judge({
          type: 'kscore',
          mint,
          components,
          calculatedScore: kScore,
        }, {
          requestId,
          type: 'kscore',
          source: 'api',
        });

        res.status(201).json({
          requestId,
          status: 'finalized',
          mint,
          kScore: parseFloat(kScore.toFixed(2)),
          components: {
            D: parseFloat(D.toFixed(4)),
            O: parseFloat(O.toFixed(4)),
            L: parseFloat(L.toFixed(4)),
          },
          judgment: {
            score: judgment.score,
            verdict: judgment.verdict,
            confidence: judgment.confidence,
          },
          height: this.node.state.chain?.height || 0,
          timestamp: Date.now(),
        });
      } catch (err) {
        console.error('K-Score error:', err);
        res.status(500).json({
          error: 'Internal Server Error',
          message: err.message,
        });
      }
    });

    // Merkle proof
    this.app.get('/merkle/proof/:hash', (req, res) => {
      if (!this.node) {
        return res.status(503).json({ error: 'Node not available' });
      }

      try {
        const { hash } = req.params;
        const knowledge = this.node.state.knowledge;

        if (!knowledge) {
          return res.status(503).json({ error: 'Knowledge tree not available' });
        }

        // Get proof from merkle tree
        const proof = knowledge.getProof ? knowledge.getProof(hash) : null;

        if (!proof) {
          return res.status(404).json({
            error: 'Not Found',
            message: 'No proof found for hash',
            hash,
          });
        }

        res.json({
          hash,
          proof,
          root: knowledge.root || null,
          verified: true,
          timestamp: Date.now(),
        });
      } catch (err) {
        console.error('Merkle proof error:', err);
        res.status(500).json({
          error: 'Internal Server Error',
          message: err.message,
        });
      }
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // FEEDBACK ENDPOINT (Gap #3)
    // External feedback → LearningService → Weight adjustment
    // ═══════════════════════════════════════════════════════════════════════════
    this.app.post('/feedback/:judgmentId', async (req, res) => {
      if (!this.node) {
        return res.status(503).json({ error: 'Node not available' });
      }

      try {
        const { judgmentId } = req.params;
        const { feedbackType, source, confidence, details } = req.body;

        // Validate required fields
        if (!judgmentId) {
          return res.status(400).json({
            error: 'Bad Request',
            message: 'Missing judgmentId in path',
          });
        }

        // Validate feedbackType
        const validTypes = ['positive', 'negative', 'correct', 'incorrect'];
        if (!feedbackType || !validTypes.includes(feedbackType)) {
          return res.status(400).json({
            error: 'Bad Request',
            message: `Invalid feedbackType. Must be one of: ${validTypes.join(', ')}`,
          });
        }

        // Access learning service
        const learningService = this.node._learningService;
        if (!learningService) {
          return res.status(503).json({
            error: 'Service Unavailable',
            message: 'Learning service not initialized',
          });
        }

        // Process feedback
        const feedbackResult = learningService.processFeedback({
          judgmentId,
          feedbackType,
          source: source || 'api',
          confidence: typeof confidence === 'number' ? confidence : PHI_INV,
          details: details || {},
          timestamp: Date.now(),
        });

        // Get updated patterns stats
        const stats = learningService.getPatterns?.() || {};

        res.status(200).json({
          status: 'accepted',
          judgmentId,
          feedbackType,
          source: source || 'api',
          message: '*tail wag* Feedback ingested into learning loop',
          learningStats: {
            totalFeedback: stats.overall?.feedbackCount || 0,
            positiveRatio: stats.overall?.positiveRatio || 0,
            anomalyCount: stats.overall?.anomalyCount || 0,
          },
          timestamp: Date.now(),
        });
      } catch (err) {
        console.error('Feedback error:', err);
        res.status(500).json({
          error: 'Internal Server Error',
          message: err.message,
        });
      }
    });

    // 404 handler
    this.app.use((req, res) => {
      res.status(404).json({
        error: 'Not Found',
        message: `Route ${req.method} ${req.path} not found`,
        available: ['/', '/health', '/info', '/consensus/status', '/judge', '/judge/kscore', '/merkle/proof/:hash', '/burns/verify/:signature', '/burns/stats', '/emergence/*', '/explorer', '/explorer/ui', '/explorer/judgments', '/explorer/blocks', '/explorer/operators', '/explorer/burns', '/explorer/search', '/feedback/:judgmentId'],
      });
    });

    // Error handler
    this.app.use((err, req, res, _next) => {
      console.error('API Error:', err);
      res.status(500).json({
        error: 'Internal Server Error',
        message: err.message,
      });
    });
  }

  /**
   * Setup burns verification routes
   * @private
   */
  _setupBurnsRoutes() {
    this.burnsService = setupBurnsRoutes(this.app, {
      cluster: this.burnsCluster,
    });
    console.log(`🔥 Burns API enabled (${this.burnsCluster})`);
  }

  /**
   * Setup emergence layer routes (Layer 7)
   * @private
   */
  _setupEmergenceRoutes() {
    if (this.node) {
      setupEmergenceRoutes(this.app, { node: this.node });
    }
  }

  /**
   * Setup explorer routes (solscan-style)
   * @private
   */
  _setupExplorerRoutes() {
    if (this.node) {
      setupExplorerRoutes(this.app, { node: this.node });
      setupExplorerUI(this.app);
    }
  }

  /**
   * Start the server
   * @returns {Promise<{port: number, url: string}>}
   */
  async start() {
    return new Promise((resolve, reject) => {
      try {
        this.server = this.app.listen(this.port, () => {
          const url = `http://localhost:${this.port}`;
          console.log(`🐕 CYNIC API listening on ${url}`);
          resolve({ port: this.port, url });
        });

        this.server.on('error', (err) => {
          if (err.code === 'EADDRINUSE') {
            reject(new Error(`Port ${this.port} already in use`));
          } else {
            reject(err);
          }
        });
      } catch (err) {
        reject(err);
      }
    });
  }

  /**
   * Stop the server
   * @returns {Promise<void>}
   */
  async stop() {
    return new Promise((resolve) => {
      if (this.server) {
        this.server.close(() => {
          console.log('🐕 CYNIC API stopped');
          this.server = null;
          resolve();
        });
      } else {
        resolve();
      }
    });
  }
}

export default APIServer;
