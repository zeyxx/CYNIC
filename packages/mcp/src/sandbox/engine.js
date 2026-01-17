/**
 * CYNIC Engine - Architecture Dashboard Controller
 * Simplified for clear visualization of CYNIC's architecture
 */

const CYNICEngine = {
  connected: false,
  apiBase: null,
  state: {
    health: null,
    chain: null,
    dogs: null,
    codebase: null,
  },
  currentView: 'packages', // 'packages', 'dogs', 'flow', 'codebase'

  /**
   * Initialize the dashboard
   */
  async init() {
    console.log('CYNIC Dashboard initializing...');

    // Initialize components
    this.initConsole();
    this.initViz();
    this.initMatrix();
    this.initFormulas();
    this.initNavigation();

    // Connect to MCP
    await this.connectMCP();

    // Load data
    await this.loadData();

    console.log('CYNIC Dashboard ready');
  },

  /**
   * Initialize console
   */
  initConsole() {
    CYNICConsole.init('consoleOutput', 'consoleInput');
    CYNICConsole.setEngine(this);
  },

  /**
   * Initialize 3D visualization
   */
  initViz() {
    const container = document.getElementById('viz3d');
    if (container && typeof CYNICViz !== 'undefined') {
      CYNICViz.init3D('viz3d');

      // Add click handler for codebase navigation
      container.addEventListener('click', (e) => {
        if (this.currentView === 'codebase') {
          CYNICViz.onCodebaseClick(e, container);
        }
      });

      // Right-click to go back
      container.addEventListener('contextmenu', (e) => {
        if (this.currentView === 'codebase') {
          e.preventDefault();
          CYNICViz.navigateBack();
        }
      });
    }

    // View controls
    document.querySelectorAll('.viz-controls button').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.viz-controls button').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        this.onViewChange(btn.dataset.view);
      });
    });
  },

  /**
   * Initialize 25-dimension matrix preview
   */
  initMatrix() {
    const container = document.getElementById('matrixMini');
    if (!container) return;

    // Create 25 cells (5x5 grid)
    const dimensions = [];
    for (const dims of Object.values(CYNICFormulas.DIMENSIONS)) {
      dimensions.push(...dims);
    }

    dimensions.forEach((dim, i) => {
      const cell = document.createElement('div');
      cell.className = 'matrix-cell';
      cell.title = dim.name;
      // Color based on random score for demo
      const score = Math.random() * 0.6 + 0.2;
      cell.style.background = this.scoreToColor(score);
      container.appendChild(cell);
    });
  },

  /**
   * Initialize formulas display
   */
  initFormulas() {
    const container = document.getElementById('formulaList');
    if (!container || typeof CYNICFormulas === 'undefined') return;

    const formulas = CYNICFormulas.getFormulasLatex().slice(0, 3);
    formulas.forEach(f => {
      const item = document.createElement('div');
      item.style.cssText = 'padding: 6px 0; border-bottom: 1px solid var(--border); font-size: 11px;';
      CYNICFormulas.renderFormula(f.latex, item);
      container.appendChild(item);
    });
  },

  /**
   * Initialize navigation interactions
   */
  initNavigation() {
    // Package clicks
    document.querySelectorAll('#packagesList .nav-item').forEach(item => {
      item.addEventListener('click', () => {
        document.querySelectorAll('#packagesList .nav-item').forEach(i => i.classList.remove('active'));
        item.classList.add('active');
        const id = item.dataset.id;
        CYNICConsole.log('Selected package: ' + id, 'info');
        if (typeof CYNICViz !== 'undefined') {
          CYNICViz.focusObject(id);
        }
      });
    });

    // Dog clicks
    document.querySelectorAll('#dogsList .nav-item').forEach(item => {
      item.addEventListener('click', () => {
        const id = item.dataset.id;
        CYNICConsole.log('Selected dog: ' + id, 'info');
      });
    });

    // Codebase search
    const searchInput = document.getElementById('codebaseSearch');
    if (searchInput) {
      let searchTimeout;
      searchInput.addEventListener('input', (e) => {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
          const query = e.target.value.trim();
          if (query && typeof CYNICViz !== 'undefined') {
            CYNICViz.highlightSearch(query);
          } else if (typeof CYNICViz !== 'undefined') {
            CYNICViz.clearSearchHighlight();
          }
        }, 300);
      });
    }

    // Listen for codebase events
    document.addEventListener('codebase:itemSelected', (e) => {
      this.showSelectedItem(e.detail);
    });

    document.addEventListener('codebase:symbolSelected', (e) => {
      this.showSymbolDetails(e.detail);
    });

    document.addEventListener('codebase:levelChanged', (e) => {
      CYNICConsole.log('Level: ' + e.detail.level, 'system');
    });

    // Back button (Escape key)
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.currentView === 'codebase') {
        if (typeof CYNICViz !== 'undefined') {
          CYNICViz.navigateBack();
        }
      }
    });
  },

  /**
   * Initialize codebase view
   */
  async initCodebase() {
    if (!this.connected) {
      CYNICConsole.log('Codebase view requires MCP connection', 'error');
      return;
    }

    CYNICConsole.log('Loading codebase structure...', 'system');

    const result = await this.callTool('brain_codebase', { action: 'tree' });

    if (result.success && result.result) {
      this.state.codebase = result.result;
      this.updateCodebaseStats(result.result.stats);

      if (typeof CYNICViz !== 'undefined') {
        CYNICViz.loadCodebase(result.result);
      }

      CYNICConsole.log(result.result.message || 'Codebase loaded', 'output');
    } else {
      CYNICConsole.log('Failed to load codebase: ' + (result.error || 'Unknown error'), 'error');
    }
  },

  /**
   * Update codebase stats display
   */
  updateCodebaseStats(stats) {
    if (!stats) return;

    const elements = {
      statPackages: stats.packages,
      statModules: stats.modules,
      statClasses: stats.classes,
      statMethods: stats.methods,
      statLines: stats.lines,
    };

    for (const [id, value] of Object.entries(elements)) {
      const el = document.getElementById(id);
      if (el) el.textContent = value?.toLocaleString() || '--';
    }
  },

  /**
   * Show selected item details
   */
  showSelectedItem(item) {
    const infoSection = document.getElementById('selectedInfo');
    const nameEl = document.getElementById('selectedName');
    const typeEl = document.getElementById('selectedType');
    const detailsEl = document.getElementById('selectedDetails');

    if (!infoSection || !nameEl || !typeEl || !detailsEl) return;

    infoSection.style.display = 'block';
    nameEl.textContent = item.name || '--';
    typeEl.textContent = item.type || '--';

    // Build details based on type
    let details = '';
    const data = item.data || {};

    switch (item.type) {
      case 'package':
        details = `Modules: ${data.stats?.modules || 0}\nClasses: ${data.stats?.classes || 0}\nLines: ${data.stats?.lines || 0}`;
        break;
      case 'module':
        details = `Path: ${data.path || '--'}\nLines: ${data.lines || 0}\nClasses: ${(data.classes?.length || 0)}\nFunctions: ${(data.functions?.length || 0)}`;
        break;
      case 'class':
        details = `Line: ${data.line || '--'}\nMethods: ${(data.methods?.length || 0)}`;
        if (data.description) {
          details += '\n\n' + data.description.slice(0, 100);
        }
        break;
      case 'method':
      case 'function':
        details = `Line: ${data.line || '--'}\nParams: ${(data.params || []).join(', ') || 'none'}`;
        if (data.visibility) details += '\nVisibility: ' + data.visibility;
        if (data.async) details += '\nAsync: yes';
        break;
    }

    detailsEl.textContent = details;
  },

  /**
   * Show symbol details (deepest level)
   */
  showSymbolDetails(symbol) {
    CYNICConsole.log(`Symbol: ${symbol.name} @ line ${symbol.line}`, 'info');
    CYNICConsole.log(`  Package: ${symbol.package}, Module: ${symbol.module}`, 'info');
    if (symbol.params?.length) {
      CYNICConsole.log(`  Params: ${symbol.params.join(', ')}`, 'info');
    }
  },

  /**
   * Connect to MCP server
   */
  async connectMCP() {
    const statusEl = document.getElementById('liveStatus');

    try {
      // Try same-origin first (when served by MCP server)
      const origins = [
        '', // Same origin
        'http://localhost:3000',
        'http://localhost:3001',
      ];

      for (const origin of origins) {
        try {
          const healthUrl = origin ? origin + '/health' : '/health';
          const response = await fetch(healthUrl);

          if (response.ok) {
            const health = await response.json();
            this.connected = true;
            this.apiBase = origin ? origin + '/api/tools' : '/api/tools';
            this.state.health = health;

            if (statusEl) {
              statusEl.textContent = '● LIVE';
              statusEl.classList.add('connected');
            }

            CYNICConsole.log('Connected to CYNIC MCP', 'system');
            return;
          }
        } catch (e) {
          continue;
        }
      }

      // Offline mode
      if (statusEl) {
        statusEl.textContent = '● OFFLINE';
      }
      CYNICConsole.log('Running in offline mode', 'system');

    } catch (err) {
      console.warn('MCP connection failed:', err.message);
      if (statusEl) {
        statusEl.textContent = '● OFFLINE';
      }
    }
  },

  /**
   * Load all data
   */
  async loadData() {
    if (this.connected) {
      await Promise.all([
        this.loadHealth(),
        this.loadChain(),
        this.loadDogs()
      ]);
    } else {
      this.loadMockData();
    }
  },

  /**
   * Load health data
   */
  async loadHealth() {
    const result = await this.callTool('brain_health', { verbose: true });
    if (result.success) {
      this.state.health = result.result;
      this.updateServiceStatus(result.result);
    }
  },

  /**
   * Load chain data
   */
  async loadChain() {
    const result = await this.callTool('brain_poj_chain', { action: 'recent', limit: 5 });
    if (result.success && result.result) {
      this.state.chain = result.result;
      this.updateChainDisplay(result.result);
    }
  },

  /**
   * Load dogs data
   */
  async loadDogs() {
    const result = await this.callTool('brain_agents_status', { verbose: true });
    if (result.success) {
      this.state.dogs = result.result;
      this.updateDogsDisplay(result.result);
    }
  },

  /**
   * Load mock data for offline mode
   */
  loadMockData() {
    // Mock chain
    this.updateChainDisplay({
      head: { blockNumber: 47 },
      stats: { totalJudgments: 847 },
      blocks: [
        { blockNumber: 43, judgmentCount: 3 },
        { blockNumber: 44, judgmentCount: 5 },
        { blockNumber: 45, judgmentCount: 2 },
        { blockNumber: 46, judgmentCount: 4 },
        { blockNumber: 47, judgmentCount: 1 },
      ]
    });
  },

  /**
   * Update service status indicators
   */
  updateServiceStatus(health) {
    const pgEl = document.getElementById('svcPostgres');
    const redisEl = document.getElementById('svcRedis');

    if (pgEl) {
      const pgOk = health.persistence?.postgres?.status === 'ok';
      pgEl.classList.toggle('active', pgOk);
      pgEl.classList.toggle('inactive', !pgOk);
    }

    if (redisEl) {
      const redisOk = health.persistence?.redis?.status === 'ok';
      redisEl.classList.toggle('active', redisOk);
      redisEl.classList.toggle('inactive', !redisOk);
    }
  },

  /**
   * Update chain display (using safe DOM methods)
   */
  updateChainDisplay(chain) {
    // Sidebar stats
    const headEl = document.getElementById('chainHead');
    const totalEl = document.getElementById('chainTotal');
    const pendingEl = document.getElementById('chainPending');

    if (headEl && chain.head) {
      headEl.textContent = '#' + chain.head.blockNumber;
    }
    if (totalEl && chain.stats) {
      totalEl.textContent = chain.stats.totalJudgments || '0';
    }
    if (pendingEl) {
      pendingEl.textContent = chain.pending || '0';
    }

    // Mini chain visualization
    const container = document.getElementById('chainMini');
    if (container && chain.blocks) {
      // Clear container safely
      while (container.firstChild) {
        container.removeChild(container.firstChild);
      }

      chain.blocks.slice(-5).forEach((block, i) => {
        if (i > 0) {
          const conn = document.createElement('span');
          conn.className = 'chain-connector';
          conn.textContent = '→';
          container.appendChild(conn);
        }

        const blockEl = document.createElement('div');
        blockEl.className = 'chain-block';

        const numberSpan = document.createElement('span');
        numberSpan.className = 'number';
        numberSpan.textContent = '#' + block.blockNumber;

        const countSpan = document.createElement('span');
        countSpan.className = 'count';
        countSpan.textContent = block.judgmentCount || 0;

        blockEl.appendChild(numberSpan);
        blockEl.appendChild(countSpan);
        container.appendChild(blockEl);
      });
    }
  },

  /**
   * Update dogs display
   */
  updateDogsDisplay(dogsData) {
    if (!dogsData.agents) return;

    for (const [name, data] of Object.entries(dogsData.agents)) {
      const item = document.querySelector('#dogsList [data-id="' + name + '"]');
      if (item) {
        const indicator = item.querySelector('.indicator');
        if (indicator) {
          indicator.classList.toggle('active', data.enabled);
          indicator.classList.toggle('inactive', !data.enabled);
        }
      }
    }
  },

  /**
   * Convert score to color
   */
  scoreToColor(score) {
    // Low = red, mid = yellow, high = green
    const r = Math.round(255 * (1 - score));
    const g = Math.round(200 * score);
    const b = Math.round(100 * score);
    return 'rgb(' + r + ',' + g + ',' + b + ')';
  },

  /**
   * Handle view change
   */
  onViewChange(view) {
    this.currentView = view;
    CYNICConsole.log('View: ' + view, 'system');

    // Toggle UI sections based on view
    const codebaseStats = document.getElementById('codebaseStats');
    const axiomsSection = document.getElementById('axiomsSection');
    const selectedInfo = document.getElementById('selectedInfo');
    const searchBox = document.querySelector('.viz-search');
    const breadcrumb = document.getElementById('breadcrumb');
    const vizHint = document.getElementById('vizHint');

    if (view === 'codebase') {
      // Show codebase UI
      if (codebaseStats) codebaseStats.style.display = 'block';
      if (axiomsSection) axiomsSection.style.display = 'none';
      if (searchBox) searchBox.style.display = 'flex';
      if (breadcrumb) breadcrumb.style.display = 'flex';
      if (vizHint) vizHint.style.display = 'block';

      // Load codebase if not already loaded
      if (!this.state.codebase) {
        this.initCodebase();
      } else if (typeof CYNICViz !== 'undefined') {
        CYNICViz.loadCodebase(this.state.codebase);
      }
    } else {
      // Show architecture UI
      if (codebaseStats) codebaseStats.style.display = 'none';
      if (axiomsSection) axiomsSection.style.display = 'block';
      if (selectedInfo) selectedInfo.style.display = 'none';
      if (searchBox) searchBox.style.display = 'none';
      if (breadcrumb) breadcrumb.style.display = 'none';
      if (vizHint) vizHint.style.display = 'none';

      // Restore architecture view
      if (typeof CYNICViz !== 'undefined') {
        CYNICViz.clearCodebaseObjects();
        // Re-create static architecture
        CYNICViz.createPackages();
        CYNICViz.createDogs();
        CYNICViz.createConnections();
      }
    }
  },

  /**
   * Call MCP tool
   */
  async callTool(toolName, params = {}) {
    if (!this.connected || !this.apiBase) {
      return { success: false, error: 'Not connected' };
    }

    try {
      const response = await fetch(this.apiBase + '/' + toolName, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params)
      });

      return await response.json();
    } catch (err) {
      return { success: false, error: err.message };
    }
  }
};

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
  CYNICEngine.init();
});

window.CYNICEngine = CYNICEngine;
