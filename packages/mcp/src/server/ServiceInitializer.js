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

import { CYNICJudge, createCollectivePack, LearningService, createEScoreCalculator } from '@cynic/node';
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
 * @property {Function} [onJudgment] - Callback for judgment events
 */

/**
 * @typedef {Object} Services
 * @property {Object} judge - CYNICJudge instance
 * @property {Object} persistence - PersistenceManager instance
 * @property {Object} sessionManager - SessionManager instance
 * @property {Object} pojChainManager - PoJChainManager instance
 * @property {Object} librarian - LibrarianService instance
 * @property {Object} discovery - DiscoveryService instance
 * @property {Object} metrics - MetricsService instance
 * @property {Object} collective - CollectivePack instance
 * @property {Object} eScoreCalculator - E-Score calculator
 * @property {Object} learningService - Learning service
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
      metrics: this._createMetrics.bind(this),
      collective: this._createCollective.bind(this),
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
      services.pojChainManager = this.factories.pojChainManager(services);
    }

    // 6. Librarian (depends on persistence)
    if (!services.librarian) {
      services.librarian = this.factories.librarian(services);
    }

    // 7. Discovery (depends on persistence)
    if (!services.discovery) {
      services.discovery = this.factories.discovery(services);
    }

    // 8. Collective (depends on judge, persistence)
    if (!services.collective) {
      services.collective = await this.factories.collective(services);
    }

    // 9. Metrics (depends on multiple services)
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

  _createPoJChainManager(services) {
    return new PoJChainManager(services.persistence, {
      onBlockCreated: this.config.onBlockCreated,
    });
  }

  _createLibrarian(services) {
    return new LibrarianService(services.persistence);
  }

  _createDiscovery(services) {
    return new DiscoveryService(services.persistence);
  }

  async _createCollective(services) {
    const pack = createCollectivePack({
      persistence: services.persistence,
      judge: services.judge,
    });
    await pack.start();
    return pack;
  }

  _createMetrics(services) {
    return new MetricsService({
      persistence: services.persistence,
      sessionManager: services.sessionManager,
      pojChainManager: services.pojChainManager,
      librarian: services.librarian,
      judge: services.judge,
      collective: services.collective,
    });
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
