/**
 * CYNIC Dashboard - Main Application
 * Entry point, state management
 */

import { api } from './api.js';
import { router } from './router.js';
import { Utils } from './lib/utils.js';
import { Formulas } from './lib/formulas.js';

// Import components
import { Header } from './components/header.js';
import { Sidebar } from './components/sidebar.js';
import { Console } from './components/console.js';

// Import views
import { OperatorView } from './views/operator.js';
import { DevView } from './views/dev.js';
import { ArchView } from './views/arch.js';

/**
 * Main Application State
 */
class App {
  constructor() {
    this.state = {
      connected: false,
      mode: 'operator',
      health: null,
      metrics: null,
      chain: null,
      collective: null,
      alerts: [],
      consoleOpen: false,
    };

    this.components = {};
    this.views = {};
    this.refreshInterval = null;
  }

  /**
   * Initialize application
   */
  async init() {
    console.log('🐕 CYNIC Dashboard initializing...');

    // Initialize router
    router.init();
    this.state.mode = router.getMode();

    // Listen for mode changes
    router.on('modeChange', ({ mode }) => {
      this.state.mode = mode;
      this._onModeChange(mode);
    });

    // Initialize API
    const connected = await api.init();
    this.state.connected = connected;

    // Listen for connection changes
    api.on('connection', ({ status }) => {
      this.state.connected = status === 'connected';
      this._updateConnectionStatus();
    });

    // Listen for SSE events
    api.on('alert', (alert) => this._handleAlert(alert));
    api.on('judgment', (judgment) => this._handleJudgment(judgment));
    api.on('block', (block) => this._handleBlock(block));

    // Initialize components
    await this._initComponents();

    // Initialize views
    await this._initViews();

    // Load initial data
    await this._loadInitialData();

    // Start refresh interval
    this._startRefreshInterval();

    console.log('🐕 CYNIC Dashboard ready');
  }

  /**
   * Initialize UI components
   */
  async _initComponents() {
    // Header
    this.components.header = new Header({
      onModeChange: (mode) => router.navigate(mode),
      onConsoleToggle: () => this._toggleConsole(),
    });
    this.components.header.render(document.getElementById('header'));

    // Sidebar
    this.components.sidebar = new Sidebar({
      onNavClick: (item) => this._handleNavClick(item),
    });
    this.components.sidebar.render(document.getElementById('sidebar'));

    // Console
    this.components.console = new Console({
      api,
      onCommand: (cmd) => this._handleConsoleCommand(cmd),
    });
    this.components.console.render(document.getElementById('console-container'));
  }

  /**
   * Initialize views
   */
  async _initViews() {
    // Operator view
    this.views.operator = new OperatorView({ api });
    this.views.operator.render(document.getElementById('view-operator'));

    // Dev view
    this.views.dev = new DevView({ api });
    this.views.dev.render(document.getElementById('view-dev'));

    // Arch view
    this.views.arch = new ArchView({ api });
    this.views.arch.render(document.getElementById('view-arch'));
  }

  /**
   * Load initial data
   */
  async _loadInitialData() {
    // Get health
    const healthResult = await api.health();
    if (healthResult.success) {
      this.state.health = healthResult.result;
    }

    // Get collective status
    const collectiveResult = await api.collectiveStatus(true);
    if (collectiveResult.success) {
      this.state.collective = collectiveResult.result;
      this.components.sidebar.updateDogs(collectiveResult.result.agents);
    }

    // Get chain status
    const chainResult = await api.chain('status');
    if (chainResult.success) {
      this.state.chain = chainResult.result;
    }

    // Update UI
    this._updateConnectionStatus();
  }

  /**
   * Start periodic refresh
   */
  _startRefreshInterval() {
    // Refresh every 30 seconds
    this.refreshInterval = setInterval(async () => {
      if (this.state.connected) {
        await this._refreshData();
      }
    }, 30000);
  }

  /**
   * Refresh data
   */
  async _refreshData() {
    // Refresh based on current mode
    switch (this.state.mode) {
      case 'operator':
        await this.views.operator.refresh();
        break;
      case 'dev':
        // Dev view doesn't need periodic refresh
        break;
      case 'arch':
        // Arch view doesn't need periodic refresh
        break;
    }

    // Always refresh collective status
    const collectiveResult = await api.collectiveStatus();
    if (collectiveResult.success) {
      this.state.collective = collectiveResult.result;
      this.components.sidebar.updateDogs(collectiveResult.result.agents);
    }
  }

  /**
   * Handle mode change
   */
  _onModeChange(mode) {
    // Activate the new view
    if (this.views[mode]) {
      this.views[mode].activate();
    }

    // Deactivate other views
    for (const [key, view] of Object.entries(this.views)) {
      if (key !== mode) {
        view.deactivate();
      }
    }

    // Update sidebar navigation
    this.components.sidebar.setActiveNav(mode);
  }

  /**
   * Update connection status in header
   */
  _updateConnectionStatus() {
    this.components.header.setConnectionStatus(
      this.state.connected ? 'connected' : 'disconnected'
    );
  }

  /**
   * Toggle console
   */
  _toggleConsole() {
    this.state.consoleOpen = !this.state.consoleOpen;
    this.components.console.toggle(this.state.consoleOpen);
    this.components.header.setConsoleActive(this.state.consoleOpen);
    document.body.classList.toggle('console-open', this.state.consoleOpen);
  }

  /**
   * Handle navigation click
   */
  _handleNavClick(item) {
    if (item.mode) {
      router.navigate(item.mode);
    } else if (item.action) {
      // Handle actions like 'settings', 'help', etc.
      console.log('Nav action:', item.action);
    }
  }

  /**
   * Handle console command
   */
  async _handleConsoleCommand(cmd) {
    console.log('Console command:', cmd);
  }

  /**
   * Handle incoming alert
   */
  _handleAlert(alert) {
    this.state.alerts.unshift(alert);
    if (this.state.alerts.length > 50) {
      this.state.alerts = this.state.alerts.slice(0, 50);
    }

    // Update operator view if active
    if (this.state.mode === 'operator') {
      this.views.operator.addAlert(alert);
    }

    // Log to console
    this.components.console.log(
      `[ALERT] ${alert.type}: ${alert.message}`,
      alert.type === 'error' ? 'error' : 'warning'
    );
  }

  /**
   * Handle incoming judgment
   */
  _handleJudgment(judgment) {
    // Update operator view if active
    if (this.state.mode === 'operator') {
      this.views.operator.onJudgment(judgment);
    }

    // Log to console
    this.components.console.log(
      `[JUDGMENT] ${judgment.verdict} (Q: ${judgment.qScore})`,
      'info'
    );
  }

  /**
   * Handle incoming block
   */
  _handleBlock(block) {
    // Update operator view if active
    if (this.state.mode === 'operator') {
      this.views.operator.onNewBlock(block);
    }

    // Log to console
    this.components.console.log(
      `[BLOCK] #${block.blockNumber} created (${block.judgmentCount} judgments)`,
      'success'
    );
  }

  /**
   * Get current state
   */
  getState() {
    return { ...this.state };
  }

  /**
   * Destroy application
   */
  destroy() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
    }
    api.disconnect();

    for (const view of Object.values(this.views)) {
      view.destroy?.();
    }
  }
}

// Create and start application when DOM is ready
const app = new App();

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => app.init());
} else {
  app.init();
}

// Export to window
window.CYNICApp = app;
