/**
 * PlanningGate - Meta-Cognition Layer for CYNIC
 *
 * Determines WHEN to pause and plan vs act immediately.
 * Implements the "think before acting" pattern.
 *
 * Triggers:
 * - Complexity uncertain (< 33% classifier confidence)
 * - Borderline confidence (38.2% - 61.8%)
 * - High risk + low trust
 * - High entropy (> φ⁻¹ + 15%)
 * - Agent disagreement (< 70% consensus)
 * - Explicit request
 *
 * "Un système qui pense avant d'agir" - κυνικός
 *
 * @module @cynic/node/orchestration/planning-gate
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

const log = createLogger('PlanningGate');

/**
 * Planning triggers - why planning was requested
 * @enum {string}
 */
export const PlanningTrigger = {
  /** Complexity classifier confidence < 33% */
  COMPLEXITY_UNCERTAIN: 'complexity_uncertain',
  /** Confidence between 38.2% and 61.8% */
  BORDERLINE_CONFIDENCE: 'borderline_confidence',
  /** Risk is critical/high AND trust < STEWARD */
  HIGH_RISK_LOW_TRUST: 'high_risk_low_trust',
  /** Entropy > φ⁻¹ + 15% (~76.8%) */
  HIGH_ENTROPY: 'high_entropy',
  /** Max agent agreement < 70% */
  AGENT_DISAGREEMENT: 'agent_disagreement',
  /** User explicitly requested planning */
  EXPLICIT_REQUEST: 'explicit_request',
  /** Chaos testing triggered planning */
  CHAOS_TEST: 'chaos_test',
};

/**
 * Planning decisions
 * @enum {string}
 */
export const PlanningDecision = {
  /** No planning needed, proceed immediately */
  CONTINUE: 'continue',
  /** Generate plan and wait for human approval */
  PAUSE: 'pause',
  /** Generate plan but auto-proceed if confident */
  CONSULT: 'consult',
};

/**
 * Thresholds for planning triggers (φ-aligned)
 */
export const PLANNING_THRESHOLDS = {
  /** Complexity must be above this to skip planning */
  COMPLEXITY_MIN: 0.33,
  /** Confidence below this triggers planning */
  CONFIDENCE_LOW: PHI_INV_2,  // 0.382
  /** Confidence above this allows skip */
  CONFIDENCE_HIGH: PHI_INV,   // 0.618
  /** Entropy above this triggers planning */
  ENTROPY_MAX: PHI_INV + 0.15, // ~0.768
  /** Consensus below this triggers escalation */
  CONSENSUS_MIN: 0.70,
  /** Risk levels that require planning with low trust */
  HIGH_RISK_LEVELS: ['critical', 'high'],
  /** Trust levels considered "low" */
  LOW_TRUST_LEVELS: ['GUEST', 'OBSERVER', 'BUILDER'],
};

/**
 * Planning result - what the gate decided
 */
export class PlanningResult {
  constructor(data = {}) {
    this.needed = data.needed || false;
    this.decision = data.decision || PlanningDecision.CONTINUE;
    this.triggers = data.triggers || [];
    this.alternatives = data.alternatives || [];
    this.plan = data.plan || null;
    this.confidence = Math.min(data.confidence || 0, PHI_INV);
    this.timestamp = Date.now();
  }

  toJSON() {
    return {
      needed: this.needed,
      decision: this.decision,
      triggers: this.triggers,
      alternativeCount: this.alternatives.length,
      confidence: this.confidence,
      timestamp: this.timestamp,
    };
  }
}

/**
 * PlanningGate - The meta-cognition layer
 *
 * Sits between routing and judgment in the pipeline.
 * Decides when to pause for human approval.
 */
export class PlanningGate extends EventEmitter {
  /**
   * Create a PlanningGate
   *
   * @param {Object} options
   * @param {Object} [options.thresholds] - Custom thresholds
   * @param {Object} [options.chaosGenerator] - ChaosGenerator for random testing
   * @param {Object} [options.brain] - Brain instance for plan generation
   */
  constructor(options = {}) {
    super();

    this.thresholds = {
      ...PLANNING_THRESHOLDS,
      ...options.thresholds,
    };
    this.chaosGenerator = options.chaosGenerator || null;
    this.brain = options.brain || null;

    // Statistics
    this.stats = {
      checked: 0,
      planningTriggered: 0,
      byTrigger: {},
      byDecision: {
        [PlanningDecision.CONTINUE]: 0,
        [PlanningDecision.PAUSE]: 0,
        [PlanningDecision.CONSULT]: 0,
      },
    };

    log.debug('PlanningGate created', { thresholds: this.thresholds });
  }

  /**
   * Check if planning is needed for an event
   *
   * @param {DecisionEvent} event - The decision event to check
   * @param {Object} [context] - Additional context
   * @param {number} [context.complexity] - Complexity classifier confidence (0-1)
   * @param {number} [context.confidence] - Overall confidence (0-1)
   * @param {number} [context.entropy] - Current entropy level (0-1)
   * @param {number} [context.consensusRatio] - Agent consensus ratio (0-1)
   * @returns {PlanningResult}
   */
  shouldPlan(event, context = {}) {
    this.stats.checked++;
    const triggers = [];

    // 1. Check explicit request
    if (event.context?.requestPlanning === true) {
      triggers.push(PlanningTrigger.EXPLICIT_REQUEST);
    }

    // 2. Check complexity uncertainty
    const complexity = context.complexity ?? 1.0;
    if (complexity < this.thresholds.COMPLEXITY_MIN) {
      triggers.push(PlanningTrigger.COMPLEXITY_UNCERTAIN);
    }

    // 3. Check borderline confidence
    const confidence = context.confidence ?? event.judgment?.consensusRatio ?? 0.5;
    if (confidence >= this.thresholds.CONFIDENCE_LOW &&
        confidence <= this.thresholds.CONFIDENCE_HIGH) {
      triggers.push(PlanningTrigger.BORDERLINE_CONFIDENCE);
    }

    // 4. Check high risk + low trust
    const risk = event.routing?.risk || 'low';
    const trustLevel = event.userContext?.trustLevel || 'BUILDER';
    if (this.thresholds.HIGH_RISK_LEVELS.includes(risk) &&
        this.thresholds.LOW_TRUST_LEVELS.includes(trustLevel)) {
      triggers.push(PlanningTrigger.HIGH_RISK_LOW_TRUST);
    }

    // 5. Check high entropy
    const entropy = context.entropy ?? 0;
    if (entropy > this.thresholds.ENTROPY_MAX) {
      triggers.push(PlanningTrigger.HIGH_ENTROPY);
    }

    // 6. Check agent disagreement
    const consensusRatio = context.consensusRatio ?? event.judgment?.consensusRatio ?? 1.0;
    if (consensusRatio < this.thresholds.CONSENSUS_MIN) {
      triggers.push(PlanningTrigger.AGENT_DISAGREEMENT);
    }

    // 7. Check chaos injection (if available)
    if (this.chaosGenerator) {
      const chaosResult = this.chaosGenerator.shouldForcePlanning({
        content: event.content,
        complexity,
        confidence,
      });
      if (chaosResult.force) {
        triggers.push(PlanningTrigger.CHAOS_TEST);
      }
    }

    // Determine decision based on triggers
    const decision = this._determineDecision(triggers, context);

    // Update stats
    if (triggers.length > 0) {
      this.stats.planningTriggered++;
      for (const trigger of triggers) {
        this.stats.byTrigger[trigger] = (this.stats.byTrigger[trigger] || 0) + 1;
      }
    }
    this.stats.byDecision[decision]++;

    const result = new PlanningResult({
      needed: triggers.length > 0,
      decision,
      triggers,
      confidence,
    });

    if (result.needed) {
      log.info('Planning triggered', {
        eventId: event.id,
        triggers,
        decision,
      });
      this.emit('planning:triggered', { event, result });
    }

    return result;
  }

  /**
   * Generate a plan for the event
   *
   * @param {DecisionEvent} event - The decision event
   * @param {PlanningResult} planningResult - Previous shouldPlan result
   * @returns {Promise<PlanningResult>}
   */
  async generatePlan(event, planningResult) {
    if (!planningResult.needed) {
      return planningResult;
    }

    const plan = {
      id: `plan-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      eventId: event.id,
      triggers: planningResult.triggers,
      alternatives: [],
      recommendation: null,
      reasoning: [],
    };

    // Generate alternatives using brain if available
    if (this.brain) {
      try {
        const thought = await this.brain.think({
          content: `Generate alternatives for: ${event.content}`,
          type: 'planning',
          context: {
            routing: event.routing,
            triggers: planningResult.triggers,
          },
        }, {
          requestJudgment: true,
          requestSynthesis: true,
          checkPatterns: true,
        });

        // Extract alternatives from synthesis if available
        if (thought.synthesis?.consultations) {
          for (const consultation of thought.synthesis.consultations) {
            if (consultation.response?.alternatives) {
              plan.alternatives.push(...consultation.response.alternatives);
            }
          }
        }

        // Add reasoning from thought
        if (thought.decision?.reason) {
          plan.reasoning.push(thought.decision.reason);
        }

        plan.recommendation = thought.decision?.action || 'consider';
        planningResult.confidence = thought.confidence;
      } catch (err) {
        log.warn('Brain planning failed', { error: err.message });
        plan.reasoning.push(`Planning error: ${err.message}`);
      }
    }

    // Generate basic alternatives if none were produced
    if (plan.alternatives.length === 0) {
      plan.alternatives = this._generateBasicAlternatives(event, planningResult);
    }

    planningResult.plan = plan;
    planningResult.alternatives = plan.alternatives;

    this.emit('planning:generated', { event, plan });

    return planningResult;
  }

  /**
   * Determine decision based on triggers
   * @private
   */
  _determineDecision(triggers, context) {
    if (triggers.length === 0) {
      return PlanningDecision.CONTINUE;
    }

    // Critical triggers always pause
    const criticalTriggers = [
      PlanningTrigger.HIGH_RISK_LOW_TRUST,
      PlanningTrigger.EXPLICIT_REQUEST,
    ];

    if (triggers.some(t => criticalTriggers.includes(t))) {
      return PlanningDecision.PAUSE;
    }

    // Multiple triggers = pause
    if (triggers.length >= 2) {
      return PlanningDecision.PAUSE;
    }

    // Single non-critical trigger = consult (auto-proceed if confident)
    return PlanningDecision.CONSULT;
  }

  /**
   * Generate basic alternatives when brain unavailable
   * @private
   */
  _generateBasicAlternatives(event, planningResult) {
    const alternatives = [];
    const triggers = planningResult.triggers;

    // Always offer "proceed" and "cancel"
    alternatives.push({
      id: 'proceed',
      label: 'Proceed with current approach',
      description: 'Continue without changes',
      risk: event.routing?.risk || 'unknown',
    });

    alternatives.push({
      id: 'cancel',
      label: 'Cancel and reconsider',
      description: 'Stop and gather more information',
      risk: 'none',
    });

    // Add trigger-specific alternatives
    if (triggers.includes(PlanningTrigger.COMPLEXITY_UNCERTAIN)) {
      alternatives.push({
        id: 'simplify',
        label: 'Simplify the request',
        description: 'Break down into smaller, clearer steps',
        risk: 'low',
      });
    }

    if (triggers.includes(PlanningTrigger.HIGH_RISK_LOW_TRUST)) {
      alternatives.push({
        id: 'sandbox',
        label: 'Run in sandbox first',
        description: 'Test in isolated environment',
        risk: 'low',
      });
    }

    if (triggers.includes(PlanningTrigger.AGENT_DISAGREEMENT)) {
      alternatives.push({
        id: 'escalate',
        label: 'Escalate to human',
        description: 'Dogs disagree, human decides',
        risk: 'none',
      });
    }

    return alternatives;
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      planningRate: this.stats.checked > 0
        ? (this.stats.planningTriggered / this.stats.checked * 100).toFixed(1) + '%'
        : '0%',
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      checked: 0,
      planningTriggered: 0,
      byTrigger: {},
      byDecision: {
        [PlanningDecision.CONTINUE]: 0,
        [PlanningDecision.PAUSE]: 0,
        [PlanningDecision.CONSULT]: 0,
      },
    };
  }

  /**
   * Set chaos generator
   */
  setChaosGenerator(chaosGenerator) {
    this.chaosGenerator = chaosGenerator;
  }

  /**
   * Set brain
   */
  setBrain(brain) {
    this.brain = brain;
  }
}

/**
 * Create a PlanningGate instance
 *
 * @param {Object} options
 * @returns {PlanningGate}
 */
export function createPlanningGate(options = {}) {
  return new PlanningGate(options);
}

// Singleton
let _globalPlanningGate = null;

/**
 * Get the global PlanningGate instance
 *
 * @param {Object} [options]
 * @returns {PlanningGate}
 */
export function getPlanningGate(options) {
  if (!_globalPlanningGate) {
    _globalPlanningGate = new PlanningGate(options);
  }
  return _globalPlanningGate;
}

/**
 * Reset global planning gate (for testing)
 */
export function _resetPlanningGateForTesting() {
  _globalPlanningGate = null;
}

export default PlanningGate;
