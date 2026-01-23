/**
 * Tool Registry - OCP-Compliant Tool Management
 *
 * "Open for extension, closed for modification"
 * - Tools register themselves
 * - No switch/if-else chains
 * - Adding a tool doesn't modify this file
 *
 * @module @cynic/mcp/tools/registry
 */

'use strict';

/**
 * @typedef {Object} ToolDefinition
 * @property {string} name - Tool name (e.g., 'brain_cynic_judge')
 * @property {string} description - Tool description for MCP
 * @property {Object} inputSchema - JSON Schema for input validation
 * @property {Function} handler - Async function handling tool calls
 */

/**
 * @typedef {Object} ToolFactory
 * @property {string} name - Factory name for identification
 * @property {string} domain - Domain category (judgment, ecosystem, etc.)
 * @property {string[]} [requires] - Required dependencies
 * @property {Function} create - Factory function (options) => ToolDefinition[]
 */

/**
 * Tool Registry - OCP Pattern
 *
 * Usage:
 * ```javascript
 * const registry = new ToolRegistry();
 *
 * // Register a tool factory
 * registry.register({
 *   name: 'judge',
 *   domain: 'judgment',
 *   requires: ['judge', 'persistence'],
 *   create: (options) => [createJudgeTool(options.judge, options.persistence)]
 * });
 *
 * // Create all tools with provided dependencies
 * const tools = registry.createAll({
 *   judge: judgeInstance,
 *   persistence: persistenceInstance
 * });
 * ```
 */
export class ToolRegistry {
  constructor() {
    /** @type {Map<string, ToolFactory>} */
    this._factories = new Map();

    /** @type {Map<string, ToolDefinition>} */
    this._tools = new Map();

    /** @type {Set<string>} */
    this._domains = new Set();
  }

  /**
   * Register a tool factory
   * @param {ToolFactory} factory
   * @returns {ToolRegistry} this (for chaining)
   */
  register(factory) {
    if (!factory.name) {
      throw new Error('Tool factory must have a name');
    }
    if (!factory.create || typeof factory.create !== 'function') {
      throw new Error(`Tool factory '${factory.name}' must have a create function`);
    }

    this._factories.set(factory.name, factory);

    if (factory.domain) {
      this._domains.add(factory.domain);
    }

    return this;
  }

  /**
   * Register multiple factories at once
   * @param {ToolFactory[]} factories
   * @returns {ToolRegistry} this
   */
  registerAll(factories) {
    for (const factory of factories) {
      this.register(factory);
    }
    return this;
  }

  /**
   * Create all tools from registered factories
   * @param {Object} options - Dependencies and configuration
   * @returns {ToolDefinition[]} All created tools
   */
  createAll(options = {}) {
    const tools = [];

    for (const [name, factory] of this._factories) {
      // Check if required dependencies are available
      if (factory.requires) {
        const missing = factory.requires.filter(dep => !options[dep]);
        if (missing.length > 0) {
          console.error(`   Tool factory '${name}' skipped: missing ${missing.join(', ')}`);
          continue;
        }
      }

      try {
        const created = factory.create(options);
        const createdTools = Array.isArray(created) ? created : [created];

        for (const tool of createdTools) {
          if (tool && tool.name) {
            this._tools.set(tool.name, tool);
            tools.push(tool);
          }
        }
      } catch (err) {
        console.error(`   Tool factory '${name}' error:`, err.message);
      }
    }

    return tools;
  }

  /**
   * Create tools for a specific domain only
   * @param {string} domain
   * @param {Object} options
   * @returns {ToolDefinition[]}
   */
  createByDomain(domain, options = {}) {
    const tools = [];

    for (const [name, factory] of this._factories) {
      if (factory.domain !== domain) continue;

      if (factory.requires) {
        const missing = factory.requires.filter(dep => !options[dep]);
        if (missing.length > 0) continue;
      }

      try {
        const created = factory.create(options);
        const createdTools = Array.isArray(created) ? created : [created];
        tools.push(...createdTools.filter(t => t && t.name));
      } catch (err) {
        console.error(`   Tool factory '${name}' error:`, err.message);
      }
    }

    return tools;
  }

  /**
   * Get a specific tool by name
   * @param {string} name
   * @returns {ToolDefinition|undefined}
   */
  get(name) {
    return this._tools.get(name);
  }

  /**
   * Get all registered tool names
   * @returns {string[]}
   */
  getToolNames() {
    return Array.from(this._tools.keys());
  }

  /**
   * Get all registered domains
   * @returns {string[]}
   */
  getDomains() {
    return Array.from(this._domains);
  }

  /**
   * Get factory count
   * @returns {number}
   */
  get factoryCount() {
    return this._factories.size;
  }

  /**
   * Get tool count (after createAll)
   * @returns {number}
   */
  get toolCount() {
    return this._tools.size;
  }

  /**
   * Clear all registered factories and tools
   */
  clear() {
    this._factories.clear();
    this._tools.clear();
    this._domains.clear();
  }
}

/**
 * Global default registry instance
 */
export const defaultRegistry = new ToolRegistry();

/**
 * Convenience function to register a tool factory
 * @param {ToolFactory} factory
 */
export function registerTool(factory) {
  return defaultRegistry.register(factory);
}

/**
 * Convenience function to register multiple factories
 * @param {ToolFactory[]} factories
 */
export function registerTools(factories) {
  return defaultRegistry.registerAll(factories);
}
