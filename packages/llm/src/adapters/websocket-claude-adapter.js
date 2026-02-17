/**
 * CYNIC WebSocket Claude Adapter
 *
 * Connects to Claude Code via WebSocket protocol (like Vibe Companion).
 * Enables streaming, multi-sessions, and zero-cost using Claude Code subscription.
 *
 * Based on research from:
 * - Vibe Companion (https://github.com/The-Vibe-Company/companion)
 * - Claude Code --sdk-url flag
 *
 * Protocol: NDJSON over WebSocket
 *
 * @module @cynic/llm/adapters/websocket-claude
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { LLMAdapter } from './base.js';

const log = createLogger('WebSocketClaudeAdapter');

/**
 * WebSocket Claude Adapter
 *
 * Spawns Claude Code with --sdk-url and communicates via WebSocket.
 * Supports streaming, tool calls, and multi-sessions.
 */
export class WebSocketClaudeAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({
      provider: 'claude-websocket',
      model: options.model || 'claude-opus-4-5-20251101',
      ...options,
    });

    this.wsUrl = options.wsUrl || 'ws://localhost:3456';
    this.sessionId = options.sessionId || this._generateSessionId();
    this.timeout = options.timeout || 60000;
    this.streamingEnabled = options.streamingEnabled !== false;

    // Connection state
    this._ws = null;
    this._connected = false;
    this._pendingRequests = new Map();
    this._currentRequestId = null;

    // Message buffer for streaming
    this._messageBuffer = [];

    // Stats
    this.stats = {
      sessionsCreated: 0,
      messagesSent: 0,
      messagesReceived: 0,
      toolCallsApproved: 0,
      toolCallsDenied: 0,
    };
  }

  /**
   * Generate unique session ID
   * @private
   */
  _generateSessionId() {
    return `cynic-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  }

  /**
   * Connect to WebSocket server
   * @private
   */
  async _connect() {
    if (this._connected && this._ws) {
      return;
    }

    return new Promise((resolve, reject) => {
      const wsUrl = `${this.wsUrl}/ws/cli/${this.sessionId}`;
      
      try {
        // Note: In browser environment, use native WebSocket
        // In Node.js, we'd use ws package
        this._ws = new (globalThis.WebSocket || require('ws').WebSocket)(wsUrl);
        
        const timeout = setTimeout(() => {
          reject(new Error('WebSocket connection timeout'));
        }, this.timeout);

        this._ws.on('open', () => {
          clearTimeout(timeout);
          this._connected = true;
          this.stats.sessionsCreated++;
          log.info('WebSocket connected', { sessionId: this.sessionId });
          resolve();
        });

        this._ws.on('message', (data) => {
          this._handleMessage(data);
        });

        this._ws.on('error', (error) => {
          clearTimeout(timeout);
          log.error('WebSocket error', { error: error.message });
          reject(error);
        });

        this._ws.on('close', () => {
          this._connected = false;
          log.info('WebSocket closed');
        });

      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Handle incoming WebSocket message
   * @private
   */
  _handleMessage(data) {
    this.stats.messagesReceived++;

    try {
      // NDJSON - each line is a separate JSON message
      const lines = data.toString().split('\n').filter(Boolean);
      
      for (const line of lines) {
        try {
          const message = JSON.parse(line);
          this._processMessage(message);
        } catch (e) {
          // Skip non-JSON lines
        }
      }
    } catch (error) {
      log.debug('Failed to parse message', { error: error.message });
    }
  }

  /**
   * Process individual message
   * @private
   */
  _processMessage(message) {
    const { type, requestId, content, error } = message;

    // Handle response to pending request
    if (requestId && this._pendingRequests.has(requestId)) {
      const { resolve, reject } = this._pendingRequests.get(requestId);
      
      if (error) {
        reject(new Error(error));
      } else if (type === 'content' || type === 'done') {
        this._messageBuffer.push(content);
        
        if (type === 'done') {
          const fullContent = this._messageBuffer.join('');
          this._messageBuffer = [];
          resolve(fullContent);
        }
      } else if (type === 'tool_use') {
        // Handle tool call - emit event for approval
        this.emit('toolCall', message.tool);
      }
    }
  }

  /**
   * Send message via WebSocket
   * @private
   */
  _sendMessage(message) {
    return new Promise((resolve, reject) => {
      if (!this._connected) {
        return reject(new Error('Not connected'));
      }

      const requestId = `req-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      message.requestId = requestId;

      const timeout = setTimeout(() => {
        this._pendingRequests.delete(requestId);
        reject(new Error('Request timeout'));
      }, this.timeout);

      this._pendingRequests.set(requestId, {
        resolve: (result) => {
          clearTimeout(timeout);
          this._pendingRequests.delete(requestId);
          resolve(result);
        },
        reject: (error) => {
          clearTimeout(timeout);
          this._pendingRequests.delete(requestId);
          reject(error);
        },
      });

      this._ws.send(JSON.stringify(message));
      this.stats.messagesSent++;
    });
  }

  /**
   * Complete a prompt
   */
  async complete(prompt, options = {}) {
    this.stats.requests++;

    // Connect if not already
    if (!this._connected) {
      await this._connect();
    }

    const message = {
      type: 'prompt',
      content: prompt,
      system: options.system || null,
      model: options.model || this.model,
      temperature: options.temperature ?? 0.7,
      maxTokens: options.maxTokens ?? 4096,
      stream: options.stream || false,
    };

    try {
      const content = await this._sendMessage(message);

      const response = this._createResponse({
        content,
        model: this.model,
        confidence: PHI_INV,
        duration: 0, // TODO: track duration
        metadata: {
          type: 'websocket-claude',
          sessionId: this.sessionId,
          streaming: options.stream || false,
        },
      });

      this.stats.successes++;
      this.emit('complete', response);

      return response;

    } catch (error) {
      this.stats.failures++;
      throw error;
    }
  }

  /**
   * Complete with streaming
   */
  async completeStream(prompt, options = {}) {
    if (!this.streamingEnabled) {
      throw new Error('Streaming not enabled');
    }

    if (!this._connected) {
      await this._connect();
    }

    const message = {
      type: 'prompt',
      content: prompt,
      stream: true,
    };

    // For streaming, return an async iterator
    const self = this;
    
    return {
      async *[Symbol.asyncIterator]() {
        // This would need proper streaming implementation
        // For now, just yield the complete response
        const result = await self.complete(prompt, { ...options, stream: false });
        yield result.content;
      }
    };
  }

  /**
   * Approve a tool call
   */
  async approveToolCall(toolCallId, approved = true) {
    const message = {
      type: 'tool_response',
      toolCallId,
      approved,
    };

    await this._sendMessage(message);

    if (approved) {
      this.stats.toolCallsApproved++;
    } else {
      this.stats.toolCallsDenied++;
    }
  }

  /**
   * Check if adapter is available
   */
  async isAvailable() {
    try {
      // Try to connect
      await this._connect();
      return this._connected;
    } catch {
      return false;
    }
  }

  /**
   * Disconnect from WebSocket
   */
  async disconnect() {
    if (this._ws) {
      this._ws.close();
      this._ws = null;
      this._connected = false;
    }
  }

  /**
   * Get adapter info
   */
  getInfo() {
    return {
      provider: this.provider,
      model: this.model,
      connected: this._connected,
      sessionId: this.sessionId,
      stats: this.stats,
    };
  }
}

/**
 * Create WebSocket Claude adapter
 */
export function createWebSocketClaudeAdapter(options = {}) {
  return new WebSocketClaudeAdapter(options);
}

/**
 * Factory for creating adapter with Claude Code subscription (free)
 */
export function createFreeClaudeAdapter(options = {}) {
  return new WebSocketClaudeAdapter({
    ...options,
    strategy: 'free',
  });
}

export default WebSocketClaudeAdapter;
