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

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

// Load cockpit for enhanced ecosystem awareness
const cockpitPath = path.join(__dirname, '..', 'lib', 'cockpit.cjs');
let cockpit = null;
try {
  cockpit = require(cockpitPath);
} catch (e) {
  // Cockpit not available - continue without
}

// Load contributor discovery for automatic profiling
const contributorDiscoveryPath = path.join(__dirname, '..', 'lib', 'contributor-discovery.cjs');
let contributorDiscovery = null;
try {
  contributorDiscovery = require(contributorDiscoveryPath);
} catch (e) {
  // Contributor discovery not available - continue without
}

// Load consciousness for learning loop
const consciousnessPath = path.join(__dirname, '..', 'lib', 'consciousness.cjs');
let consciousness = null;
try {
  consciousness = require(consciousnessPath);
  consciousness.init();
} catch (e) {
  // Consciousness not available - continue without
}

// Load proactive advisor for intelligent suggestions
const advisorPath = path.join(__dirname, '..', 'lib', 'proactive-advisor.cjs');
let proactiveAdvisor = null;
try {
  proactiveAdvisor = require(advisorPath);
  proactiveAdvisor.init();
} catch (e) {
  // Proactive advisor not available - continue without
}

// Load signal collector for break detection
const signalCollectorPath = path.join(__dirname, '..', 'lib', 'signal-collector.cjs');
let signalCollector = null;
try {
  signalCollector = require(signalCollectorPath);
  signalCollector.init();
} catch (e) {
  // Signal collector not available - continue without
}

// Load psychology module for state access
const psychologyPath = path.join(__dirname, '..', 'lib', 'human-psychology.cjs');
let psychology = null;
try {
  psychology = require(psychologyPath);
  psychology.init();
} catch (e) {
  // Psychology not available - continue without
}

/**
 * Main handler for SessionStart
 */
async function main() {
  try {
    // Detect user identity
    const user = cynic.detectUser();

    // ═══════════════════════════════════════════════════════════════════════════
    // ORCHESTRATION: Notify KETER of session start
    // "Le chien s'éveille. KETER coordonne."
    // ═══════════════════════════════════════════════════════════════════════════
    let orchestration = null;
    try {
      orchestration = await cynic.orchestrate('session_start', {
        content: 'Session awakening',
        source: 'awaken_hook',
      }, {
        user: user.userId,
        project: cynic.detectProject(),
      });
    } catch (e) {
      // Orchestration failed - continue with normal awakening
    }

    // Load local profile first
    let localProfile = cynic.loadUserProfile(user.userId);

    // ═══════════════════════════════════════════════════════════════════════════
    // CROSS-SESSION MEMORY: Load profile from PostgreSQL
    // ═══════════════════════════════════════════════════════════════════════════
    let remoteProfile = null;
    let learningsImport = null;

    try {
      remoteProfile = await cynic.loadProfileFromDB(user.userId);
      if (remoteProfile) {
        // Merge remote (accumulated) with local (current session)
        localProfile = cynic.mergeProfiles(remoteProfile, localProfile);
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
      // Silently fail - local profile is fallback
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
        // Silently fail - local consciousness is fallback
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
        // Silently fail - local psychology state is fallback
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
    let profile = cynic.updateUserProfile(localProfile, {
      identity: {
        name: user.name,
        email: user.email
      },
      stats: {
        sessions: (localProfile.stats?.sessions || 0) + 1
      }
    });

    // Detect ecosystem (all projects in workspace)
    const ecosystem = cynic.detectEcosystem();

    // ═══════════════════════════════════════════════════════════════════════════
    // MCP: Load relevant context from brain memory
    // "Le chien se souvient" - CYNIC remembers
    // ═══════════════════════════════════════════════════════════════════════════
    let brainMemory = null;
    let brainPsychology = null;
    try {
      // Search for relevant memories about current project/user (non-blocking)
      const searchPromise = cynic.callBrainTool('brain_search', {
        query: `${ecosystem.currentProject?.name || 'project'} ${user.name}`,
        limit: 5,
        types: ['decision', 'pattern', 'insight'],
      });

      // Get psychology state (non-blocking)
      const psychPromise = cynic.callBrainTool('brain_psychology', {
        action: 'get_state',
        userId: user.userId,
      });

      // Wait for both with timeout (don't block session start)
      const results = await Promise.race([
        Promise.all([searchPromise, psychPromise]),
        new Promise(resolve => setTimeout(() => resolve([null, null]), 3000))
      ]);

      [brainMemory, brainPsychology] = results || [null, null];
    } catch (e) {
      // MCP calls failed - continue without (non-critical)
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
        profile = cynic.updateUserProfile(profile, {
          memory: {
            recentProjects: [projectName, ...recentProjects.filter(p => p !== projectName)].slice(0, 10)
          }
        });
      }
    }

    // Format the awakening message (with learnings import info if available)
    let message = cynic.formatEcosystemStatus(ecosystem, profile, learningsImport);

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
    // BRAIN MEMORY: Inject relevant memories from MCP
    // "Le chien n'oublie jamais"
    // ═══════════════════════════════════════════════════════════════════════════
    if (brainMemory?.success && brainMemory?.result?.entries?.length > 0) {
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
        if (psySummary.confidence > 0.2) { // Only show if some confidence
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
    cynic.startBrainSession(user.userId, {
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
    cynic.sendHookToCollectiveSync('SessionStart', {
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
          const fs = require('fs');
          const os = require('os');
          const lastScanPath = path.join(os.homedir(), '.cynic', 'learning', 'last-discovery-scan.json');
          let shouldScan = true;

          try {
            if (fs.existsSync(lastScanPath)) {
              const lastScan = JSON.parse(fs.readFileSync(lastScanPath, 'utf8'));
              const hoursSinceScan = (Date.now() - lastScan.timestamp) / (1000 * 60 * 60);
              // Only full scan every 6.18 hours (φ-aligned)
              shouldScan = hoursSinceScan > 6.18;
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

    // Output directly to stdout (like asdf-brain) for banner display
    console.log(message);

  } catch (error) {
    // Minimal output on error
    console.log('🧠 CYNIC awakening... *yawn*');
  }
}

main();
