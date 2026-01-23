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

import { CYNICJudge, createCollectivePack, LearningService, createEScoreCalculator, JudgmentGraphIntegration } from '@cynic/node';
import { PeriodicScheduler, FibonacciIntervals } from '@cynic/core';
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

    // 2. Judge (depends on eScore, learning)
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

    return services;
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

  _createJudge(services) {
    return new CYNICJudge({
      eScoreProvider: services.eScoreCalculator,
      learningService: services.learningService,
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
    const pojChainManager = new PoJChainManager(services.persistence, {
      onBlockCreated: this.config.onBlockCreated,
    });
    await pojChainManager.initialize();

    // Verify chain integrity at startup
    if (services.persistence?.pojBlocks) {
      const verification = await pojChainManager.verifyIntegrity();
      if (verification.valid) {
        if (verification.blocksChecked > 0) {
          console.error(`   PoJ Chain: verified ${verification.blocksChecked} blocks`);
        }
      } else {
        console.error(`   *GROWL* PoJ Chain: INTEGRITY ERROR - ${verification.errors.length} invalid links!`);
        for (const err of verification.errors.slice(0, 3)) {
          console.error(`     Block ${err.blockNumber}: expected ${err.expected?.slice(0, 16)}...`);
        }
      }
    }

    return pojChainManager;
  }

  async _createLibrarian(services) {
    const librarian = new LibrarianService(services.persistence);
    await librarian.initialize();
    console.error('   Librarian: ready');
    return librarian;
  }

  async _createDiscovery(services) {
    const discovery = new DiscoveryService(services.persistence, {
      autoHealthCheck: true,
      githubToken: process.env.GITHUB_TOKEN,
    });
    await discovery.init();
    console.error('   Discovery: ready');
    return discovery;
  }

  async _createEcosystem(services) {
    const { EcosystemService } = await import('../ecosystem-service.js');
    const ecosystem = new EcosystemService(services.persistence, {
      autoRefresh: true,
    });
    await ecosystem.init();
    console.error('   Ecosystem: ready');
    return ecosystem;
  }

  async _createIntegrator() {
    const { IntegratorService } = await import('../integrator-service.js');
    const integrator = new IntegratorService({
      workspaceRoot: '/workspaces',
      autoCheck: false,
    });
    await integrator.init();
    console.error('   Integrator: ready');
    return integrator;
  }

  async _createGraph() {
    const graph = new GraphOverlay({
      basePath: this.config.dataDir ? `${this.config.dataDir}/graph` : './data/graph',
    });
    await graph.init();
    console.error('   Graph: ready');
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
    console.error('   GraphIntegration: ready');
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
      console.error(`   Collective: ${awakening.greeting}`);
    } else {
      console.error('   Collective: ready (CYNIC dormant)');
    }

    return pack;
  }

  _createScheduler() {
    const scheduler = new PeriodicScheduler({
      onError: this.config.onSchedulerError || ((task, error) => {
        console.error(`🐕 [Scheduler] Task "${task.name}" failed: ${error.message}`);
      }),
    });
    console.error(`   Scheduler: ready`);
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
    console.error('   Metrics: ready');
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
