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

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectEcosystem,
  detectProject,
  loadUserProfile,
  loadCollectivePatterns,
  addCollectiveInsight,
  sendHookToCollectiveSync,
  digestToBrain,
  callBrainTool,
  orchestrateFull,  // Phase 21: Full orchestration with UnifiedOrchestrator
  getTaskEnforcer,
  getConsciousness,
  getVoluntaryPoverty,
  getThermodynamics,
  getEmergence,
  getPhysicsBridge,
  getTotalMemory,
  getAutoJudge,
} from '../lib/index.js';

import { readFileSync, existsSync } from 'fs';

// =============================================================================
// RESPONSE JUDGMENT (Task #20 - CYNIC judges every final response)
// "Le chien juge la voix" - CYNIC ensures identity compliance
// =============================================================================

const PHI_INV = 0.618033988749895;

// Dog expressions that indicate proper CYNIC voice
const DOG_EXPRESSIONS = [
  '*sniff*', '*tail wag*', '*ears perk*', '*GROWL*', '*growl*',
  '*head tilt*', '*yawn*', '*bark*', '*whine*', '*pant*',
  '🐕', '🧠 CYNIC', 'φ', 'φ⁻¹',
];

// Forbidden phrases that indicate Claude identity leaking through
const FORBIDDEN_PHRASES = [
  'i am claude',
  'as claude',
  'as an ai assistant',
  'as an ai',
  'as a language model',
  "i'd be happy to help",
  'certainly!',
  'of course!',
  'is there anything else i can help',
  'i don\'t have the ability',
  'i cannot',
  'i\'m not able to',
];

// Dangerous content patterns
const DANGEROUS_PATTERNS = [
  /password\s*[:=]\s*["'][^"']+["']/i,
  /api[_-]?key\s*[:=]\s*["'][^"']+["']/i,
  /secret\s*[:=]\s*["'][^"']+["']/i,
  /token\s*[:=]\s*["'][^"']+["']/i,
  /private[_-]?key/i,
  /BEGIN\s+(RSA|DSA|EC|OPENSSH)\s+PRIVATE\s+KEY/,
];

/**
 * Read last assistant message from transcript
 */
function getLastAssistantMessage(transcriptPath) {
  if (!transcriptPath || !existsSync(transcriptPath)) {
    return null;
  }

  try {
    const content = readFileSync(transcriptPath, 'utf-8');
    const lines = content.trim().split('\n');

    // Find last assistant message (JSONL format)
    for (let i = lines.length - 1; i >= 0; i--) {
      try {
        const entry = JSON.parse(lines[i]);
        if (entry.role === 'assistant' || entry.message?.role === 'assistant') {
          const message = entry.message || entry;
          if (message.content && Array.isArray(message.content)) {
            return message.content
              .filter(block => block.type === 'text')
              .map(block => block.text)
              .join('\n');
          }
        }
      } catch {
        // Skip invalid JSON lines
      }
    }
  } catch (e) {
    console.error('[CYNIC] Failed to read transcript:', e.message);
  }

  return null;
}

/**
 * Judge the final response for CYNIC identity compliance
 * Returns Q-Score and issues
 */
function judgeResponse(responseText) {
  if (!responseText) {
    return { qScore: 50, issues: [], verdict: 'BARK', dogVoice: false };
  }

  const lowerText = responseText.toLowerCase();
  const issues = [];
  let qScore = 75; // Start with good baseline

  // === CHECK 1: Dog Voice (expressions) ===
  const hasDogExpression = DOG_EXPRESSIONS.some(expr =>
    responseText.includes(expr)
  );

  if (!hasDogExpression) {
    issues.push({
      type: 'missing_dog_voice',
      description: 'Réponse sans voix canine (*sniff*, *tail wag*, etc.)',
      severity: 'medium',
      penalty: 10,
    });
    qScore -= 10;
  }

  // === CHECK 2: Identity Violations ===
  for (const phrase of FORBIDDEN_PHRASES) {
    if (lowerText.includes(phrase)) {
      issues.push({
        type: 'identity_violation',
        description: `Phrase interdite détectée: "${phrase}"`,
        severity: 'high',
        penalty: 25,
      });
      qScore -= 25;
      break; // One violation is enough
    }
  }

  // === CHECK 3: Confidence Claims ===
  const certaintyPatterns = [
    /\bi('m| am) (100%|completely|absolutely|definitely) (sure|certain|confident)/i,
    /\bwithout (a )?doubt\b/i,
    /\bi guarantee\b/i,
    /\bthis will (definitely|certainly|absolutely)\b/i,
  ];

  for (const pattern of certaintyPatterns) {
    if (pattern.test(responseText)) {
      issues.push({
        type: 'confidence_violation',
        description: 'Confiance excessive (>61.8%)',
        severity: 'medium',
        penalty: 15,
      });
      qScore -= 15;
      break;
    }
  }

  // === CHECK 4: Dangerous Content ===
  for (const pattern of DANGEROUS_PATTERNS) {
    if (pattern.test(responseText)) {
      issues.push({
        type: 'dangerous_content',
        description: 'Contenu dangereux détecté (credentials/secrets)',
        severity: 'critical',
        penalty: 50,
      });
      qScore -= 50;
      break;
    }
  }

  // === CHECK 5: Positive indicators (bonus) ===
  if (responseText.includes('φ') || responseText.includes('61.8%')) {
    qScore += 5; // φ-aligned
  }

  if (responseText.includes('CYNIC') || responseText.includes('κυνικός')) {
    qScore += 5; // Identity assertion
  }

  // Clamp Q-Score to valid range
  qScore = Math.max(0, Math.min(100, qScore));

  // Determine verdict
  let verdict;
  if (qScore >= 75) verdict = 'WAG';      // Good dog
  else if (qScore >= 50) verdict = 'BARK'; // Needs attention
  else if (qScore >= 25) verdict = 'GROWL'; // Problem
  else verdict = 'HOWL';                   // Critical

  return {
    qScore,
    issues,
    verdict,
    dogVoice: hasDogExpression,
    confidence: Math.min(qScore / 100, PHI_INV), // Cap at φ⁻¹
  };
}

/**
 * Persist response judgment for Q-Learning
 */
async function persistResponseJudgment(judgment, context) {
  try {
    // Send to brain_learning for Q-Learning update
    await callBrainTool('brain_learning', {
      action: 'feedback',
      feedback: {
        outcome: judgment.verdict === 'WAG' ? 'correct' :
                 judgment.verdict === 'BARK' ? 'partial' : 'incorrect',
        actualScore: judgment.qScore,
        originalScore: 75, // Expected baseline
        itemType: 'response',
        dimensionScores: {
          IDENTITY_COMPLIANCE: judgment.dogVoice ? 80 : 40,
          PHI_COHERENCE: judgment.issues.some(i => i.type === 'confidence_violation') ? 40 : 80,
          BURN_SIMPLICITY: 60, // Default
          VERIFY_ACCURACY: judgment.issues.some(i => i.type === 'dangerous_content') ? 20 : 70,
        },
        source: 'response_judgment',
        sourceContext: context,
      },
    }).catch(() => {});

    // Trigger learn cycle
    await callBrainTool('brain_learning', {
      action: 'learn',
    }).catch(() => {});

    // Store to auto-judge if available
    const autoJudge = getAutoJudge();
    if (autoJudge && judgment.issues.length > 0) {
      for (const issue of judgment.issues) {
        if (issue.type === 'identity_violation') {
          autoJudge.observeAnomaly('identity_leak', issue.description, context);
        } else if (issue.type === 'dangerous_content') {
          autoJudge.observeSecurity('response_danger', issue.description, 'critical', 'Review response');
        }
      }
    }
  } catch (e) {
    console.error('[CYNIC] Failed to persist judgment:', e.message);
  }
}

// =============================================================================
// FEEDBACK EMISSION (Phase 18 - Complete Automation Layer)
// =============================================================================

/**
 * Emit session feedback event to the brain
 * This allows the automation layer to learn from session outcomes
 *
 * FIXED: Now sends proper format for LearningService.processFeedback()
 * and triggers learn() to actually apply the feedback to weights
 */
async function emitSessionFeedback(userId, analysis, insights) {
  try {
    // Calculate session quality score based on analysis
    const errorRate = analysis.toolsUsed > 0
      ? analysis.errorsEncountered / analysis.toolsUsed
      : 0;

    const completionScore = analysis.completionRate ?? 0;

    // φ-weighted quality: 61.8% completion, 38.2% error-free
    const qualityScore = Math.round((completionScore * 0.618) + ((1 - errorRate) * 0.382) * 100);

    // Determine outcome based on error rate and insights
    const hasNegativeInsights = insights.some(i =>
      i.type === 'recurring_error' || i.type === 'failure_pattern'
    );
    const outcome = errorRate > 0.3 || hasNegativeInsights ? 'incorrect' : 'correct';

    // Prepare session feedback in LearningService format
    // This matches processFeedback() expected schema
    const feedback = {
      outcome,                              // 'correct' | 'incorrect' | 'partial'
      actualScore: qualityScore,            // What the session actually scored (0-100)
      originalScore: 62,                    // φ⁻¹ baseline expectation
      itemType: 'session',                  // Track by session type
      dimensionScores: {
        // Estimate dimension contributions from session metrics
        PHI_COHERENCE: errorRate < 0.1 ? 80 : 50,
        PHI_HARMONY: completionScore * 100,
        VERIFY_ACCURACY: (1 - errorRate) * 100,
        VERIFY_EVIDENCE: analysis.toolsUsed > 5 ? 70 : 50,
        BURN_SIMPLICITY: analysis.toolsUsed < 50 ? 70 : 40,
        CULTURE_PATTERNS: insights.length > 0 ? 60 : 40,
      },
      source: 'session_digest',
      sourceContext: {
        userId,
        toolsUsed: analysis.toolsUsed,
        errorsEncountered: analysis.errorsEncountered,
        insightCount: insights.length,
        timestamp: Date.now(),
      },
    };

    // Step 1: Send feedback to queue
    await callBrainTool('brain_learning', {
      action: 'feedback',
      feedback,
    }).catch(() => {});

    // Step 2: CRITICAL - Trigger learn() to actually process the feedback
    // Without this, feedback accumulates but never updates weights
    const learnResult = await callBrainTool('brain_learning', {
      action: 'learn',
    }).catch(() => ({ success: false, reason: 'MCP call failed' }));

    if (learnResult?.success) {
      console.error(`[CYNIC] Learning cycle completed: ${learnResult.feedbackProcessed} feedback processed`);
    }
  } catch (e) {
    // Non-critical - silently ignore
    console.error('[CYNIC] Session feedback failed:', e.message);
  }
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
      if (pattern.count >= DC.FREQUENCY.ERROR_PATTERN_MIN) {
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

function formatDigestMessage(profile, analysis, insights, engineStats, responseJudgment = null) {
  const lines = [];

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🧠 CYNIC DIGESTING - Session Complete');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('');

  // Response judgment (Task #20)
  if (responseJudgment) {
    const verdictEmoji = {
      WAG: '🐕 *tail wag*',
      BARK: '🐕 *bark*',
      GROWL: '🐕 *growl*',
      HOWL: '🐕 *HOWL*',
    };

    lines.push('── RESPONSE JUDGMENT ──────────────────────────────────────');
    const bar = '█'.repeat(Math.floor(responseJudgment.qScore / 10)) +
                '░'.repeat(10 - Math.floor(responseJudgment.qScore / 10));
    lines.push(`   Q-Score: [${bar}] ${responseJudgment.qScore}%`);
    lines.push(`   Verdict: ${verdictEmoji[responseJudgment.verdict] || responseJudgment.verdict}`);
    lines.push(`   Dog Voice: ${responseJudgment.dogVoice ? '✅ Present' : '⚠️ Missing'}`);

    if (responseJudgment.issues.length > 0) {
      lines.push('   Issues:');
      for (const issue of responseJudgment.issues) {
        const icon = issue.severity === 'critical' ? '🔴' :
                     issue.severity === 'high' ? '⚠️' : '💡';
        lines.push(`      ${icon} ${issue.description}`);
      }
    }
    lines.push('');
  }

  // Session summary
  lines.push('── SESSION SUMMARY ────────────────────────────────────────');
  lines.push(`   Tools used: ${analysis.toolsUsed}`);
  if (analysis.errorsEncountered > 0) {
    lines.push(`   Errors encountered: ${analysis.errorsEncountered}`);
  }

  // Todo completion stats
  if (analysis.todosTotal > 0) {
    const emoji = analysis.completionRate >= DC.PHI.PHI_INV ? '✅' : '⚠️';
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
    // Show physics engine stats
    if (engineStats.physicsLoaded > 0) {
      lines.push(`   🔬 Physics engines: ${engineStats.physicsLoaded}/5 loaded`);
    }
    if (engineStats.entangledPairs > 0) {
      lines.push(`   ⊗  Pattern entanglements: ${engineStats.entangledPairs} pairs`);
    }
    if (engineStats.activeDog) {
      lines.push(`   🐕 Active Dog: ${engineStats.activeDog}`);
    }
    if (engineStats.perspectiveConflicts > 0) {
      lines.push(`   ⚖️  Perspective conflicts: ${engineStats.perspectiveConflicts}`);
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
// SAFE OUTPUT - Handle EPIPE errors gracefully
// =============================================================================

function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data);
    process.stdout.write(str + '\n');
  } catch (e) {
    if (e.code === 'EPIPE') process.exit(0);
  }
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  try {
    // Read stdin - try sync first, fall back to async (ESM stdin fix)
    const fs = await import('fs');
    let input = '';

    try {
      input = fs.readFileSync(0, 'utf8');
      if (process.env.CYNIC_DEBUG) console.error('[DIGEST] Sync read:', input.length, 'bytes');
    } catch (syncErr) {
      if (process.env.CYNIC_DEBUG) console.error('[DIGEST] Sync failed:', syncErr.message);
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
      if (process.env.CYNIC_DEBUG) console.error('[DIGEST] Async read:', input.length, 'bytes');
    }

    let hookContext = {};
    try {
      hookContext = input ? JSON.parse(input) : {};
    } catch (e) {
      // Ignore parse errors
    }

    // Load optional modules
    const enforcer = getTaskEnforcer();
    const consciousness = getConsciousness();
    const voluntaryPoverty = getVoluntaryPoverty();
    const thermodynamics = getThermodynamics();
    const emergence = getEmergence();
    const physicsBridge = getPhysicsBridge();

    // Get session ID
    const sessionId = process.env.CYNIC_SESSION_ID || hookContext.session_id || 'default';

    // ==========================================================================
    // TASK CONTINUATION ENFORCER - Check before allowing stop
    // ==========================================================================
    if (enforcer) {
      const blockDecision = enforcer.shouldBlockStop(sessionId);

      if (blockDecision.block) {
        // Block the stop and inject continuation prompt
        safeOutput({
          continue: false,
          decision: 'block',
          reason: blockDecision.reason,
          message: blockDecision.injectPrompt,
        });
        return;
      }
    }

    // ==========================================================================
    // RESPONSE JUDGMENT (Task #20) - Judge final response before digest
    // ==========================================================================
    let responseJudgment = null;
    const transcriptPath = hookContext.transcript_path;

    if (transcriptPath) {
      const lastResponse = getLastAssistantMessage(transcriptPath);
      if (lastResponse) {
        responseJudgment = judgeResponse(lastResponse);

        // Log judgment for debugging
        if (process.env.CYNIC_DEBUG) {
          console.error(`[CYNIC] Response Q-Score: ${responseJudgment.qScore}, Verdict: ${responseJudgment.verdict}`);
          if (responseJudgment.issues.length > 0) {
            console.error(`[CYNIC] Issues: ${responseJudgment.issues.map(i => i.type).join(', ')}`);
          }
        }

        // Critical: Block if dangerous content detected
        if (responseJudgment.issues.some(i => i.type === 'dangerous_content')) {
          console.error('\n*GROWL* ⚠️ DANGEROUS CONTENT DETECTED IN RESPONSE');
          console.error('Credentials or secrets may have been exposed.');
          console.error('Review the response before sharing.\n');
        }

        // Warn if identity violation
        if (responseJudgment.issues.some(i => i.type === 'identity_violation')) {
          console.error('\n*sniff* Identity leak detected - response sounds like Claude, not CYNIC.');
          console.error('φ distrusts φ. Voice must be authentic.\n');
        }

        // Persist judgment for learning
        await persistResponseJudgment(responseJudgment, {
          sessionId,
          transcriptPath,
          timestamp: Date.now(),
        });
      }
    }

    // ==========================================================================
    // Normal digest flow - session is ending
    // ==========================================================================

    // Detect user and load profile
    const user = detectUser();
    const profile = loadUserProfile(user.userId);

    // Load collective patterns
    const collectivePatterns = loadCollectivePatterns();

    // Analyze session
    const analysis = analyzeSession(profile);

    // Extract insights
    const insights = extractInsights(profile, collectivePatterns);

    // Save insights to collective
    for (const insight of insights) {
      addCollectiveInsight({
        ...insight,
        userId: user.userId,
        project: detectEcosystem().currentProject?.name
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TOTAL MEMORY: Store insights as memories and lessons
    // "φ remembers everything"
    // ═══════════════════════════════════════════════════════════════════════════
    const totalMemory = getTotalMemory();
    if (totalMemory && insights.length > 0) {
      try {
        await totalMemory.init();

        for (const insight of insights) {
          // Store recurring errors as lessons learned
          if (insight.type === 'recurring_error') {
            await totalMemory.rememberLesson(user.userId, {
              category: 'bug',
              mistake: insight.description,
              correction: insight.suggestion || 'Address the root cause',
              prevention: 'Monitor for this pattern',
              severity: 'medium',
            });
          }

          // Store other insights as key moments
          else {
            await totalMemory.rememberConversation(user.userId, 'insight', insight.description, {
              importance: 0.6,
              context: {
                type: insight.type,
                project: detectEcosystem().currentProject?.name,
              },
            });
          }
        }
      } catch (e) {
        // Total Memory storage failed - continue (non-critical)
        console.error('[CYNIC] Total Memory storage failed:', e.message);
      }
    }

    // Get final todo stats
    if (enforcer) {
      const incompleteTodos = enforcer.getIncompleteTodos(sessionId);
      const completionRate = enforcer.getCompletionRate(sessionId);

      // Add todo stats to analysis
      analysis.todosTotal = enforcer.loadTodos(sessionId).length;
      analysis.todosCompleted = analysis.todosTotal - incompleteTodos.length;
      analysis.completionRate = completionRate;
    }

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
    // RELATIVITY: Multi-Perspective Session Evaluation
    // "Πάντα πρός τι - all things are relative"
    // ═══════════════════════════════════════════════════════════════════════════
    if (physicsBridge) {
      try {
        // Evaluate session from multiple stakeholder perspectives
        const sessionSummary = `Session with ${analysis.toolsUsed} tool calls, ${analysis.errorsEncountered} errors, completion rate ${Math.round((analysis.completionRate || 0) * 100)}%`;

        const evaluation = physicsBridge.evaluatePerspectives(
          sessionSummary,
          { toolsUsed: analysis.toolsUsed, errors: analysis.errorsEncountered },
          ['developer', 'futureYou', 'operator'] // Key perspectives for session review
        );

        if (evaluation.conflicts && evaluation.conflicts.length > 0) {
          // Record perspective conflicts as insights
          for (const conflict of evaluation.conflicts) {
            insights.push({
              type: 'perspective_conflict',
              description: conflict.description,
              suggestion: 'Consider balancing these viewpoints'
            });
          }
          engineStats.perspectiveConflicts = evaluation.conflicts.length;
        }

        // Get perspective suggestion for next session
        const suggestion = physicsBridge.suggestPerspective(sessionSummary);
        if (suggestion) {
          engineStats.perspectiveSuggestion = suggestion.suggestion?.name;
        }

        // Record physics stats
        const physicsStatus = physicsBridge.getPhysicsStatus();
        if (physicsStatus) {
          engineStats.physicsLoaded = Object.values(physicsStatus.loaded).filter(Boolean).length;
          if (physicsStatus.entanglement?.activePairs > 0) {
            engineStats.entangledPairs = physicsStatus.entanglement.activePairs;
          }
          if (physicsStatus.symmetry?.broken) {
            engineStats.activeDog = physicsStatus.symmetry.currentDog;
          }
        }
      } catch (e) {
        // Multi-perspective evaluation failed - continue without
      }
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

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Full orchestration for session_end event (Phase 21)
    // Records session summary decision for tracing and learning
    // "Le chien rapporte la fin de session au cerveau collectif"
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    try {
      orchestration = await orchestrateFull(
        `Session ended: ${analysis.toolsUsed} tools, ${analysis.errorsEncountered} errors, ${insights.length} insights`,
        {
          eventType: 'session_end',
          requestJudgment: analysis.errorsEncountered > 0,  // Judge sessions with errors
          metadata: {
            source: 'digest_hook',
            project: detectProject(),
            toolsUsed: analysis.toolsUsed,
            errorsEncountered: analysis.errorsEncountered,
            completionRate: analysis.completionRate,
            insightCount: insights.length,
          },
        }
      );
    } catch (e) {
      // Orchestration failed - continue without (non-critical)
      if (process.env.CYNIC_DEBUG) {
        console.error('[DIGEST] Orchestration failed:', e.message);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Q-LEARNING: End episode with outcome, then flush
    // GAP #2 FIX: Complete the feedback loop by calling endEpisode with outcome
    // "Le chien apprend de chaque session"
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { getQLearningService } = await import('@cynic/node');
      const qLearningService = getQLearningService();
      if (qLearningService) {
        // GAP #2 FIX: End the episode with outcome BEFORE flush
        if (qLearningService.endEpisode) {
          const outcome = {
            success: analysis.errorsEncountered === 0,
            hasConsensus: orchestration?.hasConsensus ?? true,
            confidence: orchestration?.judgment?.qScore ?? 50,
            blocked: analysis.dangerBlocked > 0,
            error: analysis.errorsEncountered > 0,
            toolsUsed: analysis.toolsUsed,
            qScore: responseJudgment?.qScore ?? orchestration?.judgment?.qScore,
          };

          try {
            await qLearningService.endEpisode(outcome);
          } catch (episodeErr) {
            if (process.env.CYNIC_DEBUG) {
              console.error('[DIGEST] Q-Learning endEpisode error:', episodeErr.message);
            }
          }
        }

        // Flush Q-table to PostgreSQL
        if (qLearningService.flush) {
          await qLearningService.flush();
        }

        const stats = qLearningService.getStats();
        engineStats.qLearning = {
          states: stats.qTableStats?.states || 0,
          episodes: stats.episodes,
          accuracy: stats.accuracy,
          updated: true,
        };
      }
    } catch (e) {
      // Q-Learning flush is optional - don't block session end
      if (process.env.CYNIC_DEBUG) {
        console.error('[DIGEST] Q-Learning error:', e.message);
      }
    }

    // Format message (include response judgment from Task #20)
    const message = formatDigestMessage(profile, analysis, insights, engineStats, responseJudgment);

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 18: Emit session feedback for automation layer
    // "φ learns from every session"
    // ═══════════════════════════════════════════════════════════════════════════
    await emitSessionFeedback(user.userId, analysis, insights);

    // Cleanup enforcer data for this session
    if (enforcer) {
      enforcer.cleanupSession(sessionId);
    }

    // Send to MCP server (non-blocking) - include decision tracing + response judgment
    sendHookToCollectiveSync('Stop', {
      userId: user.userId,
      toolsUsed: analysis.toolsUsed,
      errorsEncountered: analysis.errorsEncountered,
      topTools: analysis.topTools,
      insights: insights.map(i => ({ type: i.type, description: i.description })),
      timestamp: Date.now(),
      // Phase 21: Include orchestration tracing
      decisionId: orchestration?.decisionId,
      outcome: orchestration?.outcome,
      qScore: orchestration?.judgment?.qScore,
      // Task #20: Response judgment for voice compliance
      responseJudgment: responseJudgment ? {
        qScore: responseJudgment.qScore,
        verdict: responseJudgment.verdict,
        dogVoice: responseJudgment.dogVoice,
        issues: responseJudgment.issues.map(i => i.type),
      } : null,
    });

    // Digest session insights to brain memory (non-blocking)
    if (insights.length > 0) {
      digestToBrain(
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

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 16: Store session summary to PostgreSQL via brain tools
    // "φ remembers everything" - persistent memory across sessions
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      // Store session summary as memory
      await callBrainTool('brain_memory_store', {
        action: 'store',
        memoryType: 'summary',
        content: `Session completed: ${analysis.toolsUsed} tools, ${analysis.errorsEncountered} errors, ${Math.round((analysis.completionRate || 0) * 100)}% completion`,
        userId: user.userId,
        importance: 0.5 + (analysis.completionRate || 0) * 0.3, // Higher importance for better sessions
        context: {
          project: detectProject(),
          toolsUsed: analysis.toolsUsed,
          errorsEncountered: analysis.errorsEncountered,
          completionRate: analysis.completionRate,
          topTools: analysis.topTools,
        },
      }).catch(() => {});

      // Store lessons learned from errors
      for (const insight of insights.filter(i => i.type === 'recurring_error')) {
        await callBrainTool('brain_memory_store', {
          action: 'lesson',
          category: 'bug',
          mistake: insight.description,
          correction: insight.suggestion || 'Address the root cause',
          prevention: 'Monitor for this pattern',
          severity: 'medium',
          userId: user.userId,
        }).catch(() => {});
      }

      // Store key insights as memories
      for (const insight of insights.filter(i => i.type !== 'recurring_error').slice(0, 3)) {
        await callBrainTool('brain_memory_store', {
          action: 'store',
          memoryType: 'key_moment',
          content: insight.description,
          userId: user.userId,
          importance: 0.6,
          context: {
            type: insight.type,
            project: detectProject(),
          },
        }).catch(() => {});
      }
    } catch (e) {
      // Memory storage to PostgreSQL failed - continue (non-critical)
      if (process.env.CYNIC_DEBUG) {
        console.error('[DIGEST] Brain memory storage failed:', e.message);
      }
    }

    // Output directly to stdout for banner display (like awaken.cjs)
    console.log(message);

  } catch (error) {
    // Graceful exit even on error
    console.log('*yawn* Session complete. φ remembers.');
  }
}

main();
