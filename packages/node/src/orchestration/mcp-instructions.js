/**
 * MCP Instructions - Custom instructions per MCP server
 *
 * Loads configurable instructions from mcp-instructions.json and injects
 * them into context based on server usage.
 *
 * P3.2: Allow MCP servers to provide usage hints and context.
 *
 * "φ guides the tools" - κυνικός
 *
 * @module @cynic/node/orchestration/mcp-instructions
 */

'use strict';

import { createLogger } from '@cynic/core';
import { promises as fs } from 'fs';
import { existsSync } from 'fs';
import { resolve } from 'path';

const log = createLogger('MCPInstructions');

/**
 * Default configuration
 */
export const MCP_INSTRUCTIONS_CONFIG = {
  enabled: true,
  injectOnSessionStart: true,
  injectOnToolCall: false,
  maxInstructionsPerServer: 10,
  showToolHints: true,
  instructionsPath: null,
};

/**
 * Server instruction set
 */
export class ServerInstructions {
  /**
   * @param {string} serverId - Server identifier
   * @param {Object} config - Server configuration
   */
  constructor(serverId, config) {
    this.serverId = serverId;
    this.name = config.name || serverId;
    this.description = config.description || '';
    this.instructions = config.instructions || [];
    this.contextInjection = config.contextInjection || {};
    this.toolHints = config.toolHints || {};
  }

  /**
   * Get instructions as formatted string
   */
  format() {
    if (this.instructions.length === 0) return '';

    const lines = [`## ${this.name}`];
    if (this.description) {
      lines.push(this.description);
    }
    lines.push('');

    for (const instruction of this.instructions) {
      lines.push(`- ${instruction}`);
    }

    return lines.join('\n');
  }

  /**
   * Get tool hint for a specific tool
   */
  getToolHint(toolName) {
    // Handle prefixed tool names (e.g., mcp__cynic__brain_judge)
    const baseName = toolName.split('__').pop();
    return this.toolHints[baseName] || this.toolHints[toolName] || null;
  }

  /**
   * Should inject on session start?
   */
  get shouldInjectOnStart() {
    return this.contextInjection.onSessionStart !== false;
  }

  /**
   * Should inject on tool call?
   */
  get shouldInjectOnToolCall() {
    return this.contextInjection.onToolCall === true;
  }
}

/**
 * MCP Instructions Manager
 *
 * Loads and manages server-specific instructions.
 */
export class MCPInstructions {
  /**
   * @param {Object} options
   * @param {string} [options.instructionsPath] - Path to mcp-instructions.json
   * @param {Object} [options.config] - Override config
   */
  constructor(options = {}) {
    this.config = { ...MCP_INSTRUCTIONS_CONFIG, ...options.config };
    this.instructionsPath = options.instructionsPath || this.config.instructionsPath;

    // Loaded server instructions
    this.servers = new Map();
    this.version = null;
    this.settings = {};

    // Track which servers have been used
    this._usedServers = new Set();

    // Statistics
    this.stats = {
      serversLoaded: 0,
      injections: 0,
      toolHintsProvided: 0,
    };

    // Auto-load if path provided
    if (this.instructionsPath) {
      this.loadInstructions(this.instructionsPath).catch(err => {
        log.warn('Failed to auto-load instructions', { error: err.message });
      });
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Loading
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Load instructions from JSON file
   *
   * @param {string} filePath - Path to mcp-instructions.json
   * @returns {Promise<boolean>} Success
   */
  async loadInstructions(filePath) {
    try {
      const resolvedPath = resolve(filePath);

      if (!existsSync(resolvedPath)) {
        log.warn('Instructions file not found', { path: resolvedPath });
        return false;
      }

      const content = await fs.readFile(resolvedPath, 'utf-8');
      const data = JSON.parse(content);

      // Validate structure
      if (!data.servers || typeof data.servers !== 'object') {
        throw new Error('Invalid instructions file: missing "servers" object');
      }

      // Store settings
      this.version = data.version || '1.0.0';
      this.settings = data.settings || {};
      Object.assign(this.config, this.settings);

      // Load server instructions
      this.servers.clear();
      for (const [serverId, serverConfig] of Object.entries(data.servers)) {
        this.servers.set(serverId, new ServerInstructions(serverId, serverConfig));
      }

      this.stats.serversLoaded = this.servers.size;
      log.info('Instructions loaded', { servers: this.servers.size, version: this.version });

      return true;
    } catch (err) {
      log.error('Failed to load instructions', { path: filePath, error: err.message });
      return false;
    }
  }

  /**
   * Add server instructions dynamically
   *
   * @param {string} serverId - Server ID
   * @param {Object} config - Server configuration
   */
  addServer(serverId, config) {
    this.servers.set(serverId, new ServerInstructions(serverId, config));
    this.stats.serversLoaded = this.servers.size;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Context Injection
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get instructions to inject at session start
   *
   * @returns {string} Formatted instructions for all servers
   */
  getSessionStartInstructions() {
    if (!this.config.enabled || !this.config.injectOnSessionStart) {
      return '';
    }

    const sections = [];

    for (const [serverId, server] of this.servers) {
      if (server.shouldInjectOnStart) {
        const formatted = server.format();
        if (formatted) {
          sections.push(formatted);
        }
      }
    }

    if (sections.length === 0) return '';

    this.stats.injections++;
    return `# MCP Server Instructions\n\n${sections.join('\n\n')}`;
  }

  /**
   * Get instructions for a specific tool call
   *
   * @param {string} toolName - Tool being called
   * @returns {Object|null} Injection info or null
   */
  getToolCallInstructions(toolName) {
    if (!this.config.enabled) return null;

    // Find which server this tool belongs to
    const serverId = this._getServerIdFromTool(toolName);
    if (!serverId) return null;

    const server = this.servers.get(serverId);
    if (!server) return null;

    // Mark server as used
    this._usedServers.add(serverId);

    // Get tool hint
    const toolHint = server.getToolHint(toolName);

    // Build injection
    const result = {
      serverId,
      serverName: server.name,
    };

    // Include full instructions if configured
    if (server.shouldInjectOnToolCall && !this._usedServers.has(serverId)) {
      result.instructions = server.format();
      this.stats.injections++;
    }

    // Include tool hint
    if (toolHint && this.config.showToolHints) {
      result.toolHint = toolHint;
      this.stats.toolHintsProvided++;
    }

    return result.instructions || result.toolHint ? result : null;
  }

  /**
   * Extract server ID from tool name
   * @private
   */
  _getServerIdFromTool(toolName) {
    // Pattern: mcp__serverId__toolName
    const match = toolName.match(/^mcp__([^_]+)__/);
    if (match) {
      return match[1];
    }

    // Check if any server has hints for this tool
    for (const [serverId, server] of this.servers) {
      if (server.toolHints[toolName]) {
        return serverId;
      }
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Query Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get server instructions by ID
   *
   * @param {string} serverId - Server ID
   * @returns {ServerInstructions|null}
   */
  getServer(serverId) {
    return this.servers.get(serverId) || null;
  }

  /**
   * Get all server IDs
   *
   * @returns {string[]}
   */
  getServerIds() {
    return [...this.servers.keys()];
  }

  /**
   * Get tool hints for a server
   *
   * @param {string} serverId - Server ID
   * @returns {Object|null} Tool hints object
   */
  getToolHints(serverId) {
    const server = this.servers.get(serverId);
    return server ? { ...server.toolHints } : null;
  }

  /**
   * Get servers that have been used this session
   *
   * @returns {string[]}
   */
  getUsedServers() {
    return [...this._usedServers];
  }

  /**
   * Reset used servers tracking
   */
  resetUsedServers() {
    this._usedServers.clear();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Statistics
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      enabled: this.config.enabled,
      usedServers: this._usedServers.size,
      version: this.version,
    };
  }

  /**
   * Export configuration for persistence
   *
   * @returns {Object}
   */
  export() {
    const servers = {};
    for (const [id, server] of this.servers) {
      servers[id] = {
        name: server.name,
        description: server.description,
        instructions: server.instructions,
        contextInjection: server.contextInjection,
        toolHints: server.toolHints,
      };
    }

    return {
      version: this.version,
      servers,
      settings: this.settings,
    };
  }
}

/**
 * Create MCPInstructions instance
 *
 * @param {Object} options - Options
 * @returns {MCPInstructions}
 */
export function createMCPInstructions(options = {}) {
  return new MCPInstructions(options);
}

// Singleton
let _instance = null;

/**
 * Get or create global MCPInstructions
 *
 * @param {Object} [options] - Options for creation
 * @returns {MCPInstructions}
 */
export function getMCPInstructions(options = {}) {
  if (!_instance) {
    _instance = createMCPInstructions(options);
  }
  return _instance;
}

/**
 * Find mcp-instructions.json in project
 *
 * @param {string} [startDir] - Directory to start search
 * @returns {string|null} Path or null
 */
export function findInstructionsFile(startDir = process.cwd()) {
  const candidates = [
    resolve(startDir, '.claude', 'mcp-instructions.json'),
    resolve(startDir, 'mcp-instructions.json'),
    resolve(startDir, '.cynic', 'mcp-instructions.json'),
  ];

  for (const path of candidates) {
    if (existsSync(path)) {
      return path;
    }
  }

  return null;
}

export default MCPInstructions;
