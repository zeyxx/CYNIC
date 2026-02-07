/**
 * CYNIC Human Judge - C5.2 (HUMAN × JUDGE)
 *
 * Judges human state: wellbeing, productivity, engagement, burnout risk.
 * Uses φ-aligned thresholds to produce verdicts on human condition.
 *
 * "Le chien juge l'état du maître, pas ses intentions" - κυνικός
 *
 * @module @cynic/node/symbiosis/human-judge
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('HumanJudge');

export const HumanVerdict = {
  THRIVING: 'thriving',     // ≥ φ⁻¹ — peak state
  STEADY: 'steady',         // ≥ φ⁻² — sustainable
  STRAINED: 'strained',     // ≥ φ⁻³ — needs attention
  CRITICAL: 'critical',     // < φ⁻³ — intervention needed
};

export const JudgmentDomain = {
  WELLBEING: 'wellbeing',
  PRODUCTIVITY: 'productivity',
  ENGAGEMENT: 'engagement',
  BURNOUT_RISK: 'burnout_risk',
};

export class HumanJudge extends EventEmitter {
  constructor(options = {}) {
    super();

    this._history = [];
    this._maxHistory = 89; // Fib(11)

    this._stats = {
      judgmentsTotal: 0,
      byVerdict: {},
      byDomain: {},
      avgScore: 0,
      lastJudgment: null,
    };

    for (const v of Object.values(HumanVerdict)) this._stats.byVerdict[v] = 0;
    for (const d of Object.values(JudgmentDomain)) this._stats.byDomain[d] = 0;
  }

  /**
   * Judge human state from perceived data
   *
   * @param {Object} perception - From HumanPerceiver.perceive()
   * @param {Object} [context] - Session context, history
   * @returns {Object} Judgment with scores, verdict, recommendations
   */
  judge(perception, context = {}) {
    const startTime = Date.now();

    const scores = {
      wellbeing: this._scoreWellbeing(perception),
      productivity: this._scoreProductivity(perception, context),
      engagement: this._scoreEngagement(perception, context),
      burnoutRisk: this._scoreBurnoutRisk(perception, context),
    };

    // Composite Q-Score (φ-weighted)
    const qScore = (
      scores.wellbeing * PHI_INV +
      scores.productivity * PHI_INV_2 +
      scores.engagement * PHI_INV_3 +
      (1 - scores.burnoutRisk) * PHI_INV_3
    ) / (PHI_INV + PHI_INV_2 + PHI_INV_3 + PHI_INV_3);

    const verdict = this._deriveVerdict(qScore);

    const result = {
      qScore,
      verdict,
      scores,
      cell: 'C5.2',
      dimension: 'HUMAN',
      analysis: 'JUDGE',
      recommendations: this._generateRecommendations(scores, verdict),
      timestamp: Date.now(),
      judgmentTimeMs: Date.now() - startTime,
    };

    this._updateStats(result);
    this._history.push(result);
    while (this._history.length > this._maxHistory) this._history.shift();

    this.emit('judgment', result);
    globalEventBus.emit('human:judgment', {
      ...result,
      cell: 'C5.2',
    });

    log.debug('Human judgment', { verdict, qScore: qScore.toFixed(3) });

    return result;
  }

  _scoreWellbeing(p) {
    // Energy and low frustration = wellbeing
    const energyScore = p.energy || PHI_INV_2;
    const frustrationPenalty = (p.frustration || 0) * 0.5;
    return Math.min(PHI_INV, Math.max(0, energyScore - frustrationPenalty));
  }

  _scoreProductivity(p, ctx) {
    // Focus + manageable cognitive load = productive
    const focusScore = p.focus || PHI_INV_2;
    const loadPenalty = p.cognitiveLoad > 7 ? 0.2 : p.cognitiveLoad > 5 ? 0.1 : 0;
    return Math.min(PHI_INV, Math.max(0, focusScore - loadPenalty));
  }

  _scoreEngagement(p, ctx) {
    // Active tool use + non-zero energy = engaged
    const sessionMinutes = p.sessionMinutes || 0;
    if (sessionMinutes < 2) return PHI_INV; // too early to judge
    if (sessionMinutes > 180) return PHI_INV_3; // 3h+ → engagement drops

    // Engagement decays with session length (circadian)
    const decay = Math.max(PHI_INV_3, PHI_INV * Math.exp(-sessionMinutes / 120));
    return Math.min(PHI_INV, decay);
  }

  _scoreBurnoutRisk(p, ctx) {
    let risk = 0;

    // Long session
    const sessionMinutes = p.sessionMinutes || 0;
    if (sessionMinutes > 240) risk += 0.3;       // 4h+
    else if (sessionMinutes > 120) risk += 0.15;  // 2h+

    // Low energy
    if ((p.energy || PHI_INV) < PHI_INV_3) risk += 0.3;
    else if ((p.energy || PHI_INV) < PHI_INV_2) risk += 0.15;

    // High frustration
    if ((p.frustration || 0) > PHI_INV_2) risk += 0.2;

    // Cognitive overload
    if ((p.cognitiveLoad || 0) > 7) risk += 0.15;

    return Math.min(1, risk);
  }

  _deriveVerdict(qScore) {
    if (qScore >= PHI_INV) return HumanVerdict.THRIVING;
    if (qScore >= PHI_INV_2) return HumanVerdict.STEADY;
    if (qScore >= PHI_INV_3) return HumanVerdict.STRAINED;
    return HumanVerdict.CRITICAL;
  }

  _generateRecommendations(scores, verdict) {
    const recs = [];

    if (verdict === HumanVerdict.CRITICAL) {
      recs.push({ action: 'break', urgency: 'high', reason: 'Critical state detected' });
    }

    if (scores.burnoutRisk > PHI_INV_2) {
      recs.push({ action: 'pace_down', urgency: 'medium', reason: `Burnout risk: ${(scores.burnoutRisk * 100).toFixed(0)}%` });
    }

    if (scores.wellbeing < PHI_INV_3) {
      recs.push({ action: 'hydrate_stretch', urgency: 'medium', reason: 'Low wellbeing' });
    }

    if (scores.engagement < PHI_INV_3) {
      recs.push({ action: 'refocus', urgency: 'low', reason: 'Engagement dropping' });
    }

    return recs;
  }

  _updateStats(result) {
    this._stats.judgmentsTotal++;
    this._stats.byVerdict[result.verdict] = (this._stats.byVerdict[result.verdict] || 0) + 1;
    this._stats.lastJudgment = Date.now();

    const n = this._stats.judgmentsTotal;
    this._stats.avgScore = ((n - 1) * this._stats.avgScore + result.qScore) / n;
  }

  getStats() { return { ...this._stats }; }

  getHealth() {
    return {
      status: this._stats.avgScore >= PHI_INV_2 ? 'healthy' : 'concerning',
      score: Math.min(PHI_INV, this._stats.avgScore),
      judgmentsTotal: this._stats.judgmentsTotal,
      avgScore: this._stats.avgScore,
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

export function getHumanJudge(options = {}) {
  if (!_instance) _instance = new HumanJudge(options);
  return _instance;
}

export function resetHumanJudge() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { HumanJudge, HumanVerdict, JudgmentDomain, getHumanJudge, resetHumanJudge };
