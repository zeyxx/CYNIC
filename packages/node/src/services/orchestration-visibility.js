/**
 * OrchestrationVisibility - Real-time Orchestration Display
 *
 * Makes CYNIC's internal orchestration VISIBLE to humans:
 * - Dog voting in progress
 * - Consensus forming
 * - Decisions being made
 * - Conflicts being resolved
 *
 * Renders TUI-friendly output via system-reminders or inline status.
 *
 * "L'orchestre joue, et le public voit" - κυνικός
 *
 * @module @cynic/node/services/orchestration-visibility
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { getEventBus, EventType } from './event-bus.js';

const log = createLogger('OrchestrationVisibility');

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Dog emoji mapping (Sefirot)
 */
const DOG_EMOJI = {
  cynic: '🧠',
  guardian: '🛡️',
  analyst: '📊',
  scholar: '📚',
  sage: '🦉',
  architect: '🏗️',
  oracle: '🔮',
  scout: '🔍',
  deployer: '🚀',
  janitor: '🧹',
  cartographer: '🗺️',
};

/**
 * Verdict emojis
 */
const VERDICT_EMOJI = {
  HOWL: '✅',
  WAG: '🐕',
  BARK: '⚠️',
  GROWL: '🔴',
};

/**
 * Visibility levels
 */
export const VisibilityLevel = {
  SILENT: 'silent',       // No output
  COMPACT: 'compact',     // Single line status
  NORMAL: 'normal',       // Key events only
  VERBOSE: 'verbose',     // All events
  DEBUG: 'debug',         // Everything including timing
};

// ═══════════════════════════════════════════════════════════════════════════
// ORCHESTRATION VISIBILITY SERVICE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * OrchestrationVisibility Service
 *
 * Subscribes to orchestration events and renders them for humans.
 */
export class OrchestrationVisibility extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} [options.eventBus] - EventBus instance
   * @param {string} [options.level] - Visibility level
   * @param {boolean} [options.showTiming] - Show timing info
   * @param {Function} [options.output] - Custom output function
   */
  constructor(options = {}) {
    super();

    this.eventBus = options.eventBus || getEventBus();
    this.level = options.level || VisibilityLevel.NORMAL;
    this.showTiming = options.showTiming || false;
    this.output = options.output || console.log;

    // Current orchestration state
    this._currentOrchestration = null;
    this._votes = new Map();
    this._conflicts = [];

    // Statistics
    this.stats = {
      orchestrationsShown: 0,
      votesShown: 0,
      conflictsShown: 0,
    };

    // Subscribe to events
    this._subscribeToEvents();

    log.debug('OrchestrationVisibility created', { level: this.level });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EVENT SUBSCRIPTIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Subscribe to relevant events
   * @private
   */
  _subscribeToEvents() {
    // Orchestration events
    this.eventBus.subscribe('orchestration:start', (data, meta) => {
      this._onOrchestrationStart(data, meta);
    });

    this.eventBus.subscribe('orchestration:complete', (data, meta) => {
      this._onOrchestrationComplete(data, meta);
    });

    // Dog voting events
    this.eventBus.subscribe('dog:vote:start', (data, meta) => {
      this._onDogVoteStart(data, meta);
    });

    this.eventBus.subscribe('dog:vote:cast', (data, meta) => {
      this._onDogVoteCast(data, meta);
    });

    this.eventBus.subscribe('dog:vote:complete', (data, meta) => {
      this._onDogVoteComplete(data, meta);
    });

    // Consensus events
    this.eventBus.subscribe('consensus:forming', (data, meta) => {
      this._onConsensusForming(data, meta);
    });

    this.eventBus.subscribe('consensus:reached', (data, meta) => {
      this._onConsensusReached(data, meta);
    });

    // Conflict events
    this.eventBus.subscribe('conflict:detected', (data, meta) => {
      this._onConflictDetected(data, meta);
    });

    this.eventBus.subscribe('conflict:resolved', (data, meta) => {
      this._onConflictResolved(data, meta);
    });

    // Block events
    this.eventBus.subscribe('danger:blocked', (data, meta) => {
      this._onDangerBlocked(data, meta);
    });

    // Judgment events
    this.eventBus.subscribe(EventType.JUDGMENT_CREATED, (data, meta) => {
      this._onJudgmentCreated(data, meta);
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EVENT HANDLERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Handle orchestration start
   * @private
   */
  _onOrchestrationStart(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    this._currentOrchestration = {
      id: data.decisionId,
      startedAt: Date.now(),
      eventType: data.eventType,
      content: data.content?.substring(0, 50),
    };

    this._votes.clear();
    this._conflicts = [];

    if (this.level === VisibilityLevel.VERBOSE || this.level === VisibilityLevel.DEBUG) {
      this._render(`\n┌─ 🧠 ORCHESTRATION START ─────────────────────────────────┐`);
      this._render(`│ Type: ${data.eventType || 'unknown'}`);
      if (data.content) {
        this._render(`│ Content: ${data.content.substring(0, 50)}...`);
      }
      this._render(`└──────────────────────────────────────────────────────────┘`);
    }
  }

  /**
   * Handle dog vote start (pack is voting)
   * @private
   */
  _onDogVoteStart(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    if (this.level !== VisibilityLevel.COMPACT) {
      this._render(`\n── 🐕 COLLECTIVE VOTING ────────────────────────────────────`);
      this._render(`   Dogs: ${data.dogs?.map(d => DOG_EMOJI[d] || '🐕').join(' ') || '🛡️📊📚🦉🏗️🔮🔍🚀🧹🗺️'}`);
    }
  }

  /**
   * Handle individual dog vote
   * @private
   */
  _onDogVoteCast(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    const dogId = data.dogId;
    const emoji = DOG_EMOJI[dogId] || '🐕';
    const score = data.score ?? '?';
    const response = data.response;
    const confidence = data.confidence ? ` (${(data.confidence * 100).toFixed(0)}%)` : '';

    // Store vote
    this._votes.set(dogId, {
      score,
      response,
      confidence: data.confidence,
      timestamp: Date.now(),
    });

    if (this.level === VisibilityLevel.VERBOSE || this.level === VisibilityLevel.DEBUG) {
      const responseEmoji = response === 'block' ? '🔴' : response === 'ask' ? '🟡' : '🟢';
      this._render(`   ${emoji} ${dogId}: ${score}/100 ${responseEmoji}${confidence}`);
      this.stats.votesShown++;
    }
  }

  /**
   * Handle dog voting complete
   * @private
   */
  _onDogVoteComplete(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    if (this.level === VisibilityLevel.COMPACT) {
      // Compact one-liner
      const voteCount = this._votes.size;
      const avgScore = this._calculateAverageScore();
      this._render(`[🐕 ${voteCount} votes │ Q:${avgScore.toFixed(0)} │ ${data.verdict || '...'}]`);
    }

    this.stats.orchestrationsShown++;
  }

  /**
   * Handle consensus forming (intermediate state)
   * @private
   */
  _onConsensusForming(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    if (this.level === VisibilityLevel.VERBOSE || this.level === VisibilityLevel.DEBUG) {
      const ratio = data.agreementRatio ? `${(data.agreementRatio * 100).toFixed(0)}%` : '...';
      const threshold = data.threshold ? `(need ${(data.threshold * 100).toFixed(0)}%)` : '';
      this._render(`   📊 Consensus: ${ratio} ${threshold}`);
    }
  }

  /**
   * Handle consensus reached
   * @private
   */
  _onConsensusReached(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    const verdict = data.verdict || 'BARK';
    const emoji = VERDICT_EMOJI[verdict] || '🐕';
    const score = data.score ?? '?';
    const ratio = data.agreementRatio ? `${(data.agreementRatio * 100).toFixed(0)}%` : '';

    if (this.level !== VisibilityLevel.COMPACT) {
      this._render(`\n── CONSENSUS REACHED ───────────────────────────────────────`);
      this._render(`   ${emoji} ${verdict} │ Q-Score: ${score}/100 │ Agreement: ${ratio}`);
      if (data.blocked) {
        this._render(`   🔴 ACTION BLOCKED`);
      }
    }
  }

  /**
   * Handle conflict detected
   * @private
   */
  _onConflictDetected(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    this._conflicts.push(data);

    if (this.level === VisibilityLevel.VERBOSE || this.level === VisibilityLevel.DEBUG) {
      const dogA = DOG_EMOJI[data.dogA] || '🐕';
      const dogB = DOG_EMOJI[data.dogB] || '🐕';
      this._render(`   ⚔️ Conflict: ${dogA} ${data.dogA} ↔ ${dogB} ${data.dogB}`);
      this.stats.conflictsShown++;
    }
  }

  /**
   * Handle conflict resolved
   * @private
   */
  _onConflictResolved(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    if (this.level === VisibilityLevel.VERBOSE || this.level === VisibilityLevel.DEBUG) {
      const winner = DOG_EMOJI[data.winner] || '🐕';
      this._render(`   ✅ Resolved: ${winner} ${data.winner} (${data.resolution})`);
    }
  }

  /**
   * Handle danger blocked
   * @private
   */
  _onDangerBlocked(data, meta) {
    // Always show danger blocks (except in silent mode)
    if (this.level === VisibilityLevel.SILENT) return;

    this._render(`\n┌─────────────────────────────────────────────────────────┐`);
    this._render(`│ *GROWL* 🛡️ GUARDIAN BLOCKED                             │`);
    this._render(`├─────────────────────────────────────────────────────────┤`);
    this._render(`│ ${(data.reason || 'Dangerous operation detected').substring(0, 55)}`);
    if (data.severity) {
      this._render(`│ Severity: ${data.severity}`);
    }
    this._render(`└─────────────────────────────────────────────────────────┘`);
  }

  /**
   * Handle orchestration complete
   * @private
   */
  _onOrchestrationComplete(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    const duration = this._currentOrchestration
      ? Date.now() - this._currentOrchestration.startedAt
      : 0;

    if (this.level === VisibilityLevel.DEBUG) {
      this._render(`\n── ORCHESTRATION COMPLETE ──────────────────────────────────`);
      this._render(`   Outcome: ${data.outcome}`);
      this._render(`   Duration: ${duration}ms`);
      this._render(`   Votes: ${this._votes.size}`);
      this._render(`   Conflicts: ${this._conflicts.length}`);
    }

    // Reset state
    this._currentOrchestration = null;
    this._votes.clear();
    this._conflicts = [];
  }

  /**
   * Handle judgment created
   * @private
   */
  _onJudgmentCreated(data, meta) {
    if (this.level === VisibilityLevel.SILENT) return;

    if (this.level !== VisibilityLevel.COMPACT && data.verdict) {
      const emoji = VERDICT_EMOJI[data.verdict] || '🐕';
      this._render(`   ${emoji} Judgment: ${data.verdict} (${data.score || '?'}/100)`);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // RENDERING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Render a line
   * @private
   */
  _render(line) {
    // Emit for external listeners
    this.emit('render', line);

    // Output
    this.output(line);
  }

  /**
   * Calculate average score from current votes
   * @private
   */
  _calculateAverageScore() {
    if (this._votes.size === 0) return 0;

    let sum = 0;
    let count = 0;
    for (const vote of this._votes.values()) {
      if (typeof vote.score === 'number') {
        sum += vote.score;
        count++;
      }
    }

    return count > 0 ? sum / count : 0;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MANUAL RENDERING (for system-reminder integration)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get compact status line for inline display
   *
   * @returns {string} Single-line status
   */
  getCompactStatus() {
    if (!this._currentOrchestration) {
      return '';
    }

    const voteCount = this._votes.size;
    const avgScore = this._calculateAverageScore();
    const conflicts = this._conflicts.length;

    let status = `[🧠 ${voteCount}/11 dogs`;
    if (avgScore > 0) {
      status += ` │ Q:${avgScore.toFixed(0)}`;
    }
    if (conflicts > 0) {
      status += ` │ ⚔️${conflicts}`;
    }
    status += ']';

    return status;
  }

  /**
   * Get full orchestration panel for system-reminder
   *
   * @returns {string} Multi-line panel
   */
  getOrchestrationPanel() {
    if (!this._currentOrchestration) {
      return '';
    }

    const lines = [];
    lines.push('── 🧠 ORCHESTRATION IN PROGRESS ──────────────────────────');

    // Show votes
    if (this._votes.size > 0) {
      const voteLines = [];
      for (const [dogId, vote] of this._votes) {
        const emoji = DOG_EMOJI[dogId] || '🐕';
        const responseEmoji = vote.response === 'block' ? '🔴' : vote.response === 'ask' ? '🟡' : '🟢';
        voteLines.push(`${emoji} ${vote.score || '?'} ${responseEmoji}`);
      }
      lines.push(`   Votes: ${voteLines.join(' │ ')}`);
    }

    // Show consensus progress
    const avgScore = this._calculateAverageScore();
    if (avgScore > 0) {
      lines.push(`   Score: ${avgScore.toFixed(0)}/100`);
    }

    // Show conflicts
    if (this._conflicts.length > 0) {
      lines.push(`   Conflicts: ${this._conflicts.length} pending`);
    }

    return lines.join('\n');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Set visibility level
   * @param {string} level - VisibilityLevel value
   */
  setLevel(level) {
    this.level = level;
    log.debug('Visibility level changed', { level });
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      level: this.level,
      currentOrchestration: !!this._currentOrchestration,
      activeVotes: this._votes.size,
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      orchestrationsShown: 0,
      votesShown: 0,
      conflictsShown: 0,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY & SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create OrchestrationVisibility instance
 *
 * @param {Object} options
 * @returns {OrchestrationVisibility}
 */
export function createOrchestrationVisibility(options) {
  return new OrchestrationVisibility(options);
}

// Singleton instance
let _globalVisibility = null;

/**
 * Get global visibility instance
 *
 * @param {Object} [options] - Options for creation if not exists
 * @returns {OrchestrationVisibility}
 */
export function getOrchestrationVisibility(options) {
  if (!_globalVisibility) {
    _globalVisibility = new OrchestrationVisibility(options);
  }
  return _globalVisibility;
}

export default OrchestrationVisibility;
