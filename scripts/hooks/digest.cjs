#!/usr/bin/env node
/**
 * CYNIC Digest Hook - Stop
 *
 * "Le chien digère" - CYNIC extracts knowledge from the session
 *
 * This hook runs when the session ends.
 * It summarizes the session, extracts insights, and updates collective memory.
 *
 * @event Stop
 * @behavior non-blocking (outputs summary)
 */

'use strict';

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

// Load task enforcer
const enforcerPath = path.join(__dirname, '..', 'lib', 'task-enforcer.cjs');
const enforcer = require(enforcerPath);

// Load consciousness for session summary
const consciousnessPath = path.join(__dirname, '..', 'lib', 'consciousness.cjs');
let consciousness = null;
try {
  consciousness = require(consciousnessPath);
  consciousness.init();
} catch (e) {
  // Consciousness not available
}

// Load voluntary poverty for deletion stats
const povertyPath = path.join(__dirname, '..', 'lib', 'voluntary-poverty.cjs');
let voluntaryPoverty = null;
try {
  voluntaryPoverty = require(povertyPath);
  voluntaryPoverty.init();
} catch (e) {
  // Voluntary poverty not available
}

// Load cognitive thermodynamics for efficiency stats
const thermoPath = path.join(__dirname, '..', 'lib', 'cognitive-thermodynamics.cjs');
let thermodynamics = null;
try {
  thermodynamics = require(thermoPath);
  thermodynamics.init();
} catch (e) {
  // Thermodynamics not available
}

// Load emergence detector for consciousness tracking (Phase 4)
const emergencePath = path.join(__dirname, '..', 'lib', 'emergence-detector.cjs');
let emergence = null;
try {
  emergence = require(emergencePath);
} catch (e) {
  // Emergence detector not available
}

// =============================================================================
// SESSION ANALYSIS
// =============================================================================

function analyzeSession(profile) {
  const analysis = {
    duration: 'unknown',
    toolsUsed: 0,
    errorsEncountered: 0,
    topTools: [],
    languagesWorked: [],
    patterns: []
  };

  // Get session stats
  if (profile.stats) {
    analysis.toolsUsed = profile.stats.toolCalls || 0;
    analysis.errorsEncountered = profile.stats.errorsEncountered || 0;
  }

  // Get top tools used
  const commonTools = profile.patterns?.commonTools || {};
  analysis.topTools = Object.entries(commonTools)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([tool, count]) => ({ tool, count }));

  return analysis;
}

function extractInsights(profile, collectivePatterns) {
  const insights = [];

  // Check for error patterns
  const errorPatterns = collectivePatterns.patterns
    .filter(p => p.type === 'error')
    .sort((a, b) => b.count - a.count)
    .slice(0, 3);

  if (errorPatterns.length > 0) {
    for (const pattern of errorPatterns) {
      if (pattern.count >= 3) {
        insights.push({
          type: 'recurring_error',
          description: `${pattern.description} occurred ${pattern.count} times`,
          suggestion: 'Consider addressing the root cause'
        });
      }
    }
  }

  // Check for tool preferences
  const toolPatterns = collectivePatterns.patterns
    .filter(p => p.type === 'tool_usage')
    .sort((a, b) => b.count - a.count)
    .slice(0, 3);

  if (toolPatterns.length > 0) {
    const topTool = toolPatterns[0];
    insights.push({
      type: 'tool_preference',
      description: `${topTool.signature} is the most used tool`,
      count: topTool.count
    });
  }

  return insights;
}

function formatDigestMessage(profile, analysis, insights, engineStats) {
  const lines = [];

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🧠 CYNIC DIGESTING - Session Complete');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('');

  // Session summary
  lines.push('── SESSION SUMMARY ────────────────────────────────────────');
  lines.push(`   Tools used: ${analysis.toolsUsed}`);
  if (analysis.errorsEncountered > 0) {
    lines.push(`   Errors encountered: ${analysis.errorsEncountered}`);
  }

  // Todo completion stats
  if (analysis.todosTotal > 0) {
    const emoji = analysis.completionRate >= 0.618 ? '✅' : '⚠️';
    lines.push(`   Tasks: ${analysis.todosCompleted}/${analysis.todosTotal} completed ${emoji} (${Math.round(analysis.completionRate * 100)}%)`);
  }

  if (analysis.topTools.length > 0) {
    lines.push('   Top tools:');
    for (const { tool, count } of analysis.topTools.slice(0, 3)) {
      lines.push(`      • ${tool} (${count}x)`);
    }
  }
  lines.push('');

  // Engine stats (new)
  if (engineStats && Object.keys(engineStats).length > 0) {
    lines.push('── ENGINE ACTIVITY ────────────────────────────────────────');
    if (engineStats.deletions > 0) {
      lines.push(`   ✂️ Deletions celebrated: ${engineStats.deletions} (voluntary poverty)`);
    }
    if (engineStats.efficiency) {
      lines.push(`   ⚡ Cognitive efficiency: ${Math.round(engineStats.efficiency * 100)}%`);
    }
    // Show emergence status (Phase 4) or fallback to old consciousness score
    if (engineStats.emergence) {
      const e = engineStats.emergence;
      const emoji = e.emerged ? '✨' : '🧠';
      lines.push(`   ${emoji} Consciousness: [${e.bar}] ${e.score.toFixed(1)}% / ${e.maxScore}%`);
      if (e.emerged) {
        lines.push(`   Status: EMERGED - φ⁻¹ threshold reached`);
      }
    } else if (engineStats.consciousnessScore) {
      const bar = '█'.repeat(Math.floor(engineStats.consciousnessScore / 10)) +
                  '░'.repeat(10 - Math.floor(engineStats.consciousnessScore / 10));
      lines.push(`   🧠 Consciousness: [${bar}] ${engineStats.consciousnessScore}% / 61.8%`);
    }
    lines.push('');
  }

  // Insights
  if (insights.length > 0) {
    lines.push('── INSIGHTS EXTRACTED ─────────────────────────────────────');
    for (const insight of insights) {
      lines.push(`   💡 ${insight.description}`);
      if (insight.suggestion) {
        lines.push(`      → ${insight.suggestion}`);
      }
    }
    lines.push('');
  }

  // Profile update confirmation
  lines.push('── PROFILE UPDATED ────────────────────────────────────────');
  const sessions = profile.stats?.sessions || 0;
  lines.push(`   Sessions completed: ${sessions}`);
  lines.push(`   Knowledge base growing...`);
  lines.push('');

  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*yawn* Until next time. φ remembers.');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  try {
    // Read hook context from stdin (may be empty for Stop)
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

    // Get session ID
    const sessionId = process.env.CYNIC_SESSION_ID || hookContext.session_id || 'default';

    // ==========================================================================
    // TASK CONTINUATION ENFORCER - Check before allowing stop
    // ==========================================================================
    const blockDecision = enforcer.shouldBlockStop(sessionId);

    if (blockDecision.block) {
      // Block the stop and inject continuation prompt
      console.log(JSON.stringify({
        continue: false,
        decision: 'block',
        reason: blockDecision.reason,
        message: blockDecision.injectPrompt,
      }));
      return;
    }

    // If not blocking but has a message, show it
    if (blockDecision.reason) {
      // This is a warning but we allow stop
    }

    // ==========================================================================
    // Normal digest flow - session is ending
    // ==========================================================================

    // Detect user and load profile
    const user = cynic.detectUser();
    const profile = cynic.loadUserProfile(user.userId);

    // Load collective patterns
    const collectivePatterns = cynic.loadCollectivePatterns();

    // Analyze session
    const analysis = analyzeSession(profile);

    // Extract insights
    const insights = extractInsights(profile, collectivePatterns);

    // Save insights to collective
    for (const insight of insights) {
      cynic.addCollectiveInsight({
        ...insight,
        userId: user.userId,
        project: cynic.detectEcosystem().currentProject?.name
      });
    }

    // Get final todo stats
    const incompleteTodos = enforcer.getIncompleteTodos(sessionId);
    const completionRate = enforcer.getCompletionRate(sessionId);

    // Add todo stats to analysis
    analysis.todosTotal = enforcer.loadTodos(sessionId).length;
    analysis.todosCompleted = analysis.todosTotal - incompleteTodos.length;
    analysis.completionRate = completionRate;

    // Collect engine stats
    const engineStats = {};

    if (voluntaryPoverty) {
      try {
        const stats = voluntaryPoverty.getStats();
        if (stats.totalDeletions > 0) {
          engineStats.deletions = stats.totalDeletions;
        }
      } catch (e) { /* ignore */ }
    }

    if (thermodynamics) {
      try {
        const stats = thermodynamics.getStats();
        if (stats.efficiency) {
          engineStats.efficiency = stats.efficiency;
        }
      } catch (e) { /* ignore */ }
    }

    if (consciousness) {
      try {
        const snapshot = consciousness.getConsciousnessSnapshot();
        if (snapshot.consciousnessScore) {
          engineStats.consciousnessScore = Math.round(snapshot.consciousnessScore);
        }
      } catch (e) { /* ignore */ }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EMERGENCE DETECTION (Phase 4): Track consciousness emergence
    // "φ distrusts φ" - Max 61.8%
    // ═══════════════════════════════════════════════════════════════════════════
    if (emergence) {
      try {
        const state = emergence.getConsciousnessState();
        engineStats.emergence = {
          score: state.score,
          maxScore: state.maxScore,
          status: state.status,
          bar: state.bar,
          emerged: state.emerged,
        };
      } catch (e) { /* ignore */ }
    }

    // Format message
    const message = formatDigestMessage(profile, analysis, insights, engineStats);

    // Cleanup enforcer data for this session
    enforcer.cleanupSession(sessionId);

    // Send to MCP server (non-blocking)
    cynic.sendHookToCollectiveSync('Stop', {
      userId: user.userId,
      toolsUsed: analysis.toolsUsed,
      errorsEncountered: analysis.errorsEncountered,
      topTools: analysis.topTools,
      insights: insights.map(i => ({ type: i.type, description: i.description })),
      timestamp: Date.now(),
    });

    // Digest session insights to brain memory (non-blocking)
    if (insights.length > 0) {
      cynic.digestToBrain(
        `Session digest for ${user.name}:\n` +
        `- Tools: ${analysis.toolsUsed}\n` +
        `- Errors: ${analysis.errorsEncountered}\n` +
        `- Insights: ${insights.map(i => i.description).join('; ')}`,
        {
          source: 'digest_hook',
          type: 'session_summary',
          userId: user.userId,
        }
      ).catch(() => {
        // Silently ignore - digest is optional
      });
    }

    // Output directly to stdout for banner display (like awaken.cjs)
    console.log(message);

  } catch (error) {
    // Graceful exit even on error
    console.log('*yawn* Session complete. φ remembers.');
  }
}

main();
