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

/**
 * Main handler for SessionStart
 */
async function main() {
  try {
    // Detect user identity
    const user = cynic.detectUser();

    // Load or create user profile
    let profile = cynic.loadUserProfile(user.userId);

    // Update profile with current identity info
    profile = cynic.updateUserProfile(profile, {
      identity: {
        name: user.name,
        email: user.email
      },
      stats: {
        sessions: (profile.stats?.sessions || 0) + 1
      }
    });

    // Detect ecosystem (all projects in workspace)
    const ecosystem = cynic.detectEcosystem();

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

    // Format the awakening message
    const message = cynic.formatEcosystemStatus(ecosystem, profile);

    // Output directly to stdout (like asdf-brain) for banner display
    console.log(message);

  } catch (error) {
    // Minimal output on error
    console.log('🧠 CYNIC awakening... *yawn*');
  }
}

main();
