/**
 * CYNIC Daemon Server
 *
 * The independent runtime. Boots once, runs always.
 * Thin hooks delegate to this via HTTP — no more re-initialization per prompt.
 *
 * "Le chien ne vit plus dans le collier" - CYNIC
 *
 * @module @cynic/node/daemon
 */

'use strict';

import express from 'express';
import { createLogger, PHI_INV, processRegistry } from '@cynic/core';
import { handleHookEvent } from './hook-handlers.js';
import { setupLLMEndpoints } from './llm-endpoints.js';

const log = createLogger('Daemon');

/** Default port: 6180 (φ × 3820 ≈ 6180) */
const DEFAULT_PORT = 6180;

/** Rate limit: 100 requests per φ⁻¹ × 100s = 61.8s */
const RATE_LIMIT_WINDOW = Math.round(PHI_INV * 100_000); // ~61800ms
const RATE_LIMIT_MAX = 100;

/**
 * CYNIC Daemon HTTP Server
 *
 * Serves hook requests, health checks, and status queries.
 * All singletons live in-memory — hooks get instant responses.
 */
export class DaemonServer {
  /**
   * @param {Object} [options]
   * @param {number} [options.port=6180] - HTTP port
   * @param {string} [options.host='127.0.0.1'] - Bind host
   */
  constructor(options = {}) {
    this.port = options.port ?? DEFAULT_PORT;
    this.host = options.host ?? '127.0.0.1';
    this.app = express();
    this.server = null;
    this.startTime = null;
    this._requestCounts = new Map();
    this._rateLimitTimer = null;

    this._configure();
    this._setupRoutes();
  }

  /**
   * Configure Express middleware
   * @private
   */
  _configure() {
    // JSON parsing — hooks send small payloads
    this.app.use(express.json({ limit: '512kb' }));

    // Request timing
    this.app.use((req, res, next) => {
      req.startTime = Date.now();
      res.on('finish', () => {
        const duration = Date.now() - req.startTime;
        if (duration > 500) {
          log.warn('Slow request', { method: req.method, path: req.path, duration });
        }
      });
      next();
    });

    // Rate limiting (simple in-memory counter)
    this.app.use((req, res, next) => {
      const key = req.ip || 'local';
      const count = this._requestCounts.get(key) || 0;
      if (count >= RATE_LIMIT_MAX) {
        return res.status(429).json({ error: 'Rate limited', retryAfter: RATE_LIMIT_WINDOW / 1000 });
      }
      this._requestCounts.set(key, count + 1);
      next();
    });
  }

  /**
   * Setup route handlers
   * @private
   */
  _setupRoutes() {
    // Hook event handler — the main entry point for thin hooks
    this.app.post('/hook/:event', async (req, res) => {
      const { event } = req.params;
      const hookInput = req.body;

      try {
        const result = await handleHookEvent(event, hookInput);
        res.json(result);
      } catch (err) {
        log.error('Hook handler error', { event, error: err.message });
        res.status(500).json({ error: err.message, continue: true });
      }
    });

    // Health check
    this.app.get('/health', (req, res) => {
      const uptime = this.startTime ? Date.now() - this.startTime : 0;

      res.json({
        status: 'healthy',
        pid: process.pid,
        uptime,
        uptimeHuman: formatUptime(uptime),
        memoryMB: Math.round(process.memoryUsage().heapUsed / 1024 / 1024),
        port: this.port,
      });
    });

    // Full status — ProcessRegistry snapshot
    this.app.get('/status', async (req, res) => {
      try {
        const processes = processRegistry.discover();
        const uptime = this.startTime ? Date.now() - this.startTime : 0;

        res.json({
          daemon: {
            pid: process.pid,
            uptime,
            uptimeHuman: formatUptime(uptime),
            memoryMB: Math.round(process.memoryUsage().heapUsed / 1024 / 1024),
            port: this.port,
          },
          processes,
        });
      } catch (err) {
        res.status(500).json({ error: err.message });
      }
    });

    // LLM endpoints — Phase 2: CYNIC calls LLMs directly
    setupLLMEndpoints(this.app);
  }

  /**
   * Start the daemon server
   * @returns {Promise<void>}
   */
  async start() {
    if (this.server) {
      throw new Error('Daemon already running');
    }

    return new Promise((resolve, reject) => {
      this.server = this.app.listen(this.port, this.host, () => {
        this.startTime = Date.now();

        // Start rate limit reset timer
        this._rateLimitTimer = setInterval(() => {
          this._requestCounts.clear();
        }, RATE_LIMIT_WINDOW);
        this._rateLimitTimer.unref();

        // Announce to ProcessRegistry
        try {
          processRegistry.announce({
            mode: 'daemon',
            endpoint: `http://${this.host}:${this.port}`,
            capabilities: ['hooks', 'health', 'status', 'llm'],
            meta: { version: '0.1.0' },
          });
        } catch (err) {
          log.warn('ProcessRegistry announce failed (non-fatal)', { error: err.message });
        }

        log.info(`Daemon listening on ${this.host}:${this.port}`);
        resolve();
      });

      this.server.on('error', (err) => {
        if (err.code === 'EADDRINUSE') {
          log.warn(`Port ${this.port} in use — daemon may already be running`);
          reject(new Error(`Port ${this.port} already in use`));
        } else {
          reject(err);
        }
      });
    });
  }

  /**
   * Stop the daemon server gracefully
   * @returns {Promise<void>}
   */
  async stop() {
    if (this._rateLimitTimer) {
      clearInterval(this._rateLimitTimer);
      this._rateLimitTimer = null;
    }

    // Depart from ProcessRegistry
    try {
      processRegistry.depart();
    } catch (err) {
      log.debug('ProcessRegistry depart failed', { error: err.message });
    }

    if (!this.server) return;

    return new Promise((resolve) => {
      this.server.close(() => {
        this.server = null;
        this.startTime = null;
        log.info('Daemon stopped');
        resolve();
      });
    });
  }
}

/**
 * Format uptime in human-readable form
 * @param {number} ms - Milliseconds
 * @returns {string}
 */
function formatUptime(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

export default DaemonServer;
