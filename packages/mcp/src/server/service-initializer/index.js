/**
 * Service Initializer - Thin Orchestrator
 *
 * DIP: Dependency Inversion Principle
 * - Creates services via factories (not direct `new`)
 * - Allows injection of custom implementations
 * - Centralized service wiring
 *
 * Delegates to extracted factory modules:
 *   - core-factories: EScore, Learning, Persistence, Session, Judge
 *   - chain-factories: PoJ Chain, Anchor Integration
 *   - ecosystem-factories: Librarian, Discovery, Ecosystem, Graph, Collective
 *   - claude-flow-factories: ComplexityClassifier, TieredRouter, SONA, etc.
 *   - bus-subscriptions: Event bus cross-layer communication
 *
 * @module @cynic/mcp/server/service-initializer
 */

'use strict';

import { createLogger } from '@cynic/core';

// Factory modules
import {
  createEScoreCalculatorFactory,
  createLearningServiceFactory,
  createEngineRegistryFactory,
  createJudgeFactory,
  createPersistenceFactory,
  createSessionManagerFactory,
  createSchedulerFactory,
  createMetricsFactory,
  createLearningManagerFactory,
  createAutomationExecutorFactory,
} from './core-factories.js';

import {
  createPoJChainManagerFactory,
  createAnchorIntegrationFactory,
} from './chain-factories.js';

import {
  createLibrarianFactory,
  createDiscoveryFactory,
  createEcosystemFactory,
  createIntegratorFactory,
  createGraphFactory,
  createGraphIntegrationFactory,
  createCollectiveFactory,
} from './ecosystem-factories.js';

import {
  createComplexityClassifierFactory,
  createTieredRouterFactory,
  createAgentBoosterFactory,
  createTokenOptimizerFactory,
  createHyperbolicSpaceFactory,
  createSONAFactory,
  createBehaviorModifierFactory,
  createMetaCognitionFactory,
} from './claude-flow-factories.js';

import { setupBusSubscriptions, cleanupBusSubscriptions } from './bus-subscriptions.js';

const log = createLogger('ServiceInitializer');

/**
 * Service Initializer - DIP-compliant service creation
 */
export class ServiceInitializer {
  /**
   * @param {Object} config
   * @param {string} [config.dataDir] - Data directory for file persistence
   * @param {Function} [config.onBlockCreated] - Callback for PoJ block creation
   * @param {Function} [config.onDogDecision] - Callback for collective decisions
   * @param {Function} [config.onSchedulerError] - Callback for scheduler errors
   */
  constructor(config = {}) {
    this.config = config;
    this._busSubscriptions = [];

    // Service factories (can be overridden for testing)
    this.factories = {
      // Core
      eScoreCalculator: () => createEScoreCalculatorFactory(),
      learningService: (s) => createLearningServiceFactory(s),
      engineRegistry: () => createEngineRegistryFactory(),
      judge: (s) => createJudgeFactory(s),
      persistence: () => createPersistenceFactory(this.config),
      sessionManager: (s) => createSessionManagerFactory(s),
      scheduler: () => createSchedulerFactory(this.config),
      metrics: (s) => createMetricsFactory(s),
      learningManager: (s) => createLearningManagerFactory(s),
      automationExecutor: (s) => createAutomationExecutorFactory(s),

      // Chain
      pojChainManager: (s) => createPoJChainManagerFactory(s, this.config),
      anchorIntegration: (s) => createAnchorIntegrationFactory(s),

      // Ecosystem
      librarian: (s) => createLibrarianFactory(s),
      discovery: (s) => createDiscoveryFactory(s),
      ecosystem: (s) => createEcosystemFactory(s),
      integrator: () => createIntegratorFactory(),
      graph: () => createGraphFactory(this.config),
      graphIntegration: (s) => createGraphIntegrationFactory(s),
      collective: (s) => createCollectiveFactory(s, this.config),

      // Claude Flow
      complexityClassifier: () => createComplexityClassifierFactory(),
      tieredRouter: (s) => createTieredRouterFactory(s),
      agentBooster: () => createAgentBoosterFactory(),
      tokenOptimizer: () => createTokenOptimizerFactory(),
      hyperbolicSpace: () => createHyperbolicSpaceFactory(),
      sona: (s) => createSONAFactory(s),
      behaviorModifier: (s) => createBehaviorModifierFactory(s),
      metaCognition: () => createMetaCognitionFactory(),
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
   * @param {Object} [provided={}] - Pre-created services to use
   * @returns {Promise<Object>} All initialized services
   */
  async initialize(provided = {}) {
    const services = { ...provided };

    // Order matters - some services depend on others

    // 1. Core calculators (no dependencies)
    if (!services.eScoreCalculator) {
      services.eScoreCalculator = this.factories.eScoreCalculator(services);
    }

    // 2. Persistence FIRST (async, needed by learningService)
    if (!services.persistence) {
      services.persistence = await this.factories.persistence(services);
    }

    // 3. LearningService (depends on persistence)
    if (!services.learningService) {
      services.learningService = this.factories.learningService(services);
    }

    // 4. Engine Registry (73 philosophy engines)
    if (!services.engineRegistry) {
      services.engineRegistry = await this.factories.engineRegistry(services);
    }

    // 5. Judge (depends on eScore, learning, engines)
    if (!services.judge) {
      services.judge = this.factories.judge(services);
    }

    // 6. Session manager
    if (!services.sessionManager) {
      services.sessionManager = this.factories.sessionManager(services);
    }

    // 7. PoJ Chain manager
    if (!services.pojChainManager) {
      services.pojChainManager = await this.factories.pojChainManager(services);
    }

    // 8. Anchor Integration
    if (!services.anchorIntegration) {
      services.anchorIntegration = await this.factories.anchorIntegration(services);
    }

    // 9. Librarian
    if (!services.librarian) {
      services.librarian = await this.factories.librarian(services);
    }

    // 10. Discovery
    if (!services.discovery) {
      services.discovery = await this.factories.discovery(services);
    }

    // 11. Ecosystem
    if (!services.ecosystem) {
      services.ecosystem = await this.factories.ecosystem(services);
    }

    // 12. Integrator
    if (!services.integrator) {
      services.integrator = await this.factories.integrator(services);
    }

    // 13. Graph
    if (!services.graph) {
      services.graph = await this.factories.graph(services);
    }

    // 14. GraphIntegration
    if (!services.graphIntegration) {
      services.graphIntegration = await this.factories.graphIntegration(services);
    }

    // 15. Collective
    if (!services.collective) {
      services.collective = await this.factories.collective(services);
    }

    // 16. Scheduler
    if (!services.scheduler) {
      services.scheduler = this.factories.scheduler(services);
    }

    // 17. Metrics
    if (!services.metrics) {
      services.metrics = this.factories.metrics(services);
    }

    // 18. Learning Manager
    if (!services.learningManager) {
      services.learningManager = await this.factories.learningManager(services);
    }

    // 19. Automation Executor
    if (!services.automationExecutor) {
      services.automationExecutor = await this.factories.automationExecutor(services);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLAUDE FLOW SERVICES (Phase 21)
    // ═══════════════════════════════════════════════════════════════════════════

    // 20. Complexity Classifier
    if (!services.complexityClassifier) {
      services.complexityClassifier = this.factories.complexityClassifier(services);
    }

    // 21. Tiered Router
    if (!services.tieredRouter) {
      services.tieredRouter = this.factories.tieredRouter(services);
    }

    // 22. Agent Booster
    if (!services.agentBooster) {
      services.agentBooster = this.factories.agentBooster(services);
    }

    // 23. Token Optimizer
    if (!services.tokenOptimizer) {
      services.tokenOptimizer = this.factories.tokenOptimizer(services);
    }

    // 24. Hyperbolic Space
    if (!services.hyperbolicSpace) {
      services.hyperbolicSpace = this.factories.hyperbolicSpace(services);
    }

    // 25. SONA
    if (!services.sona) {
      services.sona = this.factories.sona(services);
    }

    // 26. BehaviorModifier (feedback → behavior changes)
    if (!services.behaviorModifier) {
      services.behaviorModifier = this.factories.behaviorModifier(services);
    }

    // 27. MetaCognition (self-monitoring + strategy switching)
    if (!services.metaCognition) {
      services.metaCognition = this.factories.metaCognition(services);
    }

    // 28. Setup event bus subscriptions
    this._busSubscriptions = setupBusSubscriptions(services);

    return services;
  }

  /**
   * Cleanup bus subscriptions
   */
  cleanupBusSubscriptions() {
    cleanupBusSubscriptions(this._busSubscriptions);
    this._busSubscriptions = [];
  }
}

/**
 * Create default service initializer
 * @param {Object} config
 * @returns {ServiceInitializer}
 */
export function createServiceInitializer(config = {}) {
  return new ServiceInitializer(config);
}

export default ServiceInitializer;
