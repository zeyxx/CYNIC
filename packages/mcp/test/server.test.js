/**
 * CYNIC MCP Server Tests
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';
import { Readable, Writable } from 'node:stream';
import { MCPServer } from '../src/server.js';

/**
 * Create mock streams for testing
 */
function createMockStreams() {
  const output = [];
  const input = new Readable({ read() {} });
  const outputStream = new Writable({
    write(chunk, encoding, callback) {
      output.push(chunk.toString());
      callback();
    },
  });

  return { input, outputStream, output };
}

/**
 * Send JSON-RPC request and get response
 */
async function sendRequest(input, output, method, params = {}, id = 1) {
  const request = JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n';
  input.push(request);

  // Wait for response
  await new Promise(resolve => setTimeout(resolve, 50));

  // Parse response
  const responseStr = output[output.length - 1];
  if (responseStr) {
    return JSON.parse(responseStr.trim());
  }
  return null;
}

describe('MCPServer', () => {
  let server;
  let input;
  let outputStream;
  let output;

  beforeEach(async () => {
    const streams = createMockStreams();
    input = streams.input;
    outputStream = streams.outputStream;
    output = streams.output;

    server = new MCPServer({
      input,
      output: outputStream,
    });

    await server.start();
  });

  describe('initialize', () => {
    it('responds with server info and capabilities', async () => {
      const response = await sendRequest(input, output, 'initialize', {
        protocolVersion: '2024-11-05',
        clientInfo: { name: 'test-client', version: '1.0.0' },
      });

      assert.equal(response.jsonrpc, '2.0');
      assert.equal(response.id, 1);
      assert.ok(response.result);
      assert.equal(response.result.serverInfo.name, 'cynic-mcp');
      assert.ok(response.result.capabilities.tools);
    });
  });

  describe('tools/list', () => {
    it('returns all 9 tools', async () => {
      const response = await sendRequest(input, output, 'tools/list');

      assert.ok(response.result);
      assert.ok(Array.isArray(response.result.tools));
      assert.equal(response.result.tools.length, 9);

      const toolNames = response.result.tools.map(t => t.name);
      assert.ok(toolNames.includes('brain_cynic_judge'));
      assert.ok(toolNames.includes('brain_cynic_digest'));
      assert.ok(toolNames.includes('brain_health'));
      assert.ok(toolNames.includes('brain_search'));
      assert.ok(toolNames.includes('brain_patterns'));
      assert.ok(toolNames.includes('brain_cynic_feedback'));
      assert.ok(toolNames.includes('brain_agents_status'));
      // Session management tools (GAP #2)
      assert.ok(toolNames.includes('brain_session_start'));
      assert.ok(toolNames.includes('brain_session_end'));
    });
  });

  describe('tools/call brain_cynic_judge', () => {
    it('judges an item and returns Q-Score', async () => {
      const response = await sendRequest(input, output, 'tools/call', {
        name: 'brain_cynic_judge',
        arguments: {
          item: { type: 'test', content: 'This is a test item', verified: true },
          context: { source: 'unit-test' },
        },
      });

      assert.ok(response.result);
      assert.ok(response.result.content);

      const result = JSON.parse(response.result.content[0].text);
      assert.ok(result.requestId.startsWith('jdg_'));
      assert.ok(typeof result.score === 'number');
      assert.ok(['HOWL', 'WAG', 'GROWL', 'BARK'].includes(result.verdict));
      assert.ok(result.confidence <= 0.618); // φ⁻¹
      assert.ok(result.phi.maxConfidence === 0.618033988749895);
    });
  });

  describe('tools/call brain_health', () => {
    it('returns health status', async () => {
      const response = await sendRequest(input, output, 'tools/call', {
        name: 'brain_health',
        arguments: { verbose: true },
      });

      assert.ok(response.result);
      const result = JSON.parse(response.result.content[0].text);

      assert.equal(result.status, 'healthy');
      assert.ok(result.identity);
      assert.equal(result.identity.name, 'CYNIC');
      assert.ok(result.phi.maxConfidence === 0.618033988749895);
      assert.ok(result.tools); // verbose includes tools
    });
  });

  describe('tools/call brain_cynic_digest', () => {
    it('digests content and extracts patterns', async () => {
      const content = `
        This is a test document with some code:
        \`\`\`javascript
        const x = 1;
        \`\`\`
        And a link: https://example.com
        We decided to use PostgreSQL.
      `;

      const response = await sendRequest(input, output, 'tools/call', {
        name: 'brain_cynic_digest',
        arguments: {
          content,
          source: 'test',
          type: 'document',
        },
      });

      assert.ok(response.result);
      const result = JSON.parse(response.result.content[0].text);

      assert.ok(result.digestId.startsWith('dig_'));
      assert.equal(result.source, 'test');
      assert.ok(result.stats.words > 0);
      assert.ok(result.patterns.length >= 2); // code + link patterns
    });
  });

  describe('tools/call brain_cynic_feedback', () => {
    it('records feedback on a judgment', async () => {
      const response = await sendRequest(input, output, 'tools/call', {
        name: 'brain_cynic_feedback',
        arguments: {
          judgmentId: 'jdg_test123',
          outcome: 'correct',
          reason: 'The judgment was accurate',
        },
      });

      assert.ok(response.result);
      const result = JSON.parse(response.result.content[0].text);

      assert.ok(result.feedbackId.startsWith('fb_'));
      assert.equal(result.judgmentId, 'jdg_test123');
      assert.equal(result.outcome, 'correct');
      assert.ok(result.message.includes('*wag*'));
    });
  });

  describe('ping', () => {
    it('responds with pong', async () => {
      const response = await sendRequest(input, output, 'ping');

      assert.ok(response.result);
      assert.equal(response.result.pong, true);
      assert.ok(response.result.timestamp);
    });
  });

  describe('error handling', () => {
    it('returns error for unknown method', async () => {
      const response = await sendRequest(input, output, 'unknown/method');

      assert.ok(response.error);
      assert.equal(response.error.code, -32601);
      assert.ok(response.error.message.includes('Method not found'));
    });

    it('returns error for unknown tool', async () => {
      const response = await sendRequest(input, output, 'tools/call', {
        name: 'unknown_tool',
        arguments: {},
      });

      assert.ok(response.error);
      assert.equal(response.error.code, -32000);
      assert.ok(response.error.message.includes('Tool not found'));
    });
  });
});
