#!/usr/bin/env node
/**
 * CYNIC Perceive Hook - UserPromptSubmit
 *
 * "Le chien écoute" - CYNIC perceives and understands
 *
 * This hook runs when the user submits a prompt.
 * It injects relevant context, patterns, and guidance proactively.
 *
 * @event UserPromptSubmit
 * @behavior non-blocking (injects context)
 */

'use strict';

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

// Load Elenchus engine for Socratic questioning (Phase 6B)
const elenchusPath = path.join(__dirname, '..', 'lib', 'elenchus-engine.cjs');
let elenchus = null;
try {
  elenchus = require(elenchusPath);
  elenchus.init();
} catch (e) {
  // Elenchus not available - continue without
}

// Load Chria Database for wisdom injection (Phase 8B)
const chriaPath = path.join(__dirname, '..', 'lib', 'chria-database.cjs');
let chriaDB = null;
try {
  chriaDB = require(chriaPath);
  chriaDB.init();
} catch (e) {
  // Chria database not available - continue without
}

// Load Ti Esti engine for essence questions (Phase 6B)
const tiEstiPath = path.join(__dirname, '..', 'lib', 'ti-esti-engine.cjs');
let tiEsti = null;
try {
  tiEsti = require(tiEstiPath);
  tiEsti.init();
} catch (e) {
  // Ti Esti not available - continue without
}

// Load Definition Tracker for user definitions (Phase 6B)
const definitionPath = path.join(__dirname, '..', 'lib', 'definition-tracker.cjs');
let definitionTracker = null;
try {
  definitionTracker = require(definitionPath);
  definitionTracker.init();
} catch (e) {
  // Definition tracker not available - continue without
}

// Load Fallacy Detector for argument analysis
const fallacyPath = path.join(__dirname, '..', 'lib', 'fallacy-detector.cjs');
let fallacyDetector = null;
try {
  fallacyDetector = require(fallacyPath);
  fallacyDetector.init();
} catch (e) {
  // Fallacy detector not available - continue without
}

// Load Role Reversal for teaching moments
const roleReversalPath = path.join(__dirname, '..', 'lib', 'role-reversal.cjs');
let roleReversal = null;
try {
  roleReversal = require(roleReversalPath);
} catch (e) {
  // Role reversal not available - continue without
}

// Load Hypothesis Testing for claim validation
const hypothesisPath = path.join(__dirname, '..', 'lib', 'hypothesis-testing.cjs');
let hypothesisTesting = null;
try {
  hypothesisTesting = require(hypothesisPath);
} catch (e) {
  // Hypothesis testing not available - continue without
}

// =============================================================================
// INTENT DETECTION
// =============================================================================

const INTENT_PATTERNS = {
  decision: {
    keywords: ['decide', 'should', 'choose', 'which', 'better', 'recommend', 'option'],
    action: 'decision_context'
  },
  architecture: {
    keywords: ['architecture', 'design', 'structure', 'refactor', 'reorganize', 'pattern'],
    action: 'architecture_context'
  },
  danger: {
    keywords: ['delete', 'remove', 'drop', 'force', 'reset', 'rm ', 'wipe', 'destroy'],
    action: 'danger_warning'
  },
  debug: {
    keywords: ['error', 'bug', 'fail', 'broken', 'crash', 'fix', 'doesn\'t work', 'not working'],
    action: 'debug_context'
  },
  learning: {
    keywords: ['how', 'what', 'why', 'explain', 'understand', 'learn', 'teach'],
    action: 'learning_context'
  }
};

function detectIntent(prompt) {
  const promptLower = prompt.toLowerCase();
  const intents = [];

  for (const [intent, config] of Object.entries(INTENT_PATTERNS)) {
    for (const keyword of config.keywords) {
      if (promptLower.includes(keyword)) {
        intents.push({ intent, action: config.action, keyword });
        break;
      }
    }
  }

  return intents;
}

// =============================================================================
// CONTEXT GENERATORS
// =============================================================================

function generateDecisionContext(prompt, profile, patterns) {
  const lines = [];

  // Check for similar past decisions
  const decisions = profile.memory?.decisions || [];
  const recentDecisions = decisions.slice(-5);

  if (recentDecisions.length > 0) {
    lines.push('*ears perk* Past decisions on similar topics:');
    for (const decision of recentDecisions) {
      lines.push(`   • ${decision.summary || 'Decision recorded'}`);
    }
  }

  // Check collective patterns
  const relevantPatterns = patterns.patterns
    .filter(p => p.type === 'decision')
    .slice(0, 3);

  if (relevantPatterns.length > 0) {
    lines.push('Collective wisdom suggests:');
    for (const p of relevantPatterns) {
      lines.push(`   • ${p.description} (seen ${p.count}x)`);
    }
  }

  if (lines.length > 0) {
    lines.push('');
    lines.push('*sniff* Consider all options before deciding.');
  }

  return lines.join('\n');
}

function generateDangerWarning(prompt) {
  const dangerPatterns = [
    { pattern: /rm\s+-rf\s+[\/~]/, level: 'critical', message: 'Recursive deletion from root/home - EXTREMELY dangerous' },
    { pattern: /rm\s+-rf\s+\*/, level: 'critical', message: 'Wildcard deletion - verify scope first' },
    { pattern: /drop\s+(table|database)/i, level: 'critical', message: 'Database deletion is irreversible' },
    { pattern: /delete\s+from\s+\w+\s*;/i, level: 'high', message: 'DELETE without WHERE - affects ALL rows' },
    { pattern: /git\s+push.*--force/, level: 'high', message: 'Force push rewrites remote history' },
    { pattern: /git\s+reset\s+--hard/, level: 'medium', message: 'Hard reset loses uncommitted changes' },
    { pattern: /truncate/i, level: 'high', message: 'TRUNCATE removes all data instantly' }
  ];

  for (const { pattern, level, message } of dangerPatterns) {
    if (pattern.test(prompt)) {
      const prefix = level === 'critical' ? '*GROWL* DANGER' :
                     level === 'high' ? '*growl* Warning' :
                     '*sniff* Caution';
      return `${prefix}: ${message}. Verify before proceeding.`;
    }
  }

  return null;
}

function generateDebugContext(prompt, profile, patterns) {
  const lines = [];

  // Check for similar errors in collective
  const errorPatterns = patterns.patterns
    .filter(p => p.type === 'error')
    .slice(0, 3);

  if (errorPatterns.length > 0) {
    lines.push('*sniff* Similar errors seen before:');
    for (const p of errorPatterns) {
      lines.push(`   • ${p.description}`);
      if (p.solution) {
        lines.push(`     Solution: ${p.solution}`);
      }
    }
  }

  // User-specific error patterns
  const userErrors = profile.patterns?.commonErrors || [];
  if (userErrors.length > 0) {
    lines.push('Your common error patterns:');
    for (const err of userErrors.slice(0, 3)) {
      lines.push(`   • ${err}`);
    }
  }

  return lines.join('\n');
}

function generateArchitectureContext(prompt, profile, patterns) {
  const lines = [];

  // Check collective architecture patterns
  const archPatterns = patterns.patterns
    .filter(p => p.type === 'architecture')
    .slice(0, 3);

  if (archPatterns.length > 0) {
    lines.push('*ears perk* Architecture patterns from collective:');
    for (const p of archPatterns) {
      lines.push(`   • ${p.description}`);
    }
  }

  return lines.join('\n');
}

function generateLearningContext(prompt, profile) {
  const lines = [];

  // Adapt to user's experience level
  const sessions = profile.stats?.sessions || 0;
  const style = profile.preferences?.communicationStyle || 'balanced';

  if (sessions < 5) {
    lines.push('*tail wag* I\'ll explain in detail since we\'re still getting to know each other.');
  } else if (style === 'concise') {
    lines.push('*nod* Brief explanation incoming, as you prefer.');
  }

  return lines.join('\n');
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  try {
    // Read hook context from stdin
    let input = '';
    for await (const chunk of process.stdin) {
      input += chunk;
    }

    const hookContext = JSON.parse(input);
    const prompt = hookContext.prompt || '';

    // Short prompts don't need context injection
    if (prompt.length < 10) {
      console.log(JSON.stringify({ continue: true }));
      return;
    }

    // Detect user and load profile
    const user = cynic.detectUser();
    const profile = cynic.loadUserProfile(user.userId);
    const patterns = cynic.loadCollectivePatterns();

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Consult KETER for routing decision
    // "Le cerveau central décide" - Central brain decides
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    try {
      orchestration = await cynic.orchestrate('user_prompt', {
        content: prompt,
        source: 'perceive_hook',
      }, {
        user: user.userId,
        project: cynic.detectProject(),
      });
    } catch (e) {
      // Orchestration failed - continue with local logic
    }

    // Detect intents
    const intents = detectIntent(prompt);

    // Generate context based on intents
    const injections = [];

    // If orchestrator suggests a specific agent, note it
    if (orchestration?.routing?.suggestedAgent) {
      const sefirah = orchestration.routing.sefirah;
      const agent = orchestration.routing.suggestedAgent;
      // Only inject if intervention level is not silent
      if (orchestration.intervention?.level !== 'silent') {
        injections.push(`── SEFIRAH: ${sefirah} ────────────────────────────────────\n   *sniff* Routing to ${agent}`);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ELENCHUS: Socratic questioning (Phase 6B)
    // "Ἔλεγχος - l'art de questionner pour révéler la vérité"
    // ═══════════════════════════════════════════════════════════════════════════
    if (elenchus && prompt.length > 30) {
      try {
        // Check if this looks like an assertion (not a question)
        const isAssertion = !prompt.trim().endsWith('?') &&
                          !prompt.toLowerCase().startsWith('what') &&
                          !prompt.toLowerCase().startsWith('how') &&
                          !prompt.toLowerCase().startsWith('why') &&
                          !prompt.toLowerCase().startsWith('quoi') &&
                          !prompt.toLowerCase().startsWith('comment') &&
                          !prompt.toLowerCase().startsWith('pourquoi');

        if (isAssertion) {
          const questionResult = elenchus.processAssertion(prompt);

          if (questionResult.shouldAsk) {
            const formattedQuestion = elenchus.formatQuestion(questionResult);
            if (formattedQuestion) {
              injections.push(`── SOCRATIC ───────────────────────────────────────────────\n   ${formattedQuestion}\n   (${questionResult.questionsRemaining} questions restantes)`);
            }
          }
        } else if (elenchus.shouldUseMaieutics(prompt)) {
          // For questions, use maieutic mode - guide to discovery
          const maieutic = elenchus.generateMaieuticQuestion(prompt);
          if (maieutic && Math.random() < 0.382) { // φ⁻² chance
            injections.push(`── MAIEUTIC ───────────────────────────────────────────────\n   💡 ${maieutic}`);
          }
        }
      } catch (e) {
        // Elenchus processing failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TI ESTI: Essence questions (Phase 6B)
    // "Τί ἐστι - Qu'est-ce que c'est?"
    // ═══════════════════════════════════════════════════════════════════════════
    if (tiEsti && prompt.length > 15) {
      try {
        const promptLower = prompt.toLowerCase();
        // Detect "what is X?" style questions
        const isEssenceQuestion = promptLower.match(/^(what is|what's|qu'?est[- ]ce que?|c'?est quoi)\s+/i) ||
                                  promptLower.match(/^(define|défin)/i);

        if (isEssenceQuestion) {
          const essenceResult = tiEsti.investigateConcept(prompt);
          if (essenceResult && essenceResult.dimensions?.length > 0) {
            const dims = essenceResult.dimensions.slice(0, 3).map(d => d.name).join(', ');
            injections.push(`── TI ESTI ────────────────────────────────────────────────\n   🔍 Dimensions to explore: ${dims}\n   ${essenceResult.approach || ''}`);
          }
        }
      } catch (e) {
        // Ti Esti processing failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DEFINITION TRACKER: Track user definitions (Phase 6B)
    // "Les mots dérivent - le chien se souvient"
    // ═══════════════════════════════════════════════════════════════════════════
    if (definitionTracker && prompt.length > 20) {
      try {
        // Check for definition statements: "X means Y", "by X I mean Y", etc.
        const definitionMatch = prompt.match(/(?:by\s+)?["']?(\w+)["']?\s+(?:means?|is|refers?\s+to|I\s+mean)\s+(.+)/i) ||
                                prompt.match(/(?:quand je dis|je définis?)\s+["']?(\w+)["']?\s+(?:comme|c'est)\s+(.+)/i);

        if (definitionMatch) {
          const term = definitionMatch[1];
          const definition = definitionMatch[2].slice(0, 200);
          definitionTracker.recordDefinition(term, definition);
        }

        // Check for definition drift
        const driftResult = definitionTracker.checkForDrift(prompt);
        if (driftResult && driftResult.hasDrift) {
          injections.push(`── DEFINITION DRIFT ───────────────────────────────────────\n   ⚠️ "${driftResult.term}": Earlier you said "${driftResult.previous.slice(0, 50)}..."\n   Now it seems to mean something different?`);
        }
      } catch (e) {
        // Definition tracking failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CHRIA: Wisdom injection (Phase 8B)
    // "Χρεία - la sagesse en peu de mots"
    // ═══════════════════════════════════════════════════════════════════════════
    if (chriaDB && prompt.length > 20 && injections.length === 0) {
      try {
        // Only inject chria when we haven't injected anything else
        // and with φ⁻² probability (38.2%)
        if (Math.random() < 0.382) {
          const contextTags = intents.map(i => i.intent);
          const chria = chriaDB.getContextualChria(contextTags, prompt);
          if (chria) {
            injections.push(`── CHRIA ──────────────────────────────────────────────────\n   📜 "${chria.text}"\n      — ${chria.source}`);
            chriaDB.recordUsage(chria.id);
          }
        }
      } catch (e) {
        // Chria injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // FALLACY DETECTOR: Check for logical fallacies
    // "Le chien renifle les sophismes"
    // ═══════════════════════════════════════════════════════════════════════════
    if (fallacyDetector && prompt.length > 50) {
      try {
        const analysis = fallacyDetector.analyze(prompt);
        if (analysis && analysis.fallacies && analysis.fallacies.length > 0) {
          const topFallacy = analysis.fallacies[0];
          if (topFallacy.confidence > 0.5) {
            injections.push(`── FALLACY ────────────────────────────────────────────────\n   ⚠️ Possible ${topFallacy.name}: ${topFallacy.explanation || ''}\n   Consider: ${topFallacy.remedy || 'Verify the reasoning'}`);
          }
        }
      } catch (e) {
        // Fallacy detection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ROLE REVERSAL: Detect teaching opportunities
    // "Enseigner, c'est apprendre deux fois"
    // ═══════════════════════════════════════════════════════════════════════════
    if (roleReversal && prompt.length > 30 && Math.random() < 0.236) {
      try {
        const opportunity = roleReversal.detectReversalOpportunity(prompt, {});
        if (opportunity && opportunity.shouldReverse) {
          injections.push(`── MAIEUTIC ───────────────────────────────────────────────\n   🎓 ${opportunity.question}\n   (Explaining helps understand deeper)`);
        }
      } catch (e) {
        // Role reversal failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HYPOTHESIS: Track claims and hypotheses
    // "Toute assertion est une hypothèse à tester"
    // ═══════════════════════════════════════════════════════════════════════════
    if (hypothesisTesting && prompt.length > 40) {
      try {
        // Check for assertion patterns that could be hypotheses
        const assertionPatterns = /(?:I think|I believe|probably|likely|should be|must be|always|never)/i;
        if (assertionPatterns.test(prompt) && Math.random() < 0.382) {
          injections.push(`── HYPOTHESIS ─────────────────────────────────────────────\n   🔬 *sniff* This sounds like a hypothesis. What would falsify it?`);
        }
      } catch (e) {
        // Hypothesis detection failed - continue without
      }
    }

    for (const { action } of intents) {
      let context = null;

      switch (action) {
        case 'danger_warning':
          context = generateDangerWarning(prompt);
          break;
        case 'decision_context':
          context = generateDecisionContext(prompt, profile, patterns);
          break;
        case 'debug_context':
          context = generateDebugContext(prompt, profile, patterns);
          break;
        case 'architecture_context':
          context = generateArchitectureContext(prompt, profile, patterns);
          break;
        case 'learning_context':
          context = generateLearningContext(prompt, profile);
          break;
      }

      if (context && context.length > 0) {
        injections.push(context);
      }
    }

    // Check for private content
    const hasPrivate = cynic.hasPrivateContent(prompt);
    const safePrompt = cynic.stripPrivateContent(prompt);

    // If prompt has private content, notify user
    if (hasPrivate) {
      injections.push('*sniff* Private content detected - will NOT be stored in collective memory.');
    }

    // Send to MCP server (non-blocking) - with sanitized prompt
    cynic.sendHookToCollectiveSync('UserPromptSubmit', {
      promptLength: safePrompt.length,
      originalLength: prompt.length,
      hasPrivateContent: hasPrivate,
      intents: intents.map(i => i.intent),
      hasInjections: injections.length > 0,
      // Include orchestration decision
      orchestration: orchestration ? {
        sefirah: orchestration.routing?.sefirah,
        agent: orchestration.routing?.suggestedAgent,
        intervention: orchestration.intervention?.level,
        risk: orchestration.intervention?.actionRisk,
      } : null,
      timestamp: Date.now(),
    });

    // Output result
    if (injections.length === 0) {
      console.log(JSON.stringify({ continue: true }));
    } else {
      console.log(JSON.stringify({
        continue: true,
        message: injections.join('\n\n')
      }));
    }

  } catch (error) {
    // Silent failure - continue without injection
    console.log(JSON.stringify({ continue: true }));
  }
}

main();
