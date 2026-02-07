/**
 * CYNIC Human Perceiver - C5.1 (HUMAN × PERCEIVE)
 *
 * Perceives human state from tool usage, timing patterns, and psychology signals.
 * The dog's nose for human energy.
 *
 * "Le chien sent avant que l'humain ne sache" - κυνικός
 *
 * Perceives:
 * - Energy level (from response times, error rates)
 * - Focus depth (from context switches, tool diversity)
 * - Frustration signals (from retries, rapid edits, undo patterns)
 * - Cognitive load (from concurrent files, task complexity)
 * - Session rhythm (from activity patterns over time)
 *
 * @module @cynic/node/symbiosis/human-perceiver
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('HumanPerceiver');

export const PerceptionSignal = {
  ENERGY: 'energy',
  FOCUS: 'focus',
  FRUSTRATION: 'frustration',
  COGNITIVE_LOAD: 'cognitive_load',
  SESSION_RHYTHM: 'session_rhythm',
};

export const EnergyLevel = {
  HIGH: 'high',
  MEDIUM: 'medium',
  LOW: 'low',
  DEPLETED: 'depleted',
};

export class HumanPerceiver extends EventEmitter {
  constructor(options = {}) {
    super();

    this._psychology = options.psychology || null;

    // Sliding windows for perception
    this._toolTimestamps = [];        // recent tool call timestamps
    this._errorTimestamps = [];       // recent error timestamps
    this._editTimestamps = [];        // recent edit timestamps
    this._filesAccessed = new Set();  // unique files in current window
    this._windowMs = 300000;          // 5 min window
    this._maxSamples = 89;            // Fib(11)

    // Current perceived state
    this._state = {
      energy: PHI_INV,
      focus: PHI_INV,
      frustration: 0,
      cognitiveLoad: 0,
      sessionMinutes: 0,
      lastUpdate: null,
    };

    this._sessionStart = Date.now();

    this._stats = {
      perceptions: 0,
      signalsEmitted: 0,
      lastPerception: null,
    };
  }

  /**
   * Record a tool use event and update perception
   */
  recordToolUse(toolName, durationMs, success) {
    const now = Date.now();
    this._toolTimestamps.push(now);
    this._trimWindow(this._toolTimestamps);

    if (!success) {
      this._errorTimestamps.push(now);
      this._trimWindow(this._errorTimestamps);
    }

    if (toolName === 'Edit' || toolName === 'Write') {
      this._editTimestamps.push(now);
      this._trimWindow(this._editTimestamps);
    }

    if (toolName === 'Read' || toolName === 'Edit' || toolName === 'Write') {
      // Track unique files for cognitive load
      // (file path comes through context, not here — track count as proxy)
      this._filesAccessed.add(`${toolName}_${Math.floor(now / 60000)}`);
    }

    this._updatePerception();
  }

  /**
   * Record a file access for cognitive load tracking
   */
  recordFileAccess(filePath) {
    this._filesAccessed.add(filePath);
  }

  /**
   * Get current perceived state
   */
  perceive() {
    this._updatePerception();
    return { ...this._state };
  }

  _updatePerception() {
    const now = Date.now();

    // Energy: inverse of error rate + response time degradation
    const errorRate = this._errorTimestamps.length / Math.max(1, this._toolTimestamps.length);
    const energy = Math.min(PHI_INV, PHI_INV * (1 - errorRate * 1.5));

    // Focus: inverse of tool diversity in window (many different tools = scattered)
    const toolsPerMinute = this._toolTimestamps.length / Math.max(1, this._windowMs / 60000);
    const focus = toolsPerMinute > 20
      ? PHI_INV_3  // frantic
      : toolsPerMinute > 10
        ? PHI_INV_2  // busy
        : PHI_INV;   // focused

    // Frustration: rapid edits + errors signal frustration
    const recentEdits = this._editTimestamps.filter(t => now - t < 60000).length;
    const recentErrors = this._errorTimestamps.filter(t => now - t < 60000).length;
    const frustration = Math.min(1, (recentEdits > 5 ? 0.3 : 0) + (recentErrors * 0.15));

    // Cognitive load: unique files accessed (Miller's Law: 7±2)
    const uniqueFiles = this._filesAccessed.size;
    const cognitiveLoad = Math.min(9, uniqueFiles); // cap at 9 (Miller max)

    // Session duration
    const sessionMinutes = (now - this._sessionStart) / 60000;

    const previousState = { ...this._state };

    this._state = {
      energy,
      focus,
      frustration,
      cognitiveLoad,
      sessionMinutes,
      energyLevel: energy >= PHI_INV * 0.8 ? EnergyLevel.HIGH
        : energy >= PHI_INV_2 ? EnergyLevel.MEDIUM
          : energy >= PHI_INV_3 ? EnergyLevel.LOW
            : EnergyLevel.DEPLETED,
      lastUpdate: now,
    };

    this._stats.perceptions++;
    this._stats.lastPerception = now;

    // Emit if significant change
    const energyDelta = Math.abs(this._state.energy - previousState.energy);
    const frustrationDelta = Math.abs(this._state.frustration - previousState.frustration);

    if (energyDelta > 0.1 || frustrationDelta > 0.15) {
      this._stats.signalsEmitted++;
      this.emit('perception', this._state);
      globalEventBus.emit('human:perceived', {
        ...this._state,
        cell: 'C5.1',
        timestamp: now,
      });

      log.debug('Human perception updated', {
        energy: energy.toFixed(2),
        focus: focus.toFixed(2),
        frustration: frustration.toFixed(2),
        cognitiveLoad,
      });
    }
  }

  _trimWindow(arr) {
    const cutoff = Date.now() - this._windowMs;
    while (arr.length > 0 && arr[0] < cutoff) arr.shift();
    while (arr.length > this._maxSamples) arr.shift();
  }

  /**
   * Reset for new session
   */
  resetSession() {
    this._toolTimestamps = [];
    this._errorTimestamps = [];
    this._editTimestamps = [];
    this._filesAccessed.clear();
    this._sessionStart = Date.now();
    this._state = {
      energy: PHI_INV,
      focus: PHI_INV,
      frustration: 0,
      cognitiveLoad: 0,
      sessionMinutes: 0,
      lastUpdate: null,
    };
    this.emit('session_reset');
  }

  getStats() { return { ...this._stats }; }

  getHealth() {
    return {
      status: this._state.energy >= PHI_INV_2 ? 'healthy' : 'concerning',
      score: this._state.energy,
      perceptions: this._stats.perceptions,
      currentState: { ...this._state },
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getHumanPerceiver(options = {}) {
  if (!_instance) _instance = new HumanPerceiver(options);
  return _instance;
}

export function resetHumanPerceiver() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { HumanPerceiver, PerceptionSignal, EnergyLevel, getHumanPerceiver, resetHumanPerceiver };
