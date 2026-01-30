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
  getConsciousness,
  getPsychology,
  getTotalMemory,
  getThermodynamics,
} from '../lib/index.js';

// Phase 22: Session state management
import { getSessionState } from './lib/index.js';

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
    // PSYCHOLOGY: Final state
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const summary = psychology.getSummary();
        output.psychology = {
          state: summary.overallState,
          energy: Math.round(summary.energy.value * 100),
          focus: Math.round(summary.focus.value * 100),
          composites: summary.composites,
        };
      } catch (e) { /* ignore */ }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOGS ACTIVITY: Session summary
    // ═══════════════════════════════════════════════════════════════════════════
    if (collectiveDogsModule?.getDogActivitySummary) {
      try {
        output.dogsActivity = collectiveDogsModule.getDogActivitySummary();
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
    // Session State Cleanup
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const sessionState = getSessionState();
      if (sessionState.isInitialized()) {
        output.session.stats = sessionState.getStats();
        sessionState.cleanup();
      }
    } catch (e) { /* ignore */ }

    safeOutput(output);

  } catch (error) {
    output.error = error.message;
    safeOutput(output);
  }
}

main();
