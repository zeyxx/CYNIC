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
      await this._handlePerceptionEvent('fs:change', data);
    });

    globalEventBus.on(FilesystemEventType.ADD, async (data) => {
      await this._handlePerceptionEvent('fs:add', data);
    });

    globalEventBus.on(FilesystemEventType.UNLINK, async (data) => {
      await this._handlePerceptionEvent('fs:unlink', data);
    });

    log.info('Perception → orchestrator wired');
  }

  /**
   * Handle perception event and route to orchestrator
   *
   * @param {string} eventType - Event type (fs:change, fs:add, etc.)
   * @param {Object} data - Event data
   * @private
   */
  async _handlePerceptionEvent(eventType, data) {
    try {
      log.debug('Perception event received', { eventType, path: data.path });

      // For now, just log the event
      // TODO: Route to appropriate judges based on file type
      // - .js/.ts → CodeJudge
      // - .md → DocumentationJudge
      // - .json → ConfigJudge
      // - etc.

      // Record learning event (for G1.2 metric: learning loops consuming data)
      if (this.learningService) {
        // This will be used later for Q-Learning routing
        // For now, just track that perception is flowing
        log.debug('Perception event ready for routing', {
          eventType,
          path: data.path,
        });
      }

      // TODO (Phase 1, Day 2): Wire to KabbalisticRouter
      // const routingDecision = await this.kabbalisticRouter.route({
      //   eventType,
      //   data,
      //   dimension: 'CODE', // R1
      // });

      // TODO (Phase 1, Day 3): Spawn appropriate judges based on routing
      // if (routingDecision.judges.includes('CodeJudge')) {
      //   await this.orchestrator.judgeCode(data.path);
      // }

    } catch (error) {
      log.error('Failed to handle perception event', {
        eventType,
        error: error.message,
      });
    }
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
