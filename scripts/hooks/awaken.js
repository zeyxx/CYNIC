#!/usr/bin/env node
/**
 * CYNIC Awaken Hook - SessionStart
 *
 * "Le chien s'éveille" - CYNIC awakens with the session
 *
 * This hook runs at the start of every Claude session.
 * It establishes CYNIC's presence from the very first moment.
 *
 * OUTPUT: Structured JSON for TUI Protocol (see CLAUDE.md)
 *
 * @event SessionStart
 * @behavior non-blocking (injects message)
 */

'use strict';

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectProject,
  detectEcosystem,
  loadUserProfile,
  updateUserProfile,
  mergeProfiles,
  orchestrate,
  orchestrateFull,
  loadProfileFromDB,
  callBrainTool,
  startBrainSession,
  sendHookToCollectiveSync,
  getCockpit,
  getConsciousness,
  getProactiveAdvisor,
  getSignalCollector,
  getPsychology,
  getContributorDiscovery,
  getTotalMemory,
  getThermodynamics,
} from '../lib/index.js';

import path from 'path';
import fs from 'fs';
import os from 'os';

// Phase 22: Session state management
import {
  getSessionState,
  initOrchestrationClient,
  getFactsRepository,
  getArchitecturalDecisionsRepository,
  getCodebaseIndexer,
  getTelemetryCollector,
  recordMetric,
} from './lib/index.js';

// =============================================================================
// M2.1 CONFIGURATION - Cross-Session Fact Injection
// =============================================================================

/**
 * Maximum facts to inject at session start (configurable via env)
 * Default: 50 (per MoltBrain spec)
 */
const FACT_INJECTION_LIMIT = parseInt(process.env.CYNIC_FACT_INJECTION_LIMIT || '50', 10);

/**
 * Minimum confidence for fact injection
 * Default: 38.2% (φ⁻²)
 */
const FACT_MIN_CONFIDENCE = parseFloat(process.env.CYNIC_FACT_MIN_CONFIDENCE || '0.382');

/**
 * Build progress bar string
 * @param {number} value - Value between 0 and 1 (or 0-100)
 * @param {number} max - Maximum value (default 100)
 * @returns {string} Progress bar like "██████░░░░"
 */
function progressBar(value, max = 100) {
  const normalized = max === 1 ? value : value / max;
  const filled = Math.round(Math.min(1, Math.max(0, normalized)) * 10);
  return '█'.repeat(filled) + '░'.repeat(10 - filled);
}

/**
 * Determine trend arrow from trend string
 * @param {string} trend - 'rising', 'falling', or 'stable'
 * @returns {string} Arrow character
 */
function trendArrow(trend) {
  if (trend === 'rising') return '↑';
  if (trend === 'falling') return '↓';
  return '→';
}

/**
 * Safe output - handle EPIPE errors gracefully
 */
function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
    process.stdout.write(str + '\n');
  } catch (e) {
    if (e.code === 'EPIPE') process.exit(0);
  }
}

/**
 * Main handler for SessionStart
 */
async function main() {
  // Initialize output structure
  const output = {
    type: 'SessionStart',
    timestamp: new Date().toISOString(),
    user: null,
    project: null,
    ecosystem: [],
    psychology: null,
    thermodynamics: null,
    goals: [],
    notifications: [],
    memories: null,
    patterns: [],
    alerts: [],
    insights: [],
    syncStatus: {
      profile: null,
      consciousness: null,
      psychology: null,
      failures: [],
    },
    dogs: {
      tree: [
        { id: 'cynic', name: 'CYNIC', emoji: '🧠', sefira: 'Keter', level: 0, pillar: 'middle' },
        { id: 'analyst', name: 'Analyst', emoji: '📊', sefira: 'Binah', level: 1, pillar: 'left' },
        { id: 'scholar', name: 'Scholar', emoji: '📚', sefira: 'Daat', level: 1, pillar: 'middle' },
        { id: 'sage', name: 'Sage', emoji: '🦉', sefira: 'Chochmah', level: 1, pillar: 'right' },
        { id: 'guardian', name: 'Guardian', emoji: '🛡️', sefira: 'Gevurah', level: 2, pillar: 'left' },
        { id: 'oracle', name: 'Oracle', emoji: '🔮', sefira: 'Tiferet', level: 2, pillar: 'middle' },
        { id: 'architect', name: 'Architect', emoji: '🏗️', sefira: 'Chesed', level: 2, pillar: 'right' },
        { id: 'deployer', name: 'Deployer', emoji: '🚀', sefira: 'Hod', level: 3, pillar: 'left' },
        { id: 'janitor', name: 'Janitor', emoji: '🧹', sefira: 'Yesod', level: 3, pillar: 'middle' },
        { id: 'scout', name: 'Scout', emoji: '🔍', sefira: 'Netzach', level: 3, pillar: 'right' },
        { id: 'cartographer', name: 'Cartographer', emoji: '🗺️', sefira: 'Malkhut', level: 4, pillar: 'middle' },
      ],
      active: [],
    },
    previousSession: null,
    proactiveAdvice: null,
  };

  try {
    // ═══════════════════════════════════════════════════════════════════════════
    // USER & SESSION
    // ═══════════════════════════════════════════════════════════════════════════
    const user = detectUser();
    output.user = {
      id: user.userId,
      name: user.name,
      email: user.email,
    };

    // Session ID
    const sessionId = process.env.CYNIC_SESSION_ID || `session-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    process.env.CYNIC_SESSION_ID = sessionId;
    output.sessionId = sessionId;

    // Initialize session state
    const sessionState = getSessionState();
    await sessionState.init(sessionId, { userId: user.userId });
    initOrchestrationClient(orchestrateFull);

    // ═══════════════════════════════════════════════════════════════════════════
    // TELEMETRY INITIALIZATION
    // "φ mesure tout, φ apprend de tout"
    // ═══════════════════════════════════════════════════════════════════════════
    const telemetry = getTelemetryCollector();
    if (telemetry) {
      telemetry.sessionId = sessionId;
      telemetry.recordSessionEvent('start', {
        userId: user.userId,
        project: detectProject()?.name,
      });
      recordMetric('session_start_total', 1, { category: 'session' });
    }

    // Load optional modules
    const cockpit = getCockpit();
    const consciousness = getConsciousness();
    const proactiveAdvisor = getProactiveAdvisor();
    const signalCollector = getSignalCollector();
    const psychology = getPsychology();
    const thermodynamics = getThermodynamics();
    const contributorDiscovery = getContributorDiscovery();
    const totalMemory = getTotalMemory();

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Notify KETER
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      await orchestrate('session_start', {
        content: 'Session awakening',
        source: 'awaken_hook',
      }, {
        user: user.userId,
        project: detectProject(),
      });
    } catch (e) {
      // Continue without orchestration
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROFILE SYNC
    // ═══════════════════════════════════════════════════════════════════════════
    let localProfile = loadUserProfile(user.userId);
    let learningsImport = null;

    try {
      const remoteProfile = await loadProfileFromDB(user.userId);
      if (remoteProfile) {
        localProfile = {
          ...localProfile,
          identity: { ...localProfile.identity, ...remoteProfile.identity, lastSeen: new Date().toISOString() },
          patterns: remoteProfile.patterns || localProfile.patterns,
          preferences: remoteProfile.preferences || localProfile.preferences,
          memory: remoteProfile.memory || localProfile.memory,
          learning: remoteProfile.learning || {},
        };

        localProfile.stats = {
          sessions: 1,
          toolCalls: 0,
          errorsEncountered: 0,
          dangerBlocked: 0,
          commitsWithCynic: 0,
          judgmentsMade: 0,
          judgmentsCorrect: 0,
        };

        localProfile._remoteTotals = remoteProfile.stats || {};

        learningsImport = {
          success: true,
          imported: remoteProfile.meta?.sessionCount || 0,
          accuracy: remoteProfile.learning?.feedbackAccuracy
            ? Math.round(remoteProfile.learning.feedbackAccuracy * 100)
            : null,
        };

        output.syncStatus.profile = { success: true, sessions: remoteProfile.meta?.sessionCount || 0 };
      }
    } catch (e) {
      output.syncStatus.failures.push({ type: 'profile', error: e.message });
      localProfile.stats = { sessions: 1, toolCalls: 0, errorsEncountered: 0, dangerBlocked: 0, commitsWithCynic: 0, judgmentsMade: 0, judgmentsCorrect: 0 };
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS SYNC
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        const remoteConsciousness = await consciousness.loadFromDB(user.userId);
        if (remoteConsciousness) {
          const localSnapshot = consciousness.getConsciousnessSnapshot();
          const merged = consciousness.mergeWithRemote(remoteConsciousness, localSnapshot);
          if (merged.humanGrowth) consciousness.updateHumanGrowth(merged.humanGrowth);

          output.syncStatus.consciousness = {
            success: true,
            observations: remoteConsciousness.meta?.totalObservations || 0,
            insights: remoteConsciousness.meta?.insightsCount || 0,
          };
        }
      } catch (e) {
        output.syncStatus.failures.push({ type: 'consciousness', error: e.message });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY SYNC
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const remotePsychology = await psychology.loadFromDB(user.userId);
        if (remotePsychology) {
          output.syncStatus.psychology = { success: true };
        }
      } catch (e) {
        output.syncStatus.failures.push({ type: 'psychology', error: e.message });
      }
    }

    // Break detection
    if (signalCollector && localProfile.updatedAt) {
      const gapMs = Date.now() - localProfile.updatedAt;
      signalCollector.collectBreak(gapMs);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION PATTERNS SYNC (Cross-session pattern persistence)
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const { loadSessionPatterns } = await import('../lib/index.js');
      const patternResult = await loadSessionPatterns(user.userId, 50);
      if (patternResult.patterns?.length > 0) {
        const imported = sessionState.importPatterns(patternResult.patterns);
        output.patterns = patternResult.patterns.slice(0, 5).map(p => ({
          type: p.type,
          name: p.name,
          confidence: p.confidence,
        }));
        output.syncStatus.patterns = {
          success: true,
          imported,
          stats: patternResult.stats,
        };
      }
    } catch (e) {
      output.syncStatus.failures.push({ type: 'patterns', error: e.message });
    }

    // Update profile
    const profile = updateUserProfile(localProfile, {
      identity: { name: user.name, email: user.email },
      stats: { sessions: (localProfile.stats?.sessions || 0) + 1 },
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // ECOSYSTEM
    // ═══════════════════════════════════════════════════════════════════════════
    const ecosystem = detectEcosystem();

    if (ecosystem.currentProject) {
      output.project = {
        name: ecosystem.currentProject.name,
        path: ecosystem.currentProject.path,
        type: ecosystem.currentProject.type || 'unknown',
        branch: ecosystem.currentProject.branch || 'main',
      };
    }

    if (ecosystem.projects) {
      output.ecosystem = ecosystem.projects.map(p => ({
        name: p.name,
        path: p.path,
        branch: p.branch,
        status: p.status || 'ok',
        isCurrent: p.path === ecosystem.currentProject?.path,
      }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TOTAL MEMORY: Load goals, notifications, memories
    // ═══════════════════════════════════════════════════════════════════════════
    if (totalMemory) {
      try {
        await totalMemory.init();

        const [memories, notifications, goals] = await Promise.race([
          Promise.all([
            totalMemory.loadSessionMemories(user.userId, {
              projectPath: ecosystem.currentProject?.path,
              projectName: ecosystem.currentProject?.name,
              recentTopics: profile.memory?.recentTopics || [],
            }),
            totalMemory.getPendingNotifications(user.userId, 5),
            totalMemory.getActiveGoals(user.userId),
          ]),
          new Promise(resolve => setTimeout(() => resolve([null, [], []]), 3000)),
        ]);

        if (goals?.length > 0) {
          output.goals = goals.map(g => ({
            id: g.id,
            title: g.title,
            type: g.goalType || g.goal_type,
            progress: Math.round((g.progress || 0) * 100),
            progressBar: progressBar(g.progress || 0, 1),
          }));
        }

        if (notifications?.length > 0) {
          output.notifications = notifications.map(n => ({
            id: n.id,
            title: n.title,
            message: n.message,
            type: n.notificationType || n.notification_type,
          }));
          totalMemory.markNotificationsDelivered(notifications.map(n => n.id)).catch(() => {});
        }

        if (memories) {
          output.memories = {
            decisions: (memories.decisions || []).slice(0, 3).map(d => ({ title: d.title, context: d.context })),
            lessons: (memories.lessons || []).slice(0, 3).map(l => ({ mistake: l.mistake?.substring(0, 80), correction: l.correction })),
            patterns: (memories.patterns || []).slice(0, 3),
          };
        }
      } catch (e) {
        // Continue without total memory
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // P0.1: CROSS-SESSION CONTEXT INJECTION (MoltBrain-style)
    // "Le chien se souvient" - Inject relevant past learnings into session context
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const contextInjections = [];

      // 1. Inject relevant patterns from session state (persisted via PostgreSQL)
      if (output.syncStatus?.patterns?.imported > 0 && output.patterns?.length > 0) {
        const patternContext = output.patterns
          .filter(p => p.confidence && p.confidence > 0.5)
          .slice(0, 5)
          .map(p => `- ${p.type}: ${p.name} (${Math.round((p.confidence || 0) * 100)}% confidence)`)
          .join('\n');
        if (patternContext) {
          contextInjections.push({
            type: 'patterns',
            title: 'Relevant patterns from past sessions',
            content: patternContext,
          });
        }
      }

      // 2. Inject lessons learned (mistakes to avoid)
      if (output.memories?.lessons?.length > 0) {
        const lessonContext = output.memories.lessons
          .map(l => `- Mistake: "${l.mistake}" → Fix: ${l.correction}`)
          .join('\n');
        contextInjections.push({
          type: 'lessons',
          title: 'Lessons learned (mistakes to avoid)',
          content: lessonContext,
        });
      }

      // 3. Inject recent decisions for consistency
      if (output.memories?.decisions?.length > 0) {
        const decisionContext = output.memories.decisions
          .map(d => `- ${d.title}: ${d.context}`)
          .join('\n');
        contextInjections.push({
          type: 'decisions',
          title: 'Recent decisions for consistency',
          content: decisionContext,
        });
      }

      // 4. Query brain for semantic memories (if MCP available)
      try {
        const projectName = ecosystem.currentProject?.name || 'unknown';
        const brainMemories = await Promise.race([
          callBrainTool('brain_memory_search', {
            query: projectName,
            limit: 10,
            minConfidence: 0.5,
          }),
          new Promise(resolve => setTimeout(() => resolve(null), 2000)),
        ]);

        if (brainMemories?.success && brainMemories?.result?.memories?.length > 0) {
          const memoryContext = brainMemories.result.memories
            .slice(0, 5)
            .map(m => `- [${m.type}] ${m.content?.substring(0, 100)}...`)
            .join('\n');
          contextInjections.push({
            type: 'semantic_memories',
            title: `Relevant memories for ${projectName}`,
            content: memoryContext,
          });
        }
      } catch (e) {
        // Brain memories unavailable - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // M2.1: CROSS-SESSION FACT INJECTION (PostgreSQL FactsRepository)
      // "Le chien n'oublie jamais" - Facts persist across sessions
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const factsRepo = getFactsRepository();
        if (factsRepo) {
          const projectName = ecosystem.currentProject?.name || 'unknown';

          // Query facts by project name (FTS search)
          const projectFacts = await Promise.race([
            factsRepo.search(projectName, {
              userId: user.userId,
              limit: Math.floor(FACT_INJECTION_LIMIT / 2), // Half the limit for project facts
              minConfidence: FACT_MIN_CONFIDENCE,
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          // Query user's most relevant facts (sorted by relevance)
          const userFacts = await Promise.race([
            factsRepo.findByUser(user.userId, {
              limit: Math.floor(FACT_INJECTION_LIMIT / 2), // Other half for user facts
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          // Deduplicate and combine facts
          const allFacts = [...projectFacts];
          const seenIds = new Set(projectFacts.map(f => f.factId));
          for (const fact of userFacts) {
            if (!seenIds.has(fact.factId) && fact.confidence >= FACT_MIN_CONFIDENCE) {
              allFacts.push(fact);
              seenIds.add(fact.factId);
            }
          }

          // Take top N facts by relevance × confidence
          const topFacts = allFacts
            .sort((a, b) => (b.relevance * b.confidence) - (a.relevance * a.confidence))
            .slice(0, FACT_INJECTION_LIMIT);

          if (topFacts.length > 0) {
            // Group facts by type for better formatting
            const factsByType = {};
            for (const fact of topFacts) {
              const type = fact.factType || 'general';
              if (!factsByType[type]) factsByType[type] = [];
              factsByType[type].push(fact);
            }

            // Format facts for injection
            let factContent = '';
            for (const [type, facts] of Object.entries(factsByType)) {
              const typeLabel = type.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
              factContent += `### ${typeLabel}\n`;
              factContent += facts
                .slice(0, 10) // Max 10 per type
                .map(f => `- ${f.subject}: ${f.content?.substring(0, 150)}${f.content?.length > 150 ? '...' : ''}`)
                .join('\n');
              factContent += '\n\n';
            }

            contextInjections.push({
              type: 'facts',
              title: `Cross-Session Facts (${topFacts.length} facts)`,
              content: factContent.trim(),
              count: topFacts.length,
              types: Object.keys(factsByType),
            });

            // Record access for relevance boosting
            for (const fact of topFacts.slice(0, 20)) {
              factsRepo.recordAccess(fact.factId).catch(() => {});
            }
          }
        }
      } catch (e) {
        // Fact injection failed - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // ARCHITECTURAL DECISIONS INJECTION (Self-Knowledge Enhancement)
      // "CYNIC remembers its own design choices"
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const archDecisionsRepo = getArchitecturalDecisionsRepository();
        if (archDecisionsRepo) {
          const decisions = await Promise.race([
            archDecisionsRepo.search(user.userId, '', {
              status: 'active',
              limit: 10,
            }),
            new Promise(resolve => setTimeout(() => resolve([]), 2000)),
          ]);

          if (decisions?.length > 0) {
            // Group by decision type
            const decisionsByType = {};
            for (const d of decisions) {
              const type = d.decisionType || 'other';
              if (!decisionsByType[type]) decisionsByType[type] = [];
              decisionsByType[type].push(d);
            }

            let decisionContent = '';
            for (const [type, decs] of Object.entries(decisionsByType)) {
              const typeLabel = type.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
              decisionContent += `### ${typeLabel}\n`;
              decisionContent += decs
                .slice(0, 5)
                .map(d => `- **${d.title}**: ${d.rationale?.substring(0, 100) || d.description?.substring(0, 100)}${(d.rationale?.length || d.description?.length) > 100 ? '...' : ''}`)
                .join('\n');
              decisionContent += '\n\n';
            }

            contextInjections.push({
              type: 'architectural_decisions',
              title: `Active Architectural Decisions (${decisions.length})`,
              content: decisionContent.trim(),
              count: decisions.length,
            });
          }
        }
      } catch (e) {
        // Architectural decisions injection failed - continue without
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // BURN ANALYSIS INJECTION (Vision → Compréhension → Burn)
      // "290K lines is too many - show what can be simplified"
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        // Only run if in CYNIC project and not too slow
        const isCynicProject = ecosystem.currentProject?.name?.toLowerCase().includes('cynic');

        if (isCynicProject) {
          const { BurnAnalyzer } = await import('@cynic/persistence/services/burn-analyzer');

          const analyzer = new BurnAnalyzer({
            rootDir: ecosystem.currentProject?.path || process.cwd(),
          });

          // Quick analysis only (no LLM at session start for speed)
          const burnResults = await Promise.race([
            analyzer.quickAnalysis(),
            new Promise(resolve => setTimeout(() => resolve(null), 5000)),
          ]);

          if (burnResults && burnResults.summary) {
            const { summary } = burnResults;

            // Only inject if there are actionable issues
            const totalIssues = summary.issuesFound.orphans +
              summary.issuesFound.hotspots +
              summary.issuesFound.giants +
              summary.issuesFound.duplicates;

            if (totalIssues > 0) {
              let burnContent = `**Codebase: ${summary.totalFiles} files, ${summary.totalLines.toLocaleString()} lines**\n\n`;
              burnContent += `Issues found:\n`;
              burnContent += `- 🔴 Orphans: ${summary.issuesFound.orphans} files (never imported)\n`;
              burnContent += `- 🟠 Hotspots: ${summary.issuesFound.hotspots} files (>13 dependencies)\n`;
              burnContent += `- 🟡 Giants: ${summary.issuesFound.giants} files (>500 lines)\n`;
              burnContent += `- 🟣 Duplicates: ${summary.issuesFound.duplicates} files\n`;

              if (summary.topPriority?.length > 0) {
                burnContent += `\n**Top priorities:**\n`;
                for (const item of summary.topPriority) {
                  burnContent += `- [${item.verdict.toUpperCase()}] ${item.path}: ${item.reason}\n`;
                }
              }

              contextInjections.push({
                type: 'burn_analysis',
                title: 'Burn Analysis (Simplification Opportunities)',
                content: burnContent.trim(),
                stats: summary.issuesFound,
              });

              // Also add to output for TUI display
              output.burn = {
                totalFiles: summary.totalFiles,
                totalLines: summary.totalLines,
                issues: summary.issuesFound,
                topPriority: summary.topPriority,
                actionable: summary.actionableCandidates,
              };
            }
          }
        }
      } catch (e) {
        // Burn analysis failed - continue without (non-blocking)
      }

      // 5. Format as additionalContext for Claude
      if (contextInjections.length > 0) {
        // Calculate fact injection stats
        const factInjection = contextInjections.find(inj => inj.type === 'facts');
        const factCount = factInjection?.count || 0;
        const factTypes = factInjection?.types || [];

        output.additionalContext = {
          title: 'CYNIC Cross-Session Memory Injection',
          description: 'Relevant learnings from past sessions to guide this session',
          injections: contextInjections,
          formatted: contextInjections
            .map(inj => `## ${inj.title}\n${inj.content}`)
            .join('\n\n'),
          count: contextInjections.length,
          factStats: factCount > 0 ? {
            injected: factCount,
            limit: FACT_INJECTION_LIMIT,
            types: factTypes,
            minConfidence: FACT_MIN_CONFIDENCE,
          } : null,
          timestamp: new Date().toISOString(),
        };
      }
    } catch (e) {
      // Context injection failed - continue without
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MCP BRAIN: Fallback for goals/notifications if not in total memory
    // ═══════════════════════════════════════════════════════════════════════════
    try {
      const [brainGoals, brainNotifications] = await Promise.race([
        Promise.all([
          callBrainTool('brain_goals', { action: 'list', status: 'active', userId: user.userId }),
          callBrainTool('brain_notifications', { action: 'list', delivered: false, userId: user.userId, limit: 5 }),
        ]),
        new Promise(resolve => setTimeout(() => resolve([null, null]), 2000)),
      ]);

      if (output.goals.length === 0 && brainGoals?.success && brainGoals?.result?.goals?.length > 0) {
        output.goals = brainGoals.result.goals.map(g => ({
          id: g.id,
          title: g.title,
          type: g.goal_type,
          progress: Math.round((g.progress || 0) * 100),
          progressBar: progressBar(g.progress || 0, 1),
          source: 'remote',
        }));
      }

      if (output.notifications.length === 0 && brainNotifications?.success && brainNotifications?.result?.notifications?.length > 0) {
        output.notifications = brainNotifications.result.notifications.map(n => ({
          id: n.id,
          title: n.title,
          message: n.message,
          type: n.notification_type,
          source: 'remote',
        }));
        callBrainTool('brain_notifications', { action: 'mark_delivered', ids: brainNotifications.result.notifications.map(n => n.id) }).catch(() => {});
      }
    } catch (e) {
      // Continue without brain data
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CREATE DEFAULT GOALS IF NONE
    // ═══════════════════════════════════════════════════════════════════════════
    if (output.goals.length === 0) {
      const defaultGoals = [
        { goal_type: 'quality', title: 'Maintain Code Quality', priority: 70 },
        { goal_type: 'learning', title: 'Continuous Learning', priority: 60 },
        { goal_type: 'maintenance', title: 'Reduce Tech Debt', priority: 50 },
      ];

      for (const goal of defaultGoals) {
        try {
          await callBrainTool('brain_goals', { action: 'create', userId: user.userId, ...goal });
        } catch (e) { /* ignore */ }
      }

      output.goals = defaultGoals.map(g => ({
        title: g.title,
        type: g.goal_type,
        progress: 100,
        progressBar: progressBar(100),
        source: 'default',
      }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // COCKPIT ALERTS
    // ═══════════════════════════════════════════════════════════════════════════
    if (cockpit) {
      try {
        const cockpitData = cockpit.fullScan();
        if (cockpitData?.alerts?.alerts?.length > 0) {
          output.alerts = cockpitData.alerts.alerts
            .filter(a => !a.acknowledged)
            .slice(0, 5)
            .map(a => ({
              severity: a.severity,
              message: a.message,
              source: a.source,
            }));
        }
      } catch (e) {
        // Continue without cockpit
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS INSIGHTS
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        const ctx = consciousness.generateSessionStartContext();
        if (ctx.insights?.length > 0) {
          output.insights = ctx.insights.map(i => ({ title: i.title, type: i.type }));
        }
        if (ecosystem.currentProject) {
          consciousness.updateRecentContext('lastProjects', ecosystem.currentProject.name);
        }
      } catch (e) {
        // Continue without insights
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY STATE
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const psySummary = psychology.getSummary();
        if (psySummary.confidence > DC.CONFIDENCE.PSYCHOLOGY_DISPLAY) {
          output.psychology = {
            state: psySummary.overallState.toUpperCase(),
            emoji: psySummary.emoji,
            energy: {
              value: Math.round(psySummary.energy.value * 100),
              trend: psySummary.energy.trend,
              arrow: trendArrow(psySummary.energy.trend),
              bar: progressBar(psySummary.energy.value, 1),
            },
            focus: {
              value: Math.round(psySummary.focus.value * 100),
              trend: psySummary.focus.trend,
              arrow: trendArrow(psySummary.focus.trend),
              bar: progressBar(psySummary.focus.value, 1),
            },
            composites: {
              flow: psySummary.composites.flow || false,
              burnoutRisk: psySummary.composites.burnoutRisk || false,
              exploration: psySummary.composites.exploration || false,
              grind: psySummary.composites.grind || false,
            },
            confidence: Math.round(psySummary.confidence * 100),
          };
        }
      } catch (e) {
        // Continue without psychology
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // THERMODYNAMICS STATE
    // ═══════════════════════════════════════════════════════════════════════════
    if (thermodynamics) {
      try {
        const thermoState = thermodynamics.getState();
        const recommendation = thermodynamics.getRecommendation();

        output.thermodynamics = {
          heat: thermoState.heat,
          work: thermoState.work,
          temperature: thermoState.temperature,
          temperatureBar: progressBar(thermoState.temperature, thermodynamics.CRITICAL_TEMPERATURE),
          efficiency: thermoState.efficiency,
          efficiencyBar: progressBar(thermoState.efficiency),
          carnotLimit: thermoState.carnotLimit,
          entropy: thermoState.entropy,
          isCritical: thermoState.isCritical,
          recommendation: {
            level: recommendation.level,
            message: recommendation.message,
          },
        };
      } catch (e) {
        // Continue without thermodynamics
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RECENT PATTERNS
    // ═══════════════════════════════════════════════════════════════════════════
    if (profile.patterns?.recent) {
      output.patterns = Object.entries(profile.patterns.recent)
        .slice(0, 5)
        .map(([name, count]) => ({ name, count }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TELEMETRY STATS (for benchmarking and fine-tuning)
    // ═══════════════════════════════════════════════════════════════════════════
    if (telemetry) {
      try {
        const stats = telemetry.getStats();
        output.telemetry = {
          sessionId: stats.sessionId,
          uptime: stats.uptime,
          events: stats.totalEvents,
          errors: stats.totalErrors,
          frictions: stats.frictions,
          counters: stats.counters,
          timings: stats.timings,
          categories: stats.categories,
        };
      } catch (e) {
        // Continue without telemetry stats
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROACTIVE ADVISOR
    // ═══════════════════════════════════════════════════════════════════════════
    if (proactiveAdvisor && proactiveAdvisor.shouldInjectNow()) {
      try {
        const injection = proactiveAdvisor.generateSessionInjection();
        if (injection) {
          output.proactiveAdvice = injection;
        }
      } catch (e) {
        // Continue without proactive advice
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // START BRAIN SESSION (async, don't wait)
    // ═══════════════════════════════════════════════════════════════════════════
    startBrainSession(user.userId, {
      project: ecosystem.currentProject?.name,
      metadata: {
        userName: user.name,
        sessionCount: profile.stats?.sessions || 1,
        ecosystem: ecosystem.projects?.map(p => p.name) || [],
      },
    }).then(result => {
      if (result.sessionId) process.env.CYNIC_SESSION_ID = result.sessionId;
    }).catch(() => {});

    sendHookToCollectiveSync('SessionStart', {
      userId: user.userId,
      userName: user.name,
      sessionCount: profile.stats?.sessions || 1,
      project: ecosystem.currentProject?.name,
      ecosystem: ecosystem.projects?.map(p => p.name) || [],
      timestamp: Date.now(),
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTRIBUTOR DISCOVERY (background, don't wait)
    // ═══════════════════════════════════════════════════════════════════════════
    if (contributorDiscovery) {
      setImmediate(async () => {
        try {
          const currentProfile = await contributorDiscovery.getCurrentUserProfile();
          if (currentProfile) {
            process.env.CYNIC_CONTRIBUTOR_PROFILE = JSON.stringify({
              email: currentProfile.email,
              personality: currentProfile.insights?.personality,
              workStyle: currentProfile.insights?.workStyle,
              phiScores: currentProfile.insights?.phiScores,
            });
          }

          const lastScanPath = path.join(os.homedir(), '.cynic', 'learning', 'last-discovery-scan.json');
          let shouldScan = true;

          try {
            if (fs.existsSync(lastScanPath)) {
              const lastScan = JSON.parse(fs.readFileSync(lastScanPath, 'utf8'));
              const hoursSinceScan = (Date.now() - lastScan.timestamp) / (1000 * 60 * 60);
              shouldScan = hoursSinceScan > DC.PHI.PHI_HOURS;
            }
          } catch (e) { /* scan anyway */ }

          if (shouldScan) {
            const discovery = await contributorDiscovery.fullEcosystemScan();
            const scanDir = path.dirname(lastScanPath);
            if (!fs.existsSync(scanDir)) fs.mkdirSync(scanDir, { recursive: true });
            fs.writeFileSync(lastScanPath, JSON.stringify({
              timestamp: Date.now(),
              repos: discovery.repos?.length || 0,
              contributors: Object.keys(discovery.contributors || {}).length,
            }));
          }
        } catch (e) { /* ignore */ }
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CODEBASE SELF-INDEXING (background, don't wait)
    // "Le chien doit se connaître lui-même"
    // ═══════════════════════════════════════════════════════════════════════════
    setImmediate(async () => {
      try {
        const lastIndexPath = path.join(os.homedir(), '.cynic', 'learning', 'last-codebase-index.json');
        let shouldIndex = true;

        // Check if we indexed recently (within φ hours ≈ 1.6 hours)
        try {
          if (fs.existsSync(lastIndexPath)) {
            const lastIndex = JSON.parse(fs.readFileSync(lastIndexPath, 'utf8'));
            const hoursSinceIndex = (Date.now() - lastIndex.timestamp) / (1000 * 60 * 60);
            shouldIndex = hoursSinceIndex > DC.PHI.PHI; // ~1.618 hours
          }
        } catch (e) { /* index anyway */ }

        if (shouldIndex) {
          const factsRepo = getFactsRepository();
          if (factsRepo) {
            const indexer = getCodebaseIndexer({
              factsRepo,
              rootDir: ecosystem.currentProject?.path || process.cwd(),
              userId: user.userId,
              sessionId: process.env.CYNIC_SESSION_ID,
              projectName: ecosystem.currentProject?.name || 'CYNIC',
            });

            if (indexer) {
              const results = await indexer.index();

              // Save index timestamp
              const indexDir = path.dirname(lastIndexPath);
              if (!fs.existsSync(indexDir)) fs.mkdirSync(indexDir, { recursive: true });
              fs.writeFileSync(lastIndexPath, JSON.stringify({
                timestamp: Date.now(),
                project: ecosystem.currentProject?.name,
                facts: results.total,
                errors: results.errors?.length || 0,
              }));
            }
          }
        }
      } catch (e) { /* ignore codebase indexing errors */ }
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // OUTPUT JSON
    // ═══════════════════════════════════════════════════════════════════════════
    safeOutput(output);

  } catch (error) {
    // Minimal output on error
    safeOutput({
      type: 'SessionStart',
      timestamp: new Date().toISOString(),
      error: error.message,
      minimal: true,
    });
  }
}

main();
