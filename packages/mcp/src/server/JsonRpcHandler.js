/**
 * JsonRpcHandler - MCP JSON-RPC 2.0 Protocol Layer
 *
 * Transport-agnostic routing and tool execution.
 * Used by both StdioTransport and RouteHandlers.
 *
 * "Le protocole est la loi" - κυνικός
 *
 * @module @cynic/mcp/server/JsonRpcHandler
 */

'use strict';

import { getTelemetry } from '@cynic/persistence';

export class JsonRpcHandler {
  /**
   * @param {Object} options
   * @param {Object} options.server - MCPServer instance (dependency container)
   */
  constructor({ server }) {
    this._server = server;
  }

  /**
   * Handle a JSON-RPC request (transport-agnostic)
   * @param {Object} request - JSON-RPC 2.0 request
   * @returns {Object|null} Response or null for notifications
   */
  async handleRequest(request) {
    const { id, method, params = {} } = request;

    // DEBUG: Log every request (stderr for MCP protocol)
    console.error(`[REQUEST] method=${method} id=${id}`);

    try {
      let result;

      switch (method) {
        case 'initialize':
          result = this._handleInitialize(params);
          break;

        case 'initialized':
        case 'notifications/initialized':
          return null; // Notification - no response

        case 'tools/list':
          result = this._handleToolsList();
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
          await this._server.stop();
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
      if (method === 'tools/call' && this._server.sessionManager) {
        // Don't count blocked operations as errors - they're intentional
        if (!err.message?.includes('[BLOCKED]')) {
          this._server.sessionManager.recordError();
        }
      }

      // SAGE: Share wisdom when errors occur (via Collective)
      if (this._server.collective) {
        this._server.collective.getWisdom('error_recovery', {
          errorMessage: err.message,
          method,
          context: 'mcp_request_error',
        }).then(wisdom => {
          if (wisdom?.message) {
            console.error(`Sage wisdom: ${wisdom.message}`);
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
   * Handle initialize request
   * @private
   */
  _handleInitialize(params) {
    const { clientInfo } = params;

    // Log client info
    if (clientInfo) {
      console.error(`   Client: ${clientInfo.name} v${clientInfo.version || '?'}`);
    }

    return {
      protocolVersion: '2024-11-05',
      serverInfo: {
        name: this._server.name,
        version: this._server.version,
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
  _handleToolsList() {
    return {
      tools: Object.values(this._server.tools).map(tool => ({
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
    const s = this._server;

    // DEBUG: Log at very start of tool call (stderr for MCP protocol)
    console.error(`[TOOL_CALL] ${name} called at ${new Date().toISOString()}`);

    const tool = s.tools[name];
    if (!tool) {
      throw new Error(`Tool not found: ${name}`);
    }

    // Generate toolUseId for duration tracking
    const toolUseId = `tool_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    // ═══════════════════════════════════════════════════════════════════════════
    // UNIFIED ORCHESTRATOR: Single entry point for tool orchestration
    // Routes through: User profile -> KETER routing -> DogOrchestrator -> Learning
    // ═══════════════════════════════════════════════════════════════════════════
    if (s.unifiedOrchestrator) {
      try {
        const decision = await s.unifiedOrchestrator.processTool(name, {
          ...args,
          toolUseId,
        });

        // Check if blocked by UnifiedOrchestrator
        if (decision.outcome === 'BLOCK' || decision.preExecution?.blocked) {
          const reason = decision.preExecution?.reason || 'Operation blocked by KETER';
          console.error(`[BLOCKED] Tool "${name}" blocked: ${reason}`);

          if (s.sessionManager) {
            s.sessionManager.recordDangerBlocked();
          }

          throw new Error(`[BLOCKED] ${reason}`);
        }

        // Broadcast routing decision for visibility
        s._broadcastSSEEvent('tool_pre', {
          tool: name,
          toolUseId,
          input: args,
          routing: decision.routing,
          judgment: decision.judgment ? {
            score: decision.judgment.score,
            verdict: decision.judgment.verdict,
          } : null,
          timestamp: Date.now(),
        });
      } catch (err) {
        // If it's a BLOCK error, re-throw
        if (err.message?.includes('[BLOCKED]')) throw err;
        // Otherwise fall through to collective
        console.error(`[WARN] UnifiedOrchestrator error, falling back: ${err.message}`);
      }
    }

    // COLLECTIVE: PreToolUse -> Full pipeline (shouldTrigger -> analyze -> decide)
    // Fallback if UnifiedOrchestrator not available or failed
    if (s.collective && !s.unifiedOrchestrator) {
      const hookResult = await s.collective.receiveHookEvent({
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
        console.error(`[BLOCKED] Tool "${name}" blocked by ${blockedBy}: ${message}`);

        // Track danger blocked for user profile aggregation
        if (s.sessionManager) {
          s.sessionManager.recordDangerBlocked();
        }

        throw new Error(`[BLOCKED] ${message}`);
      }

      // Log warnings from agents
      for (const result of hookResult.agentResults || []) {
        if (result.response === 'warn' && result.message) {
          console.error(`[WARNING] ${result.agent}: ${result.message}`);
        }
      }

      // Broadcast to SSE for Live View
      s._broadcastSSEEvent('tool_pre', {
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
    if (s.sessionManager) {
      s.sessionManager.recordToolCall();
    }

    // COLLECTIVE: PostToolUse -> Full pipeline (all 11 Dogs analyze)
    if (s.collective) {
      const hookResult = await s.collective.receiveHookEvent({
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
      s._broadcastSSEEvent('tool_post', {
        tool: name,
        toolUseId,
        duration,
        success: true,
        dogsNotified: hookResult?.delivered || 0,
        agentsTriggered: hookResult.agentResults?.length || 0,
        timestamp: Date.now(),
      });
    }

    // Record outcome for adaptive perception routing
    if (s.perceptionRouter) {
      s.perceptionRouter.recordOutcome('mcp', name, true, duration);
    }

    // Record tool usage with latency for telemetry
    try {
      const telemetry = getTelemetry();
      if (telemetry) {
        telemetry.recordToolUse({ tool: name, success: true, latencyMs: duration });
      }
    } catch (_) { /* telemetry is best-effort */ }

    return {
      content: [{
        type: 'text',
        text: JSON.stringify(result, null, 2),
      }],
    };
  }
}
