/**
 * Discovery Service
 *
 * Automatic discovery of MCP servers, Claude Code plugins, and CYNIC nodes.
 *
 * "The pack finds all dens" - κυνικός
 *
 * @module @cynic/mcp/discovery-service
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger } from '@cynic/core';

const log = createLogger('DiscoveryService');

const PHI_INV = 0.618033988749895;
const HEALTH_CHECK_INTERVAL = 61800; // φ × 1000ms

/**
 * Discovery Service
 */
export class DiscoveryService extends EventEmitter {
  /**
   * @param {Object} persistence - Persistence manager with discovery repository
   * @param {Object} [options] - Options
   */
  constructor(persistence, options = {}) {
    super();

    this.persistence = persistence;
    this.options = {
      healthCheckIntervalMs: options.healthCheckIntervalMs || HEALTH_CHECK_INTERVAL,
      autoHealthCheck: options.autoHealthCheck !== false,
      githubToken: options.githubToken || process.env.GITHUB_TOKEN,
      selfEndpoint: options.selfEndpoint || null,
      ...options,
    };

    this._initialized = false;
    this._healthCheckTimer = null;

    // Stats
    this.stats = {
      mcpServersDiscovered: 0,
      pluginsDiscovered: 0,
      nodesDiscovered: 0,
      healthChecks: 0,
      scans: 0,
    };
  }

  /**
   * Initialize the service
   */
  async init() {
    if (this._initialized) return;

    if (!this.persistence?.discovery) {
      log.warn('No discovery repository - running in limited mode');
    }

    this._initialized = true;

    // Register self node if endpoint provided
    if (this.options.selfEndpoint) {
      await this.registerSelfNode();
    }

    // Start health checks if enabled
    if (this.options.autoHealthCheck && this.persistence?.discovery) {
      this._startHealthChecks();
    }

    this.emit('initialized');
  }

  /**
   * Register this node as self
   */
  async registerSelfNode() {
    if (!this.options.selfEndpoint || !this.persistence?.discovery) return;

    await this.persistence.discovery.upsertNode({
      endpoint: this.options.selfEndpoint,
      nodeName: 'self',
      status: 'active',
      trustLevel: 'self',
      discoveredBy: 'self',
      capabilities: ['judge', 'digest', 'search', 'patterns', 'poj'],
      version: process.env.CYNIC_VERSION || '1.0.0',
      protocolVersion: '1.0',
    });
  }

  // ============================================
  // MCP DISCOVERY
  // ============================================

  /**
   * Scan a repository for MCP servers
   * @param {string} owner - GitHub repo owner
   * @param {string} repo - GitHub repo name
   * @returns {Promise<Object[]>} Discovered servers
   */
  async scanRepoForMcp(owner, repo) {
    await this.init();
    this.stats.scans++;

    const sourceRepo = `github:${owner}/${repo}`;
    const discovered = [];

    try {
      // Fetch .mcp.json from GitHub
      const mcpConfig = await this._fetchGitHubFile(owner, repo, '.mcp.json');

      if (mcpConfig && mcpConfig.mcpServers) {
        for (const [serverName, serverConfig] of Object.entries(mcpConfig.mcpServers)) {
          const server = await this._processDiscoveredMcpServer(
            sourceRepo,
            serverName,
            serverConfig
          );
          discovered.push(server);
        }
      }

      // Log event
      if (this.persistence?.discovery && discovered.length > 0) {
        await this.persistence.discovery.logEvent({
          eventType: 'mcp_scan_complete',
          targetType: 'mcp',
          source: sourceRepo,
          details: { serversFound: discovered.length },
        });
      }
    } catch (error) {
      // No .mcp.json or error - that's OK
      if (error.message !== 'File not found') {
        log.warn('Error scanning for MCP', { sourceRepo, error: error.message });
      }
    }

    return discovered;
  }

  /**
   * Process a discovered MCP server
   * @private
   */
  async _processDiscoveredMcpServer(sourceRepo, serverName, serverConfig) {
    const transport = serverConfig.transport || (serverConfig.url ? 'sse' : 'stdio');

    const server = {
      sourceRepo,
      serverName,
      transport,
      config: serverConfig,
      endpoint: serverConfig.url || null,
      command: serverConfig.command || null,
      args: serverConfig.args || null,
      envVars: serverConfig.env ? Object.keys(serverConfig.env) : null,
      status: 'discovered',
    };

    if (this.persistence?.discovery) {
      const stored = await this.persistence.discovery.upsertMcpServer(server);
      this.stats.mcpServersDiscovered++;
      this.emit('mcpDiscovered', stored);
      return stored;
    }

    return server;
  }

  // ============================================
  // PLUGIN DISCOVERY
  // ============================================

  /**
   * Scan a repository for Claude Code plugins
   * @param {string} owner - GitHub repo owner
   * @param {string} repo - GitHub repo name
   * @returns {Promise<Object|null>} Discovered plugin or null
   */
  async scanRepoForPlugin(owner, repo) {
    await this.init();
    this.stats.scans++;

    const sourceRepo = `github:${owner}/${repo}`;

    try {
      // Try .claude/plugin.json first, then plugin.json
      let pluginManifest = await this._fetchGitHubFile(owner, repo, '.claude/plugin.json');

      if (!pluginManifest) {
        pluginManifest = await this._fetchGitHubFile(owner, repo, 'plugin.json');
      }

      if (pluginManifest && pluginManifest.name) {
        const plugin = await this._processDiscoveredPlugin(sourceRepo, pluginManifest);

        // Log event
        if (this.persistence?.discovery) {
          await this.persistence.discovery.logEvent({
            eventType: 'plugin_found',
            targetType: 'plugin',
            targetId: plugin.id,
            source: sourceRepo,
            details: { pluginName: plugin.plugin_name, version: plugin.version },
          });
        }

        return plugin;
      }
    } catch (error) {
      if (error.message !== 'File not found') {
        log.warn('Error scanning for plugin', { sourceRepo, error: error.message });
      }
    }

    return null;
  }

  /**
   * Process a discovered plugin
   * @private
   */
  async _processDiscoveredPlugin(sourceRepo, manifest) {
    const plugin = {
      sourceRepo,
      pluginName: manifest.name,
      displayName: manifest.displayName || manifest.name,
      version: manifest.version,
      description: manifest.description,
      author: manifest.author,
      manifest,
      hasHooks: !!manifest.hooks,
      hasAgents: !!manifest.agents,
      hasSkills: !!manifest.skills,
      hasMcpServers: !!manifest.mcpServers,
      hookCount: 0, // Would need to scan hooks dir
      agentCount: 0, // Would need to scan agents dir
      skillCount: 0, // Would need to scan skills dir
      status: 'discovered',
    };

    if (this.persistence?.discovery) {
      const stored = await this.persistence.discovery.upsertPlugin(plugin);
      this.stats.pluginsDiscovered++;
      this.emit('pluginDiscovered', stored);
      return stored;
    }

    return plugin;
  }

  // ============================================
  // NODE DISCOVERY
  // ============================================

  /**
   * Register a CYNIC node
   * @param {Object} node - Node data
   * @returns {Promise<Object>} Registered node
   */
  async registerNode(node) {
    await this.init();

    if (!this.persistence?.discovery) {
      throw new Error('Discovery repository not available');
    }

    const stored = await this.persistence.discovery.upsertNode({
      ...node,
      discoveredBy: node.discoveredBy || 'manual',
    });

    this.stats.nodesDiscovered++;
    this.emit('nodeRegistered', stored);

    // Log event
    await this.persistence.discovery.logEvent({
      eventType: 'node_registered',
      targetType: 'node',
      targetId: stored.id,
      source: node.endpoint,
      details: { nodeName: stored.node_name, trustLevel: stored.trust_level },
    });

    return stored;
  }

  /**
   * Discover a node by endpoint
   * @param {string} endpoint - Node endpoint URL
   * @returns {Promise<Object|null>} Discovered node or null
   */
  async discoverNode(endpoint) {
    await this.init();

    try {
      // Try to contact the node and get its info
      const nodeInfo = await this._probeNodeEndpoint(endpoint);

      if (nodeInfo) {
        return await this.registerNode({
          endpoint,
          nodeName: nodeInfo.name || nodeInfo.identity?.name,
          version: nodeInfo.version,
          capabilities: nodeInfo.capabilities || nodeInfo.tools?.map(t => t.name),
          protocolVersion: nodeInfo.protocolVersion,
          discoveredBy: 'probe',
        });
      }
    } catch (error) {
      log.warn('Failed to discover node', { endpoint, error: error.message });
    }

    return null;
  }

  /**
   * Probe a node endpoint for health/info
   * @private
   */
  async _probeNodeEndpoint(endpoint) {
    try {
      // For SSE endpoints, try to fetch health
      const healthUrl = new URL('/health', endpoint).toString();
      const response = await fetch(healthUrl, {
        method: 'GET',
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(5000),
      });

      if (response.ok) {
        return await response.json();
      }
    } catch (error) {
      log.debug('Health probe failed', { endpoint, error: error.message });
    }
    return null;
  }

  // ============================================
  // HEALTH CHECKS
  // ============================================

  /**
   * Run health checks on all nodes
   * @returns {Promise<Object>} Health check results
   */
  async runNodeHealthChecks() {
    await this.init();
    this.stats.healthChecks++;

    if (!this.persistence?.discovery) {
      return { checked: 0, healthy: 0, unhealthy: 0 };
    }

    const nodes = await this.persistence.discovery.getNodes({ status: 'active' });
    const results = { checked: 0, healthy: 0, unhealthy: 0, errors: [] };

    for (const node of nodes) {
      if (node.trust_level === 'self') continue; // Skip self

      results.checked++;
      const startTime = Date.now();

      try {
        const health = await this._probeNodeEndpoint(node.endpoint);
        const latencyMs = Date.now() - startTime;

        if (health && health.status === 'healthy') {
          await this.persistence.discovery.updateNodeHealth(node.id, {
            healthStatus: 'healthy',
            latencyMs,
          });
          results.healthy++;
        } else {
          await this.persistence.discovery.updateNodeHealth(node.id, {
            healthStatus: 'unhealthy',
            latencyMs,
          });
          results.unhealthy++;
        }
      } catch (error) {
        await this.persistence.discovery.updateNodeHealth(node.id, {
          status: 'unreachable',
          healthStatus: 'unreachable',
        });
        results.unhealthy++;
        results.errors.push({ nodeId: node.id, error: error.message });
      }
    }

    this.emit('healthCheckComplete', results);
    return results;
  }

  /**
   * Start periodic health checks
   * @private
   */
  _startHealthChecks() {
    if (this._healthCheckTimer) return;

    this._healthCheckTimer = setInterval(async () => {
      try {
        await this.runNodeHealthChecks();
      } catch (error) {
        log.error('Health check error', { error: error.message });
      }
    }, this.options.healthCheckIntervalMs);

    // Don't prevent process exit
    this._healthCheckTimer.unref?.();
  }

  /**
   * Stop health checks
   */
  stopHealthChecks() {
    if (this._healthCheckTimer) {
      clearInterval(this._healthCheckTimer);
      this._healthCheckTimer = null;
    }
  }

  // ============================================
  // FULL SCAN
  // ============================================

  /**
   * Scan a repository for everything (MCP, plugins, CLAUDE.md)
   * @param {string} owner - GitHub repo owner
   * @param {string} repo - GitHub repo name
   * @returns {Promise<Object>} Scan results
   */
  async scanRepo(owner, repo) {
    await this.init();

    const results = {
      sourceRepo: `github:${owner}/${repo}`,
      mcpServers: [],
      plugin: null,
      claudeMd: null,
      timestamp: Date.now(),
    };

    // Scan for MCP servers
    results.mcpServers = await this.scanRepoForMcp(owner, repo);

    // Scan for plugin
    results.plugin = await this.scanRepoForPlugin(owner, repo);

    // Check for CLAUDE.md
    try {
      const claudeMd = await this._fetchGitHubFile(owner, repo, 'CLAUDE.md');
      if (claudeMd) {
        results.claudeMd = { found: true, length: JSON.stringify(claudeMd).length };
      }
    } catch (error) {
      // Expected for repos without CLAUDE.md (404), log unexpected errors
      if (!error.message?.includes('404')) {
        log.debug('Failed to fetch CLAUDE.md', { owner, repo, error: error.message });
      }
    }

    return results;
  }

  // ============================================
  // QUERIES
  // ============================================

  /**
   * Get all discovered MCP servers
   * @param {Object} [options] - Filter options
   * @returns {Promise<Object[]>} Servers
   */
  async getMcpServers(options = {}) {
    await this.init();
    if (!this.persistence?.discovery) return [];
    return this.persistence.discovery.getMcpServers(options);
  }

  /**
   * Get all discovered plugins
   * @param {Object} [options] - Filter options
   * @returns {Promise<Object[]>} Plugins
   */
  async getPlugins(options = {}) {
    await this.init();
    if (!this.persistence?.discovery) return [];
    return this.persistence.discovery.getPlugins(options);
  }

  /**
   * Get all discovered nodes
   * @param {Object} [options] - Filter options
   * @returns {Promise<Object[]>} Nodes
   */
  async getNodes(options = {}) {
    await this.init();
    if (!this.persistence?.discovery) return [];
    return this.persistence.discovery.getNodes(options);
  }

  /**
   * Get discovery statistics
   * @returns {Promise<Object>} Stats
   */
  async getStats() {
    await this.init();

    const dbStats = this.persistence?.discovery
      ? await this.persistence.discovery.getStats()
      : {};

    return {
      ...this.stats,
      ...dbStats,
    };
  }

  // ============================================
  // GITHUB HELPERS
  // ============================================

  /**
   * Fetch a file from GitHub
   * @private
   */
  async _fetchGitHubFile(owner, repo, path) {
    const url = `https://api.github.com/repos/${owner}/${repo}/contents/${path}`;
    const headers = {
      'Accept': 'application/vnd.github.v3.raw',
      'User-Agent': 'CYNIC-Discovery/1.0',
    };

    if (this.options.githubToken) {
      headers['Authorization'] = `token ${this.options.githubToken}`;
    }

    const response = await fetch(url, { headers });

    if (response.status === 404) {
      throw new Error('File not found');
    }

    if (!response.ok) {
      throw new Error(`GitHub API error: ${response.status}`);
    }

    const text = await response.text();

    // Try to parse as JSON
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  }

  // ============================================
  // LIFECYCLE
  // ============================================

  /**
   * Shutdown service
   */
  async shutdown() {
    this.stopHealthChecks();
    this._initialized = false;
    this.emit('shutdown');
  }
}

export default DiscoveryService;
