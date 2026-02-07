/**
 * CYNIC MCP Server - Thin Orchestrator
 *
 * Model Context Protocol server for AI tool integration.
 * Delegates to extracted components for each concern:
 *   - StdioTransport: stdin/stdout JSON-RPC transport
 *   - JsonRpcHandler: Protocol routing and tool execution
 *   - RouteHandlers: HTTP route domain logic
 *   - InitializationPipeline: Post-ServiceInitializer setup
 *   - ShutdownManager: Graceful multi-component teardown
 *   - HttpAdapter: HTTP/SSE transport (existing)
 *   - ServiceInitializer: DIP service factory (existing)
 *
 * Protocol: JSON-RPC 2.0 over stdio or HTTP/SSE
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/mcp
 */

'use strict';

import { join } from 'path';
import { fileURLToPath } from 'url';
import { PHI_INV, IDENTITY, PeriodicScheduler, EcosystemMonitor } from '@cynic/core';
import { EngineOrchestrator, globalEngineRegistry } from '@cynic/core/engines';

// SRP: Extracted components
import { HttpAdapter } from './server/HttpAdapter.js';
import { ServiceInitializer } from './server/ServiceInitializer.js';
import { StdioTransport } from './server/StdioTransport.js';
import { JsonRpcHandler } from './server/JsonRpcHandler.js';
import { RouteHandlers } from './server/RouteHandlers.js';
import { InitializationPipeline } from './server/InitializationPipeline.js';
import { ShutdownManager } from './server/ShutdownManager.js';

import { CYNICJudge, LearningService, ResidualDetector, createEScoreCalculator, createEmergenceLayer, DogOrchestrator, getCollectivePack, getSharedMemory } from '@cynic/node';
import { createEngineIntegration } from '@cynic/node/judge/engine-integration.js';
import { UnifiedOrchestrator } from '@cynic/node/orchestration/unified-orchestrator.js';
import { KabbalisticRouter } from '@cynic/node/orchestration/kabbalistic-router.js';
import { createPatternDetector } from '@cynic/emergence';
import { createThermodynamicsTracker } from './thermodynamics-tracker.js';
import { createBurnVerifier } from '@cynic/burns';

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');

export class MCPServer {
  /**
   * Create MCP server
   * @param {Object} [options] - Server options
   * @param {Object} [options.node] - CYNICNode instance
   * @param {Object} [options.judge] - CYNICJudge instance
   * @param {Object} [options.persistence] - PersistenceManager instance
   * @param {Object} [options.sessionManager] - SessionManager instance
   * @param {Object} [options.pojChainManager] - PoJChainManager instance
   * @param {Object} [options.librarian] - LibrarianService instance
   * @param {Object} [options.agents] - AgentManager instance
   * @param {Object} [options.auth] - AuthService instance
   * @param {string} [options.dataDir] - Data directory for file-based persistence fallback
   * @param {string} [options.mode] - Transport mode: 'stdio' (default) or 'http'
   * @param {number} [options.port] - HTTP port (default: 3000, only for http mode)
   * @param {NodeJS.ReadableStream} [options.input] - Input stream (default: stdin)
   * @param {NodeJS.WritableStream} [options.output] - Output stream (default: stdout)
   */
  constructor(options = {}) {
    this.name = 'cynic-mcp';
    this.version = '0.1.0';

    // Transport mode: 'stdio' or 'http'
    this.mode = options.mode || 'stdio';
    this.port = options.port || 3000;

    // Authentication service (for HTTP mode)
    this.auth = options.auth || null;

    // Node instance (optional)
    this.node = options.node || null;

    // E-Score Calculator
    this.eScoreCalculator = options.eScoreCalculator || createEScoreCalculator({
      burnScale: 1e9,
      minJudgments: 10,
    });

    // Learning Service
    this.learningService = options.learningService || new LearningService({
      learningRate: 0.236,   // φ⁻³
      decayRate: 0.146,      // φ⁻⁴
      minFeedback: 5,
    });

    // Residual Detector - THE_UNNAMEABLE dimension discovery
    this.residualDetector = options.residualDetector || new ResidualDetector({
      threshold: 0.382,      // φ⁻²
      minSamples: 3,
      maxAnomalies: 1000,
    });

    // Engine Orchestrator for philosophical synthesis (73 engines)
    this.engineOrchestrator = options.engineOrchestrator || new EngineOrchestrator(globalEngineRegistry, {
      defaultStrategy: 'weighted-average',
      timeout: 5000,
    });

    // Engine Integration
    this.engineIntegration = options.engineIntegration || createEngineIntegration({
      registry: globalEngineRegistry,
      orchestrator: this.engineOrchestrator,
    });

    // Judge instance
    this.judge = options.judge || new CYNICJudge({
      eScoreProvider: this.eScoreCalculator,
      learningService: this.learningService,
      residualDetector: this.residualDetector,
      engineIntegration: this.engineIntegration,
      consultEngines: true,
    });

    // Collective Singleton - shared memory and 11 Dogs
    this.sharedMemory = options.sharedMemory || getSharedMemory();
    this.collectivePack = options.collectivePack || getCollectivePack({
      sharedMemory: this.sharedMemory,
      judge: this.judge,
      persistence: options.persistence || null,
      consensusThreshold: 0.618,
    });

    // Dog Orchestrator for parallel judgment voting
    this.dogOrchestrator = options.dogOrchestrator || new DogOrchestrator({
      collectivePack: this.collectivePack,
      sharedMemory: this.sharedMemory,
      mode: 'parallel',
      consensusThreshold: 0.618,
    });

    // LLM Router - lazy-initialized in start()
    this.llmRouter = options.llmRouter || null;
    this.perceptionRouter = options.perceptionRouter || null;

    // Kabbalistic Router
    this.kabbalisticRouter = options.kabbalisticRouter || new KabbalisticRouter({
      collectivePack: this.collectivePack,
    });

    // Unified Orchestrator
    this.unifiedOrchestrator = options.unifiedOrchestrator || new UnifiedOrchestrator({
      dogOrchestrator: this.dogOrchestrator,
      engineOrchestrator: this.engineOrchestrator,
      kabbalisticRouter: this.kabbalisticRouter,
      llmRouter: this.llmRouter,
      perceptionRouter: this.perceptionRouter,
      persistence: null,
      eventBus: null,
    });

    // Burn Verifier
    this.burnVerifier = options.burnVerifier || null;
    if (this.burnVerifier && this.eScoreCalculator) {
      this.eScoreCalculator.syncWithVerifier(this.burnVerifier);
    }

    // Blockchain Bridge
    this.blockchainBridge = options.blockchainBridge || null;

    // Data directory for file-based fallback
    this.dataDir = options.dataDir || null;

    // Core services (assigned by ServiceInitializer in _initialize)
    this.persistence = options.persistence || null;
    this.sessionManager = options.sessionManager || null;
    this.pojChainManager = options.pojChainManager || null;
    this.librarian = options.librarian || null;
    this.ecosystem = options.ecosystem || null;
    this.integrator = options.integrator || null;
    this.metrics = options.metrics || null;
    this.graph = options.graph || null;
    this.graphIntegration = options.graphIntegration || null;
    this.collective = options.collective || null;
    this.discovery = options.discovery || null;
    this.scheduler = options.scheduler || null;
    this.xProxy = options.xProxy || null;
    this.localXStore = options.localXStore || null;
    this.localPrivacyStore = options.localPrivacyStore || null;
    this.ecosystemMonitor = options.ecosystemMonitor || null;

    // Distributed Tracing (wired in InitializationPipeline)
    this.tracer = null;
    this.traceStorage = null;

    // Emergence Layer (Layer 7 - Keter)
    this.emergenceLayer = options.emergenceLayer || createEmergenceLayer({
      nodeId: `cynic_mcp_${Date.now().toString(36)}`,
      eScore: 50,
    });
    this.patternDetector = options.patternDetector || this.emergenceLayer.patterns;

    // Thermodynamics Tracker
    this.thermodynamics = options.thermodynamics || createThermodynamicsTracker();

    // Stdio streams
    this.input = options.input || process.stdin;
    this.output = options.output || process.stdout;

    // Internal state
    this._httpAdapter = null;
    this._activeRequests = new Set();
    this._running = false;
    this._collectiveReady = false;
    this.tools = {};
  }

  /**
   * Initialize components
   * DIP: Uses ServiceInitializer for core services, then InitializationPipeline for the rest
   * @private
   */
  async _initialize() {
    // Create service initializer with callbacks
    const initializer = new ServiceInitializer({
      dataDir: this.dataDir,
      onBlockCreated: (block) => this._broadcastSSEEvent('block', block),
      onDogDecision: (decision) => {
        const eventType = decision.blocked ? 'dogWarning' : 'dogDecision';
        this._broadcastSSEEvent(eventType, decision);
      },
    });

    // Initialize core services via DIP pattern
    const services = await initializer.initialize({
      judge: this.judge,
      persistence: this.persistence,
      sessionManager: this.sessionManager,
      pojChainManager: this.pojChainManager,
      librarian: this.librarian,
      discovery: this.discovery,
      ecosystem: this.ecosystem,
      integrator: this.integrator,
      graph: this.graph,
      graphIntegration: this.graphIntegration,
      collective: this.collective,
      scheduler: this.scheduler,
      metrics: this.metrics,
      eScoreCalculator: this.eScoreCalculator,
      learningService: this.learningService,
    });

    // Assign services to this instance
    Object.assign(this, services);

    // Run post-ServiceInitializer pipeline (Q-Learning, LLM routing, Solana, etc.)
    const pipeline = new InitializationPipeline({ server: this });
    await pipeline.run();
  }

  /**
   * Start MCP server
   * Mode is determined by constructor options (stdio or http)
   */
  async start() {
    if (this._running) return;

    // Initialize components
    await this._initialize();

    this._running = true;

    // Create JSON-RPC handler (shared by both transports)
    this._jsonRpcHandler = new JsonRpcHandler({ server: this });

    if (this.mode === 'http') {
      try {
        await this._startHttpServer();
      } catch (err) {
        this._running = false;
        console.error(`HTTP server startup failed: ${err.message}`);
        throw err;
      }
    } else {
      this._startStdioServer();
    }

    // Log startup to stderr (not interfering with JSON-RPC)
    console.error(`${IDENTITY.name} MCP Server started (${this.name} v${this.version})`);
    console.error(`   Mode: ${this.mode}${this.mode === 'http' ? ` (port ${this.port})` : ''}`);
    console.error(`   "${IDENTITY.philosophy.maxConfidence * 100}% max confidence"`);
    console.error(`   Tools: ${Object.keys(this.tools).join(', ')}`);
  }

  /**
   * Start stdio server (for Claude Desktop)
   * @private
   */
  _startStdioServer() {
    this._stdioTransport = new StdioTransport({
      input: this.input,
      output: this.output,
      onRequest: (message) => this._jsonRpcHandler.handleRequest(message),
      onClose: () => this.stop(),
    });
    this._stdioTransport.start();
  }

  /**
   * Start HTTP server (for remote deployment)
   * @private
   */
  async _startHttpServer() {
    this._httpAdapter = new HttpAdapter({
      port: this.port,
      dashboardPath: join(__dirname, 'dashboard'),
      auth: this.auth,
    });

    // Register route handlers
    const routes = new RouteHandlers({
      server: this,
      jsonRpcHandler: this._jsonRpcHandler,
    });
    routes.register(this._httpAdapter);

    // Start the HTTP server
    await this._httpAdapter.start();
  }

  /**
   * Stop MCP server
   */
  async stop() {
    if (!this._running) return;
    this._running = false;

    const shutdown = new ShutdownManager({ server: this });
    await shutdown.shutdown();
  }

  /**
   * Broadcast SSE event to all connected clients
   * @param {string} eventType - Event type
   * @param {Object} data - Event data
   * @private
   */
  _broadcastSSEEvent(eventType, data) {
    if (this._httpAdapter) {
      this._httpAdapter.broadcast(eventType, data);
    }
  }

  /**
   * Get server info
   * @returns {Object} Server information
   */
  getInfo() {
    return {
      name: this.name,
      version: this.version,
      running: this._running,
      tools: Object.keys(this.tools),
      hasNode: !!this.node,
      persistenceBackend: this.persistence?._backend || 'none',
      persistenceCapabilities: this.persistence?.capabilities || {},
      judgeStats: this.judge.getStats(),
      collective: this.collective?.getSummary() || { initialized: false },
      cynicState: this.collective?.cynic?.metaState || 'not_initialized',
      sessions: this.sessionManager?.getSummary() || { activeCount: 0 },
      pojChain: this.pojChainManager?.getStatus() || { initialized: false },
      librarian: this.librarian?._initialized
        ? { initialized: true, stats: this.librarian._stats }
        : { initialized: false },
    };
  }
}

export default MCPServer;
