/**
 * CYNIC Agents
 *
 * "φ distrusts φ" - κυνικός
 *
 * Two packs available:
 *
 * LEGACY: The Four Dogs (v1)
 * - Observer: Silent watcher (PostToolUse, silent)
 * - Digester: Archivist (PostConversation, non-blocking)
 * - Guardian: Watchdog (PreToolUse, blocking)
 * - Mentor: Wise elder (ContextAware, non-blocking)
 *
 * COLLECTIVE: The Five Dogs (v2) - Fib(5)
 * - Guardian (Gevurah): Enhanced watchdog with learning
 * - Analyst (Binah): Observer + Auditor, profile signals
 * - Scholar (Daat): Librarian + Digester, privacy-aware
 * - Architect (Chesed): Design review, consensus
 * - Sage (Chochmah): Mentor + Guide, adaptive teaching
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

// Event system
export {
  EVENT_CONSTANTS,
  AgentEvent,
  EventPriority,
  AgentId,
  ConsensusVote,
  AgentEventMessage,
  PatternDetectedEvent,
  AnomalyDetectedEvent,
  ThreatBlockedEvent,
  KnowledgeExtractedEvent,
  WisdomSharedEvent,
  ConsensusRequestEvent,
  ConsensusResponseEvent,
  ProfileUpdatedEvent,
  // CYNIC events
  CynicDecisionEvent,
  CynicOverrideEvent,
  CynicGuidanceEvent,
  CynicAwakeningEvent,
  CynicIntrospectionEvent,
} from './events.js';

// Event bus
export {
  AgentEventBus,
} from './event-bus.js';

// Legacy Four Dogs (v1) - Keep original names for backward compatibility
export { Observer, PatternType } from './observer.js';
export { Digester, DigestQuality } from './digester.js';
export { Guardian } from './guardian.js';
export { Mentor, ContextSignal } from './mentor.js';

// Legacy types (aliased for v2 compatibility)
export {
  KnowledgeType as LegacyKnowledgeType,
} from './digester.js';
export {
  RiskLevel as LegacyRiskLevel,
  RiskCategory as LegacyRiskCategory,
} from './guardian.js';
export {
  WisdomType as LegacyWisdomType,
} from './mentor.js';

// Collective Dogs + CYNIC (v2)
export {
  CollectivePack,
  createCollectivePack,
  // The Five Dogs
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  // The Hidden Dog (Keter)
  CollectiveCynic,
  // Additional Dogs (Sefirot)
  CollectiveJanitor,
  CollectiveScout,
  CollectiveCartographer,
  CollectiveOracle,
  CollectiveDeployer,
  // Factory functions
  createGuardian,
  createAnalyst,
  createScholar,
  createArchitect,
  createSage,
  createCynic,
  createJanitor,
  createScout,
  createCartographer,
  createOracle,
  createDeployer,
  // Constants
  COLLECTIVE_CONSTANTS,
  // Types
  RiskLevel,
  RiskCategory,
  PatternCategory,
  AnomalyType,
  KnowledgeType,
  ReviewCategory,
  FeedbackType,
  WisdomType,
  // CYNIC types
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
  // Janitor types
  JANITOR_CONSTANTS,
  QualitySeverity,
  IssueType,
  // Scout types
  SCOUT_CONSTANTS,
  DiscoveryType,
  OpportunityType,
  // Cartographer types
  CARTOGRAPHER_CONSTANTS,
  RepoType,
  ConnectionType,
  MapIssueType,
  // Oracle types
  ORACLE_CONSTANTS,
  ViewType,
  MetricType,
  AlertSeverity,
  // Deployer types
  DEPLOYER_CONSTANTS,
  DeploymentState,
  DeployTarget,
  HealthStatus,
} from './collective/index.js';

// Convenience imports for default exports
import Observer from './observer.js';
import Digester from './digester.js';
import Guardian from './guardian.js';
import Mentor from './mentor.js';

// Import collective agents for default export
import {
  CollectivePack,
  createCollectivePack,
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  CollectiveCynic,
  CollectiveJanitor,
  CollectiveScout,
  CollectiveCartographer,
  CollectiveOracle,
  CollectiveDeployer,
} from './collective/index.js';

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

    // DEBUG: Track which agents are selected for this event
    const agentNames = Object.keys(agentsToRun);
    if (agentNames.includes('digester')) {
      // Track that digester was at least selected
      if (!this._debugDigesterSelections) {
        this._debugDigesterSelections = 0;
      }
      this._debugDigesterSelections++;
    }

    for (const [name, agent] of Object.entries(agentsToRun)) {
      try {
        // DEBUG: Log every agent check
        const shouldRun = agent.shouldTrigger(event);

        if (shouldRun) {
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
      } catch (err) {
        // Log any errors during agent processing
        console.error(`[AgentManager] Error processing ${name}:`, err.message);
        results[name] = { error: err.message, response: 'error' };
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
      // DEBUG: Include digester selection count
      _debug: {
        digesterSelections: this._debugDigesterSelections || 0,
      },
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
  // Legacy Four Dogs (v1)
  Observer,
  Digester,
  Guardian,
  Mentor,
  AgentManager,
  createAgentPack,

  // Collective Dogs + CYNIC (v2)
  CollectivePack,
  createCollectivePack,
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  CollectiveCynic, // The Hidden Dog (Keter)
  CollectiveJanitor, // Foundation (Yesod)
  CollectiveScout, // Victory (Netzach)
  CollectiveCartographer, // Kingdom (Malkhut)
  CollectiveOracle, // Beauty (Tiferet)
  CollectiveDeployer, // Splendor (Hod)
};
