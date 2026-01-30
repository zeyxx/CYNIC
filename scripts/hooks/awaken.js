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
import { getSessionState, initOrchestrationClient } from './lib/index.js';

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

    // Update profile
    let profile = updateUserProfile(localProfile, {
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
