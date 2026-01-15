/**
 * CYNIC Agents - The Four Dogs
 *
 * "φ distrusts φ" - κυνικός
 *
 * The pack that guards your AI interactions:
 * - Observer: Silent watcher (PostToolUse, silent)
 * - Digester: Archivist (PostConversation, non-blocking)
 * - Guardian: Watchdog (PreToolUse, blocking)
 * - Mentor: Wise elder (ContextAware, non-blocking)
 *
 * @module @cynic/node/agents
 */

'use strict';

// Base agent
export {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from './base.js';

// The Four Dogs
export { Observer, PatternType } from './observer.js';
export { Digester, KnowledgeType, DigestQuality } from './digester.js';
export { Guardian, RiskLevel, RiskCategory } from './guardian.js';
export { Mentor, WisdomType, ContextSignal } from './mentor.js';

// Convenience imports for default exports
import Observer from './observer.js';
import Digester from './digester.js';
import Guardian from './guardian.js';
import Mentor from './mentor.js';

/**
 * Create all four agents with shared options
 * @param {Object} [options] - Shared options for all agents
 * @returns {Object} Object with all four agents
 */
export function createAgentPack(options = {}) {
  return {
    observer: new Observer(options),
    digester: new Digester(options),
    guardian: new Guardian(options),
    mentor: new Mentor(options),
  };
}

/**
 * Agent manager - coordinates all agents
 */
export class AgentManager {
  /**
   * Create agent manager
   * @param {Object} [options] - Manager options
   */
  constructor(options = {}) {
    this.agents = options.agents || createAgentPack(options);
    this.enabled = true;
    this.stats = {
      eventsProcessed: 0,
      agentInvocations: 0,
      blocks: 0,
      warnings: 0,
    };
  }

  /**
   * Process event through appropriate agents
   * @param {Object} event - Event to process
   * @param {Object} [context] - Event context
   * @returns {Promise<Object>} Combined results
   */
  async process(event, context = {}) {
    if (!this.enabled) {
      return { skipped: true, reason: 'Manager disabled' };
    }

    this.stats.eventsProcessed++;
    const results = {};
    const type = event.type || 'unknown';

    // Route to appropriate agents based on event type
    const agentsToRun = this._selectAgents(type, event);

    for (const [name, agent] of Object.entries(agentsToRun)) {
      if (agent.shouldTrigger(event)) {
        this.stats.agentInvocations++;
        results[name] = await agent.process(event, context);

        // Track blocks and warnings
        if (results[name].response === 'block') {
          this.stats.blocks++;
        } else if (results[name].response === 'warn') {
          this.stats.warnings++;
        }

        // If any agent blocks, stop processing
        if (results[name].response === 'block') {
          results._blocked = true;
          results._blockedBy = name;
          break;
        }
      }
    }

    return results;
  }

  /**
   * Select agents based on event type
   * @private
   */
  _selectAgents(type, event) {
    const agents = {};

    // PreToolUse -> Guardian first
    if (type === 'PreToolUse' || type === 'pre_tool_use') {
      agents.guardian = this.agents.guardian;
    }

    // PostToolUse -> Observer
    if (type === 'PostToolUse' || type === 'post_tool_use' || event.tool) {
      agents.observer = this.agents.observer;
    }

    // PostConversation -> Digester
    if (type === 'PostConversation' || type === 'conversation_end') {
      agents.digester = this.agents.digester;
    }

    // Context updates -> Mentor
    if (type === 'ContextAware' || type === 'context_update' || type === 'message') {
      agents.mentor = this.agents.mentor;
    }

    // If no specific match, use all
    if (Object.keys(agents).length === 0) {
      return this.agents;
    }

    return agents;
  }

  /**
   * Get specific agent
   * @param {string} name - Agent name (observer, digester, guardian, mentor)
   * @returns {BaseAgent} Agent instance
   */
  getAgent(name) {
    return this.agents[name.toLowerCase()];
  }

  /**
   * Enable/disable processing
   * @param {boolean} enabled - Enable state
   */
  setEnabled(enabled) {
    this.enabled = enabled;
  }

  /**
   * Get combined summary from all agents
   * @returns {Object} Combined summary
   */
  getSummary() {
    return {
      enabled: this.enabled,
      stats: this.stats,
      agents: {
        observer: this.agents.observer.getSummary(),
        digester: this.agents.digester.getSummary(),
        guardian: this.agents.guardian.getSummary(),
        mentor: this.agents.mentor.getSummary(),
      },
    };
  }

  /**
   * Reset all agents
   */
  clear() {
    this.agents.observer.clear();
    this.agents.digester.digests = [];
    this.agents.guardian.clear();
    this.agents.mentor.clear();
    this.stats = {
      eventsProcessed: 0,
      agentInvocations: 0,
      blocks: 0,
      warnings: 0,
    };
  }
}

// Default exports
export default {
  Observer,
  Digester,
  Guardian,
  Mentor,
  AgentManager,
  createAgentPack,
};
