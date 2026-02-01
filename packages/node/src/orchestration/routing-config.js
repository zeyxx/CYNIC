/**
 * KETER Routing Configuration
 *
 * Centralized configuration for Sefirot routing.
 * Used by UnifiedOrchestrator and MCP orchestration tools.
 *
 * "φ routes to the right Sefirah" - κυνικός
 *
 * @module @cynic/node/orchestration/routing-config
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// SEFIROT ROUTING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Sefirot routing configuration
 *
 * Maps domains to sefirot, agents, tools, and triggers.
 * This is the canonical source of truth for KETER routing.
 */
export const SEFIROT_ROUTING = {
  // Chochmah (Sage) - Wisdom queries
  wisdom: {
    sefirah: 'Chochmah',
    agent: 'cynic-sage',
    tools: ['brain_search', 'brain_wisdom'],
    triggers: ['wisdom', 'philosophy', 'why', 'meaning', 'understanding'],
  },

  // Binah (Architect) - Design and planning
  design: {
    sefirah: 'Binah',
    agent: 'cynic-architect',
    tools: ['brain_patterns'],
    triggers: ['design', 'architect', 'plan', 'structure', 'refactor'],
  },

  // Daat (Archivist) - Memory and knowledge
  memory: {
    sefirah: 'Daat',
    agent: 'cynic-archivist',
    tools: ['brain_memory_search', 'brain_memory_store', 'brain_learning', 'brain_patterns'],
    triggers: ['remember', 'learn', 'recall', 'history', 'past', 'before', 'already', 'déjà', 'souviens'],
  },

  // Chesed (Analyst) - Analysis and patterns
  analysis: {
    sefirah: 'Chesed',
    agent: 'cynic-analyst',
    tools: ['brain_patterns', 'brain_get_observations', 'brain_emergence'],
    triggers: ['analyze', 'pattern', 'trend', 'insight', 'data'],
  },

  // Gevurah (Guardian) - Security and protection
  protection: {
    sefirah: 'Gevurah',
    agent: 'cynic-guardian',
    tools: ['brain_cynic_judge'],
    triggers: ['danger', 'secure', 'verify', 'protect', 'risk', 'delete', 'rm'],
  },

  // Tiferet (Oracle) - Visualization and balance
  visualization: {
    sefirah: 'Tiferet',
    agent: 'cynic-oracle',
    tools: ['brain_health', 'brain_collective_status', 'brain_render'],
    triggers: ['dashboard', 'visualize', 'show', 'display', 'status'],
  },

  // Netzach (Scout) - Exploration and search
  exploration: {
    sefirah: 'Netzach',
    agent: 'cynic-scout',
    tools: ['brain_search', 'brain_codebase', 'brain_code_analyze', 'brain_code_deps'],
    triggers: ['find', 'search', 'explore', 'where', 'locate'],
  },

  // Yesod (Simplifier/Janitor) - Cleanup and simplification
  cleanup: {
    sefirah: 'Yesod',
    agent: 'cynic-simplifier',
    tools: [],
    triggers: ['simplify', 'clean', 'reduce', 'remove', 'prune'],
  },

  // Hod (Deployer) - Deployment and CI/CD
  deployment: {
    sefirah: 'Hod',
    agent: 'cynic-deployer',
    tools: ['brain_render', 'brain_ecosystem_monitor'],
    triggers: ['deploy', 'release', 'build', 'ci', 'cd', 'infrastructure'],
  },

  // Malkhut (Cartographer) - Mapping and overview
  mapping: {
    sefirah: 'Malkhut',
    agent: 'cynic-cartographer',
    tools: ['brain_codebase', 'brain_ecosystem', 'brain_ecosystem_monitor'],
    triggers: ['map', 'overview', 'codebase', 'structure', 'repos'],
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// TRUST LEVELS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Trust level thresholds (based on E-Score)
 */
export const TRUST_THRESHOLDS = {
  GUARDIAN: PHI_INV * 100,  // 61.8% - Full trust
  STEWARD: PHI_INV_2 * 100, // 38.2% - High trust
  BUILDER: 30,               // Medium trust
  CONTRIBUTOR: 15,           // Low trust
  OBSERVER: 0,               // No trust
};

/**
 * Calculate trust level from E-Score
 *
 * @param {number} eScore - E-Score (0-100)
 * @returns {string} Trust level
 */
export function calculateTrustLevel(eScore) {
  if (eScore >= TRUST_THRESHOLDS.GUARDIAN) return 'GUARDIAN';
  if (eScore >= TRUST_THRESHOLDS.STEWARD) return 'STEWARD';
  if (eScore >= TRUST_THRESHOLDS.BUILDER) return 'BUILDER';
  if (eScore >= TRUST_THRESHOLDS.CONTRIBUTOR) return 'CONTRIBUTOR';
  return 'OBSERVER';
}

// ═══════════════════════════════════════════════════════════════════════════
// RISK INTERVENTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Risk levels and their intervention mappings
 *
 * Matrix: [trustLevel][riskLevel] → intervention
 */
export const RISK_INTERVENTIONS = {
  critical: {
    OBSERVER: 'block',
    CONTRIBUTOR: 'block',
    BUILDER: 'ask',
    STEWARD: 'ask',
    GUARDIAN: 'notify',
  },
  high: {
    OBSERVER: 'ask',
    CONTRIBUTOR: 'ask',
    BUILDER: 'ask',
    STEWARD: 'notify',
    GUARDIAN: 'silent',
  },
  medium: {
    OBSERVER: 'ask',
    CONTRIBUTOR: 'notify',
    BUILDER: 'silent',
    STEWARD: 'silent',
    GUARDIAN: 'silent',
  },
  low: {
    OBSERVER: 'notify',
    CONTRIBUTOR: 'silent',
    BUILDER: 'silent',
    STEWARD: 'silent',
    GUARDIAN: 'silent',
  },
};

/**
 * Determine intervention level based on trust and risk
 *
 * @param {string} trustLevel - User's trust level
 * @param {string} riskLevel - Detected risk level
 * @returns {string} Intervention level
 */
export function determineIntervention(trustLevel, riskLevel) {
  const matrix = RISK_INTERVENTIONS[riskLevel] || RISK_INTERVENTIONS.low;
  return matrix[trustLevel] || 'silent';
}

// ═══════════════════════════════════════════════════════════════════════════
// DANGER PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Dangerous command patterns (regex)
 */
export const DANGER_PATTERNS = [
  /rm\s+-rf\s+[\/~]/i,
  /drop\s+table/i,
  /delete\s+from\s+\w+\s*;?\s*$/i,
  /format\s+[a-z]:/i,
  /mkfs/i,
  />>\s*\/etc\//i,
  /sudo\s+chmod\s+777/i,
  /curl.*\|\s*bash/i,
  /wget.*\|\s*sh/i,
];

/**
 * Detect risk level from content
 *
 * @param {string} content - Content to analyze
 * @returns {string} Risk level: 'critical' | 'high' | 'medium' | 'low'
 */
export function detectRisk(content) {
  const lower = content.toLowerCase();

  // Critical patterns
  for (const pattern of DANGER_PATTERNS) {
    if (pattern.test(content)) {
      return 'critical';
    }
  }

  // High risk patterns
  if (/delete|remove|destroy|overwrite|force/.test(lower)) {
    return 'high';
  }

  // Medium risk patterns
  if (/modify|change|update|edit|write/.test(lower)) {
    return 'medium';
  }

  return 'low';
}

// ═══════════════════════════════════════════════════════════════════════════
// ROUTING HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Find matching routing for content
 *
 * @param {string} content - Content to route
 * @returns {Object|null} Matching routing or null
 */
export function findRouting(content) {
  const lower = content.toLowerCase();

  for (const [domain, routing] of Object.entries(SEFIROT_ROUTING)) {
    for (const trigger of routing.triggers) {
      if (lower.includes(trigger)) {
        return { domain, ...routing };
      }
    }
  }

  return null;
}

/**
 * Get all available domains
 * @returns {string[]}
 */
export function getDomains() {
  return Object.keys(SEFIROT_ROUTING);
}

/**
 * Get routing for a specific domain
 *
 * @param {string} domain - Domain name
 * @returns {Object|null}
 */
export function getRoutingForDomain(domain) {
  return SEFIROT_ROUTING[domain] || null;
}

export default {
  SEFIROT_ROUTING,
  TRUST_THRESHOLDS,
  RISK_INTERVENTIONS,
  DANGER_PATTERNS,
  calculateTrustLevel,
  determineIntervention,
  detectRisk,
  findRouting,
  getDomains,
  getRoutingForDomain,
};
