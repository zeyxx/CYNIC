/**
 * StdioTransport - JSON-RPC over stdin/stdout
 *
 * Handles buffered newline-delimited JSON-RPC message parsing,
 * response serialization, and size-aware truncation.
 *
 * "Le chien parle en JSON" - κυνικός
 *
 * @module @cynic/mcp/server/StdioTransport
 */

'use strict';

const MAX_RESPONSE_SIZE = 100 * 1024; // 100KB max response size (prevents Claude Code blocking)

export class StdioTransport {
  /**
   * @param {Object} options
   * @param {NodeJS.ReadableStream} options.input - Input stream (stdin)
   * @param {NodeJS.WritableStream} options.output - Output stream (stdout)
   * @param {(message: Object) => Promise<Object|null>} options.onRequest - JSON-RPC request handler
   * @param {() => void} options.onClose - Called when input stream ends
   */
  constructor({ input, output, onRequest, onClose }) {
    this._input = input;
    this._output = output;
    this._onRequest = onRequest;
    this._onClose = onClose;
    this._buffer = '';
  }

  /**
   * Start listening on input stream
   */
  start() {
    this._input.setEncoding('utf8');
    this._input.on('data', (chunk) => this._handleInput(chunk));
    this._input.on('end', () => this._onClose());
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
        this.sendError(message.id, -32600, 'Invalid JSON-RPC version');
        return;
      }

      // Handle request messages
      if (message.method) {
        const response = await this._onRequest(message);

        // Notifications don't get responses
        if (response === null) {
          return;
        }

        // Send response via stdio
        if (response.error) {
          this.sendError(response.id, response.error.code, response.error.message);
        } else {
          this.sendResponse(response.id, response.result);
        }
      }
    } catch (err) {
      this.sendError(null, -32700, `Parse error: ${err.message}`);
    }
  }

  /**
   * Send JSON-RPC response
   */
  sendResponse(id, result) {
    const response = {
      jsonrpc: '2.0',
      id,
      result,
    };
    let json = JSON.stringify(response);

    // Truncate very large responses to prevent Claude Code blocking
    if (json.length > MAX_RESPONSE_SIZE) {
      const sizeKB = (json.length / 1024).toFixed(1);
      console.error(`Warning: Truncating large MCP response: ${sizeKB}KB -> 100KB for request ${id}`);

      // Try to preserve structure by truncating content arrays/strings
      const truncatedResult = this._truncateResult(result, MAX_RESPONSE_SIZE - 500);
      const truncatedResponse = {
        jsonrpc: '2.0',
        id,
        result: truncatedResult,
      };
      json = JSON.stringify(truncatedResponse);
    }

    this._output.write(json + '\n');
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
   */
  sendError(id, code, message) {
    const response = {
      jsonrpc: '2.0',
      id,
      error: { code, message },
    };
    this._output.write(JSON.stringify(response) + '\n');
  }

  /**
   * Send JSON-RPC notification
   */
  sendNotification(method, params) {
    const notification = {
      jsonrpc: '2.0',
      method,
      params,
    };
    this._output.write(JSON.stringify(notification) + '\n');
  }
}
