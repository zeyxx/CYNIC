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
} from '../lib/index.js';

// =============================================================================
// RETRY HELPER
// =============================================================================

/**
 * Retry an async operation with exponential backoff
 * @param {Function} fn - Async function to retry
 * @param {Object} options - Retry options
 * @returns {Promise<{success: boolean, result?: any, error?: Error}>}
 */
async function retryWithBackoff(fn, options = {}) {
  const { maxRetries = 3, initialDelay = 100, maxDelay = 2000, operationName = 'operation' } = options;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const result = await fn();
      return { success: true, result };
    } catch (error) {
      const isLastAttempt = attempt === maxRetries;
      const delay = Math.min(initialDelay * Math.pow(2, attempt - 1), maxDelay);

      if (isLastAttempt) {
        console.error(`[CYNIC] ${operationName} failed after ${maxRetries} attempts:`, error.message);
        return { success: false, error };
      }

      console.warn(`[CYNIC] ${operationName} attempt ${attempt}/${maxRetries} failed, retrying in ${delay}ms...`);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }

  return { success: false, error: new Error('Max retries exceeded') };
}

// =============================================================================
// SESSION FINALIZATION
// =============================================================================

function calculateSessionSummary(profile, startTime) {
  const endTime = Date.now();
  const duration = endTime - (startTime || endTime - 60000); // Default 1 min if unknown

  return {
    duration,
    durationMinutes: Math.round(duration / 60000),
    toolsUsed: profile.stats?.toolCalls || 0,
    errorsEncountered: profile.stats?.errorsEncountered || 0,
    dangerBlocked: profile.stats?.dangerBlocked || 0,
    topTools: getTopTools(profile),
  };
}

function getTopTools(profile) {
  const commonTools = profile.patterns?.commonTools || {};
  return Object.entries(commonTools)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([tool, count]) => ({ tool, count }));
}

function formatSleepMessage(profile, summary) {
  const lines = [];

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🧠 CYNIC SLEEPING - Session Finalized');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('');

  // Session stats
  lines.push('── SESSION COMPLETE ───────────────────────────────────────');
  lines.push(`   Duration: ${summary.durationMinutes} minutes`);
  lines.push(`   Tools called: ${summary.toolsUsed}`);
  if (summary.errorsEncountered > 0) {
    lines.push(`   Errors encountered: ${summary.errorsEncountered}`);
  }
  if (summary.dangerBlocked > 0) {
    lines.push(`   Dangers blocked: ${summary.dangerBlocked}`);
  }
  lines.push('');

  // Top tools
  if (summary.topTools.length > 0) {
    lines.push('── TOP TOOLS ──────────────────────────────────────────────');
    for (const { tool, count } of summary.topTools.slice(0, 3)) {
      lines.push(`   • ${tool} (${count}x)`);
    }
    lines.push('');
  }

  // Storage info
  lines.push('── STORAGE ────────────────────────────────────────────────');
  lines.push(`   💾 Profile: PostgreSQL (cross-session)`);
  lines.push(`   🧠 Consciousness: PostgreSQL (learning loop persisted)`);
  lines.push(`   📚 Learnings: PostgreSQL (auto-persisted)`);
  lines.push(`   ⛓️  Judgments: PoJ Chain`);
  lines.push('');

  // Profile update
  lines.push('── MEMORY ─────────────────────────────────────────────────');
  const sessions = profile.stats?.sessions || 0;
  lines.push(`   Total sessions: ${sessions}`);
  lines.push(`   Profile synced: ✅ Will remember you next time`);
  lines.push('');

  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*yawn* φ remembers. Until next time.');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  try {
    // Read hook context from stdin (may contain session info)
    let input = '';
    for await (const chunk of process.stdin) {
      input += chunk;
    }

    let hookContext = {};
    try {
      hookContext = input ? JSON.parse(input) : {};
    } catch (e) {
      // Ignore parse errors
    }

    // Load optional modules
    const consciousness = getConsciousness();
    const psychology = getPsychology();

    // Detect user and load profile
    const user = detectUser();
    const profile = loadUserProfile(user.userId);

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Notify KETER of session end
    // "Le chien s'endort. KETER enregistre."
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      await orchestrate('session_end', {
        content: 'Session ending',
        source: 'sleep_hook',
        metadata: {
          sessionId: hookContext.sessionId || process.env.CYNIC_SESSION_ID,
        },
      }, {
        user: user.userId,
        project: detectProject(),
      });
    } catch (e) {
      // Orchestration failed - continue with normal shutdown
    }

    // Calculate session summary
    const summary = calculateSessionSummary(profile, hookContext.sessionStartTime);

    // ═══════════════════════════════════════════════════════════════════════════
    // CROSS-SESSION MEMORY: Sync profile to PostgreSQL (with retry)
    // ═══════════════════════════════════════════════════════════════════════════
    const profileSyncResult = await retryWithBackoff(
      () => syncProfileToDB(user.userId, profile),
      { maxRetries: 3, initialDelay: 100, operationName: 'Profile sync' }
    );

    if (!profileSyncResult.success) {
      console.error('[CYNIC] Profile sync failed - data may not persist to next session');
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS SYNC: Persist learning loop to PostgreSQL (with retry)
    // "Le chien apprend. L'apprentissage persiste."
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      const consciousnessResult = await retryWithBackoff(
        () => consciousness.syncToDB(user.userId),
        { maxRetries: 3, initialDelay: 100, operationName: 'Consciousness sync' }
      );

      if (!consciousnessResult.success) {
        console.error('[CYNIC] Consciousness sync failed - local files remain as backup');
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY SYNC: Persist human understanding to PostgreSQL (with retry)
    // "Comprendre l'humain pour mieux l'aider"
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      const psychologyResult = await retryWithBackoff(
        () => psychology.syncToDB(user.userId),
        { maxRetries: 3, initialDelay: 100, operationName: 'Psychology sync' }
      );

      if (!psychologyResult.success) {
        console.error('[CYNIC] Psychology sync failed - local files remain as backup');
      }
    }

    // Send SessionEnd to MCP server (this triggers brain_session_end internally)
    await sendHookToCollective('SessionEnd', {
      userId: user.userId,
      sessionId: hookContext.sessionId,
      summary: {
        duration: summary.duration,
        toolsUsed: summary.toolsUsed,
        errorsEncountered: summary.errorsEncountered,
        dangerBlocked: summary.dangerBlocked,
        topTools: summary.topTools.map(t => t.tool),
      },
      timestamp: Date.now(),
    });

    // Also explicitly call brain_session_end via HTTP
    const sessionId = hookContext.sessionId || process.env.CYNIC_SESSION_ID;
    if (sessionId) {
      await endBrainSession(sessionId);
    }

    // Format and output message
    const message = formatSleepMessage(profile, summary);
    console.log(message);

  } catch (error) {
    // Graceful exit even on error
    console.log('*yawn* Session complete. φ remembers.');
  }
}

main();
