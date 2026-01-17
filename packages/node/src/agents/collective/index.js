/**
 * @cynic/node - Collective Agents
 *
 * The Five Dogs (Fib(5)) + CYNIC (Keter): A harmonious collective consciousness.
 *
 * Inspired by Kabbalah's Sefirot, implemented with professional naming:
 *
 *                    ╭─────────────────╮
 *                    │     CYNIC       │  ← The Hidden 6th Dog
 *                    │    (Keter)      │  Meta-consciousness
 *                    │   κυνικός       │  "Loyal to truth"
 *                    ╰────────┬────────╯
 *                             │
 *        ╭────────────────────┼────────────────────╮
 *        │                    │                    │
 *   ╭────▼────╮          ╭────▼────╮          ╭────▼────╮
 *   │  SAGE   │◄────────►│ SCHOLAR │◄────────►│GUARDIAN │
 *   │(Chochmah)│  φ-bus  │ (Daat)  │  φ-bus   │(Gevurah)│
 *   │ Wisdom  │          │Knowledge│          │Strength │
 *   ╰────┬────╯          ╰────┬────╯          ╰────┬────╯
 *        │                    │                    │
 *        ╰────────────────────┼────────────────────╯
 *                             │
 *   ╭─────────────────────────┼─────────────────────────╮
 *   │                         │                         │
 *╭──▼──────╮            ╭─────▼─────╮            ╭──────▼───╮
 *│ ANALYST │◄──────────►│ EVENT BUS │◄──────────►│ARCHITECT │
 *│ (Binah) │            │  φ-aligned │            │ (Chesed) │
 *│Understand│            │  987 events│            │ Kindness │
 *╰─────────╯            ╰───────────╯            ╰──────────╯
 *
 * CYNIC observes ALL events and orchestrates the collective.
 * φ⁻¹ (61.8%) = consensus threshold, max confidence
 * φ⁻² (38.2%) = veto threshold, override threshold
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective
 */

'use strict';

import { PHI_INV } from '@cynic/core';
import { AgentEventBus } from '../event-bus.js';
import { AgentEvent, AgentId, ConsensusVote } from '../events.js';
import { ProfileCalculator, ProfileLevel } from '../../profile/calculator.js';
import { OrganicSignals } from '../../profile/organic-signals.js';
import { LocalStore } from '../../privacy/local-store.js';

// Import collective agents (The Five Dogs)
import { CollectiveGuardian, RiskLevel, RiskCategory } from './guardian.js';
import { CollectiveAnalyst, PatternCategory, AnomalyType } from './analyst.js';
import { CollectiveScholar, KnowledgeType } from './scholar.js';
import { CollectiveArchitect, ReviewCategory, FeedbackType } from './architect.js';
import { CollectiveSage, WisdomType } from './sage.js';

// Import CYNIC - The Hidden Sixth Dog (Keter)
import {
  CollectiveCynic,
  CYNIC_CONSTANTS,
  CynicDecisionType,
  CynicGuidanceType,
  MetaState,
} from './cynic.js';

// Re-export agents
export {
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  CollectiveCynic,
};

// Re-export types
export {
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
};

/**
 * φ-aligned constants for collective
 */
export const COLLECTIVE_CONSTANTS = {
  /** Number of dogs (Fib(5) = 5) */
  DOG_COUNT: 5,

  /** Total agents including CYNIC (Fib(5) + 1 = 6) */
  AGENT_COUNT: 6,

  /** Max collective confidence (φ⁻¹) */
  MAX_CONFIDENCE: PHI_INV,

  /** Consensus threshold (φ⁻¹) */
  CONSENSUS_THRESHOLD: PHI_INV,

  /** Default profile level */
  DEFAULT_PROFILE_LEVEL: ProfileLevel.PRACTITIONER,
};

/**
 * Collective Pack - The Five Dogs working as one
 */
export class CollectivePack {
  /**
   * Create the collective
   * @param {Object} [options] - Options
   * @param {Object} [options.judge] - CYNIC judge instance
   * @param {Object} [options.state] - State manager instance
   * @param {number} [options.profileLevel] - Initial profile level
   * @param {Object} [options.localStore] - Local store for privacy
   */
  constructor(options = {}) {
    // Shared infrastructure
    this.eventBus = new AgentEventBus();
    this.profileCalculator = new ProfileCalculator();
    this.signalCollector = new OrganicSignals();
    this.localStore = options.localStore || null;

    // Current profile level
    this.profileLevel = options.profileLevel || COLLECTIVE_CONSTANTS.DEFAULT_PROFILE_LEVEL;

    // Register all agents with event bus BEFORE creating them
    // (agents subscribe to events in their constructors)
    this.eventBus.registerAgent(AgentId.GUARDIAN);
    this.eventBus.registerAgent(AgentId.ANALYST);
    this.eventBus.registerAgent(AgentId.SCHOLAR);
    this.eventBus.registerAgent(AgentId.ARCHITECT);
    this.eventBus.registerAgent(AgentId.SAGE);
    this.eventBus.registerAgent(AgentId.CYNIC); // The Hidden 6th Dog (Keter)
    this.eventBus.registerAgent('collective'); // For pack-level subscriptions

    // Create agents with shared infrastructure
    this.guardian = new CollectiveGuardian({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
    });

    this.analyst = new CollectiveAnalyst({
      eventBus: this.eventBus,
      profileCalculator: this.profileCalculator,
      signalCollector: this.signalCollector,
      judge: options.judge,
      state: options.state,
    });

    this.scholar = new CollectiveScholar({
      eventBus: this.eventBus,
      localStore: this.localStore,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
    });

    this.architect = new CollectiveArchitect({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
    });

    this.sage = new CollectiveSage({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
    });

    // CYNIC - The Hidden 6th Dog (Keter) - Meta-consciousness
    this.cynic = new CollectiveCynic({
      eventBus: this.eventBus,
      profileLevel: this.profileLevel,
      judge: options.judge,
      state: options.state,
    });

    // Agent map for lookup (5 Dogs + CYNIC)
    this.agents = new Map([
      [AgentId.GUARDIAN, this.guardian],
      [AgentId.ANALYST, this.analyst],
      [AgentId.SCHOLAR, this.scholar],
      [AgentId.ARCHITECT, this.architect],
      [AgentId.SAGE, this.sage],
      [AgentId.CYNIC, this.cynic], // Keter - The Crown
    ]);

    // Stats
    this.collectiveStats = {
      created: Date.now(),
      totalProcessed: 0,
      profileUpdates: 0,
      consensusRequests: 0,
    };

    // Subscribe to profile updates
    this._subscribeToProfileUpdates();
  }

  /**
   * Subscribe to profile updates from Analyst
   * @private
   */
  _subscribeToProfileUpdates() {
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      'collective',
      this._handleProfileUpdate.bind(this)
    );
  }

  /**
   * Handle profile update from Analyst
   * @private
   */
  _handleProfileUpdate(event) {
    const { newLevel } = event.payload;

    // Update all agents (5 Dogs + CYNIC)
    this.profileLevel = newLevel;
    this.guardian.setProfileLevel(newLevel);
    this.scholar.setProfileLevel(newLevel);
    this.architect.setProfileLevel(newLevel);
    this.sage.setProfileLevel(newLevel);
    this.cynic.setProfileLevel(newLevel); // CYNIC adapts too

    this.collectiveStats.profileUpdates++;
  }

  /**
   * Get agent by ID
   * @param {string} agentId - Agent ID
   * @returns {Object|null} Agent or null
   */
  getAgent(agentId) {
    return this.agents.get(agentId) || null;
  }

  /**
   * Get all agents
   * @returns {Object[]} Array of agents
   */
  getAllAgents() {
    return Array.from(this.agents.values());
  }

  /**
   * Process event through appropriate agents
   * @param {Object} event - Event to process
   * @param {Object} [context] - Context
   * @returns {Promise<Object[]>} Results from all triggered agents
   */
  async processEvent(event, context = {}) {
    const results = [];
    this.collectiveStats.totalProcessed++;

    for (const [id, agent] of this.agents) {
      if (agent.shouldTrigger(event)) {
        try {
          const result = await agent.process(event, context);
          results.push({
            agent: id,
            ...result,
          });
        } catch (err) {
          results.push({
            agent: id,
            error: err.message,
          });
        }
      }
    }

    return results;
  }

  /**
   * Check command safety (Guardian)
   * @param {string} command - Command to check
   * @returns {Promise<Object>} Safety assessment
   */
  async checkCommand(command) {
    return this.guardian.checkCommand(command);
  }

  /**
   * Extract knowledge (Scholar)
   * @param {string} content - Content to extract from
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Extraction result
   */
  async extractKnowledge(content, options = {}) {
    return this.scholar.extract(content, options);
  }

  /**
   * Review code (Architect)
   * @param {string} code - Code to review
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Review result
   */
  async reviewCode(code, options = {}) {
    return this.architect.review(code, options);
  }

  /**
   * Get wisdom (Sage)
   * @param {string} topic - Topic
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Wisdom
   */
  async getWisdom(topic, options = {}) {
    return this.sage.shareWisdom(topic, options);
  }

  /**
   * Get current profile
   * @returns {Object} Profile data
   */
  getProfile() {
    return {
      level: this.profileLevel,
      ...this.profileCalculator.getProfile(),
    };
  }

  /**
   * Get collective summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      dogCount: COLLECTIVE_CONSTANTS.DOG_COUNT,
      agentCount: COLLECTIVE_CONSTANTS.AGENT_COUNT,
      profileLevel: this.profileLevel,
      eventBusStats: this.eventBus.getStats(),
      agents: {
        // The Five Dogs
        guardian: this.guardian.getSummary(),
        analyst: this.analyst.getSummary(),
        scholar: this.scholar.getSummary(),
        architect: this.architect.getSummary(),
        sage: this.sage.getSummary(),
        // The Hidden 6th - CYNIC (Keter)
        cynic: this.cynic.getSummary(),
      },
      collectiveState: this.cynic.getCollectiveState(),
      collectiveStats: this.collectiveStats,
    };
  }

  /**
   * Awaken CYNIC for a new session
   * @param {Object} sessionInfo - Session information
   * @returns {Promise<Object>} Awakening result
   */
  async awakenCynic(sessionInfo = {}) {
    return this.cynic.awaken(sessionInfo);
  }

  /**
   * Get CYNIC's view of the collective state
   * @returns {Object} Collective state
   */
  getCollectiveState() {
    return this.cynic.getCollectiveState();
  }

  /**
   * Issue guidance from CYNIC to the collective
   * @param {Object} guidance - Guidance to issue
   * @returns {Promise<Object>} Result
   */
  async issueGuidance(guidance) {
    return this.cynic.issueGuidance(guidance);
  }

  /**
   * Request introspection from CYNIC
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Result
   */
  async introspect(options = {}) {
    return this.cynic.introspect(options);
  }

  /**
   * Get event bus
   * @returns {Object} Event bus
   */
  getEventBus() {
    return this.eventBus;
  }

  /**
   * Clear all agent data
   */
  clear() {
    this.guardian.clear();
    this.analyst.clear();
    this.scholar.clear();
    this.architect.clear();
    this.sage.clear();
    this.cynic.clear();
    this.eventBus.reset();
  }

  /**
   * Shutdown collective
   */
  async shutdown() {
    await this.eventBus.shutdown();
    this.clear();
  }
}

/**
 * Create a collective pack with all Five Dogs
 * @param {Object} [options] - Options
 * @returns {CollectivePack} Collective pack
 */
export function createCollectivePack(options = {}) {
  return new CollectivePack(options);
}

/**
 * Create individual agents (for custom configurations)
 */
export function createGuardian(options = {}) {
  return new CollectiveGuardian(options);
}

export function createAnalyst(options = {}) {
  return new CollectiveAnalyst(options);
}

export function createScholar(options = {}) {
  return new CollectiveScholar(options);
}

export function createArchitect(options = {}) {
  return new CollectiveArchitect(options);
}

export function createSage(options = {}) {
  return new CollectiveSage(options);
}

export function createCynic(options = {}) {
  return new CollectiveCynic(options);
}

export default {
  CollectivePack,
  createCollectivePack,
  // The Five Dogs
  CollectiveGuardian,
  CollectiveAnalyst,
  CollectiveScholar,
  CollectiveArchitect,
  CollectiveSage,
  // The Hidden 6th Dog (Keter)
  CollectiveCynic,
  // Factory functions
  createGuardian,
  createAnalyst,
  createScholar,
  createArchitect,
  createSage,
  createCynic,
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
};
