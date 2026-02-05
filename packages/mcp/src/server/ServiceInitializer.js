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

import {
  CYNICJudge, LearningService, LearningManager,
  createEScoreCalculator, JudgmentGraphIntegration, createEngineIntegration,
  createAutomationExecutor, getEventBus,
  // Collective Singleton - "One pack, one truth"
  getCollectivePack, awakenCynic,
  // Claude Flow Integration (Phase 21)
  createSONA, createTieredRouter, createAgentBooster,
  createTokenOptimizer, createHyperbolicSpace, createComplexityClassifier,
} from '@cynic/node';
import { PeriodicScheduler, FibonacciIntervals, EngineRegistry, loadPhilosophyEngines, globalEventBus, EventType, createLogger } from '@cynic/core';

const log = createLogger('ServiceInitializer');
import { GraphOverlay } from '@cynic/persistence/graph';
import { PersistenceManager } from '../persistence.js';
import { SessionManager } from '../session-manager.js';
import { PoJChainManager } from '../poj-chain-manager.js';
import { LibrarianService } from '../librarian-service.js';
import { DiscoveryService } from '../discovery-service.js';
import { MetricsService } from '../metrics-service.js';

// ═══════════════════════════════════════════════════════════════════════════
// SOLANA ANCHORING (Task #29) - "Onchain is truth"
// ═══════════════════════════════════════════════════════════════════════════
let anchorModule = null;
async function getAnchorModule() {
  if (anchorModule) return anchorModule;
  try {
    anchorModule = await import('@cynic/anchor');
    return anchorModule;
  } catch (e) {
    log.debug('Anchor module not available (optional)', { error: e.message });
    return null;
  }
}

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
 * @property {Object} [sona] - SONA adaptive learning
 * @property {Object} [tieredRouter] - Complexity-based routing
 * @property {Object} [agentBooster] - Fast code transforms
 * @property {Object} [tokenOptimizer] - Token compression
 * @property {Object} [hyperbolicSpace] - Hierarchical embeddings
 * @property {Object} [complexityClassifier] - Complexity classification
 * @property {Object} [anchorIntegration] - Solana anchoring for PoJ blocks
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
      learningManager: this._createLearningManager.bind(this),
      automationExecutor: this._createAutomationExecutor.bind(this),
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
      // Claude Flow Integration (Phase 21)
      complexityClassifier: this._createComplexityClassifier.bind(this),
      tieredRouter: this._createTieredRouter.bind(this),
      agentBooster: this._createAgentBooster.bind(this),
      tokenOptimizer: this._createTokenOptimizer.bind(this),
      hyperbolicSpace: this._createHyperbolicSpace.bind(this),
      sona: this._createSONA.bind(this),
      // Solana Anchoring (Task #29)
      anchorIntegration: this._createAnchorIntegration.bind(this),
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

    // 2. Persistence FIRST (async, needed by learningService for FeedbackRepository)
    // Task #55: Connect FeedbackRepository → CYNICJudge
    if (!services.persistence) {
      services.persistence = await this.factories.persistence(services);
    }

    // 3. LearningService (depends on persistence for FeedbackRepository)
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

    // 6. Session manager (depends on persistence)
    if (!services.sessionManager) {
      services.sessionManager = this.factories.sessionManager(services);
    }

    // 5. PoJ Chain manager (depends on persistence)
    if (!services.pojChainManager) {
      services.pojChainManager = await this.factories.pojChainManager(services);
    }

    // 5b. Anchor Integration (depends on pojChainManager) - "Onchain is truth"
    if (!services.anchorIntegration) {
      services.anchorIntegration = await this.factories.anchorIntegration(services);
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

    // 15. Learning Manager (unified learning with event bus integration)
    if (!services.learningManager) {
      services.learningManager = await this.factories.learningManager(services);
    }

    // 16. Automation Executor (scheduled learning cycles and trigger evaluation)
    if (!services.automationExecutor) {
      services.automationExecutor = await this.factories.automationExecutor(services);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLAUDE FLOW INTEGRATION (Phase 21) - Optional enhanced services
    // ═══════════════════════════════════════════════════════════════════════════

    // 17. Complexity Classifier (no dependencies)
    if (!services.complexityClassifier) {
      services.complexityClassifier = this.factories.complexityClassifier(services);
    }

    // 18. Tiered Router (depends on complexity classifier)
    if (!services.tieredRouter) {
      services.tieredRouter = this.factories.tieredRouter(services);
    }

    // 19. Agent Booster (no dependencies - fast code transforms)
    if (!services.agentBooster) {
      services.agentBooster = this.factories.agentBooster(services);
    }

    // 20. Token Optimizer (no dependencies - compression)
    if (!services.tokenOptimizer) {
      services.tokenOptimizer = this.factories.tokenOptimizer(services);
    }

    // 21. Hyperbolic Space (no dependencies - hierarchical embeddings)
    if (!services.hyperbolicSpace) {
      services.hyperbolicSpace = this.factories.hyperbolicSpace(services);
    }

    // 22. SONA (depends on learning service - adaptive learning)
    if (!services.sona) {
      services.sona = this.factories.sona(services);
    }

    // 23. Setup event bus subscriptions for cross-layer communication
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

    // ═══════════════════════════════════════════════════════════════════════════
    // CLAUDE FLOW BUS SUBSCRIPTIONS (Phase 21)
    // ═══════════════════════════════════════════════════════════════════════════

    // Route requests through tiered router for cost optimization
    this._busSubscriptions.push(
      globalEventBus.subscribe('request:classify', (event) => {
        const { content } = event.payload || {};
        if (services.tieredRouter && content) {
          const tier = services.complexityClassifier?.classify({ content });
          services.metrics?.recordEvent('request_routed', { tier: tier?.tier });
        }
      })
    );

    // Feed SONA with judgment feedback for adaptive learning
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.JUDGMENT_CREATED, (event) => {
        const { qScore, verdict, dimensions } = event.payload || {};
        if (services.sona && dimensions) {
          services.sona.observe({
            patternId: event.id,
            dimensionScores: dimensions,
          });
        }
      })
    );

    // Process feedback through SONA for correlation
    this._busSubscriptions.push(
      globalEventBus.subscribe(EventType.USER_FEEDBACK, (event) => {
        const { success, itemId, impact } = event.payload || {};
        if (services.sona && itemId) {
          services.sona.processFeedback({
            patternId: itemId,
            success: !!success,
            impact: impact || (success ? 0.7 : 0.3),
          });
        }
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

  _createLearningService(services) {
    // Task #55: Connect FeedbackRepository → CYNICJudge
    // LearningService uses persistence.feedback to process corrections
    return new LearningService({
      persistence: services.persistence, // ← Wired! Feedback now feeds learning
      learningRate: 0.236,  // φ⁻³
      decayRate: 0.146,     // φ⁻⁴
      minFeedback: 5,
    });
  }

  async _createEngineRegistry() {
    const registry = new EngineRegistry();
    const result = loadPhilosophyEngines({ registry, silent: true });
    log.info('Philosophy engines loaded', { count: result.registered });
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

  /**
   * Create Anchor Integration - Solana anchoring for PoJ blocks
   * Task #29: "Onchain is truth" - Wire PoJ chain to Solana
   *
   * Requires:
   * - CYNIC_SOLANA_CLUSTER: devnet/testnet/mainnet-beta (default: devnet)
   * - CYNIC_WALLET_PATH: Path to Solana wallet JSON
   * - CYNIC_ANCHOR_ENABLED: true/false (default: true if wallet exists)
   */
  async _createAnchorIntegration(services) {
    const anchor = await getAnchorModule();
    if (!anchor) {
      log.debug('Anchor integration skipped (module not available)');
      return null;
    }

    // Check if pojChainManager exists
    if (!services.pojChainManager) {
      log.debug('Anchor integration skipped (no pojChainManager)');
      return null;
    }

    // Configuration from environment
    const cluster = process.env.CYNIC_SOLANA_CLUSTER || 'devnet';
    const walletPath = process.env.CYNIC_WALLET_PATH;
    const enabled = process.env.CYNIC_ANCHOR_ENABLED !== 'false';

    if (!enabled) {
      log.debug('Anchor integration disabled via CYNIC_ANCHOR_ENABLED=false');
      return null;
    }

    try {
      // Load wallet if path provided
      let wallet = null;
      if (walletPath) {
        try {
          wallet = anchor.CynicWallet.fromFile(walletPath);
          log.info('Solana wallet loaded', { path: walletPath, publicKey: wallet.publicKey });
        } catch (e) {
          log.warn('Wallet load failed, using simulation mode', { error: e.message });
        }
      }

      // Create standalone anchorer (not full integration, since pojChainManager
      // uses globalEventBus instead of EventEmitter)
      const anchorer = anchor.createAnchorer({
        cluster,
        wallet,
        useAnchorProgram: true,
        onAnchor: (record) => {
          log.info('Block anchored to Solana', {
            merkleRoot: record.merkleRoot?.slice(0, 16) + '...',
            signature: record.signature?.slice(0, 20) + '...',
            slot: record.slot,
          });
          // Emit via global event bus for metrics
          globalEventBus.publish('poj:block:anchored', {
            merkleRoot: record.merkleRoot,
            signature: record.signature,
            slot: record.slot,
            timestamp: Date.now(),
          });
        },
        onError: (record, error) => {
          log.error('Anchor failed', {
            merkleRoot: record.merkleRoot?.slice(0, 16) + '...',
            error: error.message,
          });
        },
      });

      // Subscribe to globalEventBus for block creation events
      // This bridges the gap: pojChainManager → globalEventBus → anchorer
      const unsubscribe = globalEventBus.subscribe('poj:block:created', async (event) => {
        const { slot, hash, judgmentCount, judgmentsRoot, judgmentIds } = event.payload || {};

        // Use judgmentsRoot (the actual field name from PoJChainManager)
        const merkleRoot = judgmentsRoot;

        if (!merkleRoot) {
          log.debug('Block has no merkle root, skipping anchor', { slot });
          return;
        }

        log.info('Anchoring PoJ block', { slot, judgmentCount, merkleRoot: merkleRoot?.slice(0, 16) + '...' });

        try {
          // Use judgment IDs from event, or fetch from persistence
          let itemIds = judgmentIds || [];
          if (itemIds.length === 0 && services.persistence?.pojBlocks) {
            const block = await services.persistence.pojBlocks.findByNumber(slot);
            itemIds = block?.judgment_ids || [];
          }

          // Anchor the merkle root to Solana
          anchorer.setBlockHeight(slot);
          await anchorer.anchor(merkleRoot, itemIds);
        } catch (e) {
          log.error('Block anchor failed', { slot, error: e.message });
        }
      });

      // Create wrapper object with stats and cleanup
      const integration = {
        anchorer,
        cluster,
        hasWallet: !!wallet,
        unsubscribe,
        getStats: () => ({
          ...anchorer.getStats(),
          cluster,
          hasWallet: !!wallet,
          mode: wallet ? 'LIVE' : 'SIMULATION',
        }),
        stop: () => {
          if (typeof unsubscribe === 'function') unsubscribe();
        },
      };

      // Track stats in pojChainManager (isAnchoringEnabled is a getter — true when _anchorQueue exists)
      services.pojChainManager.getAnchorStatus = () => integration.getStats();
      services.pojChainManager.getPendingAnchors = () => anchorer.getPendingAnchors?.() || [];

      log.info('Anchor integration ready', {
        cluster,
        hasWallet: !!wallet,
        autoAnchor: true,
        mode: wallet ? 'LIVE' : 'SIMULATION',
      });

      return integration;
    } catch (e) {
      log.error('Anchor integration failed', { error: e.message });
      return null;
    }
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
    // ═══════════════════════════════════════════════════════════════════════════
    // USE SINGLETON - "One pack, one truth"
    // This ensures ServiceInitializer uses the same pack as MCP server and hooks
    // ═══════════════════════════════════════════════════════════════════════════
    const pack = getCollectivePack({
      judge: services.judge,
      profileLevel: 3,
      persistence: services.persistence,
      graphIntegration: services.graphIntegration,
      onDogDecision: this.config.onDogDecision,
    });

    // Awaken CYNIC for this session (uses singleton helper)
    const awakening = await awakenCynic({
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

  /**
   * Create Learning Manager - Unified learning system with event bus integration
   * Phase 18: Automated learning cycles
   */
  async _createLearningManager(services) {
    const eventBus = getEventBus();

    const learningManager = new LearningManager({
      persistence: services.persistence,
      eventBus,
      learningRate: 0.236,  // φ⁻³
      autoLearn: true,
      minSamples: 5,
    });

    await learningManager.init();
    log.info('Learning manager ready', { autoLearn: true });
    return learningManager;
  }

  /**
   * Create Automation Executor - Scheduled execution of learning and triggers
   * Phase 18: Background daemon for automated tasks
   */
  async _createAutomationExecutor(services) {
    const eventBus = getEventBus();

    // Create trigger adapter from persistence
    const triggerManager = services.persistence?.triggers ? {
      getEnabled: async () => services.persistence.triggers.findEnabled(),
    } : null;

    const automationExecutor = createAutomationExecutor({
      learningManager: services.learningManager,
      triggerManager,
      pool: services.persistence?.postgres,
      eventBus,
      // Phase 16: Autonomy repos for task/goal/notification processing
      goalsRepo: services.persistence?.goals,
      tasksRepo: services.persistence?.tasks,
      notificationsRepo: services.persistence?.notifications,
    });

    // Start the automation executor
    await automationExecutor.start();
    log.info('Automation executor started', {
      learningInterval: '5min',
      triggerInterval: '1min',
      cleanupInterval: '13min',
      tasksInterval: '2min',
      goalsInterval: '8min',
      notificationsInterval: '21min',
    });

    return automationExecutor;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CLAUDE FLOW FACTORIES (Phase 21)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create Complexity Classifier
   * Classifies requests into LOCAL/LIGHT/FULL tiers
   */
  _createComplexityClassifier() {
    const classifier = createComplexityClassifier();
    log.debug('ComplexityClassifier ready');
    return classifier;
  }

  /**
   * Create Tiered Router
   * Routes requests to appropriate handlers based on complexity
   */
  _createTieredRouter(services) {
    const router = createTieredRouter({
      classifier: services.complexityClassifier,
    });
    log.debug('TieredRouter ready');
    return router;
  }

  /**
   * Create Agent Booster
   * Fast code transforms without LLM (< 1ms, $0)
   */
  _createAgentBooster() {
    const booster = createAgentBooster();
    log.debug('AgentBooster ready', { transforms: 12 });
    return booster;
  }

  /**
   * Create Token Optimizer
   * Compression and caching for token efficiency
   */
  _createTokenOptimizer() {
    const optimizer = createTokenOptimizer();
    log.debug('TokenOptimizer ready', { strategies: 4 });
    return optimizer;
  }

  /**
   * Create Hyperbolic Space
   * Poincaré ball model for hierarchical embeddings
   */
  _createHyperbolicSpace() {
    const space = createHyperbolicSpace({ dim: 8 });
    log.debug('HyperbolicSpace ready', { dim: 8 });
    return space;
  }

  /**
   * Create SONA (Self-Optimizing Neural Adaptation)
   * Correlates patterns to dimensions for adaptive learning
   */
  _createSONA(services) {
    const sona = createSONA({
      learningService: services.learningService,
    });
    log.debug('SONA ready', { adaptationRate: 0.236 });
    return sona;
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
