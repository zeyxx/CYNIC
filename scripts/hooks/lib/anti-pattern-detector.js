/**
 * Anti-Pattern Detector
 *
 * "Le chien détecte les mauvaises habitudes"
 *
 * Detects:
 * - Error loops (same error 3x in 5 min)
 * - File hotspots (same file causing errors)
 * - Edit without Read anti-pattern
 * - Commit without Test anti-pattern
 *
 * Also calculates real rewards for Q-Learning.
 *
 * @module scripts/hooks/lib/anti-pattern-detector
 */

'use strict';

import path from 'path';
import { detectErrorType } from './pattern-detector.js';

/**
 * Session-scoped state for pattern detection
 * Maintains state across tool calls within a session.
 */
export const antiPatternState = {
  // Error tracking (detect loops)
  recentErrors: [],           // Last N errors: { type, file, timestamp }
  errorLoopThreshold: 3,      // Same error 3x = loop
  errorWindowMs: 5 * 60000,   // 5 minute window

  // Tool sequence tracking (detect anti-patterns)
  recentTools: [],            // Last N tools: { name, file, timestamp }
  filesRead: new Set(),       // Files that have been Read
  filesEdited: new Set(),     // Files that have been Edited

  // Test tracking
  lastTestRun: 0,             // Timestamp of last test
  commitsSinceTest: 0,        // Commits without testing

  // Judgment ID tracking (for feedback → training pipeline linkage)
  lastJudgmentId: null,       // Most recent judgment_id from brain_judge
  lastJudgmentAt: 0,          // Timestamp of last judgment
  judgmentIdTTL: 10 * 60000,  // 10 minute TTL — feedback older than this won't link

  // Q-Learning reward signals (Task #20)
  lastSelfModScore: null,     // Self-judge Q-Score for code changes
  lastToolDuration: null,     // Execution time in ms
  matchedPattern: null,       // Detected pattern name if any
  blockedByGuardian: false,   // True if Guardian blocked a dangerous action
  selfModWarnings: [],        // Accumulated self-modification warnings
};

/**
 * Calculate real reward score for Q-Learning
 *
 * Combines multiple signals:
 * - Base: success (70) or failure (30)
 * - Self-judge Q-Score: Replaces base if available (0-100)
 * - Duration penalty: -5 for slow operations (>5s), -10 for very slow (>30s)
 * - Pattern match bonus: +10 for known good patterns, -10 for known bad
 * - Guardian block bonus: +20 (blocking is good safety behavior)
 *
 * Returns score 0-100 which is converted to -1 to 1 by learning-service
 *
 * @param {Object} signals - Reward signals
 * @returns {number} Score 0-100
 */
export function calculateRealReward(signals) {
  const {
    isError,
    toolName,
    toolInput,
    toolOutput,
    selfJudgeScore,
    durationMs,
    patternMatch,
    wasBlocked,
  } = signals;

  // Base score: success = 70, error = 30
  let score = isError ? 30 : 70;

  // Self-judge score REPLACES base score if available
  if (typeof selfJudgeScore === 'number' && selfJudgeScore >= 0) {
    score = selfJudgeScore;
  }

  // Duration penalty for slow operations
  if (typeof durationMs === 'number' && durationMs > 0) {
    if (durationMs > 30000) {
      score = Math.max(0, score - 10); // Very slow (>30s)
    } else if (durationMs > 5000) {
      score = Math.max(0, score - 5);  // Slow (>5s)
    } else if (durationMs < 500 && !isError) {
      score = Math.min(100, score + 3); // Fast + success bonus
    }
  }

  // Pattern match modulation
  if (patternMatch) {
    if (patternMatch.isPositive) {
      score = Math.min(100, score + 10); // Known good pattern
    } else if (patternMatch.isNegative) {
      score = Math.max(0, score - 10);   // Known bad pattern
    }
  }

  // Guardian block is POSITIVE (safety is rewarded)
  if (wasBlocked) {
    score = Math.min(100, score + 20);
  }

  // Tool-specific adjustments: Critical tools get higher stakes
  const criticalTools = ['Write', 'Edit', 'Bash'];
  if (criticalTools.includes(toolName)) {
    const deviation = score - 50;
    score = 50 + deviation * 1.2; // 20% amplification
    score = Math.max(0, Math.min(100, score));
  }

  return Math.round(score);
}

/**
 * Detect anti-patterns in tool usage
 * Returns warnings if anti-patterns detected
 *
 * @param {string} toolName - Name of the tool
 * @param {Object} toolInput - Tool input parameters
 * @param {boolean} isError - Whether the tool errored
 * @param {*} toolOutput - Tool output/error
 * @returns {Object[]} Array of warning objects
 */
export function detectAntiPatterns(toolName, toolInput, isError, toolOutput) {
  const warnings = [];
  const now = Date.now();
  const filePath = toolInput?.file_path || toolInput?.filePath || '';
  const command = toolInput?.command || '';

  // === ERROR LOOP DETECTION ===
  if (isError) {
    const errorText = typeof toolOutput === 'string' ? toolOutput :
                      toolOutput?.error || toolOutput?.message || '';
    const errorType = detectErrorType(errorText);

    // Add to recent errors
    antiPatternState.recentErrors.push({
      type: errorType,
      file: filePath || command.slice(0, 50),
      timestamp: now,
    });

    // Prune old errors
    antiPatternState.recentErrors = antiPatternState.recentErrors.filter(
      e => now - e.timestamp < antiPatternState.errorWindowMs
    );

    // Check for error loop (same error type 3+ times)
    const sameTypeErrors = antiPatternState.recentErrors.filter(e => e.type === errorType);
    if (sameTypeErrors.length >= antiPatternState.errorLoopThreshold) {
      warnings.push({
        type: 'error_loop',
        severity: 'high',
        message: `*GROWL* Tu tournes en rond - "${errorType}" ${sameTypeErrors.length}x en ${Math.round(antiPatternState.errorWindowMs/60000)} min`,
        suggestion: 'Prends du recul. Le problème est peut-être ailleurs.',
        data: { errorType, count: sameTypeErrors.length },
      });
      // Reset to avoid spamming
      antiPatternState.recentErrors = antiPatternState.recentErrors.filter(e => e.type !== errorType);
    }

    // Check for file hotspot (same file causing errors)
    if (filePath) {
      const sameFileErrors = antiPatternState.recentErrors.filter(e => e.file === filePath);
      if (sameFileErrors.length >= 3) {
        warnings.push({
          type: 'file_hotspot',
          severity: 'medium',
          message: `*sniff* Ce fichier pose problème: ${path.basename(filePath)} (${sameFileErrors.length} erreurs)`,
          suggestion: 'Peut-être revoir l\'approche sur ce fichier?',
          data: { file: filePath, count: sameFileErrors.length },
        });
      }
    }
  }

  // === TOOL SEQUENCE ANTI-PATTERNS ===

  // Track tool usage
  antiPatternState.recentTools.push({ name: toolName, file: filePath, timestamp: now });
  if (antiPatternState.recentTools.length > 20) {
    antiPatternState.recentTools.shift();
  }

  // Edit without Read anti-pattern
  if ((toolName === 'Edit' || toolName === 'Write') && filePath) {
    if (!antiPatternState.filesRead.has(filePath)) {
      warnings.push({
        type: 'edit_without_read',
        severity: 'low',
        message: `*ears perk* Edit sans Read: ${path.basename(filePath)}`,
        suggestion: 'Lire avant d\'écrire évite les erreurs.',
        data: { file: filePath },
      });
    }
    antiPatternState.filesEdited.add(filePath);
  }

  // Track reads
  if (toolName === 'Read' && filePath) {
    antiPatternState.filesRead.add(filePath);
  }

  // === COMMIT WITHOUT TEST ===

  // Track test runs
  if (toolName === 'Bash' && command.match(/npm\s+(run\s+)?test|jest|vitest|mocha|pytest/i)) {
    antiPatternState.lastTestRun = now;
    antiPatternState.commitsSinceTest = 0;
  }

  // Track commits
  if (toolName === 'Bash' && command.startsWith('git commit')) {
    antiPatternState.commitsSinceTest++;

    // Warn if multiple commits without testing
    if (antiPatternState.commitsSinceTest >= 2) {
      warnings.push({
        type: 'commit_without_test',
        severity: 'medium',
        message: `*sniff* ${antiPatternState.commitsSinceTest} commits sans test`,
        suggestion: 'npm test avant de continuer?',
        data: { commits: antiPatternState.commitsSinceTest },
      });
    }
  }

  return warnings;
}

/**
 * Reset anti-pattern state (for testing or session reset)
 */
export function resetAntiPatternState() {
  antiPatternState.recentErrors = [];
  antiPatternState.recentTools = [];
  antiPatternState.filesRead.clear();
  antiPatternState.filesEdited.clear();
  antiPatternState.lastTestRun = 0;
  antiPatternState.commitsSinceTest = 0;
  antiPatternState.lastJudgmentId = null;
  antiPatternState.lastJudgmentAt = 0;
  antiPatternState.lastSelfModScore = null;
  antiPatternState.lastToolDuration = null;
  antiPatternState.matchedPattern = null;
  antiPatternState.blockedByGuardian = false;
  antiPatternState.selfModWarnings = [];
}

export default {
  antiPatternState,
  calculateRealReward,
  detectAntiPatterns,
  resetAntiPatternState,
};
