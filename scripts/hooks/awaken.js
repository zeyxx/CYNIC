#!/usr/bin/env node
/**
 * CYNIC Awaken Hook - SessionStart
 *
 * "Le chien s'éveille" - CYNIC awakens with the session
 *
 * This hook runs at the start of every Claude session.
 * It establishes CYNIC's presence from the very first moment.
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
  formatEcosystemStatus,
  orchestrate,
  orchestrateFull,  // Phase 22: For OrchestrationClient init
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
} from '../lib/index.js';

import path from 'path';
import fs from 'fs';
import os from 'os';

// Phase 22: Session state management
import { getSessionState, initOrchestrationClient } from './lib/index.js';

/**
 * Main handler for SessionStart
 */
async function main() {
  try {
    // Detect user identity
    const user = detectUser();

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 22: Initialize Session State
    // "Le chien se souvient de tout dans la session"
    // ═══════════════════════════════════════════════════════════════════════════
    const sessionId = process.env.CYNIC_SESSION_ID || `session-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    process.env.CYNIC_SESSION_ID = sessionId;

    const sessionState = getSessionState();
    await sessionState.init(sessionId, { userId: user.userId });

    // Initialize OrchestrationClient with orchestrateFull
    initOrchestrationClient(orchestrateFull);

    // Load optional modules
    const cockpit = getCockpit();
    const consciousness = getConsciousness();
    const proactiveAdvisor = getProactiveAdvisor();
    const signalCollector = getSignalCollector();
    const psychology = getPsychology();
    const contributorDiscovery = getContributorDiscovery();

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Notify KETER of session start
    // "Le chien s'éveille. KETER coordonne."
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    try {
      orchestration = await orchestrate('session_start', {
        content: 'Session awakening',
        source: 'awaken_hook',
      }, {
        user: user.userId,
        project: detectProject(),
      });
    } catch (e) {
      // Orchestration failed - continue with normal awakening
    }

    // Load local profile first
    let localProfile = loadUserProfile(user.userId);

    // ═══════════════════════════════════════════════════════════════════════════
    // CROSS-SESSION MEMORY: Load profile from PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    let remoteProfile = null;
    let learningsImport = null;

    try {
      remoteProfile = await loadProfileFromDB(user.userId);
      if (remoteProfile) {
        // Remote is SOURCE OF TRUTH for accumulated totals
        // Copy non-stat data from remote (preferences, patterns, memory)
        localProfile = {
          ...localProfile,
          identity: {
            ...localProfile.identity,
            ...remoteProfile.identity,
            lastSeen: new Date().toISOString(),
          },
          patterns: remoteProfile.patterns || localProfile.patterns,
          preferences: remoteProfile.preferences || localProfile.preferences,
          memory: remoteProfile.memory || localProfile.memory,
          learning: remoteProfile.learning || {},
        };

        // RESET local stats to 0 - they track SESSION DELTAS ONLY
        // At session end, these deltas are ADDED to remote totals
        localProfile.stats = {
          sessions: 1, // This session counts as 1
          toolCalls: 0,
          errorsEncountered: 0,
          dangerBlocked: 0,
          commitsWithCynic: 0,
          judgmentsMade: 0,
          judgmentsCorrect: 0,
        };

        // Store remote totals for reference (display purposes)
        localProfile._remoteTotals = remoteProfile.stats || {};

        learningsImport = {
          success: true,
          imported: remoteProfile.meta?.sessionCount || 0,
          stats: {
            accuracy: remoteProfile.learning?.feedbackAccuracy
              ? Math.round(remoteProfile.learning.feedbackAccuracy * 100)
              : null
          }
        };
      }
    } catch (e) {
      // Track failure for notification
      localProfile._syncFailures = localProfile._syncFailures || [];
      localProfile._syncFailures.push({
        type: 'profile',
        error: e.message,
        timestamp: new Date().toISOString(),
      });

      // Reset stats for clean session tracking
      localProfile.stats = {
        sessions: 1,
        toolCalls: 0,
        errorsEncountered: 0,
        dangerBlocked: 0,
        commitsWithCynic: 0,
        judgmentsMade: 0,
        judgmentsCorrect: 0,
      };

      console.error('[CYNIC] Profile sync failed:', e.message);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS SYNC: Load learning loop from PostgreSQL
    // "Le chien se souvient. L'apprentissage traverse les machines."
    // ═══════════════════════════════════════════════════════════════════════════
    let consciousnessImport = null;
    if (consciousness) {
      try {
        const remoteConsciousness = await consciousness.loadFromDB(user.userId);
        if (remoteConsciousness) {
          // Get local consciousness snapshot
          const localSnapshot = consciousness.getConsciousnessSnapshot();

          // Merge remote with local
          const merged = consciousness.mergeWithRemote(remoteConsciousness, localSnapshot);

          // Update local files with merged data (will be used during session)
          if (merged.humanGrowth) {
            consciousness.updateHumanGrowth(merged.humanGrowth);
          }

          consciousnessImport = {
            success: true,
            totalObservations: remoteConsciousness.meta?.totalObservations || 0,
            insightsCount: remoteConsciousness.meta?.insightsCount || 0,
          };
        }
      } catch (e) {
        // Track failure for notification
        localProfile._syncFailures = localProfile._syncFailures || [];
        localProfile._syncFailures.push({
          type: 'consciousness',
          error: e.message,
          timestamp: new Date().toISOString(),
        });
        console.error('[CYNIC] Consciousness sync failed:', e.message);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGY SYNC: Load human understanding from PostgreSQL
    // "Comprendre l'humain pour mieux l'aider"
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const remotePsychology = await psychology.loadFromDB(user.userId);
        if (remotePsychology) {
          // Psychology state imported from remote - learning persists
        }
      } catch (e) {
        // Track failure for notification
        localProfile._syncFailures = localProfile._syncFailures || [];
        localProfile._syncFailures.push({
          type: 'psychology',
          error: e.message,
          timestamp: new Date().toISOString(),
        });
        console.error('[CYNIC] Psychology sync failed:', e.message);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BREAK DETECTION: Check gap since last session for psychology module
    // "Le repos fait partie du travail"
    // ═══════════════════════════════════════════════════════════════════════════
    if (signalCollector && localProfile.updatedAt) {
      const gapMs = Date.now() - localProfile.updatedAt;
      signalCollector.collectBreak(gapMs);
    }

    // Update profile with current identity info and increment session
    let profile = updateUserProfile(localProfile, {
      identity: {
        name: user.name,
        email: user.email
      },
      stats: {
        sessions: (localProfile.stats?.sessions || 0) + 1
      }
    });

    // Detect ecosystem (all projects in workspace)
    const ecosystem = detectEcosystem();

    // ═══════════════════════════════════════════════════════════════════════════
    // TOTAL MEMORY: Load memories, decisions, lessons, notifications, goals
    // "φ remembers everything" - CYNIC's Total Memory system
    // ═══════════════════════════════════════════════════════════════════════════
    let totalMemoryData = null;
    const totalMemory = getTotalMemory();

    if (totalMemory) {
      try {
        await totalMemory.init();

        // Load in parallel for speed
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
          new Promise(resolve => setTimeout(() => resolve([null, [], []]), 3000))
        ]);

        totalMemoryData = { memories, notifications, goals };

        // Mark notifications as delivered
        if (notifications?.length > 0) {
          totalMemory.markNotificationsDelivered(notifications.map(n => n.id)).catch(() => {});
        }
      } catch (e) {
        // Total Memory load failed - continue without
        console.error('[CYNIC] Total Memory load failed:', e.message);
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MCP: Load relevant context from brain memory
    // "Le chien se souvient" - CYNIC remembers
    // ═══════════════════════════════════════════════════════════════════════════
    let brainMemory = null;
    let brainPsychology = null;
    let brainGoals = null;
    let brainNotifications = null;
    try {
      // Search for relevant memories about current project/user (non-blocking)
      const searchPromise = callBrainTool('brain_search', {
        query: `${ecosystem.currentProject?.name || 'project'} ${user.name}`,
        limit: 5,
        types: ['decision', 'pattern', 'insight'],
      });

      // Get psychology state (non-blocking)
      const psychPromise = callBrainTool('brain_psychology', {
        action: 'get_state',
        userId: user.userId,
      });

      // Get active goals from PostgreSQL (Phase 16)
      const goalsPromise = callBrainTool('brain_goals', {
        action: 'list',
        status: 'active',
        userId: user.userId,
      });

      // Get pending notifications from PostgreSQL (Phase 16)
      const notificationsPromise = callBrainTool('brain_notifications', {
        action: 'list',
        delivered: false,
        userId: user.userId,
        limit: 5,
      });

      // Wait for all with timeout (don't block session start)
      const results = await Promise.race([
        Promise.all([searchPromise, psychPromise, goalsPromise, notificationsPromise]),
        new Promise(resolve => setTimeout(() => resolve([null, null, null, null]), 3000))
      ]);

      [brainMemory, brainPsychology, brainGoals, brainNotifications] = results || [null, null, null, null];

      // Mark notifications as delivered (non-blocking)
      if (brainNotifications?.success && brainNotifications?.result?.notifications?.length > 0) {
        const notificationIds = brainNotifications.result.notifications.map(n => n.id);
        callBrainTool('brain_notifications', {
          action: 'mark_delivered',
          ids: notificationIds,
        }).catch(() => {});
      }
    } catch (e) {
      // MCP calls failed - continue without (non-critical)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GOAL SYSTEM: Create default goals if none exist
    // "φ pursues quality autonomously" - Phase 16 Autonomy
    // ═══════════════════════════════════════════════════════════════════════════
    const hasLocalGoals = totalMemoryData?.goals?.length > 0;
    const hasRemoteGoals = brainGoals?.success && brainGoals?.result?.goals?.length > 0;

    if (!hasLocalGoals && !hasRemoteGoals) {
      // No goals exist - create default goals for this user
      try {
        const defaultGoals = [
          {
            goal_type: 'quality',
            title: 'Maintain Code Quality',
            description: 'Keep test coverage high and lint scores clean',
            priority: 70,
          },
          {
            goal_type: 'learning',
            title: 'Continuous Learning',
            description: 'Learn from mistakes and apply lessons',
            priority: 60,
          },
          {
            goal_type: 'maintenance',
            title: 'Reduce Tech Debt',
            description: 'Simplify code and update dependencies',
            priority: 50,
          },
        ];

        for (const goal of defaultGoals) {
          await callBrainTool('brain_goals', {
            action: 'create',
            userId: user.userId,
            ...goal,
          }).catch(() => {});
        }

        // Refresh goals for display
        const refreshedGoals = await callBrainTool('brain_goals', {
          action: 'list',
          status: 'active',
          userId: user.userId,
        }).catch(() => null);

        if (refreshedGoals?.success) {
          brainGoals = refreshedGoals;
        }
      } catch (e) {
        // Goal creation failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // COCKPIT: Deep ecosystem scan with alerts
    // ═══════════════════════════════════════════════════════════════════════════
    let cockpitData = null;
    if (cockpit) {
      try {
        cockpitData = cockpit.fullScan();
      } catch (e) {
        // Cockpit scan failed - continue without
      }
    }

    // Update profile with current project
    if (ecosystem.currentProject) {
      const recentProjects = profile.memory?.recentProjects || [];
      const projectName = ecosystem.currentProject.name;

      // Add to recent if not already first
      if (recentProjects[0] !== projectName) {
        profile = updateUserProfile(profile, {
          memory: {
            recentProjects: [projectName, ...recentProjects.filter(p => p !== projectName)].slice(0, 10)
          }
        });
      }
    }

    // Format the awakening message (with learnings import info if available)
    let message = formatEcosystemStatus(ecosystem, profile, learningsImport);

    // ═══════════════════════════════════════════════════════════════════════════
    // COCKPIT ALERTS: Inject proactive warnings
    // ═══════════════════════════════════════════════════════════════════════════
    if (cockpitData?.alerts?.alerts?.length > 0) {
      const activeAlerts = cockpitData.alerts.alerts.filter(a => !a.acknowledged);
      if (activeAlerts.length > 0) {
        const alertLines = ['', '── COCKPIT ALERTS ─────────────────────────────────────────'];
        for (const alert of activeAlerts.slice(0, 5)) {
          const icon = alert.severity === 'critical' ? '🔴' :
                       alert.severity === 'warning' ? '⚠️' : 'ℹ️';
          alertLines.push(`   ${icon} ${alert.message}`);
        }
        if (activeAlerts.length > 5) {
          alertLines.push(`   ... +${activeAlerts.length - 5} more alerts`);
        }
        // Insert before the final banner
        const lines = message.split('\n');
        const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
        if (insertIdx > 0) {
          lines.splice(insertIdx, 0, ...alertLines, '');
          message = lines.join('\n');
        }
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TOTAL MEMORY: Inject notifications, goals, and memories
    // "φ remembers everything"
    // ═══════════════════════════════════════════════════════════════════════════
    if (totalMemoryData) {
      try {
        const lines = message.split('\n');
        const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));

        // Proactive notifications (delivered at session start)
        if (totalMemoryData.notifications?.length > 0) {
          const notifLines = ['', '── 📬 NOTIFICATIONS ───────────────────────────────────────'];
          for (const n of totalMemoryData.notifications.slice(0, 3)) {
            const icon = n.notificationType === 'warning' ? '⚠️' :
                         n.notificationType === 'achievement' ? '🏆' :
                         n.notificationType === 'reminder' ? '🔔' : '💡';
            notifLines.push(`   ${icon} ${n.title}`);
            if (n.message && n.message.length < 60) {
              notifLines.push(`      ${n.message}`);
            }
          }
          if (insertIdx > 0) {
            lines.splice(insertIdx, 0, ...notifLines, '');
          }
        }

        // Active goals
        if (totalMemoryData.goals?.length > 0) {
          const goalLines = ['', '── 🎯 ACTIVE GOALS ────────────────────────────────────────'];
          for (const g of totalMemoryData.goals.slice(0, 3)) {
            const progress = Math.round((g.progress || 0) * 100);
            const bar = '█'.repeat(Math.floor(progress / 10)) + '░'.repeat(10 - Math.floor(progress / 10));
            goalLines.push(`   [${bar}] ${progress}% ${g.title}`);
          }
          if (totalMemoryData.goals.length > 3) {
            goalLines.push(`   ... +${totalMemoryData.goals.length - 3} more goals`);
          }
          if (insertIdx > 0) {
            lines.splice(insertIdx, 0, ...goalLines, '');
          }
        }

        // Relevant memories (decisions, lessons)
        const memories = totalMemoryData.memories;
        if (memories?.decisions?.length > 0 || memories?.lessons?.length > 0) {
          const memLines = ['', '── 🧠 RELEVANT MEMORIES ───────────────────────────────────'];

          // Show relevant decisions
          for (const d of (memories.decisions || []).slice(0, 2)) {
            memLines.push(`   📋 ${d.title}`);
          }

          // Show relevant lessons (self-correction)
          for (const l of (memories.lessons || []).slice(0, 2)) {
            memLines.push(`   ⚠️ Lesson: ${l.mistake?.substring(0, 50)}...`);
          }

          if (insertIdx > 0) {
            lines.splice(insertIdx, 0, ...memLines, '');
          }
        }

        message = lines.join('\n');
      } catch (e) {
        // Total Memory injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BRAIN MEMORY: Inject relevant memories from MCP (legacy)
    // "Le chien n'oublie jamais"
    // ═══════════════════════════════════════════════════════════════════════════
    if (brainMemory?.success && brainMemory?.result?.entries?.length > 0 && !totalMemoryData?.memories) {
      try {
        const memoryLines = ['', '── MEMORY ─────────────────────────────────────────────────'];
        for (const entry of brainMemory.result.entries.slice(0, 3)) {
          const icon = entry.type === 'decision' ? '📋' :
                       entry.type === 'pattern' ? '🔄' :
                       entry.type === 'insight' ? '💡' : '📝';
          memoryLines.push(`   ${icon} ${entry.title || entry.content?.substring(0, 50)}...`);
        }
        const lines = message.split('\n');
        const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
        if (insertIdx > 0) {
          lines.splice(insertIdx, 0, ...memoryLines, '');
          message = lines.join('\n');
        }
      } catch (e) {
        // Memory injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // BRAIN GOALS/NOTIFICATIONS: Inject from PostgreSQL if no local data
    // "φ remembers across machines" - Phase 16 Total Memory
    // ═══════════════════════════════════════════════════════════════════════════
    if (!totalMemoryData?.goals && brainGoals?.success && brainGoals?.result?.goals?.length > 0) {
      try {
        const goalLines = ['', '── 🎯 ACTIVE GOALS (remote) ───────────────────────────────'];
        for (const g of brainGoals.result.goals.slice(0, 3)) {
          const progress = Math.round((g.progress || 0) * 100);
          const bar = '█'.repeat(Math.floor(progress / 10)) + '░'.repeat(10 - Math.floor(progress / 10));
          goalLines.push(`   [${bar}] ${progress}% ${g.title}`);
        }
        const lines = message.split('\n');
        const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
        if (insertIdx > 0) {
          lines.splice(insertIdx, 0, ...goalLines, '');
          message = lines.join('\n');
        }
      } catch (e) {
        // Goal injection failed - continue without
      }
    }

    if (!totalMemoryData?.notifications && brainNotifications?.success && brainNotifications?.result?.notifications?.length > 0) {
      try {
        const notifLines = ['', '── 📬 NOTIFICATIONS (remote) ──────────────────────────────'];
        for (const n of brainNotifications.result.notifications.slice(0, 3)) {
          const icon = n.notification_type === 'warning' ? '⚠️' :
                       n.notification_type === 'achievement' ? '🏆' :
                       n.notification_type === 'reminder' ? '🔔' : '💡';
          notifLines.push(`   ${icon} ${n.title}`);
          if (n.message && n.message.length < 60) {
            notifLines.push(`      ${n.message}`);
          }
        }
        const lines = message.split('\n');
        const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
        if (insertIdx > 0) {
          lines.splice(insertIdx, 0, ...notifLines, '');
          message = lines.join('\n');
        }
      } catch (e) {
        // Notification injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSCIOUSNESS: Inject learning loop context
    // ═══════════════════════════════════════════════════════════════════════════
    if (consciousness) {
      try {
        const ctx = consciousness.generateSessionStartContext();

        // Add insights if any
        if (ctx.insights && ctx.insights.length > 0) {
          const insightLines = ['', '── INSIGHTS ───────────────────────────────────────────────'];
          for (const insight of ctx.insights) {
            insightLines.push(`   💡 ${insight.title}`);
          }
          const lines = message.split('\n');
          const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
          if (insertIdx > 0) {
            lines.splice(insertIdx, 0, ...insightLines, '');
            message = lines.join('\n');
          }
        }

        // Track recent project in consciousness
        if (ecosystem.currentProject) {
          consciousness.updateRecentContext('lastProjects', ecosystem.currentProject.name);
        }
      } catch (e) {
        // Consciousness injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PSYCHOLOGICAL STATE: Show current state if tracked
    // "Comprendre l'humain pour mieux l'aider"
    // ═══════════════════════════════════════════════════════════════════════════
    if (psychology) {
      try {
        const psySummary = psychology.getSummary();
        if (psySummary.confidence > DC.CONFIDENCE.PSYCHOLOGY_DISPLAY) { // Only show if some confidence
          const stateLines = ['', '── ÉTAT ───────────────────────────────────────────────────'];
          stateLines.push(`   ${psySummary.emoji} ${psySummary.overallState.toUpperCase()}`);
          stateLines.push(`   énergie: ${Math.round(psySummary.energy.value * 100)}% ${psySummary.energy.trend === 'rising' ? '↑' : psySummary.energy.trend === 'falling' ? '↓' : '→'}`);
          stateLines.push(`   focus: ${Math.round(psySummary.focus.value * 100)}% ${psySummary.focus.trend === 'rising' ? '↑' : psySummary.focus.trend === 'falling' ? '↓' : '→'}`);

          if (psySummary.composites.burnoutRisk) {
            stateLines.push(`   ⚠️ Burnout risk detected - consider a break`);
          }
          if (psySummary.composites.flow) {
            stateLines.push(`   ✨ Flow state - don't interrupt!`);
          }

          const lines = message.split('\n');
          const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
          if (insertIdx > 0) {
            lines.splice(insertIdx, 0, ...stateLines, '');
            message = lines.join('\n');
          }
        }
      } catch (e) {
        // Psychology injection failed - continue without
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROACTIVE ADVISOR: Intelligent suggestions
    // ═══════════════════════════════════════════════════════════════════════════
    if (proactiveAdvisor && proactiveAdvisor.shouldInjectNow()) {
      try {
        const injection = proactiveAdvisor.generateSessionInjection();
        if (injection) {
          const lines = message.split('\n');
          const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
          if (insertIdx > 0) {
            lines.splice(insertIdx, 0, injection, '');
            message = lines.join('\n');
          }
        }
      } catch (e) {
        // Proactive injection failed - continue without
      }
    }

    // Start brain session first (async but we don't wait)
    startBrainSession(user.userId, {
      project: ecosystem.currentProject?.name,
      metadata: {
        userName: user.name,
        sessionCount: profile.stats?.sessions || 1,
        ecosystem: ecosystem.projects?.map(p => p.name) || [],
      }
    }).then(result => {
      if (result.sessionId) {
        // Store session ID in environment for other hooks
        process.env.CYNIC_SESSION_ID = result.sessionId;
      }
    }).catch(() => {
      // Silently ignore - local mode still works
    });

    // Also send to MCP collective for event distribution
    sendHookToCollectiveSync('SessionStart', {
      userId: user.userId,
      userName: user.name,
      sessionCount: profile.stats?.sessions || 1,
      project: ecosystem.currentProject?.name,
      ecosystem: ecosystem.projects?.map(p => p.name) || [],
      timestamp: Date.now(),
    });

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTRIBUTOR DISCOVERY: Background profiling of all contributors
    // "Les rails dans le cerveau" - Automatic learning infrastructure
    // ═══════════════════════════════════════════════════════════════════════════
    if (contributorDiscovery) {
      // Run discovery in background - don't block awakening
      setImmediate(async () => {
        try {
          // Discover and profile current user
          const currentProfile = await contributorDiscovery.getCurrentUserProfile();
          if (currentProfile) {
            // Store contributor profile in environment for other hooks
            process.env.CYNIC_CONTRIBUTOR_PROFILE = JSON.stringify({
              email: currentProfile.email,
              personality: currentProfile.insights?.personality,
              workStyle: currentProfile.insights?.workStyle,
              phiScores: currentProfile.insights?.phiScores,
            });
          }

          // Full ecosystem scan (only if needed - check last scan time)
          const lastScanPath = path.join(os.homedir(), '.cynic', 'learning', 'last-discovery-scan.json');
          let shouldScan = true;

          try {
            if (fs.existsSync(lastScanPath)) {
              const lastScan = JSON.parse(fs.readFileSync(lastScanPath, 'utf8'));
              const hoursSinceScan = (Date.now() - lastScan.timestamp) / (1000 * 60 * 60);
              // Only full scan every 6.18 hours (φ-aligned)
              shouldScan = hoursSinceScan > DC.PHI.PHI_HOURS;
            }
          } catch (e) { /* scan anyway */ }

          if (shouldScan) {
            // Full ecosystem discovery
            const discovery = await contributorDiscovery.fullEcosystemScan();

            // Save scan timestamp
            const scanDir = path.dirname(lastScanPath);
            if (!fs.existsSync(scanDir)) {
              fs.mkdirSync(scanDir, { recursive: true });
            }
            fs.writeFileSync(lastScanPath, JSON.stringify({
              timestamp: Date.now(),
              repos: discovery.repos?.length || 0,
              contributors: Object.keys(discovery.contributors || {}).length,
            }));
          }
        } catch (e) {
          // Silently ignore - discovery is optional enhancement
        }
      });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC FAILURES: Warn user if cross-session memory is not working
    // ═══════════════════════════════════════════════════════════════════════════
    if (localProfile._syncFailures && localProfile._syncFailures.length > 0) {
      const warnLines = ['', '── ⚠️  SYNC WARNINGS ───────────────────────────────────────'];
      warnLines.push('   Cross-session memory may be limited:');

      for (const failure of localProfile._syncFailures) {
        const icon = failure.type === 'profile' ? '👤' :
                     failure.type === 'consciousness' ? '🧠' :
                     failure.type === 'psychology' ? '💭' : '❓';
        warnLines.push(`   ${icon} ${failure.type}: ${failure.error || 'connection failed'}`);
      }

      warnLines.push('   📁 Local file backup will be used');
      warnLines.push('');

      const lines = message.split('\n');
      const insertIdx = lines.findIndex(l => l.includes('CYNIC is AWAKE'));
      if (insertIdx > 0) {
        lines.splice(insertIdx, 0, ...warnLines);
        message = lines.join('\n');
      }

      // Save profile with sync failures tracked for session end
      try {
        const { saveUserProfile } = await import('../lib/cynic-core.cjs');
        saveUserProfile(localProfile);
      } catch (e) { /* ignore */ }
    }

    // Output directly to stdout (like asdf-brain) for banner display
    console.log(message);

  } catch (error) {
    // Minimal output on error
    console.log('🧠 CYNIC awakening... *yawn*');
  }
}

main();
