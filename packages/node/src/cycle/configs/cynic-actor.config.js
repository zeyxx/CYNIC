/**
 * CynicActor Config — C6.4 (CYNIC x ACT)
 *
 * Domain-specific configuration for the CYNIC Actor.
 * Template logic lives in create-actor.js.
 *
 * Executes self-governance decisions from CynicDecider:
 * - Memory optimization (context compression, GC)
 * - Learning control (pause/resume, priority shifts)
 * - Router optimization (LLM routing, threshold adjustments)
 * - Dog rotation (rebalance collective consciousness)
 *
 * @module @cynic/node/cycle/configs/cynic-actor.config
 */

'use strict';

export const CynicActionType = {
  OPTIMIZE_MEMORY: 'optimize_memory',
  SHIFT_LLM_ROUTING: 'shift_llm_routing',
  ADJUST_LEARNING: 'adjust_learning',
  ROTATE_DOGS: 'rotate_dogs',
  ESCALATE_ALERT: 'escalate_alert',
  ADJUST_THRESHOLDS: 'adjust_thresholds',
  ACKNOWLEDGE: 'acknowledge',
};

export const cynicActorConfig = {
  name: 'CynicActor',
  cell: 'C6.4',
  dimension: 'CYNIC',
  eventPrefix: 'cynic',
  actionTypes: CynicActionType,
  maxHistory: 144, // Fib(12)

  cooldowns: {
    [CynicActionType.OPTIMIZE_MEMORY]: 13 * 60000,      // 13 min
    [CynicActionType.SHIFT_LLM_ROUTING]: 21 * 60000,    // 21 min
    [CynicActionType.ADJUST_LEARNING]: 13 * 60000,      // 13 min
    [CynicActionType.ROTATE_DOGS]: 34 * 60000,          // 34 min
    [CynicActionType.ESCALATE_ALERT]: 0,                // No cooldown for alerts
    [CynicActionType.ADJUST_THRESHOLDS]: 21 * 60000,    // 21 min
    [CynicActionType.ACKNOWLEDGE]: 0,                   // No cooldown
  },

  extraStatFields: ['optimizationsExecuted', 'alertsEscalated', 'memoryFreed'],

  // Domain init: track optimization results
  init(actor) {
    actor._optimizationLog = [];
    actor._maxOptLog = 89; // Fib(11)
    actor._totalMemoryFreed = 0;
  },

  mapDecisionToAction(decision) {
    const type = decision.type || decision.decision;

    // Map CynicDecisionType to CynicActionType
    const map = {
      // Memory decisions
      'compact_memory': CynicActionType.OPTIMIZE_MEMORY,
      'compress_context': CynicActionType.OPTIMIZE_MEMORY,
      'trigger_gc': CynicActionType.OPTIMIZE_MEMORY,
      'semantic_compress': CynicActionType.OPTIMIZE_MEMORY,
      // Learning decisions
      'pause_learning': CynicActionType.ADJUST_LEARNING,
      'prioritize_sona': CynicActionType.ADJUST_LEARNING,
      // Budget decisions
      'shift_to_ollama': CynicActionType.SHIFT_LLM_ROUTING,
      // Threshold decisions
      'adjust_thresholds': CynicActionType.ADJUST_THRESHOLDS,
      'investigate_wiring': CynicActionType.ADJUST_THRESHOLDS,
      // Dog decisions
      'rotate_dogs': CynicActionType.ROTATE_DOGS,
      // Alert decisions
      'escalate_pattern': CynicActionType.ESCALATE_ALERT,
      // Default
      'acknowledge': CynicActionType.ACKNOWLEDGE,
    };

    return map[type] ?? CynicActionType.ACKNOWLEDGE;
  },

  assessUrgency(decision) {
    const urgency = decision.urgency || 'low';

    // Pass through urgency from CynicDecider
    if (['critical', 'high', 'medium', 'low'].includes(urgency)) {
      return urgency;
    }

    // Fallback based on decision type
    const type = decision.type || decision.decision;
    if (type === 'escalate_pattern') return 'high';
    if (type === 'compact_memory') return 'medium';
    if (type === 'pause_learning') return 'medium';
    if (type === 'rotate_dogs') return 'medium';
    return 'low';
  },

  composeMessage(actionType, decision, context) {
    const qScore = context.qScore ? ` (Q:${context.qScore})` : '';
    const reason = decision.context?.reason || decision.reason || '';

    switch (actionType) {
      case CynicActionType.OPTIMIZE_MEMORY:
        return `*sniff* Optimizing memory${qScore}. ${reason || 'Memory pressure detected.'}`;

      case CynicActionType.SHIFT_LLM_ROUTING:
        return `*tail wag* Adjusting LLM routing${qScore}. ${reason || 'Budget optimization.'}`;

      case CynicActionType.ADJUST_LEARNING:
        return `*ears perk* Adjusting learning${qScore}. ${reason || 'Learning velocity shift.'}`;

      case CynicActionType.ROTATE_DOGS:
        return `*head tilt* Rotating dog priorities${qScore}. ${reason || 'Rebalancing collective.'}`;

      case CynicActionType.ESCALATE_ALERT:
        return `*GROWL* Pattern escalation${qScore}. ${reason || 'Significant anomaly detected.'}`;

      case CynicActionType.ADJUST_THRESHOLDS:
        return `*sniff* Adjusting thresholds${qScore}. ${reason || 'Consensus quality shift.'}`;

      case CynicActionType.ACKNOWLEDGE:
        return `*yawn* Acknowledged${qScore}. ${reason || 'Pattern noted.'}`;

      default:
        return `*sniff* CYNIC action: ${actionType}${qScore}`;
    }
  },

  updateExtraStats(stats, result) {
    if (result.type === CynicActionType.OPTIMIZE_MEMORY) {
      stats.optimizationsExecuted++;
      if (result.memoryFreed) stats.memoryFreed += result.memoryFreed;
    }
    if (result.type === CynicActionType.ESCALATE_ALERT) {
      stats.alertsEscalated++;
    }
  },

  postAct(result, decision, context, actor) {
    // Log optimization for analysis
    if ([
      CynicActionType.OPTIMIZE_MEMORY,
      CynicActionType.SHIFT_LLM_ROUTING,
      CynicActionType.ADJUST_LEARNING,
    ].includes(result.type)) {
      actor._optimizationLog.push({
        type: result.type,
        reason: decision.context?.reason || decision.reason,
        confidence: decision.confidence,
        timestamp: Date.now(),
        result: result.status,
      });

      while (actor._optimizationLog.length > actor._maxOptLog) {
        actor._optimizationLog.shift();
      }
    }

    // Track memory freed
    if (result.memoryFreed) {
      actor._totalMemoryFreed += result.memoryFreed;
    }
  },

  healthMetric: 'alertsEscalated',
  healthThreshold: undefined, // defaults to PHI_INV_2 (38.2%)
  healthStatusBad: 'high_alert_rate',
  healthExtraFields: {
    optimizationsExecuted: 'optimizationsExecuted',
    totalMemoryFreed: null, // handled by custom getter if needed
  },
};
