/**
 * CYNIC Code Decider - C1.3 (CODE × DECIDE)
 *
 * Routes code change decisions using φ-aligned risk assessment.
 * Part of the 7×7 Fractal Matrix decision layer.
 *
 * "Commit when φ permits, review when φ doubts" - κυνικός
 *
 * Decides:
 * - Commit safety (approve, block, require review)
 * - Refactoring priority (now, later, never)
 * - Test requirements (coverage gaps → block/warn)
 * - Deploy readiness (risk-gated)
 *
 * @module @cynic/node/code/code-decider
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('CodeDecider');

export const CodeDecisionType = {
  APPROVE: 'approve',
  QUEUE_REVIEW: 'queue_review',
  REQUIRE_TESTS: 'require_tests',
  REQUIRE_REFACTOR: 'require_refactor',
  BLOCK: 'block',
  DEFER: 'defer',
};

export const ChangeRisk = {
  TRIVIAL: 'trivial',
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

const THRESHOLDS = {
  approveConfidence: PHI_INV,         // 61.8% — auto-approve
  reviewConfidence: PHI_INV_2,        // 38.2% — needs human review
  blockConfidence: PHI_INV_3,         // 23.6% — too risky

  // Lines changed thresholds
  trivialLines: 5,
  smallLines: 21,                     // Fib(8)
  mediumLines: 89,                    // Fib(11)
  largeLines: 233,                    // Fib(13)

  // Risk multipliers
  newDependencyPenalty: 0.15,
  coreFilePenalty: 0.2,
  noTestPenalty: 0.25,
  deletionBonus: 0.05,               // BURN axiom rewards deletion
};

const CORE_PATTERNS = [
  /constants\.js$/,
  /event-bus\.js$/,
  /judge\.js$/,
  /collective-singleton\.js$/,
  /unified-orchestrator\.js$/,
  /network-node\.js$/,
  /engine\.js$/,
];

export class CodeDecider extends EventEmitter {
  constructor(options = {}) {
    super();

    this._history = [];
    this._maxHistory = 233; // Fib(13)

    this._stats = {
      decisionsTotal: 0,
      byType: {},
      approvalsTotal: 0,
      blocksTotal: 0,
      avgDecisionTime: 0,
      lastDecision: null,
    };

    for (const type of Object.values(CodeDecisionType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Decide on a code change
   *
   * @param {Object} change - { files, linesAdded, linesRemoved, dependencies, hasTests }
   * @param {Object} [context] - { branch, author, recentFailures }
   * @returns {Object} Decision result
   */
  decide(change, context = {}) {
    const startTime = Date.now();

    const risk = this._assessRisk(change, context);
    const confidence = this._calculateConfidence(risk, change);
    const decision = this._makeDecision(risk, confidence, change, context);

    const result = {
      type: decision.type,
      risk: risk.level,
      confidence,
      cell: 'C1.3',
      dimension: 'CODE',
      analysis: 'DECIDE',
      reasoning: decision.reasoning,
      parameters: decision.parameters,
      riskFactors: risk.factors,
      timestamp: Date.now(),
      decisionTimeMs: Date.now() - startTime,
    };

    this._updateStats(result);
    this._history.push(result);
    while (this._history.length > this._maxHistory) this._history.shift();

    this.emit('decision', result);
    globalEventBus.emit('code:decision', result);

    log.debug('Code decision', { type: result.type, risk: risk.level, confidence: confidence.toFixed(3) });

    return result;
  }

  /**
   * Decide if a commit is safe
   */
  decideCommit(change, metadata = {}) {
    const result = this.decide(change, { ...metadata, decisionContext: 'commit' });

    if (result.type === CodeDecisionType.BLOCK) {
      this.emit('commit_blocked', result);
    } else if (result.type === CodeDecisionType.APPROVE) {
      this.emit('commit_approved', result);
    }

    return result;
  }

  /**
   * Decide refactoring priority
   */
  decideRefactor(file, suggestion = {}) {
    const complexity = suggestion.complexity || 0;
    const churnRate = suggestion.churnRate || 0;
    const couplingScore = suggestion.coupling || 0;

    // High churn + high complexity = refactor NOW
    const urgency = (churnRate * 0.4 + complexity * 0.4 + couplingScore * 0.2);

    if (urgency >= PHI_INV) {
      return { type: 'refactor_now', urgency, reasoning: 'High churn + complexity — refactor pays off fast' };
    }
    if (urgency >= PHI_INV_2) {
      return { type: 'refactor_soon', urgency, reasoning: 'Medium urgency — schedule refactor' };
    }
    return { type: 'defer', urgency, reasoning: 'Low urgency — not worth the disruption yet' };
  }

  /**
   * Decide deploy readiness
   */
  decideDeploy(change, state = {}) {
    const testsPassing = state.testsPassing ?? true;
    const coverageDelta = state.coverageDelta ?? 0;
    const recentRollbacks = state.recentRollbacks ?? 0;

    let confidence = PHI_INV;

    if (!testsPassing) confidence *= 0.1;
    if (coverageDelta < -5) confidence *= 0.7;
    if (recentRollbacks > 0) confidence *= (1 - recentRollbacks * 0.2);

    confidence = Math.min(PHI_INV, Math.max(0, confidence));

    if (!testsPassing) {
      return { type: CodeDecisionType.BLOCK, confidence, reasoning: 'Tests failing — deploy blocked' };
    }
    if (confidence >= THRESHOLDS.approveConfidence * 0.9) {
      return { type: CodeDecisionType.APPROVE, confidence, reasoning: 'Deploy approved' };
    }
    return { type: CodeDecisionType.QUEUE_REVIEW, confidence, reasoning: 'Deploy needs review' };
  }

  _assessRisk(change, context) {
    const factors = [];
    let riskScore = 0;

    // Size risk
    const totalLines = (change.linesAdded || 0) + (change.linesRemoved || 0);
    if (totalLines > THRESHOLDS.largeLines) {
      riskScore += 0.3;
      factors.push(`large_change (${totalLines} lines)`);
    } else if (totalLines > THRESHOLDS.mediumLines) {
      riskScore += 0.15;
      factors.push(`medium_change (${totalLines} lines)`);
    }

    // BURN bonus: net deletion is good
    const netLines = (change.linesAdded || 0) - (change.linesRemoved || 0);
    if (netLines < 0) {
      riskScore -= THRESHOLDS.deletionBonus;
      factors.push(`net_deletion (${netLines} lines — BURN)`);
    }

    // Dependency risk
    const newDeps = change.dependenciesAdded || 0;
    if (newDeps > 0) {
      riskScore += newDeps * THRESHOLDS.newDependencyPenalty;
      factors.push(`new_dependencies (${newDeps})`);
    }

    // Core file risk
    const files = change.files || [];
    const coreHits = files.filter(f => CORE_PATTERNS.some(p => p.test(f)));
    if (coreHits.length > 0) {
      riskScore += coreHits.length * THRESHOLDS.coreFilePenalty;
      factors.push(`core_files_modified (${coreHits.length})`);
    }

    // Test coverage risk
    if (!change.hasTests && totalLines > THRESHOLDS.smallLines) {
      riskScore += THRESHOLDS.noTestPenalty;
      factors.push('no_tests_for_significant_change');
    }

    // Recent failures context
    if (context.recentFailures > 3) {
      riskScore += 0.15;
      factors.push(`recent_failures (${context.recentFailures})`);
    }

    riskScore = Math.min(1, Math.max(0, riskScore));

    let level;
    if (riskScore <= 0.1) level = ChangeRisk.TRIVIAL;
    else if (riskScore <= PHI_INV_3) level = ChangeRisk.LOW;
    else if (riskScore <= PHI_INV_2) level = ChangeRisk.MEDIUM;
    else if (riskScore <= PHI_INV) level = ChangeRisk.HIGH;
    else level = ChangeRisk.CRITICAL;

    return { score: riskScore, level, factors };
  }

  _calculateConfidence(risk, change) {
    let confidence = PHI_INV;
    confidence *= (1 - risk.score * 0.8);
    return Math.min(PHI_INV, Math.max(0, confidence));
  }

  _makeDecision(risk, confidence, change, context) {
    if (risk.level === ChangeRisk.CRITICAL) {
      return {
        type: CodeDecisionType.BLOCK,
        reasoning: `Critical risk (${risk.factors.join(', ')}) — blocked`,
        parameters: { riskScore: risk.score },
      };
    }

    if (!change.hasTests && risk.level !== ChangeRisk.TRIVIAL) {
      return {
        type: CodeDecisionType.REQUIRE_TESTS,
        reasoning: 'Non-trivial change without tests',
        parameters: { suggestedCoverage: risk.level === ChangeRisk.HIGH ? 80 : 60 },
      };
    }

    if (confidence >= THRESHOLDS.approveConfidence) {
      return {
        type: CodeDecisionType.APPROVE,
        reasoning: `High confidence (${(confidence * 100).toFixed(1)}%) — approved`,
        parameters: {},
      };
    }

    if (confidence >= THRESHOLDS.reviewConfidence) {
      return {
        type: CodeDecisionType.QUEUE_REVIEW,
        reasoning: `Medium confidence (${(confidence * 100).toFixed(1)}%) — needs review`,
        parameters: { reviewPriority: risk.level },
      };
    }

    return {
      type: CodeDecisionType.DEFER,
      reasoning: `Low confidence (${(confidence * 100).toFixed(1)}%) — defer`,
      parameters: {},
    };
  }

  _updateStats(result) {
    this._stats.decisionsTotal++;
    this._stats.byType[result.type] = (this._stats.byType[result.type] || 0) + 1;
    this._stats.lastDecision = Date.now();

    if (result.type === CodeDecisionType.APPROVE) this._stats.approvalsTotal++;
    if (result.type === CodeDecisionType.BLOCK) this._stats.blocksTotal++;

    const n = this._stats.decisionsTotal;
    this._stats.avgDecisionTime = ((n - 1) * this._stats.avgDecisionTime + result.decisionTimeMs) / n;
  }

  getStats() { return { ...this._stats }; }

  getHealth() {
    const approvalRate = this._stats.decisionsTotal > 0
      ? this._stats.approvalsTotal / this._stats.decisionsTotal
      : 0;

    return {
      status: approvalRate >= PHI_INV_2 ? 'healthy' : 'cautious',
      score: Math.min(PHI_INV, approvalRate),
      decisionsTotal: this._stats.decisionsTotal,
      approvalRate,
      avgDecisionTimeMs: this._stats.avgDecisionTime,
    };
  }

  getHistory(limit = 21) {
    return this._history.slice(-limit);
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getCodeDecider(options = {}) {
  if (!_instance) _instance = new CodeDecider(options);
  return _instance;
}

export function resetCodeDecider() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { CodeDecider, CodeDecisionType, ChangeRisk, getCodeDecider, resetCodeDecider };
