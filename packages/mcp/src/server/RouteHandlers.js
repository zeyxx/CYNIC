/**
 * RouteHandlers - HTTP Route Domain Logic
 *
 * Health, metrics, API, hooks, psychology, and MCP endpoints.
 * HttpAdapter handles transport; this handles domain logic.
 *
 * "Les routes mènent à la vérité" - κυνικός
 *
 * @module @cynic/mcp/server/RouteHandlers
 */

'use strict';

import { PHI_INV } from '@cynic/core';
import { HttpAdapter } from './HttpAdapter.js';

const MAX_BODY_SIZE = 1024 * 1024; // 1MB max request body
const REQUEST_TIMEOUT_MS = 30000; // 30 second request timeout

export class RouteHandlers {
  /**
   * @param {Object} options
   * @param {Object} options.server - MCPServer instance (dependency container)
   * @param {import('./JsonRpcHandler.js').JsonRpcHandler} options.jsonRpcHandler - JSON-RPC protocol handler
   */
  constructor({ server, jsonRpcHandler }) {
    this._server = server;
    this._jsonRpc = jsonRpcHandler;
  }

  /**
   * Register all routes on an HttpAdapter
   * @param {HttpAdapter} adapter
   */
  register(adapter) {
    adapter.setRoute('health', (req, res) => this.handleHealth(req, res));
    adapter.setRoute('metrics', (req, res, url) => this.handleMetrics(req, res, url));
    adapter.setRoute('mcp', (req, res) => this.handleMcp(req, res));
    adapter.setRoute('api', (req, res, url) => this.handleApi(req, res, url));
    adapter.setRoute('hooks', (req, res, url) => this.handleHooks(req, res, url));
    adapter.setRoute('psychology', (req, res, url) => this.handlePsychology(req, res, url));
  }

  /**
   * Handle health check requests
   * Route: GET / or /health
   */
  async handleHealth(_req, res) {
    const s = this._server;
    const checks = {
      database: { status: 'unknown' },
      redis: { status: 'unknown' },
      pojChain: { status: 'unknown' },
      judge: { status: 'unknown' },
      anchoring: { status: 'unknown' },
    };

    let overallHealthy = true;

    // Check PostgreSQL
    try {
      if (s.persistence?.postgres) {
        await (s.persistence.postgres.healthCheck?.() ||
          s.persistence.postgres.query?.('SELECT 1'));
        checks.database = { status: 'healthy' };
      } else {
        checks.database = { status: 'not_configured' };
      }
    } catch (err) {
      checks.database = { status: 'unhealthy', error: err.message };
      overallHealthy = false;
    }

    // Check Redis
    try {
      if (s.persistence?.redis) {
        await s.persistence.redis.ping?.();
        checks.redis = { status: 'healthy' };
      } else {
        checks.redis = { status: 'not_configured' };
      }
    } catch (err) {
      checks.redis = { status: 'unhealthy', error: err.message };
      overallHealthy = false;
    }

    // Check PoJ Chain
    try {
      if (s.pojChainManager) {
        const pojStatus = s.pojChainManager.getStatus();
        checks.pojChain = {
          status: pojStatus.initialized ? 'healthy' : 'initializing',
          slot: pojStatus.headSlot,
          pending: pojStatus.pendingJudgments,
        };
      } else {
        checks.pojChain = { status: 'not_configured' };
      }
    } catch (err) {
      checks.pojChain = { status: 'unhealthy', error: err.message };
      overallHealthy = false;
    }

    // Check Judge
    try {
      if (s.judge) {
        checks.judge = {
          status: 'healthy',
          engines: s.judge.engineRegistry?.getRegisteredEngineIds?.()?.length || 0,
        };
      } else {
        checks.judge = { status: 'not_configured' };
      }
    } catch (err) {
      checks.judge = { status: 'unhealthy', error: err.message };
      overallHealthy = false;
    }

    // Check Anchoring
    try {
      if (s.anchorQueue) {
        const stats = s.anchorQueue.getStats?.() || {};
        checks.anchoring = {
          status: 'healthy',
          enabled: true,
          pending: stats.queueLength || 0,
          anchored: stats.totalAnchored || 0,
        };
      } else {
        checks.anchoring = { status: 'disabled' };
      }
    } catch (err) {
      checks.anchoring = { status: 'unhealthy', error: err.message };
    }

    const statusCode = overallHealthy ? 200 : 503;
    HttpAdapter.sendJson(res, statusCode, {
      status: overallHealthy ? 'healthy' : 'unhealthy',
      server: s.name,
      version: s.version,
      tools: Object.keys(s.tools).length,
      uptime: process.uptime(),
      phi: PHI_INV,
      checks,
      timestamp: Date.now(),
    });
  }

  /**
   * Handle metrics requests (Prometheus or HTML)
   * Route: GET /metrics or /metrics/html
   */
  async handleMetrics(_req, res, url) {
    const s = this._server;
    if (!s.metrics) {
      HttpAdapter.sendJson(res, 503, { error: 'Metrics service not available' });
      return;
    }

    try {
      if (url.pathname === '/metrics/html') {
        const html = await s.metrics.toHTML();
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(html);
      } else {
        const prometheus = await s.metrics.toPrometheus();
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
   */
  async handleMcp(req, res) {
    const requestId = Symbol('request');
    this._server._activeRequests.add(requestId);

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
      this._server._activeRequests.delete(requestId);
    }, REQUEST_TIMEOUT_MS);

    try {
      // Collect body with size limit
      let body = '';
      let bodySize = 0;

      for await (const chunk of req) {
        bodySize += chunk.length;
        if (bodySize > MAX_BODY_SIZE) {
          clearTimeout(timeoutId);
          this._server._activeRequests.delete(requestId);
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
        this._server._activeRequests.delete(requestId);
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          jsonrpc: '2.0',
          id: message.id,
          error: { code: -32600, message: 'Invalid JSON-RPC version' },
        }));
        return;
      }

      // Process message through JsonRpcHandler
      const result = await this._jsonRpc.handleRequest(message);

      clearTimeout(timeoutId);
      this._server._activeRequests.delete(requestId);

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
      this._server._activeRequests.delete(requestId);
      res.writeHead(400, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        jsonrpc: '2.0',
        id: null,
        error: { code: -32700, message: `Parse error: ${err.message}` },
      }));
    }
  }

  /**
   * Handle REST API requests
   * Route: /api/*
   */
  async handleApi(req, res, url) {
    const s = this._server;
    const pathname = url.pathname;

    // List all available tools (API discovery)
    if (pathname === '/api/tools' && req.method === 'GET') {
      const tools = Object.values(s.tools).map(t => ({
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
   * Handle REST API tool requests
   * @private
   */
  async _handleApiToolRequest(req, res, url) {
    const s = this._server;
    // Extract tool name from URL: /api/tools/brain_cynic_judge
    const toolName = url.pathname.replace('/api/tools/', '');

    const tool = s.tools[toolName];
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

      // COLLECTIVE: PreToolUse check (same as MCP tools/call)
      if (s.collective) {
        const preResult = await s.collective.receiveHookEvent({
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
      console.error(`[API] Tool ${toolName} called`);
      const startTime = Date.now();
      const result = await tool.handler(args);
      const duration = Date.now() - startTime;

      // COLLECTIVE: PostToolUse (non-blocking)
      if (s.collective) {
        s.collective.receiveHookEvent({
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
      console.error(`[API] Tool ${toolName} error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle hooks requests
   * Route: /hooks/*
   */
  async handleHooks(req, res, url) {
    const pathname = url.pathname;

    // Hook event endpoint - bridges Claude Code hooks to the Collective
    if (pathname === '/hooks/event' && req.method === 'POST') {
      await this._handleHookEvent(req, res);
      return;
    }

    HttpAdapter.sendJson(res, 404, { error: 'Hook endpoint not found' });
  }

  /**
   * Handle hook event from Claude Code
   * @private
   */
  async _handleHookEvent(req, res) {
    const s = this._server;
    try {
      const body = await HttpAdapter.readBody(req);
      const hookData = body ? JSON.parse(body) : {};

      // Validate required fields
      if (!hookData.hookType) {
        HttpAdapter.sendJson(res, 400, { error: 'hookType is required' });
        return;
      }

      // Check if collective is available
      if (!s.collective) {
        HttpAdapter.sendJson(res, 503, { error: 'Collective not initialized' });
        return;
      }

      // Forward to collective
      const result = await s.collective.receiveHookEvent(hookData);

      // Log for debugging
      console.error(`[HOOK] ${hookData.hookType} -> ${result.delivered || 0} dogs notified`);

      // Broadcast to SSE clients (generic message)
      s._broadcastSSEEvent('hook:received', {
        hookType: hookData.hookType,
        delivered: result.delivered || 0,
        timestamp: Date.now(),
      });

      // Also broadcast typed events for Live View (tool timeline + audio)
      if (hookData.hookType === 'PreToolUse' && hookData.payload) {
        s._broadcastSSEEvent('tool_pre', {
          tool: hookData.payload.tool || 'unknown',
          toolUseId: hookData.payload.toolUseId || `hook_${Date.now()}`,
          input: hookData.payload.input,
          timestamp: Date.now(),
        });
      } else if (hookData.hookType === 'PostToolUse' && hookData.payload) {
        s._broadcastSSEEvent('tool_post', {
          tool: hookData.payload.tool || 'unknown',
          toolUseId: hookData.payload.toolUseId || `hook_${Date.now()}`,
          duration: hookData.payload.duration,
          success: hookData.payload.success !== false,
          timestamp: Date.now(),
        });
      }

      HttpAdapter.sendJson(res, 200, result);
    } catch (err) {
      console.error(`[HOOK] Error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle psychology requests
   * Route: /psychology/*
   */
  async handlePsychology(req, res, url) {
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
   * Handle psychology sync (sleep.cjs -> PostgreSQL)
   * @private
   */
  async _handlePsychologySync(req, res) {
    const s = this._server;
    try {
      const body = await HttpAdapter.readBody(req);
      const { userId, data } = body ? JSON.parse(body) : {};

      if (!userId || !data) {
        HttpAdapter.sendJson(res, 400, { error: 'userId and data are required' });
        return;
      }

      if (!s.persistence) {
        HttpAdapter.sendJson(res, 503, { error: 'Persistence not available' });
        return;
      }

      const result = await s.persistence.syncPsychology(userId, data);

      console.error(`[PSYCHOLOGY] Synced for ${userId}`);

      HttpAdapter.sendJson(res, 200, { success: true, result });
    } catch (err) {
      console.error(`[PSYCHOLOGY] Sync error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }

  /**
   * Handle psychology load (awaken.cjs <- PostgreSQL)
   * @private
   */
  async _handlePsychologyLoad(_req, res, url) {
    const s = this._server;
    try {
      const userId = url.searchParams.get('userId');

      if (!userId) {
        HttpAdapter.sendJson(res, 400, { error: 'userId is required' });
        return;
      }

      if (!s.persistence) {
        HttpAdapter.sendJson(res, 503, { error: 'Persistence not available' });
        return;
      }

      const data = await s.persistence.loadPsychology(userId);

      if (!data) {
        HttpAdapter.sendJson(res, 404, { error: 'No psychology data found for user' });
        return;
      }

      console.error(`[PSYCHOLOGY] Loaded for ${userId}`);

      HttpAdapter.sendJson(res, 200, data);
    } catch (err) {
      console.error(`[PSYCHOLOGY] Load error: ${err.message}`);
      HttpAdapter.sendJson(res, 500, { error: err.message });
    }
  }
}
