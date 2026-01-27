/**
 * CYNIC MCP Server
 *
 * Model Context Protocol server for AI tool integration
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
import { PHI_INV, PHI_INV_2, IDENTITY, PeriodicScheduler, FibonacciIntervals, EcosystemMonitor } from '@cynic/core';

// SRP: HTTP concerns extracted to HttpAdapter
import { HttpAdapter } from './server/HttpAdapter.js';
// DIP: Service creation via ServiceInitializer
import { ServiceInitializer } from './server/ServiceInitializer.js';

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = join(__filename, '..');
import { CYNICJudge, createCollectivePack, LearningService, createEScoreCalculator } from '@cynic/node';
import { createAllTools } from './tools/index.js';
import { PersistenceManager } from './persistence.js';
import { SessionManager } from './session-manager.js';
// Solana anchoring support
import { AnchorQueue, loadWalletFromFile, loadWalletFromEnv, SolanaCluster } from '@cynic/anchor';
// Burns verification (for E-Score integration)
import { createBurnVerifier } from '@cynic/burns';
import { PoJChainManager } from './poj-chain-manager.js';
import { LibrarianService } from './librarian-service.js';
import { AuthService } from './auth-service.js';
import { DiscoveryService } from './discovery-service.js';

/**
 * MCP Server for CYNIC
 *
 * Provides brain_cynic_* tools for Claude Code integration:
 * - brain_cynic_judge: Multi-dimensional judgment
 * - brain_cynic_digest: Content extraction
 * - brain_health: System health
 * - brain_search: Knowledge search
 * - brain_patterns: Pattern detection
 * - brain_cynic_feedback: Learning from outcomes
 */
// HTTP mode constants
const MAX_BODY_SIZE = 1024 * 1024; // 1MB max request body
const REQUEST_TIMEOUT_MS = 30000; // 30 second request timeout
const SHUTDOWN_TIMEOUT_MS = 10000; // 10 second shutdown grace period
const MAX_RESPONSE_SIZE = 100 * 1024; // 100KB max response size (prevents Claude Code blocking)

export class MCPServer {
  /**
   * Create MCP server
   * @param {Object} [options] - Server options
   * @param {Object} [options.node] - CYNICNode instance
   * @param {Object} [options.judge] - CYNICJudge instance
   * @param {Object} [options.persistence] - PersistenceManager instance
   * @param {Object} [options.sessionManager] - SessionManager instance (for multi-user sessions)
   * @param {Object} [options.pojChainManager] - PoJChainManager instance (for PoJ blockchain)
   * @param {Object} [options.librarian] - LibrarianService instance (for documentation caching)
   * @param {Object} [options.agents] - AgentManager instance (The Four Dogs)
   * @param {Object} [options.auth] - AuthService instance (for HTTP authentication)
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

    // E-Score Calculator (for vote weight based on burns/uptime/quality)
    // This is the bridge between Burns and Judgment weight
    this.eScoreCalculator = options.eScoreCalculator || createEScoreCalculator({
      burnScale: 1e9,      // 1B scale for burn normalization
      minJudgments: 10,    // Minimum judgments for quality metric
    });

    // Learning Service (for RLHF-style weight adjustments)
    // Tracks feedback sources: tests, commits, PRs, builds, manual
    this.learningService = options.learningService || new LearningService({
      learningRate: 0.236,  // φ⁻³ - conservative learning
      decayRate: 0.146,     // φ⁻⁴ - slow decay
      minFeedback: 5,       // Minimum feedback before learning
    });

    // Judge instance (required) - now wired with E-Score and Learning
    this.judge = options.judge || new CYNICJudge({
      eScoreProvider: this.eScoreCalculator,
      learningService: this.learningService,
    });

    // Burn Verifier (optional - requires Solana RPC connection)
    // When available, automatically syncs with E-Score calculator
    this.burnVerifier = options.burnVerifier || null;

    // Wire Burns → E-Score if both are available
    // This auto-updates E-Score when burns are verified on-chain
    if (this.burnVerifier && this.eScoreCalculator) {
      this.eScoreCalculator.syncWithVerifier(this.burnVerifier);
    }

    // Data directory for file-based fallback
    this.dataDir = options.dataDir || null;

    // Persistence manager (PostgreSQL + Redis with automatic fallback)
    this.persistence = options.persistence || null;

    // Session manager for multi-user isolation (created after persistence)
    this.sessionManager = options.sessionManager || null;

    // PoJ Chain manager for Proof of Judgment blockchain
    this.pojChainManager = options.pojChainManager || null;

    // Librarian service for documentation caching
    this.librarian = options.librarian || null;

    // Ecosystem service for pre-loaded documentation
    this.ecosystem = options.ecosystem || null;

    // Integrator service for cross-project synchronization
    this.integrator = options.integrator || null;

    // Metrics service for monitoring
    this.metrics = options.metrics || null;

    // Graph overlay for relationship tracking
    this.graph = options.graph || null;

    // Judgment-Graph integration (connects judge to graph)
    this.graphIntegration = options.graphIntegration || null;

    // Collective Pack - All 11 Dogs (Sefirot) working as one
    // This is the unified collective consciousness system
    this.collective = options.collective || null;

    // Discovery service for MCP servers, plugins, and CYNIC nodes
    this.discovery = options.discovery || null;

    // Periodic scheduler for automated tasks (ecosystem awareness)
    this.scheduler = options.scheduler || null;

    // Ecosystem monitor for GitHub tracking
    this.ecosystemMonitor = options.ecosystemMonitor || null;

    // Stdio streams (for stdio mode)
    this.input = options.input || process.stdin;
    this.output = options.output || process.stdout;

    // HTTP adapter (for http mode) - SRP: HttpAdapter handles HTTP/SSE
    this._httpAdapter = null;
    this._activeRequests = new Set(); // Track in-flight JSON-RPC requests for graceful shutdown

    // Request buffer for stdin parsing
    this._buffer = '';

    // Running flag
    this._running = false;

    // Tool registry (populated on start)
    this.tools = {};
  }

  /**
   * Initialize components
   * DIP: Uses ServiceInitializer for core services
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
    // Pass any pre-configured services from constructor
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

    // ═══════════════════════════════════════════════════════════════════════════
    // MCPServer-specific initialization (not in ServiceInitializer)
    // ═══════════════════════════════════════════════════════════════════════════

    // Initialize Solana anchoring if wallet is configured
    await this._initializeSolanaAnchoring();

    // Initialize Ecosystem Monitor for GitHub tracking
    if (!this.ecosystemMonitor) {
      this.ecosystemMonitor = new EcosystemMonitor({
        maxCacheSize: 100,
        onUpdate: (updates, source) => {
          if (updates.length > 0) {
            this._broadcastSSEEvent('ecosystem_updates', {
              count: updates.length,
              source: source?.name || 'all',
              highPriority: updates.filter(u => u.priority === 'HIGH' || u.priority === 'CRITICAL').length,
              timestamp: Date.now(),
            });
          }
        },
      });
      this.ecosystemMonitor.registerSolanaDefaults();
      this.ecosystemMonitor.trackGitHubRepo('anza-xyz', 'agave', {
        trackReleases: true,
        trackCommits: false,
      });
      console.error(`   EcosystemMonitor: ready (${this.ecosystemMonitor.sources.size} sources)`);
    }

    // Register ecosystem awareness task with scheduler
    this._registerSchedulerTasks();

    // Initialize Auth service for HTTP mode
    if (this.mode === 'http' && !this.auth) {
      this.auth = new AuthService({
        publicPaths: ['/', '/health', '/metrics', '/dashboard', '/sse', '/api', '/mcp', '/message'],
      });
      const authStatus = this.auth.required ? 'required' : 'optional (dev mode)';
      console.error(`   Auth: ${authStatus} (${this.auth.apiKeys.size} keys configured)`);
    }

    // Register tools with current instances
    this.tools = createAllTools({
      judge: this.judge,
      node: this.node,
      persistence: this.persistence,
      collective: this.collective,
      sessionManager: this.sessionManager,
      pojChainManager: this.pojChainManager,
      librarian: this.librarian,
      ecosystem: this.ecosystem,
      integrator: this.integrator,
      metrics: this.metrics,
      graphIntegration: this.graphIntegration,
      discovery: this.discovery,
      learningService: this.learningService,
      eScoreCalculator: this.eScoreCalculator,
      onJudgment: (judgment) => this._broadcastSSEEvent('judgment', judgment),
    });
  }

  /**
   * Initialize Solana anchoring if configured
   * @private
   */
  async _initializeSolanaAnchoring() {
    const walletPath = process.env.CYNIC_SOLANA_WALLET || join(__dirname, '../../anchor/test/.devnet-wallet.json');
    const enableAnchoring = process.env.CYNIC_ENABLE_ANCHORING === 'true';

    if (enableAnchoring) {
      try {
        const wallet = loadWalletFromEnv('CYNIC_SOLANA_KEY') || loadWalletFromFile(walletPath);
        const cluster = process.env.CYNIC_SOLANA_CLUSTER || 'devnet';

        this.anchorQueue = new AnchorQueue({
          wallet,
          cluster: SolanaCluster[cluster.toUpperCase()] || SolanaCluster.DEVNET,
          batchSize: 5,
          batchTimeout: 61800,
        });

        this.pojChainManager.setAnchorQueue(this.anchorQueue);
        console.error(`   Solana Anchoring: ENABLED (${cluster})`);
        const pubKeyStr = wallet.publicKey?.toBase58
          ? wallet.publicKey.toBase58()
          : Buffer.from(wallet.publicKey || wallet._publicKey || []).toString('hex');
        console.error(`   Wallet: ${pubKeyStr.slice(0, 16)}...`);
      } catch (err) {
        console.error(`   Solana Anchoring: DISABLED (${err.message})`);
      }
    } else {
      console.error('   Solana Anchoring: disabled (set CYNIC_ENABLE_ANCHORING=true)');
    }
  }

  /**
   * Register scheduler tasks
   * @private
   */
  _registerSchedulerTasks() {
    if (!this.scheduler) return;

    // Psychology checkpoint - sync every 10 minutes to prevent data loss on crash
    this.scheduler.register({
      id: 'psychology_checkpoint',
      name: 'Psychology Checkpoint',
      intervalMs: 10 * 60 * 1000, // 10 minutes
      runImmediately: false,
      handler: async () => {
        if (!this.persistence?.psychology || !this.sessionManager?._currentSession) {
          return { skipped: true, reason: 'no_active_session' };
        }

        const session = this.sessionManager._currentSession;
        const userId = session.userId;

        if (!userId) {
          return { skipped: true, reason: 'no_user_id' };
        }

        try {
          // Get current psychology state from collective if available
          let psychologyData = null;

          if (this.collective?.cynic) {
            const cynicStats = this.collective.cynic.getStatus?.() || {};
            psychologyData = {
              sessionId: session.sessionId,
              checkpoint: true,
              timestamp: Date.now(),
              observedEvents: cynicStats.observedEvents || 0,
              synthesizedPatterns: cynicStats.synthesizedPatterns || 0,
              decisions: cynicStats.decisions || 0,
            };
          }

          if (psychologyData) {
            await this.persistence.syncPsychology(userId, psychologyData);
            console.error(`🧠 [CHECKPOINT] Psychology synced for ${userId.slice(0, 8)}...`);
            return { synced: true, userId: userId.slice(0, 8) };
          }

          return { skipped: true, reason: 'no_psychology_data' };
        } catch (err) {
          console.error(`🧠 [CHECKPOINT] Psychology sync failed: ${err.message}`);
          return { error: err.message };
        }
      },
    });

    console.error('   Psychology checkpoint: every 10 minutes');

    this.scheduler.register({
      id: 'ecosystem_awareness',
      name: 'Ecosystem Awareness',
      intervalMs: FibonacciIntervals.SIXHOURLY,
      runImmediately: false,
      handler: async () => {
        console.error('🐕 [Scheduler] Starting ecosystem awareness cycle...');

        const fetchResult = await this.ecosystemMonitor.fetchAll();
        const highPriorityUpdates = (fetchResult.updates || [])
          .filter(u => u.priority === 'HIGH' || u.priority === 'CRITICAL');

        let judgedCount = 0;
        if (highPriorityUpdates.length > 0 && this.graphIntegration) {
          for (const update of highPriorityUpdates.slice(0, 5)) {
            try {
              const item = {
                type: 'ecosystem_update',
                content: `${update.type}: ${update.title || 'Update'}\n\n${update.description || ''}`.slice(0, 500),
                source: update.source,
                sources: [update.url].filter(Boolean),
                priority: update.priority,
                meta: update.meta,
              };
              await this.graphIntegration.judgeWithGraph(item, { source: 'ecosystem_monitor' });
              judgedCount++;
            } catch (e) {
              console.error(`🐕 [Scheduler] Judge error: ${e.message}`);
            }
          }
          if (judgedCount > 0) {
            console.error(`🐕 [Scheduler] Judged ${judgedCount} high-priority updates`);
          }
        }

        this._broadcastSSEEvent('ecosystem_cycle', {
          fetched: fetchResult.fetched || 0,
          skipped: fetchResult.skipped || 0,
          errors: fetchResult.errors || 0,
          updatesFound: fetchResult.updates?.length || 0,
          highPriority: highPriorityUpdates.length,
          judged: judgedCount,
          timestamp: Date.now(),
        });

        console.error(`🐕 [Scheduler] Ecosystem cycle complete: ${fetchResult.updates?.length || 0} updates, ${judgedCount} judged`);
        return { fetched: fetchResult.fetched || 0, updatesFound: fetchResult.updates?.length || 0, highPriority: highPriorityUpdates.length, judged: judgedCount };
      },
    });

    console.error(`   Scheduler: ready (ecosystem awareness every ${FibonacciIntervals.SIXHOURLY / 3600000}h)`);
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

    if (this.mode === 'http') {
      // HTTP/SSE mode for remote deployment (Render, etc.)
      await this._startHttpServer();
    } else {
      // stdio mode for Claude Desktop integration
      this._startStdioServer();
    }

    // Log startup to stderr (not interfering with JSON-RPC)
    console.error(`🐕 ${IDENTITY.name} MCP Server started (${this.name} v${this.version})`);
    console.error(`   Mode: ${this.mode}${this.mode === 'http' ? ` (port ${this.port})` : ''}`);
    console.error(`   κυνικός - "${IDENTITY.philosophy.maxConfidence * 100}% max confidence"`);
    console.error(`   Tools: ${Object.keys(this.tools).join(', ')}`);
  }

  /**
   * Start stdio server (for Claude Desktop)
   * @private
   */
  _startStdioServer() {
    this.input.setEncoding('utf8');
    this.input.on('data', (chunk) => this._handleInput(chunk));
    this.input.on('end', () => this.stop());
  }

  /**
   * Start HTTP server (for remote deployment)
   * Uses HttpAdapter for SRP compliance
   * @private
   */
  async _startHttpServer() {
    // Create HttpAdapter with configuration
    this._httpAdapter = new HttpAdapter({
      port: this.port,
      dashboardPath: join(__dirname, 'dashboard'),
      auth: this.auth,
    });

    // Set route handlers (MCPServer provides domain logic)
    this._httpAdapter.setRoute('health', (req, res) => this._handleHealthRequest(req, res));
    this._httpAdapter.setRoute('metrics', (req, res, url) => this._handleMetricsRequest(req, res, url));
    this._httpAdapter.setRoute('mcp', (req, res) => this._handleMcpRequest(req, res));
    this._httpAdapter.setRoute('api', (req, res, url) => this._handleApiRequest(req, res, url));
    this._httpAdapter.setRoute('hooks', (req, res, url) => this._handleHooksRequest(req, res, url));
    this._httpAdapter.setRoute('psychology', (req, res, url) => this._handlePsychologyRequest(req, res, url));

    // Start the HTTP server
    await this._httpAdapter.start();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HTTP ROUTE HANDLERS (SRP: Domain-specific logic, HttpAdapter handles transport)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Handle health check requests
   * Route: GET / or /health
   * @private
   */
  _handleHealthRequest(_req, res) {
    HttpAdapter.sendJson(res, 200, {
      status: 'healthy',
      server: this.name,
      version: this.version,
      tools: Object.keys(this.tools).length,
      uptime: process.uptime(),
      phi: PHI_INV,
    });
  }

  /**
   * Handle metrics requests (Prometheus or HTML)
   * Route: GET /metrics or /metrics/html
   * @private
   */
  async _handleMetricsRequest(_req, res, url) {
    if (!this.metrics) {
      HttpAdapter.sendJson(res, 503, { error: 'Metrics service not available' });
      return;
    }

    try {
      if (url.pathname === '/metrics/html') {
        // HTML dashboard
        const html = await this.metrics.toHTML();
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(html);
      } else {
        // Prometheus format
        const prometheus = await this.metrics.toPrometheus();
        res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8' });
        res.end(prometheus);
      }
    } catch (err) {
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end(`# Error collecting metrics: ${err.message}\n`);
    }
  }

  /**
   * Handle MCP JSON-RPC requests
   * Route: POST /mcp or /message
   * @private
   */
  async _handleMcpRequest(req, res) {
    await this._handleHttpMessage(req, res);
  }

  /**
   * Handle REST API requests
   * Route: /api/*
   * @private
   */
  async _handleApiRequest(req, res, url) {
    const pathname = url.pathname;

    // List all available tools (API discovery)
    if (pathname === '/api/tools' && req.method === 'GET') {
      const tools = Object.values(this.tools).map(t => ({
        name: t.name,
        description: t.description,
        inputSchema: t.inputSchema,
      }));
      HttpAdapter.sendJson(res, 200, { tools });
      return;
    }

    // REST API for specific tool
    if (pathname.startsWith('/api/tools/')) {
      await this._handleApiToolRequest(req, res, url);
      return;
    }

    // 404 for unknown API routes
    HttpAdapter.sendJson(res, 404, { error: 'API endpoint not found' });
  }

  /**
   * Handle hooks requests
   * Route: /hooks/*
   * @private
   */
  async _handleHooksRequest(req, res, url) {
    const pathname = url.pathname;

    // Hook event endpoint - bridges Claude Code hooks to the Collective
    if (pathname === '/hooks/event' && req.method === 'POST') {
      await this._handleHookEvent(req, res);
      return;
    }

    HttpAdapter.sendJson(res, 404, { error: 'Hook endpoint not found' });
  }

  /**
   * Handle psychology requests
   * Route: /psychology/*
   * @private
   */
  async _handlePsychologyRequest(req, res, url) {
    const pathname = url.pathname;

    // Sync psychology state to database (called by sleep.cjs)
    if (pathname === '/psychology/sync' && req.method === 'POST') {
      await this._handlePsychologySync(req, res);
      return;
    }

    // Load psychology state from database (called by awaken.cjs)
    if (pathname === '/psychology/load' && req.method === 'GET') {
      await this._handlePsychologyLoad(req, res, url);
      return;
    }

    HttpAdapter.sendJson(res, 404, { error: 'Psychology endpoint not found' });
  }

  /**
   * Broadcast SSE event to all connected clients
   * Delegates to HttpAdapter for transport
   * @param {string} eventType - Event type (judgment, block, alert, etc.)
   * @param {Object} data - Event data
   * @private
   */
  _broadcastSSEEvent(eventType, data) {
    if (this._httpAdapter) {
      this._httpAdapter.broadcast(eventType, data);
    }
  }

  /**
   * Handle REST API tool requests
   * @private
   */
  async _handleApiToolRequest(req, res, url) {
    // Extract tool name from URL: /api/tools/brain_cynic_judge
    const toolName = url.pathname.replace('/api/tools/', '');

    const tool = this.tools[toolName];
    if (!tool) {
      HttpAdapter.sendJson(res, 404, { error: `Tool not found: ${toolName}` });
      return;
    }

    // GET = get tool info, POST = execute tool
    if (req.method === 'GET') {
      HttpAdapter.sendJson(res, 200, {
        name: tool.name,
        description: tool.description,
        inputSchema: tool.inputSchema,
      });
      return;
    }

    if (req.method !== 'POST') {
      HttpAdapter.sendJson(res, 405, { error: 'Method not allowed' });
      return;
    }

    try {
      const body = await HttpAdapter.readBody(req);
      const args = body ? JSON.parse(body) : {};
      const toolUseId = `api_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

      // 🐕 COLLECTIVE: PreToolUse check (same as MCP tools/call)
      if (this.collective) {
        const preResult = await this.collective.receiveHookEvent({
          hookType: 'PreToolUse',
          payload: { tool: toolName, toolUseId, input: args },
        });

        if (preResult.blocked) {
          HttpAdapter.sendJson(res, 403, {
            error: `[BLOCKED] ${preResult.blockMessage || 'Operation blocked by collective'}`,
            blockedBy: preResult.blockedBy,
          });
          return;
        }
      }

      // Execute tool
      console.error(`🐕 [API] Tool ${toolName} called`);
      const startTime = Date.now();
      const result = await tool.handler(args);
      const duration = Date.now() - startTime;

      // 🐕 COLLECTIVE: PostToolUse (non-blocking)
      if (this.collective) {
        this.collective.receiveHookEvent({
          hookType: 'PostToolUse',
          payload: {
            tool: toolName,
            toolUseId,
            input: args,
            output: typeof result === 'string' ? result.slice(0, 500) : JSON.stringify(result).slice(0, 500),
            duration,
            success: true,
          },
        }).catch(() => {});
      }

      HttpAdapter.sendJson(res, 200, { success: true, result, duration });
    } catch (err) {
      console.error(`🐕 [API] Tool ${toolName} error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle hook event from Claude Code
   * Bridges external hooks to the Collective eventBus
   *
   * POST /api/hooks/event
   * Body: { hookType, payload, userId, sessionId }
   *
   * @private
   */
  async _handleHookEvent(req, res) {
    try {
      const body = await HttpAdapter.readBody(req);
      const hookData = body ? JSON.parse(body) : {};

      // Validate required fields
      if (!hookData.hookType) {
        HttpAdapter.sendJson(res, 400, { error: 'hookType is required' });
        return;
      }

      // Check if collective is available
      if (!this.collective) {
        HttpAdapter.sendJson(res, 503, { error: 'Collective not initialized' });
        return;
      }

      // Forward to collective
      const result = await this.collective.receiveHookEvent(hookData);

      // Log for debugging
      console.error(`🐕 [HOOK] ${hookData.hookType} → ${result.delivered || 0} dogs notified`);

      // Broadcast to SSE clients (generic message)
      this._broadcastSSEEvent('hook:received', {
        hookType: hookData.hookType,
        delivered: result.delivered || 0,
        timestamp: Date.now(),
      });

      // Also broadcast typed events for Live View (tool timeline + audio)
      if (hookData.hookType === 'PreToolUse' && hookData.payload) {
        this._broadcastSSEEvent('tool_pre', {
          tool: hookData.payload.tool || 'unknown',
          toolUseId: hookData.payload.toolUseId || `hook_${Date.now()}`,
          input: hookData.payload.input,
          timestamp: Date.now(),
        });
      } else if (hookData.hookType === 'PostToolUse' && hookData.payload) {
        this._broadcastSSEEvent('tool_post', {
          tool: hookData.payload.tool || 'unknown',
          toolUseId: hookData.payload.toolUseId || `hook_${Date.now()}`,
          duration: hookData.payload.duration,
          success: hookData.payload.success !== false,
          timestamp: Date.now(),
        });
      }

      HttpAdapter.sendJson(res, 200, result);
    } catch (err) {
      console.error(`🐕 [HOOK] Error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle psychology sync (sleep.cjs → PostgreSQL)
   * "Le chien apprend. L'apprentissage persiste."
   * @private
   */
  async _handlePsychologySync(req, res) {
    try {
      const body = await HttpAdapter.readBody(req);
      const { userId, data } = body ? JSON.parse(body) : {};

      if (!userId || !data) {
        HttpAdapter.sendJson(res, 400, { error: 'userId and data are required' });
        return;
      }

      if (!this.persistence) {
        HttpAdapter.sendJson(res, 503, { error: 'Persistence not available' });
        return;
      }

      const result = await this.persistence.syncPsychology(userId, data);

      console.error(`🧠 [PSYCHOLOGY] Synced for ${userId}`);

      HttpAdapter.sendJson(res, 200, { success: true, result });
    } catch (err) {
      console.error(`🧠 [PSYCHOLOGY] Sync error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle psychology load (awaken.cjs ← PostgreSQL)
   * "Comprendre l'humain pour mieux l'aider"
   * @private
   */
  async _handlePsychologyLoad(_req, res, url) {
    try {
      const userId = url.searchParams.get('userId');

      if (!userId) {
        HttpAdapter.sendJson(res, 400, { error: 'userId is required' });
        return;
      }

      if (!this.persistence) {
        HttpAdapter.sendJson(res, 503, { error: 'Persistence not available' });
        return;
      }

      const data = await this.persistence.loadPsychology(userId);

      if (!data) {
        HttpAdapter.sendJson(res, 404, { error: 'No psychology data found for user' });
        return;
      }

      console.error(`🧠 [PSYCHOLOGY] Loaded for ${userId}`);

      HttpAdapter.sendJson(res, 200, data);
    } catch (err) {
      console.error(`🧠 [PSYCHOLOGY] Load error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle HTTP POST message
   * @private
   */
  async _handleHttpMessage(req, res) {
    const requestId = Symbol('request');
    this._activeRequests.add(requestId);

    // Set request timeout
    const timeoutId = setTimeout(() => {
      if (!res.headersSent) {
        res.writeHead(408, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          jsonrpc: '2.0',
          id: null,
          error: { code: -32000, message: 'Request timeout' },
        }));
      }
      this._activeRequests.delete(requestId);
    }, REQUEST_TIMEOUT_MS);

    try {
      // Collect body with size limit
      let body = '';
      let bodySize = 0;

      for await (const chunk of req) {
        bodySize += chunk.length;
        if (bodySize > MAX_BODY_SIZE) {
          clearTimeout(timeoutId);
          this._activeRequests.delete(requestId);
          res.writeHead(413, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({
            jsonrpc: '2.0',
            id: null,
            error: { code: -32000, message: `Request body too large (max ${MAX_BODY_SIZE} bytes)` },
          }));
          return;
        }
        body += chunk;
      }

      const message = JSON.parse(body);

      if (!message.jsonrpc || message.jsonrpc !== '2.0') {
        clearTimeout(timeoutId);
        this._activeRequests.delete(requestId);
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          jsonrpc: '2.0',
          id: message.id,
          error: { code: -32600, message: 'Invalid JSON-RPC version' },
        }));
        return;
      }

      // Process message
      const result = await this._handleRequestInternal(message);

      clearTimeout(timeoutId);
      this._activeRequests.delete(requestId);

      if (result === null) {
        // Notification - no response
        res.writeHead(204);
        res.end();
        return;
      }

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(result));
    } catch (err) {
      clearTimeout(timeoutId);
      this._activeRequests.delete(requestId);
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        jsonrpc: '2.0',
        id: null,
        error: { code: -32700, message: `Parse error: ${err.message}` },
      }));
    }
  }

  /**
   * Internal request handler (used by both stdio and HTTP)
   * @private
   * @returns {Object|null} Response or null for notifications
   */
  async _handleRequestInternal(request) {
    const { id, method, params = {} } = request;

    // DEBUG: Log every request (stderr for MCP protocol)
    console.error(`🐕 [REQUEST] method=${method} id=${id}`);

    try {
      let result;

      switch (method) {
        case 'initialize':
          result = await this._handleInitialize(params);
          break;

        case 'initialized':
        case 'notifications/initialized':
          return null; // Notification - no response

        case 'tools/list':
          result = await this._handleToolsList();
          break;

        case 'tools/call':
          result = await this._handleToolsCall(params);
          break;

        case 'resources/list':
          result = { resources: [] };
          break;

        case 'prompts/list':
          result = { prompts: [] };
          break;

        case 'ping':
          result = { pong: true, timestamp: Date.now() };
          break;

        case 'shutdown':
          await this.stop();
          return { jsonrpc: '2.0', id, result: { success: true } };

        default:
          return {
            jsonrpc: '2.0',
            id,
            error: { code: -32601, message: `Method not found: ${method}` },
          };
      }

      return { jsonrpc: '2.0', id, result };
    } catch (err) {
      // Track errors for user profile aggregation (only for tool calls)
      if (method === 'tools/call' && this.sessionManager) {
        // Don't count blocked operations as errors - they're intentional
        if (!err.message?.includes('[BLOCKED]')) {
          this.sessionManager.recordError();
        }
      }

      // 🐕 SAGE: Share wisdom when errors occur (via Collective)
      if (this.collective) {
        this.collective.getWisdom('error_recovery', {
          errorMessage: err.message,
          method,
          context: 'mcp_request_error',
        }).then(wisdom => {
          if (wisdom?.message) {
            console.error(`🐕 Sage wisdom: ${wisdom.message}`);
          }
        }).catch(() => {
          // Sage is non-blocking - ignore errors
        });
      }

      return {
        jsonrpc: '2.0',
        id,
        error: { code: -32000, message: err.message },
      };
    }
  }

  /**
   * Stop MCP server
   */
  async stop() {
    if (!this._running) return;

    this._running = false;
    console.error('🐕 CYNIC MCP Server shutting down...');

    // Stop HTTP adapter (handles SSE clients and graceful shutdown)
    if (this._httpAdapter) {
      await this._httpAdapter.stop(SHUTDOWN_TIMEOUT_MS);
    }

    // Flush PoJ chain (create final block from pending judgments)
    if (this.pojChainManager) {
      try {
        await this.pojChainManager.close();
      } catch (e) {
        console.error('Error closing PoJ chain:', e.message);
      }
    }

    // Stop periodic scheduler
    if (this.scheduler) {
      try {
        this.scheduler.stopAll();
        console.error('   Scheduler stopped');
      } catch (e) {
        console.error('Error stopping scheduler:', e.message);
      }
    }

    // Stop discovery health checks
    if (this.discovery) {
      try {
        await this.discovery.shutdown();
      } catch (e) {
        console.error('Error shutting down discovery:', e.message);
      }
    }

    // Close persistence connections (handles file-based save automatically)
    if (this.persistence) {
      try {
        await this.persistence.close();
      } catch (e) {
        console.error('Error closing persistence:', e.message);
      }
    }

    console.error('🐕 CYNIC MCP Server stopped');

    // Only exit process in stdio mode (HTTP mode should stay alive for graceful restart)
    if (this.mode === 'stdio') {
      process.exit(0);
    }
  }

  /**
   * Handle incoming stdio data
   * @private
   */
  _handleInput(chunk) {
    this._buffer += chunk;

    // Process complete JSON-RPC messages (newline-delimited)
    let newlineIndex;
    while ((newlineIndex = this._buffer.indexOf('\n')) !== -1) {
      const line = this._buffer.slice(0, newlineIndex).trim();
      this._buffer = this._buffer.slice(newlineIndex + 1);

      if (line) {
        this._processMessage(line);
      }
    }
  }

  /**
   * Process a JSON-RPC message
   * @private
   */
  async _processMessage(line) {
    try {
      const message = JSON.parse(line);

      if (!message.jsonrpc || message.jsonrpc !== '2.0') {
        this._sendError(message.id, -32600, 'Invalid JSON-RPC version');
        return;
      }

      // Handle different message types
      if (message.method) {
        await this._handleRequest(message);
      }
    } catch (err) {
      this._sendError(null, -32700, `Parse error: ${err.message}`);
    }
  }

  /**
   * Handle JSON-RPC request (stdio mode)
   * @private
   */
  async _handleRequest(request) {
    const response = await this._handleRequestInternal(request);

    // Notifications don't get responses
    if (response === null) {
      return;
    }

    // Send response via stdio
    if (response.error) {
      this._sendError(response.id, response.error.code, response.error.message);
    } else {
      this._sendResponse(response.id, response.result);
    }
  }

  /**
   * Handle initialize request
   * @private
   */
  async _handleInitialize(params) {
    const { protocolVersion, clientInfo } = params;

    // Log client info
    if (clientInfo) {
      console.error(`   Client: ${clientInfo.name} v${clientInfo.version || '?'}`);
    }

    return {
      protocolVersion: '2024-11-05',
      serverInfo: {
        name: this.name,
        version: this.version,
      },
      capabilities: {
        tools: {},
        resources: {},
        prompts: {},
      },
    };
  }

  /**
   * Handle tools/list request
   * @private
   */
  async _handleToolsList() {
    return {
      tools: Object.values(this.tools).map(tool => ({
        name: tool.name,
        description: tool.description,
        inputSchema: tool.inputSchema,
      })),
    };
  }

  /**
   * Handle tools/call request
   * @private
   */
  async _handleToolsCall(params) {
    const { name, arguments: args = {} } = params;

    // DEBUG: Log at very start of tool call (stderr for MCP protocol)
    console.error(`🐕 [TOOL_CALL] ${name} called at ${new Date().toISOString()}`);

    const tool = this.tools[name];
    if (!tool) {
      throw new Error(`Tool not found: ${name}`);
    }

    // Generate toolUseId for duration tracking (Vibecraft pattern)
    const toolUseId = `tool_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    // 🐕 COLLECTIVE: PreToolUse → Full pipeline (shouldTrigger → analyze → decide)
    // All 11 Dogs process the event, Guardian can block
    if (this.collective) {
      const hookResult = await this.collective.receiveHookEvent({
        hookType: 'PreToolUse',
        payload: {
          tool: name,
          toolUseId,
          input: args,
        },
      });

      // Check if any Dog blocked the operation
      if (hookResult.blocked) {
        const blockedBy = hookResult.blockedBy || 'guardian';
        const message = hookResult.blockMessage || 'Operation blocked by collective';
        console.error(`🐕 [BLOCKED] Tool "${name}" blocked by ${blockedBy}: ${message}`);

        // Track danger blocked for user profile aggregation
        if (this.sessionManager) {
          this.sessionManager.recordDangerBlocked();
        }

        throw new Error(`[BLOCKED] ${message}`);
      }

      // Log warnings from agents
      for (const result of hookResult.agentResults || []) {
        if (result.response === 'warn' && result.message) {
          console.error(`🐕 [WARNING] ${result.agent}: ${result.message}`);
        }
      }

      // Broadcast to SSE for Live View
      this._broadcastSSEEvent('tool_pre', {
        tool: name,
        toolUseId,
        input: args,
        dogsNotified: hookResult?.delivered || 0,
        agentsTriggered: hookResult.agentResults?.length || 0,
        timestamp: Date.now(),
      });
    }

    // Execute tool handler
    const startTime = Date.now();
    const result = await tool.handler(args);
    const duration = Date.now() - startTime;

    // Track tool call for user profile aggregation
    if (this.sessionManager) {
      this.sessionManager.recordToolCall();
    }

    // 🐕 COLLECTIVE: PostToolUse → Full pipeline (all 11 Dogs analyze)
    // Analyst tracks patterns, Scholar extracts knowledge, etc.
    if (this.collective) {
      const hookResult = await this.collective.receiveHookEvent({
        hookType: 'PostToolUse',
        payload: {
          tool: name,
          toolUseId,
          input: args,
          output: typeof result === 'string' ? result.slice(0, 500) : JSON.stringify(result).slice(0, 500),
          duration,
          success: true,
        },
      });

      // Broadcast to SSE for Live View with duration tracking
      this._broadcastSSEEvent('tool_post', {
        tool: name,
        toolUseId,
        duration,
        success: true,
        dogsNotified: hookResult?.delivered || 0,
        agentsTriggered: hookResult.agentResults?.length || 0,
        timestamp: Date.now(),
      });
    }

    // Note: Judgment storage now handled inside createJudgeTool handler
    // for better access to full judgment data including dimensionScores

    return {
      content: [{
        type: 'text',
        text: JSON.stringify(result, null, 2),
      }],
    };
  }

  /**
   * Send JSON-RPC response
   * @private
   */
  _sendResponse(id, result) {
    const response = {
      jsonrpc: '2.0',
      id,
      result,
    };
    let json = JSON.stringify(response);

    // Truncate very large responses to prevent Claude Code blocking
    if (json.length > MAX_RESPONSE_SIZE) {
      const sizeKB = (json.length / 1024).toFixed(1);
      console.error(`⚠️ Truncating large MCP response: ${sizeKB}KB → 100KB for request ${id}`);

      // Try to preserve structure by truncating content arrays/strings
      const truncatedResult = this._truncateResult(result, MAX_RESPONSE_SIZE - 500);
      const truncatedResponse = {
        jsonrpc: '2.0',
        id,
        result: truncatedResult,
      };
      json = JSON.stringify(truncatedResponse);
    }

    this.output.write(json + '\n');
  }

  /**
   * Truncate result to fit within size limit
   * @private
   */
  _truncateResult(result, maxSize) {
    // If result is a string, truncate it
    if (typeof result === 'string') {
      if (result.length > maxSize) {
        return result.slice(0, maxSize) + '\n\n... [TRUNCATED - response too large]';
      }
      return result;
    }

    // If result has content array (MCP standard format), truncate items
    if (result && Array.isArray(result.content)) {
      const truncated = { ...result };
      truncated.content = result.content.map(item => {
        if (item.type === 'text' && typeof item.text === 'string') {
          const textJson = JSON.stringify(item.text);
          if (textJson.length > maxSize / 2) {
            return {
              ...item,
              text: item.text.slice(0, maxSize / 2) + '\n\n... [TRUNCATED - response too large]',
            };
          }
        }
        return item;
      });
      truncated._truncated = true;
      return truncated;
    }

    // For other objects, add truncation warning
    if (typeof result === 'object' && result !== null) {
      return {
        ...result,
        _truncated: true,
        _warning: 'Response was truncated due to size limits',
      };
    }

    return result;
  }

  /**
   * Send JSON-RPC error
   * @private
   */
  _sendError(id, code, message) {
    const response = {
      jsonrpc: '2.0',
      id,
      error: { code, message },
    };
    this.output.write(JSON.stringify(response) + '\n');
  }

  /**
   * Send JSON-RPC notification
   * @private
   */
  _sendNotification(method, params) {
    const notification = {
      jsonrpc: '2.0',
      method,
      params,
    };
    this.output.write(JSON.stringify(notification) + '\n');
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
      // Unified persistence (PostgreSQL → File → Memory fallback)
      persistenceBackend: this.persistence?._backend || 'none',
      persistenceCapabilities: this.persistence?.capabilities || {},
      judgeStats: this.judge.getStats(),
      // 🐕 The Collective - All 11 Dogs (Sefirot)
      collective: this.collective?.getSummary() || { initialized: false },
      // CYNIC meta-state
      cynicState: this.collective?.cynic?.metaState || 'not_initialized',
      // Multi-user sessions
      sessions: this.sessionManager?.getSummary() || { activeCount: 0 },
      // PoJ Chain status
      pojChain: this.pojChainManager?.getStatus() || { initialized: false },
      // Librarian status
      librarian: this.librarian?._initialized
        ? { initialized: true, stats: this.librarian._stats }
        : { initialized: false },
    };
  }
}

export default MCPServer;
