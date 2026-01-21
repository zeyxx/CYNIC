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

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

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

function formatSleepMessage(profile, summary, learningsExport) {
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

  // Learnings export
  if (learningsExport && learningsExport.success) {
    lines.push('── LEARNINGS EXPORTED ─────────────────────────────────────');
    lines.push(`   📚 ${learningsExport.learningsCount || 0} learnings persisted`);
    lines.push(`   📊 Total feedback: ${learningsExport.stats?.totalFeedback || 0}`);
    lines.push(`   📁 Saved to: ${learningsExport.path}`);
    lines.push('');
  }

  // Profile update
  lines.push('── MEMORY PERSISTED ───────────────────────────────────────');
  const sessions = profile.stats?.sessions || 0;
  lines.push(`   Total sessions: ${sessions}`);
  lines.push(`   Profile synced to brain`);
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

    // Detect user and load profile
    const user = cynic.detectUser();
    const profile = cynic.loadUserProfile(user.userId);

    // Calculate session summary
    const summary = calculateSessionSummary(profile, hookContext.sessionStartTime);

    // ═══════════════════════════════════════════════════════════════════════════
    // EXPORT LEARNINGS - Persist accumulated learnings to cynic-learnings.md
    // ═══════════════════════════════════════════════════════════════════════════
    let learningsExport = { success: false };
    try {
      learningsExport = await cynic.exportLearningsToFile();
    } catch (e) {
      // Non-blocking - learnings export failure shouldn't block session end
    }

    // Send SessionEnd to MCP server (this triggers brain_session_end internally)
    await cynic.sendHookToCollective('SessionEnd', {
      userId: user.userId,
      sessionId: hookContext.sessionId,
      summary: {
        duration: summary.duration,
        toolsUsed: summary.toolsUsed,
        errorsEncountered: summary.errorsEncountered,
        dangerBlocked: summary.dangerBlocked,
        topTools: summary.topTools.map(t => t.tool),
      },
      learningsExported: learningsExport.success,
      timestamp: Date.now(),
    });

    // Also explicitly call brain_session_end via HTTP
    const sessionId = hookContext.sessionId || process.env.CYNIC_SESSION_ID;
    if (sessionId) {
      await cynic.endBrainSession(sessionId);
    }

    // Format and output message
    const message = formatSleepMessage(profile, summary, learningsExport);
    console.log(message);

  } catch (error) {
    // Graceful exit even on error
    console.log('*yawn* Session complete. φ remembers.');
  }
}

main();
