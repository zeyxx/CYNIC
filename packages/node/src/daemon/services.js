/**
 * Daemon Services - Orchestration + Perception + Learning
 *
 * Wires all CYNIC services together in the daemon.
 * The nervous system of the organism.
 *
 * "Connect the organs, blood flows" - κυνικός
 *
 * @module @cynic/node/daemon/services
 */

'use strict';

import { createLogger, globalEventBus } from '@cynic/core';
import { FilesystemWatcher, FilesystemEventType } from '../perception/filesystem-watcher.js';
import { UnifiedOrchestrator } from '../orchestration/unified-orchestrator.js';
import { KabbalisticRouter } from '../orchestration/kabbalistic-router.js';
import { QLearningService } from '../orchestration/learning-service.js';
import { CollectivePack } from '../agents/collective/index.js';

const log = createLogger('DaemonServices');

/**
 * Daemon Services Manager
 *
 * Coordinates all background services:
 * - Perception (FileWatcher, SolanaWatcher, MarketWatcher)
 * - Orchestration (UnifiedOrchestrator, routers)
 * - Learning (QLearningService, SONA, MetaCognition)
 */
export class DaemonServices {
  constructor(options = {}) {
    this.options = options;

    // Services
    this.collectivePack = null;
    this.filesystemWatcher = null;
    this.orchestrator = null;
    this.kabbalisticRouter = null;
    this.learningService = null;

    // State
    this._isRunning = false;
    this._startedAt = null;
  }

  /**
   * Start all daemon services
   *
   * @returns {Promise<void>}
   */
  async start() {
    if (this._isRunning) {
      log.warn('Services already running');
      return;
    }

    log.info('Starting daemon services...');
    this._startedAt = Date.now();

    try {
      // 1. Start learning service (needs to be first for routing)
      await this._startLearningService();

      // 2. Start routers
      await this._startRouters();

      // 3. Start orchestrator (wires everything together)
      await this._startOrchestrator();

      // 4. Start perception watchers
      await this._startPerception();

      // 5. Wire perception events to orchestrator
      this._wirePerceptionToOrchestrator();

      this._isRunning = true;
      log.info('Daemon services started', {
        duration: Date.now() - this._startedAt,
      });
    } catch (error) {
      log.error('Failed to start daemon services', { error: error.message });
      await this.stop(); // Cleanup partial start
      throw error;
    }
  }

  /**
   * Stop all daemon services gracefully
   *
   * @returns {Promise<void>}
   */
  async stop() {
    if (!this._isRunning) {
      return;
    }

    log.info('Stopping daemon services...');

    // Stop in reverse order
    if (this.filesystemWatcher) {
      await this.filesystemWatcher.stop();
      this.filesystemWatcher = null;
    }

    // Clean orchestrator listeners
    if (this.orchestrator) {
      // UnifiedOrchestrator doesn't have a stop() yet, just clear references
      this.orchestrator = null;
    }

    if (this.kabbalisticRouter) {
      this.kabbalisticRouter = null;
    }

    if (this.learningService) {
      await this.learningService.flush();
      this.learningService = null;
    }

    this._isRunning = false;
    log.info('Daemon services stopped');
  }

  /**
   * Start learning service
   * @private
   */
  async _startLearningService() {
    log.debug('Starting learning service...');

    this.learningService = new QLearningService({
      persistenceEnabled: true,
      flushIntervalMs: 5000, // Flush every 5 seconds
    });

    // Load persisted Q-table from PostgreSQL (if method exists)
    // Note: QLearningService loads state automatically in constructor
    // This is just a placeholder for future explicit loading
    if (typeof this.learningService.loadState === 'function') {
      await this.learningService.loadState();
    }

    log.info('Learning service started');
  }

  /**
   * Start routing services
   * @private
   */
  async _startRouters() {
    log.debug('Starting routers...');

    // CollectivePack - 11 Dogs (Sefirot)
    this.collectivePack = new CollectivePack();

    // KabbalisticRouter - 7×7 dimension scoring
    this.kabbalisticRouter = new KabbalisticRouter({
      collectivePack: this.collectivePack,
      learningService: this.learningService,
    });

    log.info('Routers started');
  }

  /**
   * Start UnifiedOrchestrator
   * @private
   */
  async _startOrchestrator() {
    log.debug('Starting orchestrator...');

    this.orchestrator = new UnifiedOrchestrator({
      kabbalisticRouter: this.kabbalisticRouter,
      learningService: this.learningService,
      eventBus: globalEventBus,
    });

    log.info('Orchestrator started');
  }

  /**
   * Start perception watchers
   * @private
   */
  async _startPerception() {
    log.debug('Starting perception watchers...');

    // FilesystemWatcher - watches code changes
    this.filesystemWatcher = new FilesystemWatcher({
      paths: [process.cwd()],
      eventBus: globalEventBus,
    });

    this.filesystemWatcher.start();

    log.info('Perception watchers started');
  }

  /**
   * Wire perception events to orchestrator
   *
   * This is THE KEY WIRING for GAP-1.
   * FileWatcher emits → globalEventBus → we listen → route to orchestrator
   *
   * @private
   */
  _wirePerceptionToOrchestrator() {
    log.debug('Wiring perception → orchestrator...');

    // Listen to all filesystem events
    globalEventBus.on(FilesystemEventType.CHANGE, async (data) => {
      log.info('FileWatcher CHANGE event received', { path: data.path });
      await this._handlePerceptionEvent('fs:change', data);
    });

    globalEventBus.on(FilesystemEventType.ADD, async (data) => {
      log.info('FileWatcher ADD event received', { path: data.path });
      await this._handlePerceptionEvent('fs:add', data);
    });

    globalEventBus.on(FilesystemEventType.UNLINK, async (data) => {
      log.info('FileWatcher UNLINK event received', { path: data.path });
      await this._handlePerceptionEvent('fs:unlink', data);
    });

    log.info('Perception → orchestrator wired');
  }

  /**
   * Handle perception event and route to orchestrator
   *
   * GAP-3 CLOSED: Perception → KabbalisticRouter → Q-Learning feedback
   *
   * @param {string} eventType - Event type (fs:change, fs:add, etc.)
   * @param {Object} data - Event data
   * @private
   */
  async _handlePerceptionEvent(eventType, data) {
    try {
      log.debug('Perception event received', { eventType, path: data.path });

      // Map file extension to task type
      const taskType = this._inferTaskTypeFromPath(data.path, eventType);

      // GAP-3: Route through KabbalisticRouter
      // This automatically:
      // 1. Starts Q-Learning episode
      // 2. Routes through Lightning Flash path
      // 3. Records actions (which dogs processed)
      // 4. Ends episode with outcome
      // 5. Updates Q-weights via hot-swap
      if (this.kabbalisticRouter) {
        const routingResult = await this.kabbalisticRouter.route({
          taskType,
          payload: {
            input: data.path,
            content: `File ${eventType}: ${data.path}`,
            filePath: data.path,
            eventType,
            stats: data.stats,
          },
          userId: 'daemon',
          sessionId: 'daemon-perception',
        });

        log.debug('Perception routed', {
          path: data.path,
          taskType,
          entrySefirah: routingResult.entrySefirah,
          consensus: routingResult.synthesis?.hasConsensus,
          blocked: routingResult.blocked,
        });

        // Emit for G1.2 metric tracking (learning loops consuming data)
        globalEventBus.publish('learning:loop:active', {
          loopName: 'perception-routing',
          taskType,
          path: data.path,
          success: routingResult.success,
        });

        // If routing suggests action, delegate to orchestrator
        // (Future: spawn judges, run skills based on routing.kabbalistic.decisions)
      }

    } catch (error) {
      log.error('Failed to handle perception event', {
        eventType,
        error: error.message,
      });
    }
  }

  /**
   * Infer task type from file path and event type
   * @private
   */
  _inferTaskTypeFromPath(filePath, eventType) {
    const ext = filePath.split('.').pop().toLowerCase();
    const basename = filePath.split(/[\\/]/).pop();

    // Security-sensitive files
    if (basename.includes('secret') || basename.includes('credential') || basename === '.env') {
      return 'security';
    }

    // Code files
    if (['js', 'ts', 'jsx', 'tsx', 'py', 'rb', 'go', 'rs'].includes(ext)) {
      return eventType === 'fs:add' ? 'design' : 'PostToolUse';
    }

    // Documentation
    if (['md', 'txt', 'rst'].includes(ext)) {
      return 'knowledge';
    }

    // Configuration
    if (['json', 'yaml', 'yml', 'toml', 'ini'].includes(ext)) {
      return 'design';
    }

    // Tests
    if (basename.includes('test') || basename.includes('spec')) {
      return 'analysis';
    }

    // Default: treat as analysis task
    return 'analysis';
  }

  /**
   * Get service status
   *
   * @returns {Object} Status information
   */
  getStatus() {
    return {
      isRunning: this._isRunning,
      startedAt: this._startedAt,
      uptime: this._startedAt ? Date.now() - this._startedAt : 0,
      services: {
        filesystemWatcher: this.filesystemWatcher ? {
          running: this.filesystemWatcher._isRunning,
          stats: this.filesystemWatcher._stats,
        } : null,
        orchestrator: this.orchestrator ? {
          stats: this.orchestrator.stats,
        } : null,
        learningService: this.learningService ? {
          stats: this.learningService.getStats(),
        } : null,
      },
    };
  }
}

export default DaemonServices;
