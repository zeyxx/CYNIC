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
import v8 from 'v8';
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

    // Health check (enhanced with watchdog + services data)
    this.app.get('/health', async (req, res) => {
      const uptime = this.startTime ? Date.now() - this.startTime : 0;
      const mem = process.memoryUsage();

      // Get heap size limit from V8
      const heapStats = v8.getHeapStatistics();
      const heapSizeLimit = heapStats.heap_size_limit;

      const health = {
        status: 'healthy',
        running: true,
        pid: process.pid,
        uptime,
        uptimeHuman: formatUptime(uptime),
        memoryMB: Math.round(mem.heapUsed / 1024 / 1024),
        heapUsedPercent: Math.round((mem.heapUsed / heapSizeLimit) * 1000) / 10,
        heapTotalMB: Math.round(mem.heapTotal / 1024 / 1024),
        heapLimitMB: Math.round(heapSizeLimit / 1024 / 1024),
        port: this.port,
      };

      // Watchdog enrichment (if running)
      if (this.watchdog) {
        const wdStatus = this.watchdog.getStatus();
        health.status = wdStatus.level || 'healthy';
        health.eventLoopLatencyMs = wdStatus.eventLoopLatencyMs;
        health.degradedSubsystems = wdStatus.degradedSubsystems;
        health.watchdogChecks = wdStatus.checkCount;
        health.consecutiveCritical = wdStatus.consecutiveCritical;
      }

      // Services status (from this.services set by entry.js)
      try {
        health.services = {
          llmRouter: !!this.services?.llmRouter,
          costLedger: !!this.services?.costLedger,
          learningPipeline: !!this.services?.learningPipeline,
          dataPipeline: !!this.services?.dataPipeline,
          researchRunner: !!this.services?.researchRunner,
        };

        // Budget status
        if (this.services?.costLedger) {
          const budget = await this.services.costLedger.getBudgetState();
          health.budget = {
            level: budget.level,
            remaining: budget.remaining,
            spent: budget.spent,
            hoursToExhaustion: budget.hoursToExhaustion,
          };
        }

        // Learning systems status
        const mi = this.services?.modelIntelligence;
        const lp = this.services?.learningPipeline;
        if (mi || lp) {
          health.learning = {
            qLearning: !!lp?.qLearning,
            thompsonSampling: !!mi,
            thompsonMaturity: mi?.getStats?.().samplerMaturity,
            metaCognition: !!lp?.metaCognition,
            sona: !!this.services?.sona,
            behaviorModifier: !!this.services?.behaviorModifier,
          };

          // Q-Learning stats (from learning pipeline)
          if (lp?.qLearning?.getStats) {
            const qStats = lp.qLearning.getStats();
            health.learning.qEpisodes = qStats.episodeCount || 0;
          }
        }

        // Data Pipeline stats
        if (this.services?.dataPipeline) {
          const dpStats = this.services.dataPipeline.getStats();
          health.dataPipeline = {
            itemsProcessed: dpStats.itemsProcessed || 0,
            cacheHitRate: dpStats.cache?.hitRate || 0,
            compressionRatio: dpStats.compressionRatio || 0,
            bytesSaved: dpStats.bytesSaved || 0,
          };
        }

        // Research Runner stats
        if (this.services?.researchRunner) {
          const rrStats = this.services.researchRunner.getStats();
          health.researchRunner = {
            totalProtocols: rrStats.totalProtocols || 0,
            completedProtocols: rrStats.completedProtocols || 0,
            failedProtocols: rrStats.failedProtocols || 0,
            avgConfidence: rrStats.avgConfidence || 0,
          };
        }
      } catch (err) {
        log.debug('Health enrichment failed', { error: err.message });
      }

      res.json(health);
    });

    // Full status — ProcessRegistry snapshot + Services status
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

    // Memory profiling endpoints (debug only)
    this._setupProfilingEndpoints();

    // LLM endpoints — Phase 2: CYNIC calls LLMs directly
    setupLLMEndpoints(this.app);
  }

  /**
   * Setup memory profiling endpoints (debug)
   * @private
   */
  _setupProfilingEndpoints() {
    // Import dynamically to avoid boot overhead
    let profiler = null;
    const getProfiler = async () => {
      if (!profiler) {
        profiler = await import('./memory-profiler.js');
      }
      return profiler;
    };

    // Get current memory stats
    this.app.get('/debug/memory', async (req, res) => {
      try {
        const p = await getProfiler();
        const stats = p.getMemoryStats();
        res.json(stats);
      } catch (err) {
        res.status(500).json({ error: err.message });
      }
    });

    // Take a heap snapshot
    this.app.post('/debug/heap-snapshot', async (req, res) => {
      try {
        const p = await getProfiler();
        const filepath = p.takeHeapSnapshot();
        res.json({ snapshot: filepath });
      } catch (err) {
        res.status(500).json({ error: err.message });
      }
    });

    // Run a full profiling session
    this.app.post('/debug/profile', async (req, res) => {
      try {
        const duration = parseInt(req.query.duration || '60', 10);
        const interval = parseInt(req.query.interval || '5', 10);

        const p = await getProfiler();

        // Start profiling in background
        p.profileMemory(duration, interval).then(result => {
          log.info('Profiling session complete', result.summary);
        });

        res.json({ status: 'started', duration, interval });
      } catch (err) {
        res.status(500).json({ error: err.message });
      }
    });
  }

  /**
   * Start the daemon server
   * @returns {Promise<void>}
   */
  async start() {
    if (this.server) {
      throw new Error('Daemon already running');
    }

    // Background services are wired by entry.js via service-wiring.js
    // (wireDaemonServices, wireLearningSystem, wireOrchestrator, wireWatchers)
    // This method only starts the HTTP server

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
            capabilities: ['hooks', 'health', 'status', 'llm', 'perception', 'orchestration'],
            meta: { version: '0.1.0' },
          });
        } catch (err) {
          log.warn('ProcessRegistry announce failed (non-fatal)', { error: err.message });
        }

        log.info(`Daemon listening on ${this.host}:${this.port}`);
        log.info('Background services: perception, orchestration, learning ACTIVE');
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
    // Background services are cleaned up by entry.js via cleanupDaemonServices()
    // This method only stops the HTTP server

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
      // Force-close keep-alive connections (prevents port linger on Windows)
      this.server.closeAllConnections();
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
