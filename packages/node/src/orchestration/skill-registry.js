/**
 * SkillRegistry - Skill-to-Domain Mapping and Auto-Invocation
 *
 * Maps KETER routing decisions to skills and auto-invokes them.
 * Bridges the gap between routing (what domain) and execution (which skill).
 *
 * "φ knows which skill to call" - κυνικός
 *
 * @module @cynic/node/orchestration/skill-registry
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('SkillRegistry');

/**
 * Default skill mappings (domain → skill)
 *
 * These map KETER routing domains to actual skills.
 */
const DEFAULT_SKILL_MAPPINGS = {
  // Wisdom domain → wisdom skill
  wisdom: {
    skill: 'wisdom',
    mcpTool: 'brain_wisdom',
    description: 'Query CYNIC philosophical wisdom',
  },

  // Protection domain → judge skill
  protection: {
    skill: 'judge',
    mcpTool: 'brain_cynic_judge',
    description: 'Judge item quality and safety',
  },

  // Analysis domain → patterns skill
  analysis: {
    skill: 'patterns',
    mcpTool: 'brain_patterns',
    description: 'Analyze patterns and trends',
  },

  // Memory domain → search skill
  memory: {
    skill: 'search',
    mcpTool: 'brain_search',
    description: 'Search CYNIC memory and knowledge',
  },

  // Visualization domain → status skill
  visualization: {
    skill: 'status',
    mcpTool: 'brain_health',
    description: 'Show system status and health',
  },

  // Exploration domain → search skill
  exploration: {
    skill: 'search',
    mcpTool: 'brain_search',
    description: 'Search and explore codebase',
  },

  // Design domain → patterns skill (for now)
  design: {
    skill: 'patterns',
    mcpTool: 'brain_patterns',
    description: 'Analyze design patterns',
  },

  // Cleanup domain → no auto-skill (human decision)
  cleanup: {
    skill: null,
    mcpTool: null,
    description: 'Cleanup requires human decision',
  },

  // Deployment domain → ecosystem skill
  deployment: {
    skill: 'ecosystem',
    mcpTool: 'brain_ecosystem_monitor',
    description: 'Monitor ecosystem and deployments',
  },

  // Mapping domain → ecosystem skill
  mapping: {
    skill: 'ecosystem',
    mcpTool: 'brain_ecosystem_monitor',
    description: 'Map codebase structure',
  },
};

/**
 * Skill invoker interface
 *
 * Skills can be invoked via:
 * 1. MCP tool call (brain_*)
 * 2. Direct skill function
 * 3. CLI command
 */
export class SkillRegistry {
  /**
   * Create the skill registry
   *
   * @param {Object} options - Options
   * @param {Object} [options.mcpClient] - MCP client for tool calls
   * @param {Object} [options.customMappings] - Custom skill mappings
   */
  constructor(options = {}) {
    this.mcpClient = options.mcpClient || null;

    // Merge default and custom mappings
    this.mappings = {
      ...DEFAULT_SKILL_MAPPINGS,
      ...options.customMappings,
    };

    // Registered skill handlers (for direct invocation)
    this._handlers = new Map();

    // Statistics
    this.stats = {
      invocations: 0,
      successes: 0,
      failures: 0,
      bySkill: {},
    };

    log.debug('SkillRegistry created', { mappings: Object.keys(this.mappings).length });
  }

  /**
   * Register a skill handler
   *
   * @param {string} skillName - Skill name
   * @param {Function} handler - Handler function (params) => Promise<result>
   */
  register(skillName, handler) {
    this._handlers.set(skillName, handler);
    log.debug('Skill registered', { skillName });
  }

  /**
   * Get skill mapping for a domain
   *
   * @param {string} domain - Domain from KETER routing
   * @returns {Object|null} Skill mapping
   */
  getSkillForDomain(domain) {
    const mapping = this.mappings[domain];
    if (!mapping || !mapping.skill) {
      return null;
    }
    return {
      name: mapping.skill,
      mcpTool: mapping.mcpTool,
      description: mapping.description,
    };
  }

  /**
   * Get MCP tool for a domain
   *
   * @param {string} domain - Domain
   * @returns {string|null} MCP tool name
   */
  getMcpToolForDomain(domain) {
    return this.mappings[domain]?.mcpTool || null;
  }

  /**
   * Invoke a skill
   *
   * @param {string} skillName - Skill to invoke
   * @param {Object} params - Invocation parameters
   * @returns {Promise<Object>} Invocation result
   */
  async invoke(skillName, params = {}) {
    this.stats.invocations++;
    this.stats.bySkill[skillName] = (this.stats.bySkill[skillName] || 0) + 1;
    const startTime = Date.now();

    try {
      // Try direct handler first
      if (this._handlers.has(skillName)) {
        const handler = this._handlers.get(skillName);
        const result = await handler(params);
        this.stats.successes++;
        this._recordToQLearning(skillName, true, Date.now() - startTime, 'handler');
        return { success: true, data: result, method: 'handler' };
      }

      // Try MCP tool
      const mapping = Object.values(this.mappings).find(m => m.skill === skillName);
      if (mapping?.mcpTool && this.mcpClient) {
        const result = await this._invokeMcpTool(mapping.mcpTool, params);
        this.stats.successes++;
        this._recordToQLearning(skillName, true, Date.now() - startTime, 'mcp');
        return { success: true, data: result, method: 'mcp' };
      }

      // No handler or MCP tool available
      log.warn('No handler found for skill', { skillName });
      this._recordToQLearning(skillName, false, Date.now() - startTime, 'none');
      return { success: false, error: `No handler for skill: ${skillName}`, method: 'none' };

    } catch (err) {
      this.stats.failures++;
      log.error('Skill invocation failed', { skillName, error: err.message });
      this._recordToQLearning(skillName, false, Date.now() - startTime, 'error');
      return { success: false, error: err.message, method: 'error' };
    }
  }

  /**
   * Record skill invocation to Q-Learning for weight optimization
   * @private
   */
  _recordToQLearning(skillName, success, latencyMs, method) {
    try {
      const { getQLearningService } = require('./learning-service.js');
      const qlearning = getQLearningService();
      if (qlearning) {
        // Find domain for this skill
        const mapping = Object.entries(this.mappings).find(([, m]) => m.skill === skillName);
        const domain = mapping?.[0] || 'unknown';

        qlearning.recordAction(skillName, {
          domain,
          success,
          latencyMs,
          method,
          source: 'skill_registry',
        });
      }
    } catch (e) {
      // Q-Learning recording is best-effort
    }
  }

  /**
   * Invoke a skill by domain (convenience method)
   *
   * @param {string} domain - Domain
   * @param {Object} params - Parameters
   * @returns {Promise<Object>}
   */
  async invokeByDomain(domain, params = {}) {
    const skill = this.getSkillForDomain(domain);
    if (!skill) {
      return { success: false, error: `No skill for domain: ${domain}` };
    }
    return this.invoke(skill.name, params);
  }

  /**
   * Invoke MCP tool
   * @private
   */
  async _invokeMcpTool(toolName, params) {
    if (!this.mcpClient) {
      throw new Error('No MCP client configured');
    }

    // Build tool-specific params
    const toolParams = this._buildToolParams(toolName, params);

    // Call MCP tool
    const result = await this.mcpClient.callTool(toolName, toolParams);
    return result;
  }

  /**
   * Build tool-specific parameters
   * @private
   */
  _buildToolParams(toolName, params) {
    // Map generic params to tool-specific params
    switch (toolName) {
      case 'brain_cynic_judge':
        return {
          content: params.content,
          itemType: params.itemType || 'text',
          context: params.context,
        };

      case 'brain_search':
        return {
          query: params.content || params.query,
          types: params.types,
          limit: params.limit || 10,
        };

      case 'brain_wisdom':
        return {
          query: params.content || params.query,
          domains: params.domains,
        };

      case 'brain_patterns':
        return {
          action: params.action || 'list',
          limit: params.limit || 20,
        };

      case 'brain_health':
        return {
          detailed: params.detailed !== false,
        };

      case 'brain_ecosystem_monitor':
        return {
          action: params.action || 'status',
        };

      default:
        return params;
    }
  }

  /**
   * List all registered skills
   *
   * @returns {Object[]} List of skills
   */
  listSkills() {
    const skills = [];

    for (const [domain, mapping] of Object.entries(this.mappings)) {
      if (mapping.skill) {
        skills.push({
          domain,
          skill: mapping.skill,
          mcpTool: mapping.mcpTool,
          description: mapping.description,
          hasHandler: this._handlers.has(mapping.skill),
        });
      }
    }

    return skills;
  }

  /**
   * Add or update a skill mapping
   *
   * @param {string} domain - Domain
   * @param {Object} mapping - Skill mapping
   */
  addMapping(domain, mapping) {
    this.mappings[domain] = mapping;
    log.debug('Mapping added', { domain, skill: mapping.skill });
  }

  /**
   * Get statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    return { ...this.stats };
  }
}

/**
 * Create a SkillRegistry instance
 *
 * @param {Object} options - Options
 * @returns {SkillRegistry}
 */
export function createSkillRegistry(options) {
  return new SkillRegistry(options);
}

export default SkillRegistry;
