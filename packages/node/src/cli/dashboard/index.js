/**
 * CYNIC TUI Dashboard
 *
 * Terminal-based dashboard for monitoring CYNIC system health,
 * agents, blockchain, patterns, and judgments.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/cli/dashboard
 */

'use strict';

import blessed from 'blessed';
import { PHI } from '@cynic/core';
import { DataFetcher } from './data-fetcher.js';
import { COLORS } from './theme.js';
import {
  createMainScreen,
  createChainScreen,
  createPatternsScreen,
  createAgentsScreen,
} from './screens/index.js';

// φ-aligned poll interval
const PHI_POLL_INTERVAL = Math.round(PHI * 1000);

/**
 * Dashboard state
 */
const SCREENS = {
  MAIN: 'main',
  CHAIN: 'chain',
  PATTERNS: 'patterns',
  AGENTS: 'agents',
};

/**
 * Create and run the dashboard
 */
export async function createDashboard(options = {}) {
  const port = options.port || 3618;
  const baseUrl = options.url || `http://localhost:${port}`;

  // Create blessed screen
  const screen = blessed.screen({
    smartCSR: true,
    title: 'CYNIC Cockpit',
    fullUnicode: true,
    forceUnicode: true,
  });

  // Create data fetcher
  const dataFetcher = new DataFetcher({
    baseUrl,
    pollInterval: PHI_POLL_INTERVAL,
    onError: (err) => {
      // Log error to main screen events if available
      if (screens.main) {
        screens.main.pushEvent({
          type: 'error',
          message: err.message,
          timestamp: Date.now(),
        });
      }
    },
    onConnect: () => {
      if (screens.main) {
        screens.main.pushEvent({
          type: 'connection',
          status: 'connected',
          timestamp: Date.now(),
        });
      }
    },
    onDisconnect: () => {
      if (screens.main) {
        screens.main.pushEvent({
          type: 'connection',
          status: 'disconnected',
          timestamp: Date.now(),
        });
      }
    },
  });

  // Track current screen
  let currentScreen = SCREENS.MAIN;

  // Create screens
  const screens = {
    main: createMainScreen(screen),
    chain: createChainScreen(screen, dataFetcher),
    patterns: createPatternsScreen(screen, dataFetcher),
    agents: createAgentsScreen(screen, dataFetcher),
  };

  // Show initial screen
  screens.main.show();

  /**
   * Switch to a different screen
   */
  function switchScreen(newScreen) {
    // Hide current
    screens[currentScreen].hide();

    // Show new
    screens[newScreen].show();
    currentScreen = newScreen;

    screen.render();
  }

  /**
   * Go back to main screen
   */
  function goBack() {
    if (currentScreen !== SCREENS.MAIN) {
      switchScreen(SCREENS.MAIN);
    }
  }

  // ═══════════════════════════════════════════════════════════
  // KEYBOARD BINDINGS
  // ═══════════════════════════════════════════════════════════

  // Global keys
  screen.key(['q', 'C-c'], () => {
    dataFetcher.stopPolling();
    screen.destroy();
    process.exit(0);
  });

  screen.key(['escape', 'b'], () => {
    goBack();
  });

  // Navigation keys
  screen.key('h', () => {
    if (currentScreen === SCREENS.MAIN) {
      // Focus health panel (scroll events)
      screens.main.focusEvents();
    }
  });

  screen.key('c', () => {
    switchScreen(SCREENS.CHAIN);
  });

  screen.key('p', () => {
    switchScreen(SCREENS.PATTERNS);
  });

  screen.key('a', () => {
    switchScreen(SCREENS.AGENTS);
  });

  screen.key('j', () => {
    // Judgments view - for now, go to main and focus events
    if (currentScreen !== SCREENS.MAIN) {
      switchScreen(SCREENS.MAIN);
    }
    screens.main.focusEvents();
  });

  screen.key('r', () => {
    // Force refresh
    dataFetcher.fetchAll().then(data => {
      screens[currentScreen].update(data);
      screen.render();
    });
  });

  // Chain screen specific keys
  screen.key('v', () => {
    if (currentScreen === SCREENS.CHAIN) {
      screens.chain.verify();
    }
  });

  screen.key('f', () => {
    if (currentScreen === SCREENS.CHAIN) {
      screens.chain.flush();
    }
  });

  // Patterns screen specific keys
  screen.key('left', () => {
    if (currentScreen === SCREENS.PATTERNS) {
      screens.patterns.prevCategory();
    }
  });

  screen.key('right', () => {
    if (currentScreen === SCREENS.PATTERNS) {
      screens.patterns.nextCategory();
    }
  });

  // Agents screen specific keys
  screen.key('d', () => {
    if (currentScreen === SCREENS.AGENTS) {
      const index = screens.agents.getSelectedIndex();
      screens.agents.runDiagnostic(index);
    }
  });

  // ═══════════════════════════════════════════════════════════
  // DATA POLLING
  // ═══════════════════════════════════════════════════════════

  // Start polling
  dataFetcher.startPolling((data) => {
    // Update current screen
    screens[currentScreen].update(data);
    screen.render();
  });

  // Initial render
  screen.render();

  // Return control interface
  return {
    screen,
    dataFetcher,
    switchScreen,
    destroy: () => {
      dataFetcher.stopPolling();
      screen.destroy();
    },
  };
}

export default createDashboard;
