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

/**
 * Main handler for SessionStart
 */
async function main() {
  try {
    // Detect user identity
    const user = cynic.detectUser();

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
