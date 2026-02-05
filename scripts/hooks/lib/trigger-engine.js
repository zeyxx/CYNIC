/**
 * Trigger Engine - Proactive Suggestion System (TPS)
 *
 * "Le chien anticipe" - CYNIC becomes proactive, not just reactive
 *
 * Based on Design #18: Proactivité CYNIC
 * 6 trigger types with φ-aligned cooldowns and Dog voting.
 *
 * @module scripts/hooks/lib/trigger-engine
 */

'use strict';

import { antiPatternState } from './anti-pattern-detector.js';

// ═══════════════════════════════════════════════════════════════════════════
// φ CONSTANTS - Golden ratio alignment
// ═══════════════════════════════════════════════════════════════════════════

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895; // 61.8% - max confidence threshold
const PHI_INV_SQ = 0.381966011250105; // 38.2% - burnout threshold

// ═══════════════════════════════════════════════════════════════════════════
// TRIGGER DEFINITIONS - From Design #18
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Trigger type definitions with conditions, actions, and cooldowns
 */
export const TRIGGER_TYPES = {
  ERROR_PATTERN: {
    name: 'ERROR_PATTERN',
    description: 'Same error type 3+ times → suggest fix',
    action: 'suggest_fix',
    urgency: 'ACTIVE',
    cooldown: 5 * 60 * 1000, // 5 minutes
    threshold: 3, // 3 consecutive errors
  },
  CONTEXT_DRIFT: {
    name: 'CONTEXT_DRIFT',
    description: 'User strays from active goal → remind',
    action: 'remind_goal',
    urgency: 'SUBTLE',
    cooldown: 10 * 60 * 1000, // 10 minutes
    threshold: 0.5, // 50% drift score
  },
  BURNOUT_RISK: {
    name: 'BURNOUT_RISK',
    description: 'Energy < 38.2% (φ⁻²) → suggest break',
    action: 'suggest_break',
    urgency: 'ACTIVE',
    cooldown: 30 * 60 * 1000, // 30 minutes
    threshold: PHI_INV_SQ, // 38.2%
  },
  PATTERN_MATCH: {
    name: 'PATTERN_MATCH',
    description: 'Similar past success found → suggest reuse',
    action: 'suggest_reuse',
    urgency: 'SUBTLE',
    cooldown: 2 * 60 * 1000, // 2 minutes
    threshold: PHI_INV, // 61.8% confidence
  },
  DEADLINE_NEAR: {
    name: 'DEADLINE_NEAR',
    description: 'Goal deadline approaching → prioritize',
    action: 'prioritize_goal',
    urgency: 'ACTIVE',
    cooldown: 15 * 60 * 1000, // 15 minutes
    threshold: 24 * 60 * 60 * 1000, // 24 hours
  },
  LEARNING_OPP: {
    name: 'LEARNING_OPP',
    description: 'New pattern emerges → highlight for learning',
    action: 'highlight_pattern',
    urgency: 'SUBTLE',
    cooldown: 5 * 60 * 1000, // 5 minutes
    threshold: 3, // 3 occurrences before surfacing
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// TRIGGER ENGINE STATE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Engine state - tracks cooldowns, outcomes, and context
 */
const engineState = {
  // Cooldown tracking: { [triggerType]: lastFiredTimestamp }
  cooldowns: {},

  // Suggestion outcomes: { [suggestionId]: { accepted: bool, timestamp, trigger } }
  outcomes: [],
  maxOutcomes: 100,

  // Context for condition evaluation
  context: {
    userEnergy: 1.0,
    userFocus: 1.0,
    currentFocus: null,
    activeGoal: null,
    goals: [],
    consecutiveErrors: 0,
    lastErrorType: null,
    similarPatternFound: null,
    patternConfidence: 0,
    emergingPatterns: [],
  },

  // Suggestion counter for IDs
  suggestionCounter: 0,

  // Pending suggestions awaiting outcome
  pendingSuggestions: [],
};

// ═══════════════════════════════════════════════════════════════════════════
// CONDITION EVALUATORS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Evaluate ERROR_PATTERN condition
 * Uses antiPatternState.recentErrors from anti-pattern-detector
 */
function evaluateErrorPattern(ctx) {
  // Check antiPatternState for error loop
  const recentErrors = antiPatternState.recentErrors || [];
  const now = Date.now();
  const windowMs = antiPatternState.errorWindowMs || 5 * 60 * 1000;

  // Filter to recent errors
  const windowErrors = recentErrors.filter(e => now - e.timestamp < windowMs);

  if (windowErrors.length < TRIGGER_TYPES.ERROR_PATTERN.threshold) {
    return { shouldFire: false };
  }

  // Group by error type
  const byType = {};
  for (const err of windowErrors) {
    byType[err.type] = (byType[err.type] || 0) + 1;
  }

  // Find the most frequent error type
  let maxType = null;
  let maxCount = 0;
  for (const [type, count] of Object.entries(byType)) {
    if (count > maxCount) {
      maxType = type;
      maxCount = count;
    }
  }

  if (maxCount >= TRIGGER_TYPES.ERROR_PATTERN.threshold) {
    return {
      shouldFire: true,
      data: {
        errorType: maxType,
        count: maxCount,
        files: [...new Set(windowErrors.filter(e => e.type === maxType).map(e => e.file))],
      },
    };
  }

  return { shouldFire: false };
}

/**
 * Evaluate CONTEXT_DRIFT condition
 * Checks if current focus differs significantly from active goal
 */
function evaluateContextDrift(ctx) {
  const { currentFocus, activeGoal } = ctx;

  if (!activeGoal || !currentFocus) {
    return { shouldFire: false };
  }

  // Simple check: are we working on something unrelated to the goal?
  const goalFocus = activeGoal.focus || activeGoal.title;
  if (!goalFocus) {
    return { shouldFire: false };
  }

  // Basic string similarity check (could be enhanced with embeddings)
  const goalWords = new Set(goalFocus.toLowerCase().split(/\s+/));
  const currentWords = currentFocus.toLowerCase().split(/\s+/);
  const overlap = currentWords.filter(w => goalWords.has(w)).length;
  const driftScore = 1 - (overlap / Math.max(goalWords.size, 1));

  if (driftScore > TRIGGER_TYPES.CONTEXT_DRIFT.threshold) {
    return {
      shouldFire: true,
      data: {
        goalTitle: activeGoal.title,
        currentFocus,
        driftScore: Math.round(driftScore * 100),
      },
    };
  }

  return { shouldFire: false };
}

/**
 * Evaluate BURNOUT_RISK condition
 * Checks if user energy is below φ⁻² (38.2%)
 */
function evaluateBurnoutRisk(ctx) {
  const { userEnergy } = ctx;

  if (typeof userEnergy !== 'number') {
    return { shouldFire: false };
  }

  if (userEnergy < TRIGGER_TYPES.BURNOUT_RISK.threshold) {
    return {
      shouldFire: true,
      data: {
        energy: Math.round(userEnergy * 100),
        threshold: Math.round(PHI_INV_SQ * 100),
        hoursActive: ctx.sessionDuration ? Math.round(ctx.sessionDuration / 3600000) : null,
      },
    };
  }

  return { shouldFire: false };
}

/**
 * Evaluate PATTERN_MATCH condition
 * Checks if a similar successful pattern was found
 */
function evaluatePatternMatch(ctx) {
  const { similarPatternFound, patternConfidence } = ctx;

  if (!similarPatternFound) {
    return { shouldFire: false };
  }

  if (patternConfidence >= TRIGGER_TYPES.PATTERN_MATCH.threshold) {
    return {
      shouldFire: true,
      data: {
        pattern: similarPatternFound,
        confidence: Math.round(patternConfidence * 100),
      },
    };
  }

  return { shouldFire: false };
}

/**
 * Evaluate DEADLINE_NEAR condition
 * Checks if any goal deadline is within threshold
 */
function evaluateDeadlineNear(ctx) {
  const { goals } = ctx;

  if (!goals || goals.length === 0) {
    return { shouldFire: false };
  }

  const now = Date.now();
  const threshold = TRIGGER_TYPES.DEADLINE_NEAR.threshold;

  for (const goal of goals) {
    if (!goal.deadline) continue;

    const deadlineMs = typeof goal.deadline === 'number' ?
      goal.deadline : new Date(goal.deadline).getTime();

    const timeUntil = deadlineMs - now;

    if (timeUntil > 0 && timeUntil <= threshold) {
      const hoursUntil = Math.round(timeUntil / 3600000);
      return {
        shouldFire: true,
        data: {
          goalTitle: goal.title,
          hoursUntil,
          deadline: new Date(deadlineMs).toISOString(),
        },
      };
    }
  }

  return { shouldFire: false };
}

/**
 * Evaluate LEARNING_OPP condition
 * Checks if a new pattern is emerging
 */
function evaluateLearningOpp(ctx) {
  const { emergingPatterns } = ctx;

  if (!emergingPatterns || emergingPatterns.length === 0) {
    return { shouldFire: false };
  }

  // Find patterns that have been seen threshold times but not yet surfaced
  for (const pattern of emergingPatterns) {
    if (pattern.count >= TRIGGER_TYPES.LEARNING_OPP.threshold && !pattern.surfaced) {
      return {
        shouldFire: true,
        data: {
          patternName: pattern.name,
          count: pattern.count,
          description: pattern.description,
        },
      };
    }
  }

  return { shouldFire: false };
}

/**
 * Condition evaluators map
 */
const CONDITION_EVALUATORS = {
  ERROR_PATTERN: evaluateErrorPattern,
  CONTEXT_DRIFT: evaluateContextDrift,
  BURNOUT_RISK: evaluateBurnoutRisk,
  PATTERN_MATCH: evaluatePatternMatch,
  DEADLINE_NEAR: evaluateDeadlineNear,
  LEARNING_OPP: evaluateLearningOpp,
};

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION FORMATTERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Format suggestion based on urgency level
 */
const SUGGESTION_TEMPLATES = {
  // ERROR_PATTERN suggestions
  suggest_fix: {
    SUBTLE: (data) => `*sniff* "${data.errorType}" répété ${data.count}x. Peut-être essayer une autre approche?`,
    ACTIVE: (data) => `*ears perk* Tu tournes en rond avec "${data.errorType}" (${data.count}x). Suggestion: prendre du recul et revoir la stratégie.`,
    URGENT: (data) => `*GROWL* Error loop détecté! "${data.errorType}" ${data.count}x. Stop et réfléchis.`,
  },

  // CONTEXT_DRIFT suggestions
  remind_goal: {
    SUBTLE: (data) => `*sniff* Objectif actif: "${data.goalTitle}". Tu travailles sur: ${data.currentFocus}`,
    ACTIVE: (data) => `*ears perk* Drift détecté (${data.driftScore}%). Rappel: ton objectif est "${data.goalTitle}"`,
    URGENT: (data) => `*GROWL* Tu t'éloignes significativement de l'objectif "${data.goalTitle}"`,
  },

  // BURNOUT_RISK suggestions
  suggest_break: {
    SUBTLE: (data) => `*yawn* Énergie à ${data.energy}%. Une pause?`,
    ACTIVE: (data) => `*ears perk* Énergie basse (${data.energy}% < ${data.threshold}%). Le chien conseille une pause.`,
    URGENT: (data) => `*GROWL* Burnout risk! Énergie critique: ${data.energy}%. Pause MAINTENANT.`,
  },

  // PATTERN_MATCH suggestions
  suggest_reuse: {
    SUBTLE: (data) => `*sniff* Pattern similaire détecté: "${data.pattern}" (${data.confidence}% match)`,
    ACTIVE: (data) => `*tail wag* J'ai vu ce pattern avant: "${data.pattern}" (${data.confidence}% confiance). Peut-être réutiliser?`,
    URGENT: (data) => `*ears perk* Pattern fortement similaire: "${data.pattern}" (${data.confidence}%). Recommande de réutiliser.`,
  },

  // DEADLINE_NEAR suggestions
  prioritize_goal: {
    SUBTLE: (data) => `*sniff* Deadline: "${data.goalTitle}" dans ${data.hoursUntil}h`,
    ACTIVE: (data) => `*ears perk* Deadline approche! "${data.goalTitle}" dans ${data.hoursUntil}h. Prioriser?`,
    URGENT: (data) => `*GROWL* DEADLINE CRITIQUE! "${data.goalTitle}" dans ${data.hoursUntil}h!`,
  },

  // LEARNING_OPP suggestions
  highlight_pattern: {
    SUBTLE: (data) => `*sniff* Nouveau pattern émergent: "${data.patternName}" (${data.count}x)`,
    ACTIVE: (data) => `*ears perk* Pattern détecté: "${data.patternName}" - ${data.description || 'vu ' + data.count + ' fois'}`,
    URGENT: (data) => `*tail wag* Apprentissage: "${data.patternName}" est maintenant un pattern établi.`,
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// TRIGGER ENGINE CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * TriggerEngine - Evaluates triggers and generates proactive suggestions
 */
export class TriggerEngine {
  constructor(options = {}) {
    this.enabled = options.enabled !== false;
    this.votingThreshold = options.votingThreshold || PHI_INV; // 61.8%
    this.getCollectivePack = options.getCollectivePack || null;
  }

  /**
   * Update context for condition evaluation
   *
   * @param {Object} updates - Context updates
   */
  updateContext(updates) {
    Object.assign(engineState.context, updates);
  }

  /**
   * Get current context
   *
   * @returns {Object} Current context
   */
  getContext() {
    return { ...engineState.context };
  }

  /**
   * Check if a trigger is on cooldown
   *
   * @param {string} triggerType - Type of trigger
   * @returns {boolean} True if on cooldown
   */
  isOnCooldown(triggerType) {
    const lastFired = engineState.cooldowns[triggerType];
    if (!lastFired) return false;

    const trigger = TRIGGER_TYPES[triggerType];
    if (!trigger) return false;

    return (Date.now() - lastFired) < trigger.cooldown;
  }

  /**
   * Record that a trigger fired
   *
   * @param {string} triggerType - Type of trigger
   */
  recordFire(triggerType) {
    engineState.cooldowns[triggerType] = Date.now();
  }

  /**
   * Evaluate a single trigger
   *
   * @param {string} triggerType - Type of trigger to evaluate
   * @returns {Object|null} Suggestion if trigger fires, null otherwise
   */
  evaluate(triggerType) {
    if (!this.enabled) return null;

    const trigger = TRIGGER_TYPES[triggerType];
    if (!trigger) return null;

    // Check cooldown
    if (this.isOnCooldown(triggerType)) {
      return null;
    }

    // Evaluate condition
    const evaluator = CONDITION_EVALUATORS[triggerType];
    if (!evaluator) return null;

    const result = evaluator(engineState.context);
    if (!result.shouldFire) return null;

    // Generate suggestion
    const suggestionId = `sug_${++engineState.suggestionCounter}_${Date.now()}`;
    const templates = SUGGESTION_TEMPLATES[trigger.action];
    const formatter = templates?.[trigger.urgency] || templates?.SUBTLE;

    const message = formatter ? formatter(result.data) : `Trigger: ${triggerType}`;

    const suggestion = {
      id: suggestionId,
      trigger: triggerType,
      action: trigger.action,
      urgency: trigger.urgency,
      message,
      data: result.data,
      timestamp: Date.now(),
    };

    // Record fire and add to pending
    this.recordFire(triggerType);
    engineState.pendingSuggestions.push(suggestion);

    return suggestion;
  }

  /**
   * Evaluate all triggers and return any that fire
   *
   * @returns {Object[]} Array of suggestions
   */
  evaluateAll() {
    if (!this.enabled) return [];

    const suggestions = [];
    for (const triggerType of Object.keys(TRIGGER_TYPES)) {
      const suggestion = this.evaluate(triggerType);
      if (suggestion) {
        suggestions.push(suggestion);
      }
    }
    return suggestions;
  }

  /**
   * Have Dogs vote on a suggestion
   * Requires getCollectivePack to be configured
   *
   * @param {Object} suggestion - Suggestion to vote on
   * @returns {Promise<Object>} Vote result with approved boolean
   */
  async voteSuggestion(suggestion) {
    if (!this.getCollectivePack) {
      // No voting configured, auto-approve
      return { approved: true, votes: [], reason: 'no_voting_configured' };
    }

    try {
      const pack = await this.getCollectivePack();
      if (!pack || !pack.vote) {
        return { approved: true, votes: [], reason: 'pack_unavailable' };
      }

      // Call collective vote
      const voteResult = await pack.vote({
        question: `Should we show this suggestion? "${suggestion.message}"`,
        context: {
          trigger: suggestion.trigger,
          urgency: suggestion.urgency,
          data: suggestion.data,
        },
        threshold: this.votingThreshold,
      });

      return {
        approved: voteResult.consensus >= this.votingThreshold,
        votes: voteResult.votes || [],
        consensus: voteResult.consensus,
        reason: voteResult.consensus >= this.votingThreshold ? 'approved' : 'rejected',
      };
    } catch (err) {
      // On error, default to showing the suggestion
      return { approved: true, votes: [], reason: 'vote_error', error: err.message };
    }
  }

  /**
   * Record suggestion outcome (accepted/rejected by user)
   *
   * @param {string} suggestionId - ID of the suggestion
   * @param {boolean} accepted - Whether user accepted the suggestion
   */
  recordOutcome(suggestionId, accepted) {
    const suggestion = engineState.pendingSuggestions.find(s => s.id === suggestionId);

    const outcome = {
      suggestionId,
      accepted,
      timestamp: Date.now(),
      trigger: suggestion?.trigger || 'unknown',
    };

    engineState.outcomes.push(outcome);

    // Prune old outcomes
    if (engineState.outcomes.length > engineState.maxOutcomes) {
      engineState.outcomes = engineState.outcomes.slice(-engineState.maxOutcomes);
    }

    // Remove from pending
    engineState.pendingSuggestions = engineState.pendingSuggestions.filter(
      s => s.id !== suggestionId
    );

    return outcome;
  }

  /**
   * Check for implicit acceptance of pending suggestions
   * Based on context changes that indicate user acted on the suggestion
   *
   * Task #31: Track suggestion acceptance
   *
   * @returns {Object[]} Array of resolved suggestions with acceptance status
   */
  checkImplicitAcceptance() {
    const resolved = [];
    const now = Date.now();
    const SUGGESTION_TTL = 5 * 60 * 1000; // 5 minutes to act on suggestion

    for (const suggestion of [...engineState.pendingSuggestions]) {
      // Expire old suggestions
      if (now - suggestion.timestamp > SUGGESTION_TTL) {
        this.recordOutcome(suggestion.id, false); // Expired = rejected
        resolved.push({ ...suggestion, accepted: false, reason: 'expired' });
        continue;
      }

      let accepted = false;
      let reason = '';

      switch (suggestion.trigger) {
        case 'ERROR_PATTERN':
          // Check if errors have stopped (error count decreased)
          const recentErrors = antiPatternState.recentErrors || [];
          const windowErrors = recentErrors.filter(e => now - e.timestamp < 60000); // Last minute
          if (windowErrors.length === 0) {
            accepted = true;
            reason = 'errors_stopped';
          }
          break;

        case 'CONTEXT_DRIFT':
          // Check if user returned to goal focus
          const { currentFocus, activeGoal } = engineState.context;
          if (activeGoal && currentFocus) {
            const goalFocus = activeGoal.focus || activeGoal.title || '';
            if (currentFocus.toLowerCase().includes(goalFocus.toLowerCase().split(' ')[0])) {
              accepted = true;
              reason = 'returned_to_goal';
            }
          }
          break;

        case 'BURNOUT_RISK':
          // Check if energy improved or user took a break
          const { userEnergy } = engineState.context;
          if (userEnergy > PHI_INV_SQ * 1.5) { // Energy improved above threshold
            accepted = true;
            reason = 'energy_improved';
          }
          break;

        case 'PATTERN_MATCH':
          // Pattern reuse is hard to detect - accept if similar code was written
          // For now, assume accepted if no new errors
          if (!antiPatternState.recentErrors?.some(e => now - e.timestamp < 60000)) {
            accepted = true;
            reason = 'no_errors_after';
          }
          break;

        case 'DEADLINE_NEAR':
          // Check if user is now working on the goal
          const { currentFocus: focus, goals } = engineState.context;
          const urgentGoal = goals?.find(g => g.title === suggestion.data?.goalTitle);
          if (urgentGoal && focus?.includes(urgentGoal.title?.split(' ')[0])) {
            accepted = true;
            reason = 'working_on_goal';
          }
          break;

        case 'LEARNING_OPP':
          // Learning opportunity is accepted if user acknowledged or used the pattern
          // Hard to detect - accept by default after some time
          if (now - suggestion.timestamp > 2 * 60 * 1000) { // 2 minutes
            accepted = true;
            reason = 'time_based';
          }
          break;
      }

      if (accepted) {
        this.recordOutcome(suggestion.id, true);
        resolved.push({ ...suggestion, accepted: true, reason });
      }
    }

    return resolved;
  }

  /**
   * Get pending suggestions (for debugging/visibility)
   *
   * @returns {Object[]} Pending suggestions
   */
  getPendingSuggestions() {
    return [...engineState.pendingSuggestions];
  }

  /**
   * Get acceptance rate for a trigger type
   *
   * @param {string} triggerType - Type of trigger (optional, all if not specified)
   * @returns {number} Acceptance rate 0-1
   */
  getAcceptanceRate(triggerType = null) {
    const relevant = triggerType ?
      engineState.outcomes.filter(o => o.trigger === triggerType) :
      engineState.outcomes;

    if (relevant.length === 0) return 0;

    const accepted = relevant.filter(o => o.accepted).length;
    return accepted / relevant.length;
  }

  /**
   * Get engine statistics
   *
   * @returns {Object} Engine stats
   */
  getStats() {
    return {
      enabled: this.enabled,
      votingThreshold: this.votingThreshold,
      cooldowns: { ...engineState.cooldowns },
      pendingSuggestions: engineState.pendingSuggestions.length,
      totalOutcomes: engineState.outcomes.length,
      acceptanceRate: this.getAcceptanceRate(),
      byTrigger: Object.keys(TRIGGER_TYPES).reduce((acc, type) => {
        acc[type] = {
          onCooldown: this.isOnCooldown(type),
          acceptanceRate: this.getAcceptanceRate(type),
          outcomeCount: engineState.outcomes.filter(o => o.trigger === type).length,
        };
        return acc;
      }, {}),
    };
  }

  /**
   * Reset engine state (for testing)
   */
  reset() {
    engineState.cooldowns = {};
    engineState.outcomes = [];
    engineState.pendingSuggestions = [];
    engineState.suggestionCounter = 0;
    engineState.context = {
      userEnergy: 1.0,
      userFocus: 1.0,
      currentFocus: null,
      activeGoal: null,
      goals: [],
      consecutiveErrors: 0,
      lastErrorType: null,
      similarPatternFound: null,
      patternConfidence: 0,
      emergingPatterns: [],
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON INSTANCE
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the TriggerEngine singleton
 *
 * @param {Object} options - Engine options
 * @returns {TriggerEngine} Singleton instance
 */
export function getTriggerEngine(options = {}) {
  if (!_instance) {
    _instance = new TriggerEngine(options);
  } else if (options.enabled !== undefined) {
    _instance.enabled = options.enabled;
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetTriggerEngine() {
  if (_instance) {
    _instance.reset();
  }
  _instance = null;
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

export default {
  TriggerEngine,
  getTriggerEngine,
  resetTriggerEngine,
  TRIGGER_TYPES,
  PHI,
  PHI_INV,
  PHI_INV_SQ,
};
