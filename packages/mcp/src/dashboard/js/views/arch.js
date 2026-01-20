/**
 * CYNIC Dashboard - Architecture View
 * 3D Sefirot tree + hierarchical codebase explorer with semantic zoom
 */

import { Utils } from '../lib/utils.js';
import { Formulas } from '../lib/formulas.js';
import { ThreeScene } from '../lib/three-scene.js';
import { CodebaseGraphData, CodebaseNodeType, CodebaseColors } from '../lib/codebase-graph-data.js';
import { CodebaseGraphScene } from '../lib/codebase-graph-scene.js';

export class ArchView {
  constructor(options = {}) {
    this.api = options.api;
    this.container = null;

    // Scenes
    this.sefirotScene = null;      // For Sefirot view
    this.codebaseScene = null;     // For Codebase view
    this.currentView = 'sefirot';

    // Data
    this.codebaseData = null;
    this.selectedNode = null;
    this.searchResults = [];

    // UI state
    this.breadcrumbPath = ['CYNIC'];
    this.isLoading = false;
  }

  /**
   * Render architecture view
   */
  render(container) {
    this.container = container;
    Utils.clearElement(container);
    container.classList.add('arch-view');

    const layout = Utils.createElement('div', { className: 'arch-layout' });

    // Left panel: Navigation + Details
    const leftPanel = this._createLeftPanel();

    // Right panel: 3D visualization
    const rightPanel = this._createRightPanel();

    layout.appendChild(leftPanel);
    layout.appendChild(rightPanel);
    container.appendChild(layout);

    // Initialize scenes
    this._initScenes();

    // Load initial data
    this._loadSefirotTree();
  }

  /**
   * Create left panel (navigation, search, tree, details)
   */
  _createLeftPanel() {
    const leftPanel = Utils.createElement('div', { className: 'arch-panel arch-panel-left' });

    // Breadcrumb
    const breadcrumb = Utils.createElement('div', { className: 'arch-breadcrumb', id: 'arch-breadcrumb' });
    this._renderBreadcrumb(breadcrumb);

    // Search
    const searchBox = Utils.createElement('div', { className: 'arch-search' }, [
      Utils.createElement('input', {
        type: 'text',
        className: 'arch-search-input',
        id: 'arch-search',
        placeholder: 'Search symbols... (Enter to search)',
      }),
      Utils.createElement('div', { className: 'arch-search-results', id: 'arch-search-results' }),
    ]);

    const searchInput = searchBox.querySelector('input');
    searchInput.addEventListener('keyup', (e) => {
      if (e.key === 'Enter') {
        this._search(e.target.value);
      } else if (e.target.value.length >= 2) {
        this._liveSearch(e.target.value);
      } else {
        this._clearSearchResults();
      }
    });

    // Stats bar
    const statsBar = Utils.createElement('div', { className: 'arch-stats', id: 'arch-stats' });
    this._renderStats(statsBar);

    // Tree view
    const treeContainer = Utils.createElement('div', { className: 'arch-tree', id: 'arch-tree' });

    // Details panel
    const detailsPanel = Utils.createElement('div', { className: 'arch-details', id: 'arch-details' });

    leftPanel.appendChild(breadcrumb);
    leftPanel.appendChild(searchBox);
    leftPanel.appendChild(statsBar);
    leftPanel.appendChild(treeContainer);
    leftPanel.appendChild(detailsPanel);

    return leftPanel;
  }

  /**
   * Create right panel (view toggle + 3D canvas + legend)
   */
  _createRightPanel() {
    const rightPanel = Utils.createElement('div', { className: 'arch-panel arch-panel-right' });

    // View mode toggle
    const viewToggle = Utils.createElement('div', { className: 'arch-view-toggle' }, [
      Utils.createElement('button', {
        className: 'arch-toggle-btn active',
        dataset: { view: 'sefirot' },
        onClick: () => this._setView('sefirot'),
      }, ['🐕 Sefirot']),
      Utils.createElement('button', {
        className: 'arch-toggle-btn',
        dataset: { view: 'codebase' },
        onClick: () => this._setView('codebase'),
      }, ['📦 Codebase']),
    ]);

    // 3D canvas containers
    const sefirotCanvas = Utils.createElement('div', {
      className: 'arch-canvas',
      id: 'arch-canvas-sefirot',
      style: 'display: block;',
    });

    const codebaseCanvas = Utils.createElement('div', {
      className: 'arch-canvas',
      id: 'arch-canvas-codebase',
      style: 'display: none;',
    });

    // Navigation buttons (for codebase view)
    const navButtons = Utils.createElement('div', {
      className: 'arch-nav-buttons',
      id: 'arch-nav-buttons',
      style: 'display: none;',
    }, [
      Utils.createElement('button', {
        className: 'arch-nav-btn',
        id: 'arch-nav-up',
        title: 'Zoom out (parent)',
        onClick: () => this._navigateUp(),
      }, ['⬆️ Up']),
      Utils.createElement('button', {
        className: 'arch-nav-btn',
        id: 'arch-nav-reset',
        title: 'Reset view',
        onClick: () => this._resetView(),
      }, ['🔄 Reset']),
    ]);

    // Legend
    const legend = Utils.createElement('div', { className: 'arch-legend', id: 'arch-legend' });
    this._renderLegend(legend, 'sefirot');

    rightPanel.appendChild(viewToggle);
    rightPanel.appendChild(sefirotCanvas);
    rightPanel.appendChild(codebaseCanvas);
    rightPanel.appendChild(navButtons);
    rightPanel.appendChild(legend);

    return rightPanel;
  }

  /**
   * Initialize Three.js scenes
   */
  _initScenes() {
    // Sefirot scene (using existing ThreeScene)
    this.sefirotScene = new ThreeScene('arch-canvas-sefirot');
    this.sefirotScene.onObjectClick = (data) => this._onSefirotClick(data);
    this.sefirotScene.onObjectHover = (data) => this._onSefirotHover(data);
    this.sefirotScene.init();

    // Codebase scene (using new CodebaseGraphScene)
    this.codebaseScene = new CodebaseGraphScene('arch-canvas-codebase');
    this.codebaseScene.onNodeClick = (nodeId) => this._onCodebaseNodeClick(nodeId);
    this.codebaseScene.onNodeDoubleClick = (nodeId) => this._onCodebaseNodeDoubleClick(nodeId);
    this.codebaseScene.onNodeHover = (nodeId) => this._onCodebaseNodeHover(nodeId);
    // Note: init() called when switching to codebase view

    // Initialize codebase data model
    this.codebaseData = new CodebaseGraphData();
    this.codebaseData.onFocusChanged = (node, prev) => this._onCodebaseFocusChanged(node, prev);
  }

  /**
   * Set view mode
   */
  _setView(view) {
    this.currentView = view;

    // Update toggle buttons
    this.container?.querySelectorAll('.arch-toggle-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.view === view);
    });

    // Toggle canvas visibility
    const sefirotCanvas = document.getElementById('arch-canvas-sefirot');
    const codebaseCanvas = document.getElementById('arch-canvas-codebase');
    const navButtons = document.getElementById('arch-nav-buttons');

    if (view === 'sefirot') {
      sefirotCanvas.style.display = 'block';
      codebaseCanvas.style.display = 'none';
      navButtons.style.display = 'none';
      this._loadSefirotTree();
    } else {
      sefirotCanvas.style.display = 'none';
      codebaseCanvas.style.display = 'block';
      navButtons.style.display = 'flex';

      // Initialize codebase scene if needed
      if (!this.codebaseScene.isInitialized) {
        this.codebaseScene.init();
      }

      this._loadCodebaseTree();
    }

    // Update legend
    const legend = document.getElementById('arch-legend');
    if (legend) this._renderLegend(legend, view);
  }

  // ═══════════════════════════════════════════════════════════
  // SEFIROT VIEW
  // ═══════════════════════════════════════════════════════════

  /**
   * Load Sefirot tree
   */
  _loadSefirotTree() {
    const treeContainer = document.getElementById('arch-tree');
    if (!treeContainer) return;

    Utils.clearElement(treeContainer);

    // Create Sefirot hierarchy
    const sefirotData = this._buildSefirotHierarchy();

    // Render tree
    const tree = this._renderSefirotTree(sefirotData);
    treeContainer.appendChild(tree);

    // Update 3D scene
    if (this.sefirotScene) {
      this.sefirotScene.loadSefirotTree(Formulas.DOGS);
    }

    // Update breadcrumb
    this.breadcrumbPath = ['CYNIC'];
    const breadcrumb = document.getElementById('arch-breadcrumb');
    if (breadcrumb) this._renderBreadcrumb(breadcrumb);
  }

  /**
   * Build Sefirot hierarchy from DOGS data
   */
  _buildSefirotHierarchy() {
    return {
      name: 'CYNIC',
      type: 'root',
      sefirot: 'Keter',
      children: [
        {
          name: 'Sage', type: 'dog', sefirot: 'Chochmah',
          children: [{ name: 'Scholar', type: 'dog', sefirot: 'Daat', children: [] }],
        },
        {
          name: 'Analyst', type: 'dog', sefirot: 'Binah',
          children: [{ name: 'Scholar', type: 'dog', sefirot: 'Daat', shared: true, children: [] }],
        },
        {
          name: 'Architect', type: 'dog', sefirot: 'Chesed',
          children: [{ name: 'Scout', type: 'dog', sefirot: 'Netzach', children: [] }],
        },
        {
          name: 'Oracle', type: 'dog', sefirot: 'Tiferet',
          children: [{ name: 'Janitor', type: 'dog', sefirot: 'Yesod', children: [] }],
        },
        {
          name: 'Guardian', type: 'dog', sefirot: 'Gevurah',
          children: [{ name: 'Deployer', type: 'dog', sefirot: 'Hod', children: [] }],
        },
        { name: 'Cartographer', type: 'dog', sefirot: 'Malkhut', children: [] },
      ],
    };
  }

  /**
   * Render Sefirot tree
   */
  _renderSefirotTree(node, depth = 0) {
    const item = Utils.createElement('div', {
      className: `tree-item ${node.type} ${node.shared ? 'shared' : ''}`,
      dataset: { name: node.name, type: node.type },
      style: `--depth: ${depth}`,
    });

    const header = Utils.createElement('div', {
      className: 'tree-item-header',
      onClick: () => this._selectSefirotNode(node),
    }, [
      Utils.createElement('span', { className: 'tree-icon' }, [this._getDogIcon(node.name)]),
      Utils.createElement('span', { className: 'tree-name' }, [node.name]),
      node.sefirot ? Utils.createElement('span', { className: 'tree-sefirot' }, [node.sefirot]) : null,
    ].filter(Boolean));

    item.appendChild(header);

    if (node.children && node.children.length > 0) {
      const childrenContainer = Utils.createElement('div', { className: 'tree-children' });
      for (const child of node.children) {
        childrenContainer.appendChild(this._renderSefirotTree(child, depth + 1));
      }
      item.appendChild(childrenContainer);
    }

    return item;
  }

  _getDogIcon(name) {
    const icons = {
      CYNIC: '🧠', Sage: '🦉', Analyst: '📊', Scholar: '📚', Architect: '🏗️',
      Guardian: '🛡️', Oracle: '🔮', Scout: '🔭', Deployer: '🚀', Janitor: '🧹', Cartographer: '🗺️',
    };
    return icons[name] || '🐕';
  }

  _selectSefirotNode(node) {
    this.selectedNode = node;

    // Update tree selection
    this.container?.querySelectorAll('.tree-item').forEach(el => {
      el.classList.toggle('selected', el.dataset.name === node.name);
    });

    // Update details
    this._showSefirotDetails(node);

    // Update 3D
    if (this.sefirotScene) {
      this.sefirotScene.highlightNode(node.name);
    }

    // Update breadcrumb
    this.breadcrumbPath = ['CYNIC', node.name];
    const breadcrumb = document.getElementById('arch-breadcrumb');
    if (breadcrumb) this._renderBreadcrumb(breadcrumb);
  }

  _showSefirotDetails(node) {
    const panel = document.getElementById('arch-details');
    if (!panel) return;

    Utils.clearElement(panel);

    const dog = Formulas.DOGS.find(d => d.name === node.name);

    const content = Utils.createElement('div', { className: 'details-content' }, [
      Utils.createElement('div', { className: 'details-header' }, [
        Utils.createElement('span', { className: 'details-icon' }, [this._getDogIcon(node.name)]),
        Utils.createElement('h3', {}, [node.name]),
      ]),
      node.sefirot ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Sefirot:']),
        Utils.createElement('span', { className: 'value sefirot' }, [node.sefirot]),
      ]) : null,
      dog?.role ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Role:']),
        Utils.createElement('span', { className: 'value' }, [dog.role]),
      ]) : null,
      dog?.active !== undefined ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Status:']),
        Utils.createElement('span', { className: `value status ${dog.active ? 'active' : 'inactive'}` }, [
          dog.active ? '● Active' : '○ Inactive',
        ]),
      ]) : null,
    ].filter(Boolean));

    panel.appendChild(content);
  }

  _onSefirotClick(data) {
    if (data?.name) {
      this._selectSefirotNode({ name: data.name, type: data.type, sefirot: data.sefirot });
    }
  }

  _onSefirotHover(data) {
    // Could show tooltip
  }

  // ═══════════════════════════════════════════════════════════
  // CODEBASE VIEW
  // ═══════════════════════════════════════════════════════════

  /**
   * Load codebase tree from API
   */
  async _loadCodebaseTree() {
    const treeContainer = document.getElementById('arch-tree');
    if (!treeContainer) return;

    // Show loading
    this.isLoading = true;
    Utils.clearElement(treeContainer);
    treeContainer.appendChild(
      Utils.createElement('div', { className: 'loading-indicator' }, ['📦 Loading codebase...'])
    );

    try {
      // Fetch from API
      if (this.api) {
        const result = await this.api.codebase('tree');
        if (result.success && result.result?.packages?.length > 0) {
          // Load into data model
          this.codebaseData.loadFromAPI(result.result);

          // Connect scene to data
          this.codebaseScene.setData(this.codebaseData);

          // Render tree UI
          this._renderCodebaseTreeUI(treeContainer);

          // Render 3D
          this.codebaseScene.render();

          // Update stats
          this._renderStats(document.getElementById('arch-stats'));

          // Update breadcrumb
          this.breadcrumbPath = ['packages'];
          const breadcrumb = document.getElementById('arch-breadcrumb');
          if (breadcrumb) this._renderBreadcrumb(breadcrumb);

          this.isLoading = false;
          return;
        }
      }
    } catch (e) {
      console.error('Failed to load codebase:', e);
    }

    // Fallback
    Utils.clearElement(treeContainer);
    treeContainer.appendChild(
      Utils.createElement('div', { className: 'error-message' }, ['Failed to load codebase. Check API connection.'])
    );
    this.isLoading = false;
  }

  /**
   * Render codebase tree UI
   */
  _renderCodebaseTreeUI(container) {
    Utils.clearElement(container);

    if (!this.codebaseData.root) return;

    // Render from current focus
    const focusNode = this.codebaseData.currentFocus || this.codebaseData.root;
    const tree = this._renderCodebaseNodeUI(focusNode, 0, true);
    container.appendChild(tree);
  }

  /**
   * Render single codebase node UI
   */
  _renderCodebaseNodeUI(node, depth, isExpanded = false) {
    const typeIcons = {
      [CodebaseNodeType.PACKAGE]: '📦',
      [CodebaseNodeType.MODULE]: '📄',
      [CodebaseNodeType.CLASS]: '🔷',
      [CodebaseNodeType.FUNCTION]: '⚡',
      [CodebaseNodeType.METHOD]: '🔸',
    };

    const item = Utils.createElement('div', {
      className: `tree-item ${node.type}${node.highlighted ? ' highlighted' : ''}`,
      dataset: { nodeId: node.id, type: node.type },
      style: `--depth: ${depth}`,
    });

    const hasChildren = node.children.length > 0;

    const header = Utils.createElement('div', {
      className: 'tree-item-header',
      onClick: () => this._onCodebaseTreeClick(node),
      onDblclick: () => this._onCodebaseTreeDoubleClick(node),
    }, [
      hasChildren ? Utils.createElement('span', { className: `tree-expand ${isExpanded ? 'expanded' : ''}` }, ['▶']) : null,
      Utils.createElement('span', { className: 'tree-icon' }, [typeIcons[node.type] || '📁']),
      Utils.createElement('span', { className: 'tree-name' }, [node.name]),
      node.children.length > 0 ? Utils.createElement('span', { className: 'tree-count' }, [`(${node.children.length})`]) : null,
    ].filter(Boolean));

    item.appendChild(header);

    // Show children if expanded
    if (isExpanded && hasChildren) {
      const childrenContainer = Utils.createElement('div', { className: 'tree-children' });
      for (const child of node.children.slice(0, 30)) { // Limit display
        childrenContainer.appendChild(this._renderCodebaseNodeUI(child, depth + 1, false));
      }
      if (node.children.length > 30) {
        childrenContainer.appendChild(
          Utils.createElement('div', { className: 'tree-more' }, [`... and ${node.children.length - 30} more`])
        );
      }
      item.appendChild(childrenContainer);
    }

    return item;
  }

  _onCodebaseTreeClick(node) {
    this.selectedNode = node;

    // Update tree selection
    this.container?.querySelectorAll('.tree-item').forEach(el => {
      el.classList.toggle('selected', el.dataset.nodeId === node.id);
    });

    // Show details
    this._showCodebaseDetails(node);

    // Highlight in 3D
    this.codebaseScene._selectNode(node.id);

    // Update breadcrumb
    this.breadcrumbPath = node.getPath().map(n => n.name);
    const breadcrumb = document.getElementById('arch-breadcrumb');
    if (breadcrumb) this._renderBreadcrumb(breadcrumb);
  }

  _onCodebaseTreeDoubleClick(node) {
    if (node.children.length > 0) {
      this.codebaseData.setFocus(node);
    }
  }

  _onCodebaseNodeClick(nodeId) {
    const node = this.codebaseData.getNode(nodeId);
    if (node) {
      this._onCodebaseTreeClick(node);
    }
  }

  _onCodebaseNodeDoubleClick(nodeId) {
    const node = this.codebaseData.getNode(nodeId);
    if (node && node.children.length > 0) {
      this.codebaseData.setFocus(node);
    }
  }

  _onCodebaseNodeHover(nodeId) {
    // Could show tooltip or highlight tree item
  }

  _onCodebaseFocusChanged(node, prev) {
    // Re-render tree from new focus
    const treeContainer = document.getElementById('arch-tree');
    if (treeContainer) {
      this._renderCodebaseTreeUI(treeContainer);
    }

    // Update breadcrumb
    this.breadcrumbPath = node.getPath().map(n => n.name);
    const breadcrumb = document.getElementById('arch-breadcrumb');
    if (breadcrumb) this._renderBreadcrumb(breadcrumb);
  }

  _showCodebaseDetails(node) {
    const panel = document.getElementById('arch-details');
    if (!panel) return;

    Utils.clearElement(panel);

    const typeLabels = {
      [CodebaseNodeType.PACKAGE]: 'Package',
      [CodebaseNodeType.MODULE]: 'Module',
      [CodebaseNodeType.CLASS]: 'Class',
      [CodebaseNodeType.FUNCTION]: 'Function',
      [CodebaseNodeType.METHOD]: 'Method',
    };

    const content = Utils.createElement('div', { className: 'details-content' }, [
      Utils.createElement('div', { className: 'details-header' }, [
        Utils.createElement('h3', {}, [node.name]),
        Utils.createElement('span', {
          className: 'details-type-badge',
          style: `background-color: #${node.color.toString(16).padStart(6, '0')}`,
        }, [typeLabels[node.type] || node.type]),
      ]),

      node.data.path ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Path:']),
        Utils.createElement('span', { className: 'value code' }, [node.data.path]),
      ]) : null,

      node.data.lines ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Lines:']),
        Utils.createElement('span', { className: 'value' }, [Utils.formatNumber(node.data.lines)]),
      ]) : null,

      node.data.line ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Line:']),
        Utils.createElement('span', { className: 'value' }, [String(node.data.line)]),
      ]) : null,

      node.data.methodCount ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Methods:']),
        Utils.createElement('span', { className: 'value' }, [String(node.data.methodCount)]),
      ]) : null,

      node.data.params?.length > 0 ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Params:']),
        Utils.createElement('span', { className: 'value code' }, [`(${node.data.params.join(', ')})`]),
      ]) : null,

      node.data.visibility ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Visibility:']),
        Utils.createElement('span', { className: 'value' }, [node.data.visibility]),
      ]) : null,

      node.data.exported ? Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Exported:']),
        Utils.createElement('span', { className: 'value' }, ['✓ Yes']),
      ]) : null,

      Utils.createElement('div', { className: 'details-row' }, [
        Utils.createElement('span', { className: 'label' }, ['Children:']),
        Utils.createElement('span', { className: 'value' }, [String(node.children.length)]),
      ]),
    ].filter(Boolean));

    panel.appendChild(content);
  }

  /**
   * Navigate up (zoom out)
   */
  _navigateUp() {
    if (this.codebaseData?.currentFocus?.parent) {
      this.codebaseData.focusParent();
    }
  }

  /**
   * Reset view to root
   */
  _resetView() {
    if (this.codebaseData?.root) {
      this.codebaseData.setFocus(this.codebaseData.root);
    }
  }

  // ═══════════════════════════════════════════════════════════
  // SEARCH
  // ═══════════════════════════════════════════════════════════

  /**
   * Live search as user types
   */
  _liveSearch(query) {
    if (this.currentView !== 'codebase' || !this.codebaseData) {
      this._clearSearchResults();
      return;
    }

    this.searchResults = this.codebaseData.search(query, 10);
    this._renderSearchResults();
  }

  /**
   * Full search on Enter
   */
  async _search(query) {
    if (!query) {
      this._clearSearchResults();
      return;
    }

    if (this.currentView === 'codebase' && this.codebaseData) {
      this.searchResults = this.codebaseData.search(query, 20);
      this._renderSearchResults();
    } else if (this.api) {
      // Search via API for Sefirot mode
      const result = await this.api.searchCodebase(query);
      if (result.success && result.result) {
        console.log('Search results:', result.result);
      }
    }
  }

  _renderSearchResults() {
    const container = document.getElementById('arch-search-results');
    if (!container) return;

    Utils.clearElement(container);

    if (this.searchResults.length === 0) {
      return;
    }

    const typeIcons = {
      [CodebaseNodeType.PACKAGE]: '📦',
      [CodebaseNodeType.MODULE]: '📄',
      [CodebaseNodeType.CLASS]: '🔷',
      [CodebaseNodeType.FUNCTION]: '⚡',
      [CodebaseNodeType.METHOD]: '🔸',
    };

    for (const node of this.searchResults) {
      const item = Utils.createElement('div', {
        className: 'search-result-item',
        onClick: () => {
          this._clearSearchResults();
          this.codebaseData.setFocus(node.parent || node);
          this._onCodebaseTreeClick(node);
        },
      }, [
        Utils.createElement('span', { className: 'search-result-icon' }, [typeIcons[node.type] || '📁']),
        Utils.createElement('span', { className: 'search-result-name' }, [node.name]),
        Utils.createElement('span', { className: 'search-result-type' }, [node.type]),
      ]);
      container.appendChild(item);
    }
  }

  _clearSearchResults() {
    this.searchResults = [];
    const container = document.getElementById('arch-search-results');
    if (container) Utils.clearElement(container);
  }

  // ═══════════════════════════════════════════════════════════
  // UI HELPERS
  // ═══════════════════════════════════════════════════════════

  /**
   * Render breadcrumb navigation
   */
  _renderBreadcrumb(container) {
    Utils.clearElement(container);

    this.breadcrumbPath.forEach((item, i) => {
      if (i > 0) {
        container.appendChild(
          Utils.createElement('span', { className: 'breadcrumb-separator' }, ['/'])
        );
      }

      const isLast = i === this.breadcrumbPath.length - 1;
      container.appendChild(
        Utils.createElement('span', {
          className: `breadcrumb-item ${isLast ? 'active' : ''}`,
          onClick: () => {
            if (!isLast && this.currentView === 'codebase') {
              // Navigate to that level
              const pathToNavigate = this.breadcrumbPath.slice(0, i + 1);
              // Find node matching path
              // For now just reset if clicking root
              if (i === 0) this._resetView();
            }
          },
        }, [item])
      );
    });
  }

  /**
   * Render stats bar
   */
  _renderStats(container) {
    if (!container) return;
    Utils.clearElement(container);

    if (this.currentView === 'codebase' && this.codebaseData) {
      const stats = this.codebaseData.getStats();
      container.appendChild(
        Utils.createElement('div', { className: 'stats-row' }, [
          Utils.createElement('span', { className: 'stat' }, [`📦 ${stats.packages}`]),
          Utils.createElement('span', { className: 'stat' }, [`📄 ${stats.modules}`]),
          Utils.createElement('span', { className: 'stat' }, [`🔷 ${stats.classes}`]),
          Utils.createElement('span', { className: 'stat' }, [`⚡ ${stats.functions}`]),
          Utils.createElement('span', { className: 'stat' }, [`📝 ${Utils.formatNumber(stats.lines)} lines`]),
        ])
      );
    }
  }

  /**
   * Render legend
   */
  _renderLegend(container, view) {
    Utils.clearElement(container);

    if (view === 'sefirot') {
      const items = [
        { color: '#4ecdc4', label: 'Keter (Crown)' },
        { color: '#667eea', label: 'Chochmah (Wisdom)' },
        { color: '#764ba2', label: 'Binah (Understanding)' },
        { color: '#f093fb', label: 'Daat (Knowledge)' },
        { color: '#00d9ff', label: 'Chesed (Mercy)' },
        { color: '#ff6b6b', label: 'Gevurah (Severity)' },
        { color: '#ffd93d', label: 'Tiferet (Beauty)' },
      ];

      for (const item of items) {
        container.appendChild(
          Utils.createElement('div', { className: 'legend-item' }, [
            Utils.createElement('span', { className: 'legend-color', style: `background-color: ${item.color}` }),
            Utils.createElement('span', { className: 'legend-label' }, [item.label]),
          ])
        );
      }
    } else {
      const items = [
        { color: CodebaseColors[CodebaseNodeType.PACKAGE], label: 'Package', shape: '●' },
        { color: CodebaseColors[CodebaseNodeType.MODULE], label: 'Module', shape: '■' },
        { color: CodebaseColors[CodebaseNodeType.CLASS], label: 'Class', shape: '◆' },
        { color: CodebaseColors[CodebaseNodeType.FUNCTION], label: 'Function', shape: '◇' },
        { color: CodebaseColors[CodebaseNodeType.METHOD], label: 'Method', shape: '▲' },
      ];

      for (const item of items) {
        const colorHex = '#' + item.color.toString(16).padStart(6, '0');
        container.appendChild(
          Utils.createElement('div', { className: 'legend-item' }, [
            Utils.createElement('span', { className: 'legend-shape', style: `color: ${colorHex}` }, [item.shape]),
            Utils.createElement('span', { className: 'legend-label' }, [item.label]),
          ])
        );
      }
    }
  }

  // ═══════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════

  /**
   * Handle SSE events
   */
  handleEvent(event) {
    // Arch view doesn't need real-time updates
  }

  /**
   * Refresh data
   */
  async refresh() {
    if (this.currentView === 'codebase') {
      await this._loadCodebaseTree();
    }
  }

  /**
   * Cleanup
   */
  destroy() {
    if (this.sefirotScene) {
      this.sefirotScene.dispose();
      this.sefirotScene = null;
    }
    if (this.codebaseScene) {
      this.codebaseScene.dispose();
      this.codebaseScene = null;
    }
    this.container = null;
  }
}

// Export to window
window.CYNICArchView = ArchView;
