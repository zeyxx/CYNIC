/**
 * Discovery Service Tests
 *
 * Comprehensive tests for the DiscoveryService that handles automatic
 * discovery of MCP servers, Claude Code plugins, and CYNIC nodes.
 * Covers initialization, MCP discovery, plugin discovery, node discovery,
 * health checks, full repo scanning, queries, lifecycle, and edge cases.
 *
 * @module @cynic/mcp/test/discovery-service
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'events';

// Store original fetch before any imports might modify it
const originalFetch = global.fetch;

import { DiscoveryService } from '../src/discovery-service.js';

// =============================================================================
// TEST HELPERS
// =============================================================================

function createMockPersistence(overrides = {}) {
  return {
    discovery: {
      upsertMcpServer: mock.fn(async (server) => ({ id: 'mcp_1', ...server })),
      upsertPlugin: mock.fn(async (plugin) => ({ id: 'plugin_1', ...plugin })),
      upsertNode: mock.fn(async (node) => ({ id: 'node_1', ...node })),
      logEvent: mock.fn(async () => {}),
      getNodes: mock.fn(async () => []),
      getMcpServers: mock.fn(async () => []),
      getPlugins: mock.fn(async () => []),
      getStats: mock.fn(async () => ({ total: 0 })),
      updateNodeHealth: mock.fn(async () => {}),
      ...overrides,
    },
  };
}

function mockFetch(responses = {}) {
  global.fetch = mock.fn(async (url, options) => {
    const urlStr = url.toString();

    // Check for predefined response
    for (const [pattern, response] of Object.entries(responses)) {
      if (urlStr.includes(pattern)) {
        if (response.error) {
          throw new Error(response.error);
        }
        return {
          ok: response.ok !== false,
          status: response.status || 200,
          json: async () => response.json || {},
          text: async () => response.text || JSON.stringify(response.json || {}),
        };
      }
    }

    // Default 404 for unknown URLs
    return {
      ok: false,
      status: 404,
      json: async () => ({}),
      text: async () => '',
    };
  });
}

function restoreFetch() {
  global.fetch = originalFetch;
}

// =============================================================================
// TESTS
// =============================================================================

describe('DiscoveryService', () => {
  let service;
  let persistence;

  beforeEach(() => {
    persistence = createMockPersistence();
    service = new DiscoveryService(persistence, {
      autoHealthCheck: false,
    });
  });

  afterEach(() => {
    service?.stopHealthChecks();
    restoreFetch();
  });

  // =========================================================================
  // CONSTRUCTOR
  // =========================================================================

  describe('constructor', () => {
    it('should create with default options', () => {
      const svc = new DiscoveryService(null);
      assert.ok(svc);
      assert.equal(svc._initialized, false);
      assert.equal(svc._healthCheckTimer, null);
    });

    it('should store persistence reference', () => {
      assert.equal(service.persistence, persistence);
    });

    it('should accept custom healthCheckIntervalMs', () => {
      const svc = new DiscoveryService(null, { healthCheckIntervalMs: 5000 });
      assert.equal(svc.options.healthCheckIntervalMs, 5000);
    });

    it('should default autoHealthCheck to true', () => {
      const svc = new DiscoveryService(null);
      assert.equal(svc.options.autoHealthCheck, true);
    });

    it('should initialize stats to zero', () => {
      assert.equal(service.stats.mcpServersDiscovered, 0);
      assert.equal(service.stats.pluginsDiscovered, 0);
      assert.equal(service.stats.nodesDiscovered, 0);
      assert.equal(service.stats.healthChecks, 0);
      assert.equal(service.stats.scans, 0);
    });

    it('should store selfEndpoint option', () => {
      const svc = new DiscoveryService(null, { selfEndpoint: 'http://self.dev' });
      assert.equal(svc.options.selfEndpoint, 'http://self.dev');
    });

    it('should inherit from EventEmitter', () => {
      assert.ok(service instanceof EventEmitter);
    });
  });

  // =========================================================================
  // INITIALIZATION
  // =========================================================================

  describe('init', () => {
    it('should initialize only once (idempotent)', async () => {
      await service.init();
      await service.init();
      assert.equal(service._initialized, true);
    });

    it('should work without persistence (limited mode)', async () => {
      const svc = new DiscoveryService(null, { autoHealthCheck: false });
      await svc.init();
      assert.ok(svc._initialized);
    });

    it('should emit initialized event', async () => {
      let emitted = false;
      service.on('initialized', () => { emitted = true; });
      await service.init();
      assert.ok(emitted);
    });

    it('should register self node if selfEndpoint is provided', async () => {
      const svc = new DiscoveryService(persistence, {
        selfEndpoint: 'http://localhost:3000',
        autoHealthCheck: false,
      });
      await svc.init();
      assert.equal(persistence.discovery.upsertNode.mock.calls.length, 1);
      const call = persistence.discovery.upsertNode.mock.calls[0];
      assert.equal(call.arguments[0].endpoint, 'http://localhost:3000');
      assert.equal(call.arguments[0].trustLevel, 'self');
      assert.equal(call.arguments[0].nodeName, 'self');
    });

    it('should not register self node without selfEndpoint', async () => {
      await service.init();
      assert.equal(persistence.discovery.upsertNode.mock.calls.length, 0);
    });

    it('should not register self node without persistence', async () => {
      const svc = new DiscoveryService(null, {
        selfEndpoint: 'http://localhost:3000',
        autoHealthCheck: false,
      });
      await svc.init();
      // No crash, no persistence calls
      assert.equal(svc._initialized, true);
    });

    it('should include capabilities when registering self', async () => {
      const svc = new DiscoveryService(persistence, {
        selfEndpoint: 'http://localhost:3000',
        autoHealthCheck: false,
      });
      await svc.init();
      const call = persistence.discovery.upsertNode.mock.calls[0];
      assert.ok(Array.isArray(call.arguments[0].capabilities));
      assert.ok(call.arguments[0].capabilities.length > 0);
    });
  });

  // =========================================================================
  // MCP DISCOVERY
  // =========================================================================

  describe('scanRepoForMcp', () => {
    it('should discover MCP servers from .mcp.json', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: {
              'my-server': {
                transport: 'sse',
                url: 'http://localhost:8080/sse',
              },
            },
          },
        },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');

      assert.equal(servers.length, 1);
      assert.equal(servers[0].serverName, 'my-server');
      assert.equal(servers[0].transport, 'sse');
      assert.equal(service.stats.scans, 1);
      assert.equal(service.stats.mcpServersDiscovered, 1);
    });

    it('should handle repos without .mcp.json', async () => {
      mockFetch({});

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.deepEqual(servers, []);
    });

    it('should detect stdio transport', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: {
              'stdio-server': {
                command: 'node',
                args: ['server.js'],
              },
            },
          },
        },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.equal(servers[0].transport, 'stdio');
      assert.equal(servers[0].command, 'node');
    });

    it('should discover multiple MCP servers in one config', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: {
              'server-a': { url: 'http://a.dev' },
              'server-b': { url: 'http://b.dev' },
              'server-c': { command: 'npx', args: ['-y', 'my-mcp'] },
            },
          },
        },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.equal(servers.length, 3);
      assert.equal(service.stats.mcpServersDiscovered, 3);
    });

    it('should emit mcpDiscovered event for each server', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: {
              'test': { url: 'http://test' },
            },
          },
        },
      });

      let discovered = null;
      service.on('mcpDiscovered', (server) => { discovered = server; });

      await service.scanRepoForMcp('owner', 'repo');
      assert.ok(discovered);
      assert.equal(discovered.serverName, 'test');
    });

    it('should log discovery event to persistence', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: { 'test': { url: 'http://test' } },
          },
        },
      });

      await service.scanRepoForMcp('owner', 'repo');

      assert.ok(persistence.discovery.logEvent.mock.calls.length > 0);
      const eventCall = persistence.discovery.logEvent.mock.calls[0];
      assert.equal(eventCall.arguments[0].eventType, 'mcp_scan_complete');
    });

    it('should detect env vars in server config', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: {
              'env-server': {
                command: 'node',
                env: { API_KEY: 'xxx', SECRET: 'yyy' },
              },
            },
          },
        },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.deepEqual(servers[0].envVars, ['API_KEY', 'SECRET']);
    });

    it('should set sourceRepo as github:owner/repo format', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: { 'test': { url: 'http://test' } },
          },
        },
      });

      const servers = await service.scanRepoForMcp('myorg', 'myrepo');
      assert.equal(servers[0].sourceRepo, 'github:myorg/myrepo');
    });

    it('should work without persistence (returns server object without id)', async () => {
      const svc = new DiscoveryService(null, { autoHealthCheck: false });
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: { 'test': { url: 'http://test' } },
          },
        },
      });

      const servers = await svc.scanRepoForMcp('owner', 'repo');
      assert.equal(servers.length, 1);
      assert.equal(servers[0].status, 'discovered');
    });

    it('should increment scans counter on each call', async () => {
      mockFetch({});

      await service.scanRepoForMcp('owner', 'repo1');
      await service.scanRepoForMcp('owner', 'repo2');
      assert.equal(service.stats.scans, 2);
    });
  });

  // =========================================================================
  // PLUGIN DISCOVERY
  // =========================================================================

  describe('scanRepoForPlugin', () => {
    it('should discover plugin from .claude/plugin.json', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: {
            name: 'my-plugin',
            version: '1.0.0',
            description: 'Test plugin',
            hooks: true,
          },
        },
      });

      const plugin = await service.scanRepoForPlugin('owner', 'repo');

      assert.ok(plugin);
      assert.equal(plugin.pluginName, 'my-plugin');
      assert.equal(plugin.version, '1.0.0');
      assert.equal(plugin.hasHooks, true);
      assert.equal(service.stats.pluginsDiscovered, 1);
    });

    it('should fallback to root plugin.json', async () => {
      mockFetch({
        'plugin.json': {
          json: {
            name: 'root-plugin',
            version: '2.0.0',
          },
        },
      });

      const plugin = await service.scanRepoForPlugin('owner', 'repo');
      assert.equal(plugin.pluginName, 'root-plugin');
    });

    it('should return null for repos without plugins', async () => {
      mockFetch({});

      const plugin = await service.scanRepoForPlugin('owner', 'repo');
      assert.equal(plugin, null);
    });

    it('should detect all plugin capabilities', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: {
            name: 'full-plugin',
            hooks: { PreToolUse: './hooks/pre.js' },
            agents: { helper: './agents/helper.md' },
            skills: { search: './skills/search.md' },
            mcpServers: { local: { command: 'node' } },
          },
        },
      });

      const plugin = await service.scanRepoForPlugin('owner', 'repo');

      assert.equal(plugin.hasHooks, true);
      assert.equal(plugin.hasAgents, true);
      assert.equal(plugin.hasSkills, true);
      assert.equal(plugin.hasMcpServers, true);
    });

    it('should correctly report false for missing capabilities', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: {
            name: 'minimal-plugin',
            version: '1.0.0',
          },
        },
      });

      const plugin = await service.scanRepoForPlugin('owner', 'repo');
      assert.equal(plugin.hasHooks, false);
      assert.equal(plugin.hasAgents, false);
      assert.equal(plugin.hasSkills, false);
      assert.equal(plugin.hasMcpServers, false);
    });

    it('should emit pluginDiscovered event', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: { name: 'emit-test' },
        },
      });

      let discovered = null;
      service.on('pluginDiscovered', (p) => { discovered = p; });

      await service.scanRepoForPlugin('owner', 'repo');
      assert.ok(discovered);
    });

    it('should log plugin_found event to persistence', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: { name: 'log-test', version: '1.0.0' },
        },
      });

      await service.scanRepoForPlugin('owner', 'repo');

      const logCalls = persistence.discovery.logEvent.mock.calls;
      const pluginEvent = logCalls.find(
        c => c.arguments[0].eventType === 'plugin_found'
      );
      assert.ok(pluginEvent);
    });

    it('should store displayName from manifest', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: {
            name: 'coded-name',
            displayName: 'Human Readable Name',
          },
        },
      });

      const plugin = await service.scanRepoForPlugin('owner', 'repo');
      assert.equal(plugin.displayName, 'Human Readable Name');
    });

    it('should fallback displayName to name when not present', async () => {
      mockFetch({
        '.claude/plugin.json': {
          json: { name: 'only-name' },
        },
      });

      const plugin = await service.scanRepoForPlugin('owner', 'repo');
      assert.equal(plugin.displayName, 'only-name');
    });
  });

  // =========================================================================
  // NODE DISCOVERY
  // =========================================================================

  describe('registerNode', () => {
    it('should register a node', async () => {
      const node = await service.registerNode({
        endpoint: 'http://node1.cynic.dev',
        nodeName: 'node1',
        capabilities: ['judge', 'digest'],
      });

      assert.ok(node.id);
      assert.equal(node.endpoint, 'http://node1.cynic.dev');
      assert.equal(service.stats.nodesDiscovered, 1);
    });

    it('should emit nodeRegistered event', async () => {
      let registered = null;
      service.on('nodeRegistered', (n) => { registered = n; });

      await service.registerNode({ endpoint: 'http://test' });
      assert.ok(registered);
    });

    it('should throw without persistence', async () => {
      const svc = new DiscoveryService(null, { autoHealthCheck: false });
      await svc.init();

      await assert.rejects(
        () => svc.registerNode({ endpoint: 'http://test' }),
        /not available/
      );
    });

    it('should log node_registered event', async () => {
      await service.registerNode({
        endpoint: 'http://logged.dev',
        nodeName: 'logged-node',
      });

      const logCalls = persistence.discovery.logEvent.mock.calls;
      const nodeEvent = logCalls.find(
        c => c.arguments[0].eventType === 'node_registered'
      );
      assert.ok(nodeEvent);
      assert.equal(nodeEvent.arguments[0].targetType, 'node');
    });

    it('should set default discoveredBy to manual', async () => {
      await service.registerNode({ endpoint: 'http://manual.dev' });

      const upsertCall = persistence.discovery.upsertNode.mock.calls[0];
      assert.equal(upsertCall.arguments[0].discoveredBy, 'manual');
    });

    it('should preserve custom discoveredBy', async () => {
      await service.registerNode({
        endpoint: 'http://custom.dev',
        discoveredBy: 'dns-scan',
      });

      const upsertCall = persistence.discovery.upsertNode.mock.calls[0];
      assert.equal(upsertCall.arguments[0].discoveredBy, 'dns-scan');
    });
  });

  describe('discoverNode', () => {
    it('should probe and register node', async () => {
      mockFetch({
        '/health': {
          json: {
            status: 'healthy',
            name: 'discovered-node',
            version: '1.0.0',
            capabilities: ['judge'],
          },
        },
      });

      const node = await service.discoverNode('http://discovered.cynic.dev');

      assert.ok(node);
      assert.equal(node.nodeName, 'discovered-node');
    });

    it('should return null for unreachable nodes', async () => {
      mockFetch({
        '/health': { error: 'Connection refused' },
      });

      const node = await service.discoverNode('http://unreachable.dev');
      assert.equal(node, null);
    });

    it('should set discoveredBy to probe', async () => {
      mockFetch({
        '/health': {
          json: {
            status: 'healthy',
            name: 'probed-node',
            version: '1.0.0',
          },
        },
      });

      await service.discoverNode('http://probed.cynic.dev');

      const upsertCall = persistence.discovery.upsertNode.mock.calls[0];
      assert.equal(upsertCall.arguments[0].discoveredBy, 'probe');
    });

    it('should extract capabilities from identity.tools format', async () => {
      mockFetch({
        '/health': {
          json: {
            status: 'healthy',
            identity: { name: 'tools-node' },
            version: '2.0.0',
            tools: [
              { name: 'brain_judge' },
              { name: 'brain_digest' },
            ],
          },
        },
      });

      await service.discoverNode('http://tools-format.cynic.dev');

      const upsertCall = persistence.discovery.upsertNode.mock.calls[0];
      assert.deepEqual(upsertCall.arguments[0].capabilities, ['brain_judge', 'brain_digest']);
    });

    it('should return null for unhealthy response (non-ok)', async () => {
      mockFetch({
        '/health': { ok: false, status: 503 },
      });

      const node = await service.discoverNode('http://unhealthy.dev');
      assert.equal(node, null);
    });
  });

  // =========================================================================
  // HEALTH CHECKS
  // =========================================================================

  describe('runNodeHealthChecks', () => {
    it('should check all active nodes', async () => {
      persistence.discovery.getNodes = mock.fn(async () => [
        { id: 'node1', endpoint: 'http://node1.dev', trust_level: 'remote' },
        { id: 'node2', endpoint: 'http://node2.dev', trust_level: 'remote' },
      ]);

      mockFetch({
        'node1.dev': { json: { status: 'healthy' } },
        'node2.dev': { json: { status: 'unhealthy' } },
      });

      const results = await service.runNodeHealthChecks();

      assert.equal(results.checked, 2);
      assert.equal(results.healthy, 1);
      assert.equal(results.unhealthy, 1);
    });

    it('should skip self nodes', async () => {
      persistence.discovery.getNodes = mock.fn(async () => [
        { id: 'self', endpoint: 'http://self.dev', trust_level: 'self' },
        { id: 'remote', endpoint: 'http://remote.dev', trust_level: 'remote' },
      ]);

      mockFetch({
        'remote.dev': { json: { status: 'healthy' } },
      });

      const results = await service.runNodeHealthChecks();
      assert.equal(results.checked, 1);
    });

    it('should handle unreachable nodes as unhealthy', async () => {
      // _probeNodeEndpoint swallows fetch errors and returns null,
      // so runNodeHealthChecks treats it as unhealthy (not unreachable)
      persistence.discovery.getNodes = mock.fn(async () => [
        { id: 'node1', endpoint: 'http://unreachable.dev', trust_level: 'remote' },
      ]);

      mockFetch({
        'unreachable.dev': { error: 'Timeout' },
      });

      const results = await service.runNodeHealthChecks();

      assert.equal(results.unhealthy, 1);
      assert.equal(results.healthy, 0);
    });

    it('should emit healthCheckComplete event', async () => {
      persistence.discovery.getNodes = mock.fn(async () => []);

      let results = null;
      service.on('healthCheckComplete', (r) => { results = r; });

      await service.runNodeHealthChecks();
      assert.ok(results);
    });

    it('should update node health in persistence for healthy nodes', async () => {
      persistence.discovery.getNodes = mock.fn(async () => [
        { id: 'node1', endpoint: 'http://node1.dev', trust_level: 'remote' },
      ]);

      mockFetch({
        'node1.dev': { json: { status: 'healthy' } },
      });

      await service.runNodeHealthChecks();

      assert.ok(persistence.discovery.updateNodeHealth.mock.calls.length > 0);
      const call = persistence.discovery.updateNodeHealth.mock.calls[0];
      assert.equal(call.arguments[0], 'node1');
      assert.equal(call.arguments[1].healthStatus, 'healthy');
    });

    it('should update node health for unhealthy nodes', async () => {
      persistence.discovery.getNodes = mock.fn(async () => [
        { id: 'node1', endpoint: 'http://node1.dev', trust_level: 'remote' },
      ]);

      mockFetch({
        'node1.dev': { json: { status: 'degraded' } },
      });

      await service.runNodeHealthChecks();

      const call = persistence.discovery.updateNodeHealth.mock.calls[0];
      assert.equal(call.arguments[1].healthStatus, 'unhealthy');
    });

    it('should increment healthChecks stat', async () => {
      persistence.discovery.getNodes = mock.fn(async () => []);

      await service.runNodeHealthChecks();
      await service.runNodeHealthChecks();

      assert.equal(service.stats.healthChecks, 2);
    });

    it('should return zero counts without persistence', async () => {
      const svc = new DiscoveryService(null, { autoHealthCheck: false });
      await svc.init();

      const results = await svc.runNodeHealthChecks();
      assert.equal(results.checked, 0);
      assert.equal(results.healthy, 0);
      assert.equal(results.unhealthy, 0);
    });

    it('should mark unreachable nodes as unhealthy in persistence', async () => {
      // _probeNodeEndpoint catches fetch errors and returns null,
      // so the health check path marks them 'unhealthy' (not 'unreachable')
      persistence.discovery.getNodes = mock.fn(async () => [
        { id: 'down_node', endpoint: 'http://down.dev', trust_level: 'remote' },
      ]);

      mockFetch({
        'down.dev': { error: 'ECONNREFUSED' },
      });

      await service.runNodeHealthChecks();

      const call = persistence.discovery.updateNodeHealth.mock.calls[0];
      assert.equal(call.arguments[0], 'down_node');
      assert.equal(call.arguments[1].healthStatus, 'unhealthy');
    });
  });

  describe('health check lifecycle', () => {
    it('should start and stop health checks', () => {
      service._startHealthChecks();
      assert.ok(service._healthCheckTimer);

      service.stopHealthChecks();
      assert.equal(service._healthCheckTimer, null);
    });

    it('should not start multiple timers', () => {
      service._startHealthChecks();
      const timer1 = service._healthCheckTimer;

      service._startHealthChecks();
      assert.equal(service._healthCheckTimer, timer1);

      service.stopHealthChecks();
    });

    it('stopHealthChecks is safe when no timer running', () => {
      assert.equal(service._healthCheckTimer, null);
      service.stopHealthChecks(); // Should not throw
      assert.equal(service._healthCheckTimer, null);
    });
  });

  // =========================================================================
  // FULL SCAN
  // =========================================================================

  describe('scanRepo', () => {
    it('should scan for everything (MCP, plugin, CLAUDE.md)', async () => {
      mockFetch({
        '.mcp.json': {
          json: { mcpServers: { 'srv': { url: 'http://srv' } } },
        },
        '.claude/plugin.json': {
          json: { name: 'test-plugin' },
        },
        'CLAUDE.md': {
          text: '# Instructions\nThis is a Claude config.',
        },
      });

      const results = await service.scanRepo('owner', 'repo');

      assert.equal(results.sourceRepo, 'github:owner/repo');
      assert.equal(results.mcpServers.length, 1);
      assert.ok(results.plugin);
      assert.ok(results.claudeMd);
      assert.ok(results.timestamp);
    });

    it('should handle partial results', async () => {
      mockFetch({
        '.mcp.json': {
          json: { mcpServers: { 'srv': { url: 'http://srv' } } },
        },
      });

      const results = await service.scanRepo('owner', 'repo');

      assert.equal(results.mcpServers.length, 1);
      assert.equal(results.plugin, null);
      assert.equal(results.claudeMd, null);
    });

    it('should handle completely empty repo', async () => {
      mockFetch({});

      const results = await service.scanRepo('owner', 'empty-repo');

      assert.deepEqual(results.mcpServers, []);
      assert.equal(results.plugin, null);
      assert.equal(results.claudeMd, null);
    });

    it('should set correct sourceRepo format', async () => {
      mockFetch({});

      const results = await service.scanRepo('my-org', 'my-project');
      assert.equal(results.sourceRepo, 'github:my-org/my-project');
    });

    it('should detect CLAUDE.md with length info', async () => {
      mockFetch({
        'CLAUDE.md': {
          text: '# CLAUDE.md content here for testing purposes',
        },
      });

      const results = await service.scanRepo('owner', 'repo');
      assert.ok(results.claudeMd);
      assert.equal(results.claudeMd.found, true);
      assert.ok(results.claudeMd.length > 0);
    });
  });

  // =========================================================================
  // QUERIES
  // =========================================================================

  describe('query methods', () => {
    it('getMcpServers should delegate to persistence', async () => {
      persistence.discovery.getMcpServers = mock.fn(async () => [{ id: 'srv1' }]);

      const servers = await service.getMcpServers({ status: 'active' });

      assert.equal(servers.length, 1);
      assert.equal(persistence.discovery.getMcpServers.mock.calls[0].arguments[0].status, 'active');
    });

    it('getPlugins should delegate to persistence', async () => {
      persistence.discovery.getPlugins = mock.fn(async () => [{ id: 'plg1' }]);

      const plugins = await service.getPlugins();
      assert.equal(plugins.length, 1);
    });

    it('getNodes should delegate to persistence', async () => {
      persistence.discovery.getNodes = mock.fn(async () => [{ id: 'node1' }]);

      const nodes = await service.getNodes();
      assert.equal(nodes.length, 1);
    });

    it('should return empty arrays without persistence', async () => {
      const svc = new DiscoveryService(null, { autoHealthCheck: false });
      await svc.init();

      assert.deepEqual(await svc.getMcpServers(), []);
      assert.deepEqual(await svc.getPlugins(), []);
      assert.deepEqual(await svc.getNodes(), []);
    });

    it('getMcpServers should pass filter options through', async () => {
      persistence.discovery.getMcpServers = mock.fn(async () => []);

      await service.getMcpServers({ transport: 'sse', status: 'discovered' });

      const args = persistence.discovery.getMcpServers.mock.calls[0].arguments[0];
      assert.equal(args.transport, 'sse');
      assert.equal(args.status, 'discovered');
    });

    it('getNodes should auto-initialize on first call', async () => {
      assert.equal(service._initialized, false);
      await service.getNodes();
      assert.equal(service._initialized, true);
    });
  });

  describe('getStats', () => {
    it('should merge runtime and db stats', async () => {
      persistence.discovery.getStats = mock.fn(async () => ({
        totalMcp: 10,
        totalPlugins: 5,
      }));

      service.stats.scans = 3;

      const stats = await service.getStats();

      assert.equal(stats.scans, 3);
      assert.equal(stats.totalMcp, 10);
    });

    it('should handle missing persistence gracefully', async () => {
      const svc = new DiscoveryService(null, { autoHealthCheck: false });
      svc.stats.scans = 5;

      const stats = await svc.getStats();
      assert.equal(stats.scans, 5);
    });

    it('should include all runtime stat fields', async () => {
      const stats = await service.getStats();

      assert.ok('mcpServersDiscovered' in stats);
      assert.ok('pluginsDiscovered' in stats);
      assert.ok('nodesDiscovered' in stats);
      assert.ok('healthChecks' in stats);
      assert.ok('scans' in stats);
    });
  });

  // =========================================================================
  // LIFECYCLE
  // =========================================================================

  describe('shutdown', () => {
    it('should stop health checks and reset state', async () => {
      await service.init();
      service._startHealthChecks();

      await service.shutdown();

      assert.equal(service._initialized, false);
      assert.equal(service._healthCheckTimer, null);
    });

    it('should emit shutdown event', async () => {
      let emitted = false;
      service.on('shutdown', () => { emitted = true; });

      await service.shutdown();
      assert.ok(emitted);
    });

    it('should allow re-initialization after shutdown', async () => {
      await service.init();
      assert.equal(service._initialized, true);

      await service.shutdown();
      assert.equal(service._initialized, false);

      await service.init();
      assert.equal(service._initialized, true);
    });
  });

  // =========================================================================
  // EDGE CASES
  // =========================================================================

  describe('edge cases', () => {
    it('should handle GitHub API errors gracefully', async () => {
      mockFetch({
        '.mcp.json': { status: 500, ok: false },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.deepEqual(servers, []);
    });

    it('should use GitHub token when provided', async () => {
      const svc = new DiscoveryService(persistence, {
        githubToken: 'test-token',
        autoHealthCheck: false,
      });

      let capturedHeaders = null;
      global.fetch = mock.fn(async (url, options) => {
        capturedHeaders = options?.headers;
        return { ok: false, status: 404, text: async () => '' };
      });

      await svc.scanRepoForMcp('owner', 'repo');

      assert.ok(capturedHeaders?.Authorization?.includes('test-token'));
    });

    it('should track stats correctly across multiple operations', async () => {
      mockFetch({
        '.mcp.json': {
          json: {
            mcpServers: {
              'srv1': { url: 'http://srv1' },
              'srv2': { url: 'http://srv2' },
            },
          },
        },
      });

      await service.scanRepoForMcp('owner', 'repo');

      assert.equal(service.stats.scans, 1);
      assert.equal(service.stats.mcpServersDiscovered, 2);
    });

    it('should handle fetch network errors during scan', async () => {
      mockFetch({
        '.mcp.json': { error: 'Network error' },
      });

      // Should not throw
      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.deepEqual(servers, []);
    });

    it('should handle malformed .mcp.json (no mcpServers key)', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: { version: '1.0.0', something: 'else' },
        },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.deepEqual(servers, []);
    });

    it('should handle .mcp.json with empty mcpServers', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: { mcpServers: {} },
        },
      });

      const servers = await service.scanRepoForMcp('owner', 'repo');
      assert.deepEqual(servers, []);
    });

    it('should handle concurrent scans without interference', async () => {
      mockFetch({
        'contents/.mcp.json': {
          json: {
            mcpServers: { 'srv': { url: 'http://srv' } },
          },
        },
      });

      const [result1, result2] = await Promise.all([
        service.scanRepoForMcp('owner1', 'repo1'),
        service.scanRepoForMcp('owner2', 'repo2'),
      ]);

      assert.equal(result1.length, 1);
      assert.equal(result2.length, 1);
      assert.equal(service.stats.scans, 2);
    });

    it('should set User-Agent header in GitHub requests', async () => {
      let capturedHeaders = null;
      global.fetch = mock.fn(async (url, options) => {
        capturedHeaders = options?.headers;
        return { ok: false, status: 404, text: async () => '' };
      });

      await service.scanRepoForMcp('owner', 'repo');
      assert.ok(capturedHeaders?.['User-Agent']?.includes('CYNIC'));
    });
  });
});
