/**
 * Service Initializer for MCP Server
 *
 * DIP: Dependency Inversion Principle
 * - Creates services via factories (not direct `new`)
 * - Allows injection of custom implementations
 * - Centralized service wiring
 *
 * @module @cynic/mcp/server/ServiceInitializer
 */

'use strict';

import { CYNICJudge, createCollectivePack, LearningService, createEScoreCalculator, JudgmentGraphIntegration, createEngineIntegration } from '@cynic/node';
import { PeriodicScheduler, FibonacciIntervals, EngineRegistry, loadPhilosophyEngines, globalEventBus, EventType, createLogger } from '@cynic/core';

const log = createLogger('ServiceInitializer');
import { GraphOverlay } from '@cynic/persistence/graph';
import { PersistenceManager } from '../persistence.js';
import { SessionManager } from '../session-manager.js';
import { PoJChainManager } from '../poj-chain-manager.js';
import { LibrarianService } from '../librarian-service.js';
import { DiscoveryService } from '../discovery-service.js';
import { MetricsService } from '../metrics-service.js';

/**
 * @typedef {Object} ServiceConfig
 * @property {string} [dataDir] - Data directory for file persistence
 * @property {Function} [onBlockCreated] - Callback for PoJ block creation
 * @property {Function} [onDogDecision] - Callback for collective decisions
 * @property {Function} [onSchedulerError] - Callback for scheduler errors
 */

/**
 * @typedef {Object} Services
 * @property {Object} eScoreCalculator - E-Score calculator
 * @property {Object} learningService - Learning service
 * @property {Object} judge - CYNICJudge instance
 * @property {Object} persistence - PersistenceManager instance
 * @property {Object} sessionManager - SessionManager instance
 * @property {Object} pojChainManager - PoJChainManager instance
 * @property {Object} librarian - LibrarianService instance
 * @property {Object} discovery - DiscoveryService instance
 * @property {Object} ecosystem - EcosystemService instance
 * @property {Object} integrator - IntegratorService instance
 * @property {Object} graph - GraphOverlay instance
 * @property {Object} graphIntegration - JudgmentGraphIntegration instance
 * @property {Object} collective - CollectivePack instance
 * @property {Object} scheduler - PeriodicScheduler instance
 * @property {Object} metrics - MetricsService instance
 */

/**
 * Service Initializer - DIP-compliant service creation
 *
 * Usage:
 * ```javascript
 * const initializer = new ServiceInitializer({
 *   dataDir: '/data',
 *   onBlockCreated: (block) => broadcast('block', block)
 * });
 *
 * // Use default factories
 * const services = await initializer.initialize();
 *
 * // Or inject custom implementations
 * const services = await initializer.initialize({
 *   judge: myCustomJudge,
 *   persistence: myCustomPersistence
 * });
 * ```
 */
export class ServiceInitializer {
  /**
   * @param {ServiceConfig} config
   */
  constructor(config = {}) {
    this.config = config;

    // Service factories (can be overridden for testing/custom implementations)
    this.factories = {
      eScoreCalculator: this._createEScoreCalculator.bind(this),
      learningService: this._createLearningService.bind(this),
      engineRegistry: this._createEngineRegistry.bind(this),
      judge: this._createJudge.bind(this),
      persistence: this._createPersistence.bind(this),
      sessionManager: this._createSessionManager.bind(this),
      pojChainManager: this._createPoJChainManager.bind(this),
      librarian: this._createLibrarian.bind(this),
      discovery: this._createDiscovery.bind(this),
      ecosystem: this._createEcosystem.bind(this),
      integrator: this._createIntegrator.bind(this),
      graph: this._createGraph.bind(this),
      graphIntegration: this._createGraphIntegration.bind(this),
      collective: this._createCollective.bind(this),
      scheduler: this._createScheduler.bind(this),
      metrics: this._createMetrics.bind(this),
    };
  }

  /**
   * Set a custom factory for a service
   * @param {string} name - Service name
   * @param {Function} factory - Factory function (services) => service
   */
  setFactory(name, factory) {
    this.factories[name] = factory;
  }

  /**
   * Initialize all services
   * @param {Partial<Services>} [provided={}] - Pre-created services to use
   * @returns {Promise<Services>}
   */
  async initialize(provided = {}) {
    const services = { ...provided };

    // Order matters - some services depend on others

    // 1. Core calculators (no dependencies)
    if (!services.eScoreCalculator) {
      services.eScoreCalculator = this.factories.eScoreCalculator(services);
    }

    if (!services.learningService) {
      services.learningService = this.factories.learningService(services);
    }

    // 2. Engine Registry (73 philosophy engines)
    if (!services.engineRegistry) {
      services.engineRegistry = await this.factories.engineRegistry(services);
    }

    // 3. Judge (depends on eScore, learning, engines)
    if (!services.judge) {
      services.judge = this.factories.judge(services);
    }

    // 3. Persistence (no dependencies, but async)
    if (!services.persistence) {
      services.persistence = await this.factories.persistence(services);
    }

    // 4. Session manager (depends on persistence)
    if (!services.sessionManager) {
      services.sessionManager = this.factories.sessionManager(services);
    }

    // 5. PoJ Chain manager (depends on persistence)
    if (!services.pojChainManager) {
      services.pojChainManager = await this.factories.pojChainManager(services);
    }

    // 6. Librarian (depends on persistence)
    if (!services.librarian) {
      services.librarian = await this.factories.librarian(services);
    }

    // 7. Discovery (depends on persistence)
    if (!services.discovery) {
      services.discovery = await this.factories.discovery(services);
    }

    // 8. Ecosystem (depends on persistence)
    if (!services.ecosystem) {
      services.ecosystem = await this.factories.ecosystem(services);
    }

    // 9. Integrator (no dependencies)
    if (!services.integrator) {
      services.integrator = await this.factories.integrator(services);
    }

    // 10. Graph (no dependencies)
    if (!services.graph) {
      services.graph = await this.factories.graph(services);
    }

    // 11. GraphIntegration (depends on judge, graph)
    if (!services.graphIntegration) {
      services.graphIntegration = await this.factories.graphIntegration(services);
    }

    // 12. Collective (depends on judge, persistence, graphIntegration)
    if (!services.collective) {
      services.collective = await this.factories.collective(services);
    }

    // 13. Scheduler (no dependencies, but needs config callbacks)
    if (!services.scheduler) {
      services.scheduler = this.factories.scheduler(services);
    }

    // 14. Metrics (depends on multiple services)
    if (!services.metrics) {
      services.metrics = this.factories.metrics(services);
    }

    // 15. Setup event bus subscriptions for cross-layer communication
    this._setupBusSubscriptions(services);

    return services;
  }

  /**
   * Setup subscriptions to global event bus
   * Bridges PoJ/Graph events to metrics and logging
   * @private
   */
  _setupBusSubscriptions(services) {
    // Track subscriptions for cleanup
    this._busSubscriptions = [];

    // Subscribe to PoJ chain events
    this._busSubscriptions.push(
      globalEventBus.subscribe('poj:block:created', (event) => {
        const { slot, judgmentCount, blockHash } = event.payload || {};
        log.info('PoJ block created', { slot, judgmentCount });
        services.metrics?.recordEvent('poj_block_created', { slot, judgmentCount });
      })
    );

    this._busSubscriptions.push(
      globalEventBus.subscribe('poj:block:finalized', (event) => {
        const { slot, blockHash } = event.payload || {};
        log.info('PoJ block finalized', { slot });
        services.metrics?.recordEvent('poj_block_finalized', { slot });
      })
    );

    // Subscribe to graph events
    this._busSubscriptions.push(
      globalEventBus.subscribe('graph:node:added', (event) => {
        const { nodeType, id } = event.payload || {};
        services.metrics?.recordEvent('graph_node_added', { nodeType });
      })
    );

    this._busSubscriptions.push(
      globalEventBus.subscribe('graph:edge:added', (event) => {
        const { edgeType, from, to } = event.payload || {};
        services.metrics?.recordEvent('graph_edge_added', { edgeType });
      })
    );

    // Subscribe to judgment events
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.JUDGMENT_CREATED, (event) => {
        const { qScore, verdict } = event.payload || {};
        services.metrics?.recordEvent('judgment_created', { qScore, verdict });
      })
    );

    // Subscribe to engine events
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.ENGINE_CONSULTED, (event) => {
        const { engineId, domain } = event.payload || {};
        services.metrics?.recordEvent('engine_consulted', { engineId, domain });
      })
    );

    // Subscribe to pattern events (for anomaly detection)
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.ANOMALY_DETECTED, (event) => {
        const { type, severity, description } = event.payload || {};
        log.warn('Anomaly detected', { type, severity });
        services.metrics?.recordEvent('anomaly_detected', { type, severity });
      })
    );

    // 🐕 PHASE 20: Subscribe to USER_FEEDBACK for Learning integration
    // This connects the hook → collective → globalEventBus → learning pipeline
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.USER_FEEDBACK, async (event) => {
        const { source, tool, success, blocked, duration, userId } = event.payload || {};
        services.metrics?.recordEvent('user_feedback', { source, success, blocked });

        // Feed to LearningService if available
        if (services.learningService) {
          try {
            // Convert event to learning feedback format
            const feedback = {
              source: source || 'tool_execution',
              itemType: 'tool',
              itemId: tool,
              positive: success && !blocked,
              context: { duration, userId, blocked },
              timestamp: event.timestamp,
            };

            // Record as implicit feedback (tool execution success/failure)
            await services.learningService.recordFeedback?.(feedback);
            log.trace('Learning feedback recorded', { tool, success });
          } catch (err) {
            log.warn('Learning feedback error', { error: err.message });
          }
        }
      })
    );

    // Subscribe to TOOL_COMPLETED for additional metrics
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.TOOL_COMPLETED, (event) => {
        const { tool, duration, success, blocked, agentCount } = event.payload || {};
        services.metrics?.recordEvent('tool_completed', { tool, duration, success, blocked, agentCount });
      })
    );

    log.debug('Bus subscriptions active', { count: this._busSubscriptions.length });
  }

  /**
   * Cleanup bus subscriptions
   */
  cleanupBusSubscriptions() {
    if (this._busSubscriptions) {
      for (const unsubscribe of this._busSubscriptions) {
        if (typeof unsubscribe === 'function') {
          unsubscribe();
        }
      }
      this._busSubscriptions = [];
      log.debug('Bus subscriptions cleaned up');
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // DEFAULT FACTORIES
  // ═══════════════════════════════════════════════════════════════════════════

  _createEScoreCalculator() {
    return createEScoreCalculator({
      burnScale: 1e9,
      minJudgments: 10,
    });
  }

  _createLearningService() {
    return new LearningService({
      learningRate: 0.236,  // φ⁻³
      decayRate: 0.146,     // φ⁻⁴
      minFeedback: 5,
    });
  }

  async _createEngineRegistry() {
    const registry = new EngineRegistry();
    const result = loadPhilosophyEngines({ registry, silent: true });
    log.info('Philosophy engines loaded', { count: result.loaded });
    return registry;
  }

  _createJudge(services) {
    // Create engine integration if registry is available
    const engineIntegration = services.engineRegistry
      ? createEngineIntegration({ registry: services.engineRegistry })
      : null;

    return new CYNICJudge({
      eScoreProvider: services.eScoreCalculator,
      learningService: services.learningService,
      engineIntegration,
      consultEngines: !!engineIntegration, // Enable if engines available
    });
  }

  async _createPersistence() {
    const persistence = new PersistenceManager({
      dataDir: this.config.dataDir,
    });
    await persistence.initialize();
    return persistence;
  }

  _createSessionManager(services) {
    return new SessionManager(services.persistence);
  }

  async _createPoJChainManager(services) {
    // P2P consensus configuration from environment
    const p2pNodeUrl = process.env.CYNIC_P2P_NODE_URL || this.config.p2pNodeUrl;
    const p2pEnabled = p2pNodeUrl && (process.env.CYNIC_P2P_ENABLED !== 'false');

    const pojChainManager = new PoJChainManager(services.persistence, {
      onBlockCreated: this.config.onBlockCreated,
      p2pNodeUrl,
      p2pEnabled,
    });
    await pojChainManager.initialize();

    if (p2pEnabled) {
      log.info('PoJ P2P consensus enabled', { nodeUrl: p2pNodeUrl });
    }

    // Verify chain integrity at startup
    if (services.persistence?.pojBlocks) {
      const verification = await pojChainManager.verifyIntegrity();
      if (verification.valid) {
        if (verification.blocksChecked > 0) {
          log.info('PoJ chain verified', { blocksChecked: verification.blocksChecked });
        }
      } else {
        log.error('PoJ chain integrity error', { errorCount: verification.errors.length });
        for (const err of verification.errors.slice(0, 3)) {
          log.error('Invalid block link', { blockNumber: err.blockNumber, expected: err.expected?.slice(0, 16) });
        }
      }
    }

    return pojChainManager;
  }

  async _createLibrarian(services) {
    const librarian = new LibrarianService(services.persistence);
    await librarian.initialize();
    log.debug('Librarian ready');
    return librarian;
  }

  async _createDiscovery(services) {
    const discovery = new DiscoveryService(services.persistence, {
      autoHealthCheck: true,
      githubToken: process.env.GITHUB_TOKEN,
    });
    await discovery.init();
    log.debug('Discovery ready');
    return discovery;
  }

  async _createEcosystem(services) {
    const { EcosystemService } = await import('../ecosystem-service.js');
    const ecosystem = new EcosystemService(services.persistence, {
      autoRefresh: true,
    });
    await ecosystem.init();
    log.debug('Ecosystem ready');
    return ecosystem;
  }

  async _createIntegrator() {
    const { IntegratorService } = await import('../integrator-service.js');
    const integrator = new IntegratorService({
      workspaceRoot: '/workspaces',
      autoCheck: false,
    });
    await integrator.init();
    log.debug('Integrator ready');
    return integrator;
  }

  async _createGraph() {
    const graph = new GraphOverlay({
      basePath: this.config.dataDir ? `${this.config.dataDir}/graph` : './data/graph',
    });
    await graph.init();
    log.debug('Graph ready');
    return graph;
  }

  async _createGraphIntegration(services) {
    if (!services.graph) return null;

    const graphIntegration = new JudgmentGraphIntegration({
      judge: services.judge,
      graph: services.graph,
      enrichContext: true,
      contextDepth: 2,
    });
    await graphIntegration.init();
    log.debug('GraphIntegration ready');
    return graphIntegration;
  }

  async _createCollective(services) {
    const pack = createCollectivePack({
      judge: services.judge,
      profileLevel: 3,
      persistence: services.persistence,
      graphIntegration: services.graphIntegration,
      onDogDecision: this.config.onDogDecision,
    });

    // Awaken CYNIC for this session
    const awakening = await pack.awakenCynic({
      sessionId: `mcp_${Date.now()}`,
      userId: 'mcp_server',
      project: 'cynic-mcp',
    });

    if (awakening.success) {
      log.info('Collective awakened', { greeting: awakening.greeting });
    } else {
      log.debug('Collective ready (CYNIC dormant)');
    }

    return pack;
  }

  _createScheduler() {
    const scheduler = new PeriodicScheduler({
      onError: this.config.onSchedulerError || ((task, error) => {
        log.error('Scheduler task failed', { task: task.name, error: error.message });
      }),
    });
    log.debug('Scheduler ready');
    return scheduler;
  }

  _createMetrics(services) {
    const metrics = new MetricsService({
      persistence: services.persistence,
      sessionManager: services.sessionManager,
      pojChainManager: services.pojChainManager,
      librarian: services.librarian,
      ecosystem: services.ecosystem,
      integrator: services.integrator,
      judge: services.judge,
      collective: services.collective,
    });
    log.debug('Metrics ready');
    return metrics;
  }
}

/**
 * Create default service initializer
 * @param {ServiceConfig} config
 * @returns {ServiceInitializer}
 */
export function createServiceInitializer(config = {}) {
  return new ServiceInitializer(config);
}
