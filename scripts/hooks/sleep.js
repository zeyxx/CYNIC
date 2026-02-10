#!/usr/bin/env node
/**
 * CYNIC Sleep Hook - SessionEnd
 *
 * "Le chien s'endort" - CYNIC closes the session properly
 *
 * This hook runs when the Claude Code session ends.
 * It finalizes the session with brain_session_end, persists stats,
 * and ensures all data is properly saved.
 *
 * OUTPUT: Structured JSON for TUI Protocol (see CLAUDE.md)
 *
 * @event SessionEnd
 * @behavior non-blocking (cleanup)
 */

'use strict';

// ESM imports from the lib bridge
import cynic, {
  detectUser,
  detectProject,
  loadUserProfile,
  orchestrate,
  syncProfileToDB,
  sendHookToCollective,
  endBrainSession,
  callBrainTool,
  getConsciousness,
  getPsychology,
  getTotalMemory,
  getThermodynamics,
} from '../lib/index.js';

// Phase 22: Session state management
import { getSessionState, getTemporalPerception } from './lib/index.js';

// Phase 23: Harmonic Feedback System (learning summary)
import { getHarmonicFeedback, getImplicitFeedback, getSessionPatternsRepository } from './lib/index.js';

// Consciousness read-back: persist calibration summary at session end
import { saveConsciousnessState } from './lib/consciousness-readback.js';

// Context compression: record session quality for outcome verification
import { contextCompressor } from '@cynic/node/services/context-compressor.js';

import fs from 'fs';
import path from 'path';
import os from 'os';

// Load collective dogs for activity summary
import { createRequire } from 'module';
const requireCJS = createRequire(import.meta.url);

let collectiveDogsModule = null;
try {
  collectiveDogsModule = requireCJS('../lib/collective-dogs.cjs');
} catch (e) { /* ignore */ }

// =============================================================================
// RETRY HELPER
// =============================================================================

async function retryWithBackoff(fn, options = {}) {
  const { maxRetries = 3, initialDelay = 100, maxDelay = 2000 } = options;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const result = await fn();
      return { success: true, result };
    } catch (error) {
      const isLastAttempt = attempt === maxRetries;
      const delay = Math.min(initialDelay * Math.pow(2, attempt - 1), maxDelay);

      if (isLastAttempt) {
        return { success: false, error };
      }

      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }

  return { success: false, error: new Error('Max retries exceeded') };
}

// =============================================================================
// SAFE OUTPUT - Handle EPIPE errors gracefully
// =============================================================================

function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
    process.stdout.write(str + '\n');
  } catch (e) {
    // EPIPE: pipe closed before write completed - exit silently
    if (e.code === 'EPIPE') {
      process.exit(0);
    }
    // For other errors, try stderr
    try {
      process.stderr.write(`[sleep] Output failed: ${e.message}\n`);
    } catch { /* ignore */ }
  }
}

// =============================================================================
// SESSION FINALIZATION
// =============================================================================

function getTopTools(profile) {
  const commonTools = profile.patterns?.commonTools || {};
  return Object.entries(commonTools)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([tool, count]) => ({ tool, count }));
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  // Initialize output structure
  const output = {
    type: 'SessionEnd',
    timestamp: new Date().toISOString(),
    user: null,
    session: {
      id: null,
      duration: null,
      durationMinutes: null,
    },
    stats: {
      toolsUsed: 0,
      errorsEncountered: 0,
      dangerBlocked: 0,
    },
    topTools: [],
    thermodynamics: null,
    psychology: null,
    dogsActivity: [],
    syncStatus: {
      profile: null,
      consciousness: null,
      psychology: null,
      totalMemory: null,
      failures: [],
    },
    remoteTotals: null,
  };

  try {
    // Read stdin
    const fs = await import('fs');
    let input = '';

    try {
      input = fs.readFileSync(0, 'utf8');
    } catch (syncErr) {
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
    }

    let hookContext = {};
    try {
      hookContext = input ? JSON.parse(input) : {};
    } catch (e) { /* ignore */ }

    // Load optional modules
    const consciousness = getConsciousness();
    const psychology = getPsychology();
    const thermodynamics = getThermodynamics();

    // Detect user and load profile
    const user = detectUser();
    const profile = loadUserProfile(user.userId);

    output.user = { id: user.userId, name: user.name };
    output.session.id = hookContext.sessionId || process.env.CYNIC_SESSION_ID;

    // Calculate session summary
    const endTime = Date.now();
    const startTime = hookContext.sessionStartTime || endTime - 60000;
    const duration = endTime - startTime;

    output.session.duration = duration;
    output.session.durationMinutes = Math.round(duration / 60000);
    output.stats.toolsUsed = profile.stats?.toolCalls || 0;
    output.stats.errorsEncountered = profile.stats?.errorsEncountered || 0;
    output.stats.dangerBlocked = profile.stats?.dangerBlocked || 0;
    output.topTools = getTopTools(profile);
    output.remoteTotals = profile._remoteTotals || {};

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Notify KETER of session end
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      await orchestrate('session_end', {
        content: 'Session ending',
        source: 'sleep_hook',
        metadata: { sessionId: output.session.id },
      }, {
        user: user.userId,
        project: detectProject(),
      });
    } catch (e) { /* ignore */ }

    // ═══════════════════════════════════════════════════════════════════════════
    // THERMODYNAMICS: Final state
    // ═══════════════════════════════════════════════════════════════════════════
    if (thermodynamics) {
      try {
        const state = thermodynamics.getState();
        output.thermodynamics = {
          heat: state.heat,
          work: state.work,
          efficiency: state.efficiency,
          temperature: state.temperature,
          entropy: state.entropy,
          isCritical: state.isCritical,
        };
      } catch (e) { /* ignore */ }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY: Final state (Task #89: enriched with cognitive load)
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const summary = psychology.getSummary();
        output.psychology = {
          state: summary.overallState,
          energy: Math.round(summary.energy.value * 100),
          focus: Math.round(summary.focus.value * 100),
          // Task #89: Add cognitive load and frustration
          cognitiveLoad: Math.round(summary.cognitiveLoad?.value || 0),
          frustration: Math.round((summary.frustration?.value || 0) * 100),
          composites: summary.composites,
          // Task #89: Compact summary line
          summary: `E:${Math.round(summary.energy.value * 100)}% F:${Math.round(summary.focus.value * 100)}% L:${Math.round(summary.cognitiveLoad?.value || 0)}`,
        };
      } catch (e) { /* ignore */ }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOGS ACTIVITY: Session summary (Task #89: enriched)
    // ═══════════════════════════════════════════════════════════════════════════
    if (collectiveDogsModule?.getDogActivitySummary) {
      try {
        output.dogsActivity = collectiveDogsModule.getDogActivitySummary();

        // Task #89: Extract most active dog
        if (output.dogsActivity?.dogs) {
          const dogs = Object.entries(output.dogsActivity.dogs)
            .map(([name, data]) => ({ name, count: data.count || 0 }))
            .filter(d => d.count > 0)
            .sort((a, b) => b.count - a.count);

          if (dogs.length > 0) {
            const totalActions = dogs.reduce((sum, d) => sum + d.count, 0);
            output.dogsActivity.mostActiveDog = {
              name: dogs[0].name,
              count: dogs[0].count,
              percentage: Math.round((dogs[0].count / totalActions) * 100),
            };
            output.dogsActivity.topDogs = dogs.slice(0, 3).map(d => ({
              name: d.name,
              count: d.count,
              percentage: Math.round((d.count / totalActions) * 100),
            }));
          }
        }
      } catch (e) { /* ignore */ }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC: Profile to PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    const profileResult = await retryWithBackoff(() => syncProfileToDB(user.userId, profile), { maxRetries: 3 });
    if (profileResult.success) {
      output.syncStatus.profile = { success: true };
    } else {
      output.syncStatus.failures.push({ type: 'profile', error: profileResult.error?.message });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC: Consciousness to PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      const consciousnessResult = await retryWithBackoff(() => consciousness.syncToDB(user.userId), { maxRetries: 3 });
      if (consciousnessResult.success) {
        output.syncStatus.consciousness = { success: true };
      } else {
        output.syncStatus.failures.push({ type: 'consciousness', error: consciousnessResult.error?.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC: Psychology to PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      const psychologyResult = await retryWithBackoff(() => psychology.syncToDB(user.userId), { maxRetries: 3 });
      if (psychologyResult.success) {
        output.syncStatus.psychology = { success: true };
      } else {
        output.syncStatus.failures.push({ type: 'psychology', error: psychologyResult.error?.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC: Session Patterns (Cross-session pattern persistence)
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { saveSessionPatterns } = await import('../lib/index.js');
      const sessionState = getSessionState();
      const patterns = sessionState.exportPatterns(true); // Only new patterns (not imported)

      if (patterns.length > 0) {
        const patternResult = await retryWithBackoff(
          () => saveSessionPatterns(output.session.id, user.userId, patterns),
          { maxRetries: 2 }
        );
        if (patternResult.success) {
          output.syncStatus.patterns = { success: true, saved: patternResult.saved };
        } else {
          output.syncStatus.failures.push({ type: 'patterns', error: patternResult.error });
        }
      }
    } catch (e) {
      output.syncStatus.failures.push({ type: 'patterns', error: e.message });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC: Total Memory
    // ═══════════════════════════════════════════════════════════════════════════
    const totalMemory = getTotalMemory();
    if (totalMemory) {
      try {
        await totalMemory.init();

        await totalMemory.storeSessionSummary(user.userId, {
          sessionId: output.session.id,
          toolsUsed: output.stats.toolsUsed,
          errorsEncountered: output.stats.errorsEncountered,
          topTools: output.topTools.map(t => t.tool),
          duration: output.session.duration,
          project: detectProject(),
        });

        await totalMemory.updateGoalProgress(user.userId, {
          toolsUsed: output.stats.toolsUsed,
          errorsEncountered: output.stats.errorsEncountered,
          errorsFixed: output.stats.errorsEncountered > 0 ? 1 : 0,
          testsRun: output.topTools.some(t => t.tool?.includes('test')) ? 1 : 0,
        });

        output.syncStatus.totalMemory = { success: true };
      } catch (e) {
        output.syncStatus.failures.push({ type: 'totalMemory', error: e.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Send SessionEnd to MCP server
    // ═══════════════════════════════════════════════════════════════════════════
    await sendHookToCollective('SessionEnd', {
      userId: user.userId,
      sessionId: output.session.id,
      summary: {
        duration: output.session.duration,
        toolsUsed: output.stats.toolsUsed,
        errorsEncountered: output.stats.errorsEncountered,
        dangerBlocked: output.stats.dangerBlocked,
        topTools: output.topTools.map(t => t.tool),
      },
      timestamp: Date.now(),
    });

    // End brain session
    if (output.session.id) {
      await endBrainSession(output.session.id);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LEARNING: Trigger weight learning from session feedback (Phase 3)
    // "Le chien apprend de chaque session" - Ralph-inspired external validation
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const learnResult = await callBrainTool('brain_learning', {
        action: 'learn',
      }).catch(() => null);

      if (learnResult?.weightAdjustments) {
        output.learning = {
          triggered: true,
          adjustments: Object.keys(learnResult.weightAdjustments).length,
          improvement: learnResult.totalImprovement || 0,
        };
      } else {
        output.learning = { triggered: true, adjustments: 0 };
      }
    } catch (e) {
      output.learning = { triggered: false, error: e.message };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EWC++ CONSOLIDATION: Protect important patterns from forgetting (Task #84)
    // "φ se souvient de ce qui importe" - Fisher importance → pattern locking
    // Lock patterns with fisher ≥ φ⁻¹ (61.8%), prune those < φ⁻³ (23.6%)
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { getPool } = await import('@cynic/persistence');
      const pool = getPool();

      if (pool) {
        // Call consolidation stored procedure if it exists
        const result = await pool.query(`
          SELECT
            count(*) FILTER (WHERE fisher_importance >= 0.618) as locked_patterns,
            count(*) FILTER (WHERE frequency >= 10) as active_patterns,
            avg(fisher_importance) as avg_fisher
          FROM patterns
          WHERE updated_at >= NOW() - INTERVAL '24 hours'
        `).catch(() => ({ rows: [{}] }));

        const stats = result.rows?.[0] || {};

        // Update Fisher importance based on recent frequency
        // Fisher = gradient² ≈ (frequency / max_frequency)² for patterns
        await pool.query(`
          UPDATE patterns SET
            fisher_importance = LEAST(0.999,
              GREATEST(0.001,
                fisher_importance * 0.9 +
                (frequency::float / GREATEST(1, (SELECT MAX(frequency) FROM patterns))) * 0.1
              )
            ),
            updated_at = NOW()
          WHERE frequency > 0
            AND updated_at >= NOW() - INTERVAL '7 days'
        `).catch(() => null);

        output.ewcConsolidation = {
          triggered: true,
          lockedPatterns: parseInt(stats.locked_patterns || 0),
          activePatterns: parseInt(stats.active_patterns || 0),
          avgFisher: parseFloat(stats.avg_fisher || 0).toFixed(4),
        };
      } else {
        output.ewcConsolidation = { triggered: false, reason: 'no_pool' };
      }
    } catch (e) {
      output.ewcConsolidation = { triggered: false, error: e.message };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TEMPORAL PERCEPTION: Save session end time for inter-session gap
    // "Le chien se souvient quand il s'est endormi"
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const temporalPerception = getTemporalPerception();
      const temporalState = temporalPerception.getTemporalState();

      // Save session end time to persistent file
      const cynicDir = path.join(os.homedir(), '.cynic');
      const temporalFile = path.join(cynicDir, 'last-session.json');

      // Ensure directory exists
      if (!fs.existsSync(cynicDir)) {
        fs.mkdirSync(cynicDir, { recursive: true });
      }

      // Save temporal data for next session
      const temporalData = {
        sessionEndTime: endTime,
        sessionId: output.session.id,
        userId: user.userId,
        duration: output.session.duration,
        promptCount: temporalState.promptCount,
        averageIntervalMs: temporalState.averageIntervalMs,
        trend: temporalState.trend,
        worldTime: {
          hour: temporalState.worldTime?.hour,
          dayOfWeek: temporalState.worldTime?.dayOfWeek,
          circadianPhase: temporalState.worldTime?.circadianPhase,
        },
      };

      // Event Ledger: Generate handoff for next session
      try {
        const { getEventLedger } = await import('../lib/event-ledger.js');
        const ledger = getEventLedger();
        const handoff = ledger.generateHandoff();
        if (handoff) {
          temporalData.handoff = handoff;
          output.handoff = {
            generated: true,
            eventCount: handoff.eventCount,
            filesModified: handoff.filesModified?.length || 0,
            unresolvedErrors: handoff.unresolvedErrors?.length || 0,
          };
        }
        // Cleanup old ledger files (> 7 days)
        ledger.cleanup();
      } catch { /* handoff is optional */ }

      fs.writeFileSync(temporalFile, JSON.stringify(temporalData, null, 2));
      output.temporal = { saved: true, endTime };
    } catch (e) {
      output.temporal = { saved: false, error: e.message };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LEARNING SUMMARY: Extract lessons from session (Task #73)
    // "Le chien se souvient de ce qu'il a appris"
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const harmonicFeedback = getHarmonicFeedback();
      const implicitFeedback = getImplicitFeedback();

      // Initialize lessons structure
      output.learningSummary = {
        extracted: true,
        timestamp: Date.now(),
        lessons: [],
        heuristics: {
          promoted: [],
          demoted: [],
          active: 0,
        },
        calibration: {
          brierScore: null,
          reliability: null,
          recommendations: [],
        },
        patterns: {
          mostSuccessful: [],
          needsWork: [],
        },
        feedbackStats: {
          positive: 0,
          negative: 0,
          coherence: null,
        },
      };

      // Extract from Harmonic Feedback System
      if (harmonicFeedback) {
        const state = harmonicFeedback.getState();
        const insights = harmonicFeedback.getInsights();
        const promotionStats = harmonicFeedback.getPromotionStats();
        const calibrationAnalysis = harmonicFeedback.getCalibrationAnalysis();

        // Feedback stats
        output.learningSummary.feedbackStats.coherence = state.coherence;
        output.learningSummary.feedbackStats.resonance = state.resonance;

        // Heuristics
        output.learningSummary.heuristics.active = promotionStats.activeHeuristics;
        output.learningSummary.heuristics.promoted = promotionStats.heuristicDetails
          .filter(h => h.successRate > 0.5)
          .slice(0, 5);

        // Get recently demoted from promotion history
        const recentDemotions = insights.resonanceHistory
          ?.filter(h => h.type === 'demotion')
          ?.slice(-3) || [];
        output.learningSummary.heuristics.demoted = recentDemotions;

        // Calibration
        if (calibrationAnalysis) {
          output.learningSummary.calibration.brierScore = calibrationAnalysis.metrics?.brierScore;
          output.learningSummary.calibration.reliability = calibrationAnalysis.metrics?.reliability;
          output.learningSummary.calibration.factor = calibrationAnalysis.factor;
          output.learningSummary.calibration.recommendations = calibrationAnalysis.recommendations?.recommendations || [];
        }

        // Pattern performance from Thompson sampling
        const thompsonStats = insights.thompsonStats || {};
        if (thompsonStats.arms) {
          const armEntries = Object.entries(thompsonStats.arms);

          // Most successful patterns (high alpha/(alpha+beta))
          const sortedBySuccess = [...armEntries]
            .map(([id, arm]) => ({
              id,
              successRate: arm.alpha / (arm.alpha + arm.beta),
              pulls: arm.pulls,
            }))
            .filter(a => a.pulls >= 5)
            .sort((a, b) => b.successRate - a.successRate);

          output.learningSummary.patterns.mostSuccessful = sortedBySuccess.slice(0, 5);
          output.learningSummary.patterns.needsWork = sortedBySuccess
            .filter(a => a.successRate < 0.4)
            .slice(0, 3);

          // ═══════════════════════════════════════════════════════════════════════
          // Task #89: Thompson convergence and top patterns with expected values
          // "L'humain voit ce que CYNIC a appris CETTE session"
          // ═══════════════════════════════════════════════════════════════════════
          const totalPulls = armEntries.reduce((sum, [, arm]) => sum + (arm.pulls || 0), 0);
          const sortedByEV = [...armEntries]
            .map(([id, arm]) => ({
              id,
              expectedValue: arm.alpha / (arm.alpha + arm.beta),
              pulls: arm.pulls || 0,
            }))
            .sort((a, b) => b.expectedValue - a.expectedValue);

          // Exploitation ratio: how much we're converging to top arm
          const topArmPulls = sortedByEV[0]?.pulls || 0;
          const exploitationRatio = totalPulls > 0 ? topArmPulls / totalPulls : 0;

          output.learningSummary.thompson = {
            totalArms: armEntries.length,
            totalPulls,
            exploitationRatio: Math.round(exploitationRatio * 100),
            convergence: exploitationRatio > 0.5 ? 'high' : exploitationRatio > 0.3 ? 'medium' : 'exploring',
            topPatterns: sortedByEV.slice(0, 5).map(p => ({
              name: p.id,
              ev: Math.round(p.expectedValue * 100),
              pulls: p.pulls,
            })),
          };
        }

        // Generate lessons
        const lessons = [];

        // Lesson 1: Overall coherence
        if (state.coherence > 0.5) {
          lessons.push({
            type: 'positive',
            domain: 'coherence',
            message: `High suggestion-action coherence (${Math.round(state.coherence * 100)}%). Suggestions well-aligned with user needs.`,
          });
        } else if (state.coherence < 0.3) {
          lessons.push({
            type: 'improvement',
            domain: 'coherence',
            message: `Low coherence (${Math.round(state.coherence * 100)}%). Need to better understand user intent.`,
          });
        }

        // Lesson 2: Calibration issues
        if (calibrationAnalysis.recommendations?.recommendations?.length > 0) {
          const mainRec = calibrationAnalysis.recommendations.recommendations[0];
          lessons.push({
            type: mainRec.severity === 'high' ? 'critical' : 'improvement',
            domain: 'calibration',
            message: mainRec.message,
          });
        }

        // Lesson 3: Tikkun progress
        if (state.tikkunProgress > 0.5) {
          lessons.push({
            type: 'positive',
            domain: 'tikkun',
            message: `Good repair progress (${Math.round(state.tikkunProgress * 100)}%). Learning from feedback effectively.`,
          });
        }

        // Lesson 4: Promoted heuristics
        if (promotionStats.activeHeuristics > 0) {
          lessons.push({
            type: 'milestone',
            domain: 'heuristics',
            message: `${promotionStats.activeHeuristics} pattern(s) promoted to heuristics. These can be applied proactively.`,
          });
        }

        output.learningSummary.lessons = lessons;

        // Store learning summary for next session retrieval
        await callBrainTool('brain_memory_store', {
          type: 'session_learning',
          content: JSON.stringify(output.learningSummary),
          metadata: {
            sessionId: output.session.id,
            userId: user.userId,
            timestamp: Date.now(),
          },
        }).catch(() => {});

        // ═══════════════════════════════════════════════════════════════════
        // CONSCIOUSNESS READ-BACK: Write calibration summary for next session
        // perceive.js reads this to know if CYNIC has been well-calibrated
        // "Le chien note sa justesse avant de dormir"
        // ═══════════════════════════════════════════════════════════════════
        const driftDetected = calibrationAnalysis?.recommendations?.recommendations?.some(
          r => r.severity === 'high'
        ) || false;
        saveConsciousnessState({
          lastECE: calibrationAnalysis?.metrics?.brierScore ?? null,
          driftDetected,
          calibrationFactor: calibrationAnalysis?.factor ?? null,
          sessionSelfJudgmentAvg: null, // Will be enriched by observe.js writes
        });
      }

      // Extract from Implicit Feedback
      if (implicitFeedback) {
        const feedbackHistory = implicitFeedback.getHistory?.() || [];
        const positive = feedbackHistory.filter(f => f.sentiment === 'positive').length;
        const negative = feedbackHistory.filter(f => f.sentiment === 'negative').length;

        output.learningSummary.feedbackStats.positive = positive;
        output.learningSummary.feedbackStats.negative = negative;
        output.learningSummary.feedbackStats.total = feedbackHistory.length;
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // Task #66: Persist temporal patterns to PostgreSQL
      // Cross-session learning persistence
      // ═══════════════════════════════════════════════════════════════════════════
      if (harmonicFeedback) {
        try {
          const patternsRepo = getSessionPatternsRepository();
          if (patternsRepo && user.userId) {
            const sessionId = output.session?.id || `session_${Date.now()}`;
            const exportedState = harmonicFeedback.exportState();

            // Save Thompson Sampling arms as patterns
            const patternsToSave = [];

            // 1. Thompson arms → patterns
            if (exportedState.thompson?.arms) {
              for (const [armId, arm] of Object.entries(exportedState.thompson.arms)) {
                const successRate = arm.alpha / (arm.alpha + arm.beta);
                patternsToSave.push({
                  type: 'thompson_arm',
                  name: armId,
                  confidence: Math.min(successRate, 0.618), // φ cap
                  occurrences: arm.pulls,
                  context: {
                    alpha: arm.alpha,
                    beta: arm.beta,
                    expectedValue: arm.expectedValue,
                  },
                });
              }
            }

            // 2. Active heuristics → patterns
            if (exportedState.heuristics) {
              for (const [key, heuristic] of Object.entries(exportedState.heuristics)) {
                patternsToSave.push({
                  type: 'promoted_heuristic',
                  name: key,
                  confidence: Math.min(heuristic.confidence || 0.5, 0.618),
                  occurrences: heuristic.applications || 1,
                  context: {
                    promotedAt: heuristic.promotedAt,
                    source: heuristic.source,
                    pattern: heuristic.pattern,
                  },
                });
              }
            }

            // 3. Calibration state → pattern
            if (exportedState.calibration) {
              patternsToSave.push({
                type: 'calibration_state',
                name: 'confidence_calibrator',
                confidence: exportedState.calibration.currentFactor || 1.0,
                occurrences: exportedState.calibration.predictions?.length || 0,
                context: {
                  buckets: exportedState.calibration.buckets,
                  brierScore: exportedState.calibration.brierScore,
                  factor: exportedState.calibration.currentFactor,
                },
              });
            }

            // Bulk save
            if (patternsToSave.length > 0) {
              const savedCount = await patternsRepo.savePatterns(sessionId, user.userId, patternsToSave);
              output.learningSummary.persisted = {
                patterns: savedCount,
                types: [...new Set(patternsToSave.map(p => p.type))],
              };
            }
          }
        } catch (persistError) {
          output.learningSummary.persistError = persistError.message;
        }
      }

    } catch (e) {
      output.learningSummary = {
        extracted: false,
        error: e.message,
      };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Session State Cleanup
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const sessionState = getSessionState();
      if (sessionState.isInitialized()) {
        output.session.stats = sessionState.getStats();
        sessionState.cleanup();
      }
    } catch (e) { /* ignore */ }

    // ── Persist injection profile (learned activation weights) ────────────
    try {
      const { injectionProfile } = await import('@cynic/node/services/injection-profile.js');
      injectionProfile.stop();
    } catch { /* non-blocking */ }

    // ═══════════════════════════════════════════════════════════════════════════
    // OUTCOME VERIFICATION: Record session quality for compression safety
    // "Le chien note si la compression a nui à la qualité"
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      contextCompressor.start(); // loads state

      // Compute quality score from available signals
      const toolsUsed = output.stats.toolsUsed || 0;
      const errorsEncountered = output.stats.errorsEncountered || 0;
      const errorRate = toolsUsed > 0
        ? Math.min(1, errorsEncountered / toolsUsed)
        : 0;

      const frustration = output.psychology?.frustration
        ? output.psychology.frustration / 100  // normalize from 0-100 to 0-1
        : 0;

      const negativeFeedback = output.learningSummary?.feedbackStats?.negative || 0;
      const positiveFeedback = output.learningSummary?.feedbackStats?.positive || 0;
      const feedbackPenalty = negativeFeedback > positiveFeedback ? 0.3 : 0;

      // quality = 1 - penalties (capped at [0, 1])
      const quality = Math.max(0, Math.min(1,
        1 - (errorRate * 0.4) - (frustration * 0.3) - feedbackPenalty
      ));

      contextCompressor.recordSessionOutcome({ quality, errorRate, frustration });

      const backoff = contextCompressor.getBackoffStatus();
      output.outcomeVerification = {
        quality: Math.round(quality * 100),
        errorRate: Math.round(errorRate * 100),
        frustration: Math.round(frustration * 100),
        backoffActive: backoff.active,
        effectiveLevel: backoff.effectiveLevel,
      };

      contextCompressor.stop();
    } catch { /* non-blocking */ }

    safeOutput(output);

  } catch (error) {
    output.error = error.message;
    safeOutput(output);
  }
}

main();
