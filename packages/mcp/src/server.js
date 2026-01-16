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

import { createServer } from 'http';
import { PHI_INV, PHI_INV_2, IDENTITY } from '@cynic/core';
import { CYNICJudge, AgentManager } from '@cynic/node';
import { createAllTools } from './tools/index.js';
import { PersistenceManager } from './persistence.js';
import { SessionManager } from './session-manager.js';
import { PoJChainManager } from './poj-chain-manager.js';

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
export class MCPServer {
  /**
   * Create MCP server
   * @param {Object} [options] - Server options
   * @param {Object} [options.node] - CYNICNode instance
   * @param {Object} [options.judge] - CYNICJudge instance
   * @param {Object} [options.persistence] - PersistenceManager instance
   * @param {Object} [options.sessionManager] - SessionManager instance (for multi-user sessions)
   * @param {Object} [options.pojChainManager] - PoJChainManager instance (for PoJ blockchain)
   * @param {Object} [options.agents] - AgentManager instance (The Four Dogs)
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

    // Node instance (optional)
    this.node = options.node || null;

    // Judge instance (required)
    this.judge = options.judge || new CYNICJudge();

    // Data directory for file-based fallback
    this.dataDir = options.dataDir || null;

    // Persistence manager (PostgreSQL + Redis with automatic fallback)
    this.persistence = options.persistence || null;

    // Session manager for multi-user isolation (created after persistence)
    this.sessionManager = options.sessionManager || null;

    // PoJ Chain manager for Proof of Judgment blockchain
    this.pojChainManager = options.pojChainManager || null;

    // Agent manager - The Four Dogs (Guardian, Observer, Digester, Mentor)
    this.agents = options.agents || new AgentManager();

    // Stdio streams (for stdio mode)
    this.input = options.input || process.stdin;
    this.output = options.output || process.stdout;

    // HTTP server (for http mode)
    this._httpServer = null;
    this._sseClients = new Set();

    // Request buffer for stdin parsing
    this._buffer = '';

    // Running flag
    this._running = false;

    // Tool registry (populated on start)
    this.tools = {};
  }

  /**
   * Initialize components
   * @private
   */
  async _initialize() {
    // Initialize persistence with automatic fallback chain:
    // PostgreSQL → File-based → In-memory
    if (!this.persistence) {
      this.persistence = new PersistenceManager({
        dataDir: this.dataDir, // Pass for file-based fallback
      });
      await this.persistence.initialize();
    }

    // Initialize session manager for multi-user isolation
    if (!this.sessionManager) {
      this.sessionManager = new SessionManager(this.persistence);
    }

    // Initialize PoJ Chain manager for blockchain
    if (!this.pojChainManager) {
      this.pojChainManager = new PoJChainManager(this.persistence);
      await this.pojChainManager.initialize();
    }

    // Register tools with current instances
    this.tools = createAllTools({
      judge: this.judge,
      node: this.node,
      persistence: this.persistence,
      agents: this.agents,
      sessionManager: this.sessionManager,
      pojChainManager: this.pojChainManager,
    });
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
   * @private
   */
  async _startHttpServer() {
    return new Promise((resolve, reject) => {
      this._httpServer = createServer((req, res) => {
        this._handleHttpRequest(req, res);
      });

      this._httpServer.on('error', (err) => {
        console.error('HTTP server error:', err.message);
        reject(err);
      });

      this._httpServer.listen(this.port, () => {
        console.error(`   HTTP server listening on port ${this.port}`);
        resolve();
      });
    });
  }

  /**
   * Handle HTTP request
   * @private
   */
  async _handleHttpRequest(req, res) {
    // CORS headers for MCP clients
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      res.end();
      return;
    }

    const url = new URL(req.url, `http://${req.headers.host}`);

    // Health check endpoint
    if (url.pathname === '/health' || url.pathname === '/') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        status: 'healthy',
        server: this.name,
        version: this.version,
        tools: Object.keys(this.tools).length,
        uptime: process.uptime(),
        phi: PHI_INV,
      }));
      return;
    }

    // SSE endpoint for MCP streaming
    if (url.pathname === '/sse') {
      this._handleSseConnection(req, res);
      return;
    }

    // POST endpoint for JSON-RPC messages
    if (url.pathname === '/message' && req.method === 'POST') {
      await this._handleHttpMessage(req, res);
      return;
    }

    // 404 for unknown routes
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found' }));
  }

  /**
   * Handle SSE connection
   * @private
   */
  _handleSseConnection(req, res) {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    });

    // Send endpoint info event
    const endpoint = `/message`;
    res.write(`event: endpoint\ndata: ${endpoint}\n\n`);

    // Track client
    this._sseClients.add(res);
    console.error(`   SSE client connected (${this._sseClients.size} total)`);

    // Handle client disconnect
    req.on('close', () => {
      this._sseClients.delete(res);
      console.error(`   SSE client disconnected (${this._sseClients.size} remaining)`);
    });

    // Keep-alive ping every 30 seconds
    const keepAlive = setInterval(() => {
      if (!res.writableEnded) {
        res.write(`: ping\n\n`);
      }
    }, 30000);

    req.on('close', () => clearInterval(keepAlive));
  }

  /**
   * Handle HTTP POST message
   * @private
   */
  async _handleHttpMessage(req, res) {
    let body = '';
    for await (const chunk of req) {
      body += chunk;
    }

    try {
      const message = JSON.parse(body);

      if (!message.jsonrpc || message.jsonrpc !== '2.0') {
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

      if (result === null) {
        // Notification - no response
        res.writeHead(204);
        res.end();
        return;
      }

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(result));
    } catch (err) {
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

    // Close HTTP server if running
    if (this._httpServer) {
      await new Promise((resolve) => {
        // Close all SSE clients
        for (const client of this._sseClients) {
          client.end();
        }
        this._sseClients.clear();

        this._httpServer.close(() => {
          console.error('   HTTP server closed');
          resolve();
        });
      });
    }

    // Flush PoJ chain (create final block from pending judgments)
    if (this.pojChainManager) {
      try {
        await this.pojChainManager.close();
      } catch (e) {
        console.error('Error closing PoJ chain:', e.message);
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

    const tool = this.tools[name];
    if (!tool) {
      throw new Error(`Tool not found: ${name}`);
    }

    // 🐕 Guardian: PreToolUse check (blocking)
    // Guardian barks BEFORE the damage - one confirmation saves hours of recovery
    const guardianResult = await this.agents.process({
      type: 'PreToolUse',
      tool: name,
      input: args,
      timestamp: Date.now(),
    });

    if (guardianResult._blocked) {
      const blockedBy = guardianResult._blockedBy || 'guardian';
      const message = guardianResult[blockedBy]?.message || 'Operation blocked by Guardian';
      console.error(`🐕 [BLOCKED] Tool "${name}" blocked by ${blockedBy}: ${message}`);
      throw new Error(`[BLOCKED] ${message}`);
    }

    // Log warning if Guardian raised one
    if (guardianResult.guardian?.response === 'warn') {
      console.error(`🐕 [WARNING] Tool "${name}": ${guardianResult.guardian.message}`);
    }

    // Execute tool handler
    const startTime = Date.now();
    const result = await tool.handler(args);
    const duration = Date.now() - startTime;

    // 🐕 Observer: PostToolUse logging (non-blocking, silent)
    // Observer watches the meta - repeated failures, unusual sequences, emerging patterns
    this.agents.process({
      type: 'PostToolUse',
      tool: name,
      input: args,
      output: result,
      duration,
      success: true,
      timestamp: Date.now(),
    }).catch(err => {
      // Observer is non-blocking - log but don't fail the request
      console.error(`🐕 Observer error: ${err.message}`);
    });

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
    this.output.write(JSON.stringify(response) + '\n');
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
      // 🐕 The Four Dogs status
      agents: this.agents.getSummary(),
      // Multi-user sessions
      sessions: this.sessionManager?.getSummary() || { activeCount: 0 },
      // PoJ Chain status
      pojChain: this.pojChainManager?.getStatus() || { initialized: false },
    };
  }
}

export default MCPServer;
