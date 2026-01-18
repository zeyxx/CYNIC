/**
 * CYNIC Dashboard - Architecture View
 * 3D Sefirot tree + codebase explorer
 */

import { Utils } from '../lib/utils.js';
import { Formulas } from '../lib/formulas.js';
import { ThreeScene } from '../lib/three-scene.js';

export class ArchView {
  constructor(options = {}) {
    this.api = options.api;
    this.container = null;
    this.threeScene = null;
    this.currentPath = [];
    this.codebaseData = null;
    this.selectedNode = null;
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
        placeholder: 'Search symbols...',
      }),
    ]);
    const searchInput = searchBox.querySelector('input');
    searchInput.addEventListener('keyup', (e) => {
      if (e.key === 'Enter') this._search(e.target.value);
    });

    // Tree view
    const treeContainer = Utils.createElement('div', { className: 'arch-tree', id: 'arch-tree' });

    // Details panel
    const detailsPanel = Utils.createElement('div', { className: 'arch-details', id: 'arch-details' });

    leftPanel.appendChild(breadcrumb);
    leftPanel.appendChild(searchBox);
    leftPanel.appendChild(treeContainer);
    leftPanel.appendChild(detailsPanel);

    // Right panel: 3D visualization
    const rightPanel = Utils.createElement('div', { className: 'arch-panel arch-panel-right' });

    // View mode toggle
    const viewToggle = Utils.createElement('div', { className: 'arch-view-toggle' }, [
      Utils.createElement('button', {
        className: 'arch-toggle-btn active',
        dataset: { view: 'sefirot' },
        onClick: () => this._setView('sefirot'),
      }, ['Sefirot']),
      Utils.createElement('button', {
        className: 'arch-toggle-btn',
        dataset: { view: 'codebase' },
        onClick: () => this._setView('codebase'),
      }, ['Codebase']),
    ]);

    // 3D canvas container
    const canvasContainer = Utils.createElement('div', { className: 'arch-canvas', id: 'arch-canvas' });

    // Legend
    const legend = Utils.createElement('div', { className: 'arch-legend', id: 'arch-legend' });
    this._renderLegend(legend);

    rightPanel.appendChild(viewToggle);
    rightPanel.appendChild(canvasContainer);
    rightPanel.appendChild(legend);

    layout.appendChild(leftPanel);
    layout.appendChild(rightPanel);
    container.appendChild(layout);

    // Initialize 3D scene
    this._initThreeScene();

    // Load initial data
    this._loadSefirotTree();
  }

  /**
   * Initialize Three.js scene
   */
  _initThreeScene() {
    const canvasContainer = document.getElementById('arch-canvas');
    if (!canvasContainer) return;

    // ThreeScene expects container ID string
    this.threeScene = new ThreeScene('arch-canvas');

    // Set event callbacks
    this.threeScene.onObjectClick = (data) => this._onNodeClick(data);
    this.threeScene.onObjectHover = (data) => this._onNodeHover(data);

    this.threeScene.init();
  }

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
    const tree = this._renderTree(sefirotData);
    treeContainer.appendChild(tree);

    // Update 3D scene
    if (this.threeScene) {
      this.threeScene.loadSefirotTree(Formulas.DOGS);
    }
  }

  /**
   * Build Sefirot hierarchy from DOGS data
   */
  _buildSefirotHierarchy() {
    // Sefirot tree structure
    return {
      name: 'CYNIC',
      type: 'root',
      sefirot: 'Keter',
      children: [
        {
          name: 'Sage',
          type: 'dog',
          sefirot: 'Chochmah',
          children: [
            { name: 'Scholar', type: 'dog', sefirot: 'Daat', children: [] },
          ],
        },
        {
          name: 'Analyst',
          type: 'dog',
          sefirot: 'Binah',
          children: [
            { name: 'Scholar', type: 'dog', sefirot: 'Daat', shared: true, children: [] },
          ],
        },
        {
          name: 'Architect',
          type: 'dog',
          sefirot: 'Chesed',
          children: [
            { name: 'Scout', type: 'dog', sefirot: 'Netzach', children: [] },
          ],
        },
        {
          name: 'Oracle',
          type: 'dog',
          sefirot: 'Tiferet',
          children: [
            { name: 'Janitor', type: 'dog', sefirot: 'Yesod', children: [] },
          ],
        },
        {
          name: 'Guardian',
          type: 'dog',
          sefirot: 'Gevurah',
          children: [
            { name: 'Deployer', type: 'dog', sefirot: 'Hod', children: [] },
          ],
        },
        {
          name: 'Cartographer',
          type: 'dog',
          sefirot: 'Malkhut',
          children: [],
        },
      ],
    };
  }

  /**
   * Render tree recursively
   */
  _renderTree(node, depth = 0) {
    const item = Utils.createElement('div', {
      className: `tree-item ${node.type} ${node.shared ? 'shared' : ''}`,
      dataset: { name: node.name, type: node.type },
      style: `--depth: ${depth}`,
    });

    const header = Utils.createElement('div', {
      className: 'tree-item-header',
      onClick: () => this._selectNode(node),
    }, [
      Utils.createElement('span', { className: 'tree-icon' }, [
        this._getNodeIcon(node),
      ]),
      Utils.createElement('span', { className: 'tree-name' }, [node.name]),
      node.sefirot ? Utils.createElement('span', { className: 'tree-sefirot' }, [node.sefirot]) : null,
    ].filter(Boolean));

    item.appendChild(header);

    if (node.children && node.children.length > 0) {
      const childrenContainer = Utils.createElement('div', { className: 'tree-children' });
      for (const child of node.children) {
        childrenContainer.appendChild(this._renderTree(child, depth + 1));
      }
      item.appendChild(childrenContainer);
    }

    return item;
  }

  /**
   * Get icon for node type
   */
  _getNodeIcon(node) {
    const icons = {
      root: '🧠',
      dog: '🐕',
      package: '📦',
      module: '📄',
      class: '🔷',
      function: '⚡',
    };

    // Special icons for specific dogs
    const dogIcons = {
      CYNIC: '🧠',
      Sage: '🦉',
      Analyst: '📊',
      Scholar: '📚',
      Architect: '🏗️',
      Guardian: '🛡️',
      Oracle: '🔮',
      Scout: '🔭',
      Deployer: '🚀',
      Janitor: '🧹',
      Cartographer: '🗺️',
    };

    return dogIcons[node.name] || icons[node.type] || '📁';
  }

  /**
   * Select a node
   */
  _selectNode(node) {
    this.selectedNode = node;

    // Update tree selection
    this.container?.querySelectorAll('.tree-item').forEach(el => {
      el.classList.toggle('selected', el.dataset.name === node.name);
    });

    // Update details panel
    this._showNodeDetails(node);

    // Update 3D scene
    if (this.threeScene) {
      this.threeScene.highlightNode(node.name);
    }

    // Update breadcrumb
    this.currentPath = this._getNodePath(node);
    const breadcrumb = document.getElementById('arch-breadcrumb');
    if (breadcrumb) this._renderBreadcrumb(breadcrumb);
  }

  /**
   * Get path to node
   */
  _getNodePath(node) {
    // Simplified: just return the node name
    return [node.name];
  }

  /**
   * Show node details
   */
  _showNodeDetails(node) {
    const panel = document.getElementById('arch-details');
    if (!panel) return;

    Utils.clearElement(panel);

    const dog = Formulas.DOGS.find(d => d.name === node.name);

    const content = Utils.createElement('div', { className: 'details-content' }, [
      Utils.createElement('div', { className: 'details-header' }, [
        Utils.createElement('span', { className: 'details-icon' }, [this._getNodeIcon(node)]),
        Utils.createElement('h3', {}, [node.name]),
      ]),
      node.sefirot ? Utils.createElement('div', { className: 'details-sefirot' }, [
        Utils.createElement('span', { className: 'label' }, ['Sefirot:']),
        Utils.createElement('span', { className: 'value' }, [node.sefirot]),
      ]) : null,
      dog?.role ? Utils.createElement('div', { className: 'details-role' }, [
        Utils.createElement('span', { className: 'label' }, ['Role:']),
        Utils.createElement('span', { className: 'value' }, [dog.role]),
      ]) : null,
      dog?.active !== undefined ? Utils.createElement('div', { className: 'details-status' }, [
        Utils.createElement('span', { className: 'label' }, ['Status:']),
        Utils.createElement('span', { className: `value ${dog.active ? 'active' : 'inactive'}` }, [
          dog.active ? 'Active' : 'Inactive',
        ]),
      ]) : null,
    ].filter(Boolean));

    panel.appendChild(content);
  }

  /**
   * Render breadcrumb
   */
  _renderBreadcrumb(container) {
    Utils.clearElement(container);

    const path = ['CYNIC', ...this.currentPath.filter(p => p !== 'CYNIC')];

    path.forEach((item, i) => {
      if (i > 0) {
        container.appendChild(
          Utils.createElement('span', { className: 'breadcrumb-separator' }, ['/'])
        );
      }
      container.appendChild(
        Utils.createElement('span', {
          className: `breadcrumb-item ${i === path.length - 1 ? 'active' : ''}`,
          onClick: () => this._navigateTo(path.slice(0, i + 1)),
        }, [item])
      );
    });
  }

  /**
   * Navigate to path
   */
  _navigateTo(path) {
    this.currentPath = path;
    const breadcrumb = document.getElementById('arch-breadcrumb');
    if (breadcrumb) this._renderBreadcrumb(breadcrumb);

    // Find and select node
    const nodeName = path[path.length - 1];
    const dog = Formulas.DOGS.find(d => d.name === nodeName);
    if (dog) {
      this._selectNode({ name: dog.name, type: 'dog', sefirot: dog.sefirot });
    }
  }

  /**
   * Render legend
   */
  _renderLegend(container) {
    Utils.clearElement(container);

    const items = [
      { color: '#4ecdc4', label: 'Crown (Keter)' },
      { color: '#667eea', label: 'Wisdom (Chochmah)' },
      { color: '#764ba2', label: 'Understanding (Binah)' },
      { color: '#f093fb', label: 'Knowledge (Daat)' },
      { color: '#00d9ff', label: 'Mercy (Chesed)' },
      { color: '#ff6b6b', label: 'Severity (Gevurah)' },
      { color: '#ffd93d', label: 'Beauty (Tiferet)' },
    ];

    for (const item of items) {
      container.appendChild(
        Utils.createElement('div', { className: 'legend-item' }, [
          Utils.createElement('span', {
            className: 'legend-color',
            style: `background-color: ${item.color}`,
          }),
          Utils.createElement('span', { className: 'legend-label' }, [item.label]),
        ])
      );
    }
  }

  /**
   * Set view mode
   */
  _setView(view) {
    // Update toggle buttons
    this.container?.querySelectorAll('.arch-toggle-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.view === view);
    });

    if (view === 'sefirot') {
      this._loadSefirotTree();
    } else {
      this._loadCodebaseTree();
    }
  }

  /**
   * Load codebase tree
   */
  async _loadCodebaseTree() {
    const treeContainer = document.getElementById('arch-tree');
    if (!treeContainer) return;

    Utils.clearElement(treeContainer);
    treeContainer.appendChild(
      Utils.createElement('div', { className: 'text-muted text-center' }, ['Loading codebase...'])
    );

    // Try to fetch codebase data from API
    if (this.api) {
      try {
        const result = await this.api.codebase('tree');
        if (result.success && result.result) {
          this.codebaseData = result.result;
          this._renderCodebaseTree(treeContainer, result.result);

          // Update 3D scene
          if (this.threeScene) {
            this.threeScene.loadCodebaseTree(result.result);
          }
          return;
        }
      } catch (e) {
        console.warn('Failed to load codebase:', e);
      }
    }

    // Fallback: show mock structure
    this._renderCodebaseTree(treeContainer, this._getMockCodebase());
  }

  /**
   * Render codebase tree
   */
  _renderCodebaseTree(container, data) {
    Utils.clearElement(container);

    const tree = this._renderCodebaseNode(data);
    container.appendChild(tree);
  }

  /**
   * Render codebase node
   */
  _renderCodebaseNode(node, depth = 0) {
    const item = Utils.createElement('div', {
      className: `tree-item ${node.type}`,
      dataset: { name: node.name, type: node.type },
      style: `--depth: ${depth}`,
    });

    const header = Utils.createElement('div', {
      className: 'tree-item-header',
      onClick: () => this._selectCodebaseNode(node),
    }, [
      Utils.createElement('span', { className: 'tree-icon' }, [
        this._getNodeIcon(node),
      ]),
      Utils.createElement('span', { className: 'tree-name' }, [node.name]),
      node.count ? Utils.createElement('span', { className: 'tree-count' }, [String(node.count)]) : null,
    ].filter(Boolean));

    item.appendChild(header);

    if (node.children && node.children.length > 0) {
      const childrenContainer = Utils.createElement('div', { className: 'tree-children collapsed' });
      for (const child of node.children) {
        childrenContainer.appendChild(this._renderCodebaseNode(child, depth + 1));
      }
      item.appendChild(childrenContainer);

      // Toggle children on click
      header.addEventListener('click', (e) => {
        e.stopPropagation();
        childrenContainer.classList.toggle('collapsed');
      });
    }

    return item;
  }

  /**
   * Select codebase node
   */
  _selectCodebaseNode(node) {
    this.selectedNode = node;

    const panel = document.getElementById('arch-details');
    if (panel) {
      Utils.clearElement(panel);
      panel.appendChild(
        Utils.createElement('div', { className: 'details-content' }, [
          Utils.createElement('h3', {}, [node.name]),
          Utils.createElement('div', {}, [`Type: ${node.type}`]),
          node.path ? Utils.createElement('div', { className: 'details-path' }, [node.path]) : null,
        ].filter(Boolean))
      );
    }

    if (this.threeScene) {
      this.threeScene.highlightNode(node.name);
    }
  }

  /**
   * Get mock codebase structure
   */
  _getMockCodebase() {
    return {
      name: 'packages',
      type: 'package',
      children: [
        {
          name: 'mcp',
          type: 'package',
          children: [
            { name: 'server.js', type: 'module' },
            { name: 'services/', type: 'package', count: 8 },
            { name: 'dashboard/', type: 'package', count: 15 },
          ],
        },
        {
          name: 'node',
          type: 'package',
          children: [
            { name: 'index.ts', type: 'module' },
            { name: 'services/', type: 'package', count: 12 },
          ],
        },
        {
          name: 'protocol',
          type: 'package',
          children: [
            { name: 'index.ts', type: 'module' },
            { name: 'types/', type: 'package', count: 6 },
          ],
        },
      ],
    };
  }

  /**
   * Search symbols
   */
  async _search(query) {
    if (!query) return;

    console.log('Searching for:', query);

    if (this.api) {
      const result = await this.api.codebase('search', { query });
      if (result.success && result.result) {
        // Show search results
        const treeContainer = document.getElementById('arch-tree');
        if (treeContainer) {
          Utils.clearElement(treeContainer);
          treeContainer.appendChild(
            Utils.createElement('div', { className: 'search-results' }, [
              Utils.createElement('div', { className: 'search-header' }, [`Results for "${query}"`]),
              // Render results...
            ])
          );
        }
      }
    }
  }

  /**
   * Handle 3D node click
   */
  _onNodeClick(node) {
    this._selectNode({ name: node.name, type: node.type, sefirot: node.sefirot });
  }

  /**
   * Handle 3D node hover
   */
  _onNodeHover(node) {
    // Could show tooltip
  }

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
    // Reload current view
    const activeView = this.container?.querySelector('.arch-toggle-btn.active');
    if (activeView?.dataset.view === 'codebase') {
      this._loadCodebaseTree();
    }
  }

  /**
   * Cleanup
   */
  destroy() {
    if (this.threeScene) {
      this.threeScene.dispose();
      this.threeScene = null;
    }
    this.container = null;
  }
}

// Export to window
window.CYNICArchView = ArchView;
