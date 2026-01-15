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

function formatDigestMessage(profile, analysis, insights) {
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

  if (analysis.topTools.length > 0) {
    lines.push('   Top tools:');
    for (const { tool, count } of analysis.topTools.slice(0, 3)) {
      lines.push(`      • ${tool} (${count}x)`);
    }
  }
  lines.push('');

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

    // Format message
    const message = formatDigestMessage(profile, analysis, insights);

    // Output directly to stdout for banner display (like awaken.cjs)
    console.log(message);

  } catch (error) {
    // Graceful exit even on error
    console.log('*yawn* Session complete. φ remembers.');
  }
}

main();
