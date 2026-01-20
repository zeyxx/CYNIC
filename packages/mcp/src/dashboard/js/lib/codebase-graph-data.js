/**
 * CYNIC Dashboard - Codebase Graph Data Model
 * Hierarchical data structures for 3D codebase visualization
 */

// PHI for proportions
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;

/**
 * Node types for codebase visualization
 */
export const CodebaseNodeType = {
  PACKAGE: 'package',
  MODULE: 'module',
  CLASS: 'class',
  FUNCTION: 'function',
  METHOD: 'method',
};

/**
 * Edge types for relationships
 */
export const CodebaseEdgeType = {
  CONTAINS: 'contains',      // Package contains modules, module contains classes
  IMPORTS: 'imports',        // Module imports another
  EXTENDS: 'extends',        // Class extends another
  IMPLEMENTS: 'implements',  // Class implements interface
  CALLS: 'calls',           // Function calls another
};

/**
 * Node colors by type
 */
export const CodebaseColors = {
  [CodebaseNodeType.PACKAGE]: 0x4ecdc4,   // Teal
  [CodebaseNodeType.MODULE]: 0x667eea,    // Purple-blue
  [CodebaseNodeType.CLASS]: 0xf093fb,     // Pink
  [CodebaseNodeType.FUNCTION]: 0xffd93d,  // Gold
  [CodebaseNodeType.METHOD]: 0x00ff88,    // Green
};

/**
 * Node shapes by type (for 3D rendering)
 */
export const CodebaseShapes = {
  [CodebaseNodeType.PACKAGE]: 'sphere',
  [CodebaseNodeType.MODULE]: 'box',
  [CodebaseNodeType.CLASS]: 'dodecahedron',
  [CodebaseNodeType.FUNCTION]: 'octahedron',
  [CodebaseNodeType.METHOD]: 'tetrahedron',
};

/**
 * Base node for codebase graph
 */
export class CodebaseNode {
  constructor(id, type, name, data = {}) {
    this.id = id;
    this.type = type;
    this.name = name;
    this.data = data;

    // Position in 3D space
    this.position = { x: 0, y: 0, z: 0 };
    this.velocity = { x: 0, y: 0, z: 0 };

    // Hierarchy
    this.parent = null;
    this.children = [];

    // Visual properties
    this.color = CodebaseColors[type] || 0xaaaaaa;
    this.shape = CodebaseShapes[type] || 'sphere';
    this.size = this._computeSize();
    this.visible = true;
    this.expanded = false;
    this.highlighted = false;
  }

  /**
   * Compute node size based on type and metrics
   */
  _computeSize() {
    const baseSize = {
      [CodebaseNodeType.PACKAGE]: 1.0,
      [CodebaseNodeType.MODULE]: 0.6,
      [CodebaseNodeType.CLASS]: 0.45,
      [CodebaseNodeType.FUNCTION]: 0.35,
      [CodebaseNodeType.METHOD]: 0.25,
    };

    let size = baseSize[this.type] || 0.3;

    // Scale by content size
    if (this.data.lines) {
      size *= Math.log10(this.data.lines + 10) / 3;
    }
    if (this.data.methodCount) {
      size *= 1 + Math.log10(this.data.methodCount + 1) / 5;
    }

    return Math.max(0.2, Math.min(size, 1.5));
  }

  /**
   * Get depth in hierarchy
   */
  getDepth() {
    let depth = 0;
    let node = this;
    while (node.parent) {
      depth++;
      node = node.parent;
    }
    return depth;
  }

  /**
   * Get all descendants
   */
  getDescendants() {
    const descendants = [];
    const stack = [...this.children];
    while (stack.length > 0) {
      const node = stack.pop();
      descendants.push(node);
      stack.push(...node.children);
    }
    return descendants;
  }

  /**
   * Get path from root
   */
  getPath() {
    const path = [this];
    let node = this;
    while (node.parent) {
      path.unshift(node.parent);
      node = node.parent;
    }
    return path;
  }

  /**
   * Convert to simple object for serialization
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      data: this.data,
      position: this.position,
      size: this.size,
      color: this.color,
      childCount: this.children.length,
    };
  }
}

/**
 * Edge between codebase nodes
 */
export class CodebaseEdge {
  constructor(source, target, type, data = {}) {
    this.source = source;
    this.target = target;
    this.type = type;
    this.data = data;
    this.visible = true;
    this.highlighted = false;
  }

  /**
   * Get edge color based on type
   */
  getColor() {
    const colors = {
      [CodebaseEdgeType.CONTAINS]: 0x333355,
      [CodebaseEdgeType.IMPORTS]: 0x667eea,
      [CodebaseEdgeType.EXTENDS]: 0xf093fb,
      [CodebaseEdgeType.IMPLEMENTS]: 0x00d9ff,
      [CodebaseEdgeType.CALLS]: 0xffd93d,
    };
    return colors[this.type] || 0x444444;
  }

  toJSON() {
    return {
      source: this.source.id,
      target: this.target.id,
      type: this.type,
    };
  }
}

/**
 * Main codebase graph data manager
 */
export class CodebaseGraphData {
  constructor() {
    this.nodes = new Map();       // id -> CodebaseNode
    this.edges = [];              // CodebaseEdge[]
    this.root = null;             // Root package node
    this.currentFocus = null;     // Currently focused node for zoom
    this.adjacency = new Map();   // node.id -> Set of connected node ids

    // Stats
    this.stats = {
      packages: 0,
      modules: 0,
      classes: 0,
      functions: 0,
      methods: 0,
      lines: 0,
    };

    // Event callbacks
    this.onNodeAdded = null;
    this.onNodeRemoved = null;
    this.onFocusChanged = null;
  }

  /**
   * Load data from API response
   */
  loadFromAPI(apiData) {
    this.clear();

    if (!apiData || !apiData.packages) {
      console.warn('No packages in API data');
      return;
    }

    // Create root node
    this.root = new CodebaseNode('root', CodebaseNodeType.PACKAGE, 'packages', {
      path: apiData.root || '.',
    });
    this.addNode(this.root);
    this.currentFocus = this.root;

    // Process packages
    for (const pkg of apiData.packages) {
      this._loadPackage(pkg, this.root);
    }

    // Build hierarchy edges (contains)
    this._buildContainsEdges();

    // Update stats
    this._updateStats();

    console.log(`Loaded codebase: ${this.stats.packages} packages, ${this.stats.modules} modules, ${this.stats.classes} classes`);
  }

  /**
   * Load a package from API data
   */
  _loadPackage(pkg, parent) {
    const pkgNode = new CodebaseNode(
      `pkg:${pkg.shortName || pkg.name}`,
      CodebaseNodeType.PACKAGE,
      pkg.shortName || pkg.name,
      {
        fullName: pkg.name,
        path: pkg.path,
        stats: pkg.stats || {},
        color: pkg.color,
      }
    );

    if (pkg.color) {
      pkgNode.color = pkg.color;
    }

    this.addNode(pkgNode);
    parent.children.push(pkgNode);
    pkgNode.parent = parent;

    // Process modules
    if (pkg.modules) {
      for (const mod of pkg.modules) {
        this._loadModule(mod, pkgNode);
      }
    }

    return pkgNode;
  }

  /**
   * Load a module from API data
   */
  _loadModule(mod, parent) {
    const modNode = new CodebaseNode(
      `mod:${parent.name}/${mod.name}`,
      CodebaseNodeType.MODULE,
      mod.name,
      {
        path: mod.path,
        lines: mod.lines || 0,
      }
    );

    this.addNode(modNode);
    parent.children.push(modNode);
    modNode.parent = parent;

    // Process classes
    if (mod.classes) {
      for (const cls of mod.classes) {
        this._loadClass(cls, modNode);
      }
    }

    // Process standalone functions
    if (mod.functions) {
      for (const fn of mod.functions) {
        this._loadFunction(fn, modNode);
      }
    }

    return modNode;
  }

  /**
   * Load a class from API data
   */
  _loadClass(cls, parent) {
    const clsNode = new CodebaseNode(
      `cls:${parent.parent.name}/${parent.name}/${cls.name}`,
      CodebaseNodeType.CLASS,
      cls.name,
      {
        line: cls.line,
        methodCount: cls.methodCount || 0,
      }
    );

    this.addNode(clsNode);
    parent.children.push(clsNode);
    clsNode.parent = parent;

    // Process methods (limit to avoid overload)
    if (cls.methods) {
      const methodsToShow = cls.methods.slice(0, 20); // Limit methods shown
      for (const method of methodsToShow) {
        // Skip parser artifacts
        if (['if', 'for', 'while', 'switch'].includes(method.name)) continue;

        this._loadMethod(method, clsNode);
      }
    }

    return clsNode;
  }

  /**
   * Load a function from API data
   */
  _loadFunction(fn, parent) {
    const fnNode = new CodebaseNode(
      `fn:${parent.parent.name}/${parent.name}/${fn.name}`,
      CodebaseNodeType.FUNCTION,
      fn.name,
      {
        line: fn.line,
        params: fn.params || [],
        exported: fn.exported || false,
      }
    );

    this.addNode(fnNode);
    parent.children.push(fnNode);
    fnNode.parent = parent;

    return fnNode;
  }

  /**
   * Load a method from API data
   */
  _loadMethod(method, parent) {
    const methodNode = new CodebaseNode(
      `method:${parent.id}/${method.name}`,
      CodebaseNodeType.METHOD,
      method.name,
      {
        line: method.line,
        params: method.params || [],
        visibility: method.visibility || 'public',
      }
    );

    this.addNode(methodNode);
    parent.children.push(methodNode);
    methodNode.parent = parent;

    return methodNode;
  }

  /**
   * Build containment edges (parent → child)
   */
  _buildContainsEdges() {
    for (const node of this.nodes.values()) {
      for (const child of node.children) {
        const edge = new CodebaseEdge(node, child, CodebaseEdgeType.CONTAINS);
        this.edges.push(edge);
        this._addToAdjacency(node.id, child.id);
      }
    }
  }

  /**
   * Add node to graph
   */
  addNode(node) {
    this.nodes.set(node.id, node);
    this.adjacency.set(node.id, new Set());
    this.onNodeAdded?.(node);
  }

  /**
   * Add to adjacency list
   */
  _addToAdjacency(nodeId1, nodeId2) {
    this.adjacency.get(nodeId1)?.add(nodeId2);
    this.adjacency.get(nodeId2)?.add(nodeId1);
  }

  /**
   * Clear all data
   */
  clear() {
    this.nodes.clear();
    this.edges = [];
    this.adjacency.clear();
    this.root = null;
    this.currentFocus = null;
    this.stats = { packages: 0, modules: 0, classes: 0, functions: 0, methods: 0, lines: 0 };
  }

  /**
   * Get node by ID
   */
  getNode(id) {
    return this.nodes.get(id);
  }

  /**
   * Get nodes at depth level
   */
  getNodesAtDepth(depth) {
    return Array.from(this.nodes.values()).filter(n => n.getDepth() === depth);
  }

  /**
   * Get visible nodes (based on current focus)
   */
  getVisibleNodes() {
    if (!this.currentFocus) {
      return Array.from(this.nodes.values()).filter(n => n.getDepth() <= 2);
    }

    // Show focused node and its immediate children
    const visible = [this.currentFocus, ...this.currentFocus.children];

    // Also show siblings if we're not at root
    if (this.currentFocus.parent) {
      visible.push(...this.currentFocus.parent.children.filter(n => n !== this.currentFocus));
    }

    return visible;
  }

  /**
   * Get visible edges
   */
  getVisibleEdges() {
    const visibleNodeIds = new Set(this.getVisibleNodes().map(n => n.id));
    return this.edges.filter(
      e => visibleNodeIds.has(e.source.id) && visibleNodeIds.has(e.target.id)
    );
  }

  /**
   * Set focus to node (for semantic zoom)
   */
  setFocus(nodeOrId) {
    const node = typeof nodeOrId === 'string' ? this.getNode(nodeOrId) : nodeOrId;
    if (!node) return;

    const previousFocus = this.currentFocus;
    this.currentFocus = node;

    this.onFocusChanged?.(node, previousFocus);
  }

  /**
   * Navigate up (zoom out)
   */
  focusParent() {
    if (this.currentFocus?.parent) {
      this.setFocus(this.currentFocus.parent);
    }
  }

  /**
   * Navigate into child
   */
  focusChild(child) {
    if (this.currentFocus?.children.includes(child)) {
      this.setFocus(child);
    } else {
      this.setFocus(child);
    }
  }

  /**
   * Search nodes by name
   */
  search(query, limit = 20) {
    if (!query) return [];

    const lowerQuery = query.toLowerCase();
    const results = [];

    for (const node of this.nodes.values()) {
      if (node.name.toLowerCase().includes(lowerQuery)) {
        results.push(node);
        if (results.length >= limit) break;
      }
    }

    // Sort by relevance (exact match first, then by type)
    results.sort((a, b) => {
      const aExact = a.name.toLowerCase() === lowerQuery;
      const bExact = b.name.toLowerCase() === lowerQuery;
      if (aExact && !bExact) return -1;
      if (!aExact && bExact) return 1;

      // Then by type priority
      const typePriority = {
        [CodebaseNodeType.CLASS]: 0,
        [CodebaseNodeType.FUNCTION]: 1,
        [CodebaseNodeType.MODULE]: 2,
        [CodebaseNodeType.PACKAGE]: 3,
        [CodebaseNodeType.METHOD]: 4,
      };
      return (typePriority[a.type] || 5) - (typePriority[b.type] || 5);
    });

    return results;
  }

  /**
   * Get connected nodes
   */
  getConnectedNodes(nodeId) {
    const connected = this.adjacency.get(nodeId);
    if (!connected) return [];
    return Array.from(connected).map(id => this.getNode(id)).filter(Boolean);
  }

  /**
   * Highlight node and its connections
   */
  highlightNode(nodeId) {
    // Reset all
    for (const node of this.nodes.values()) {
      node.highlighted = false;
    }
    for (const edge of this.edges) {
      edge.highlighted = false;
    }

    // Highlight node and connections
    const node = this.getNode(nodeId);
    if (node) {
      node.highlighted = true;

      // Highlight connected nodes
      const connected = this.getConnectedNodes(nodeId);
      for (const conn of connected) {
        conn.highlighted = true;
      }

      // Highlight edges
      for (const edge of this.edges) {
        if (edge.source.id === nodeId || edge.target.id === nodeId) {
          edge.highlighted = true;
        }
      }
    }
  }

  /**
   * Clear highlights
   */
  clearHighlights() {
    for (const node of this.nodes.values()) {
      node.highlighted = false;
    }
    for (const edge of this.edges) {
      edge.highlighted = false;
    }
  }

  /**
   * Update stats
   */
  _updateStats() {
    this.stats = { packages: 0, modules: 0, classes: 0, functions: 0, methods: 0, lines: 0 };

    for (const node of this.nodes.values()) {
      switch (node.type) {
        case CodebaseNodeType.PACKAGE:
          if (node.id !== 'root') this.stats.packages++;
          break;
        case CodebaseNodeType.MODULE:
          this.stats.modules++;
          this.stats.lines += node.data.lines || 0;
          break;
        case CodebaseNodeType.CLASS:
          this.stats.classes++;
          break;
        case CodebaseNodeType.FUNCTION:
          this.stats.functions++;
          break;
        case CodebaseNodeType.METHOD:
          this.stats.methods++;
          break;
      }
    }
  }

  /**
   * Get stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Layout nodes in radial pattern around focus
   */
  layoutRadial() {
    const visible = this.getVisibleNodes();
    if (visible.length === 0) return;

    // Position focus at center
    if (this.currentFocus) {
      this.currentFocus.position = { x: 0, y: 0, z: 0 };
    }

    // Layout children in circle around focus
    const children = this.currentFocus?.children || visible;
    const radius = 3 * PHI;
    const angleStep = (2 * Math.PI) / children.length;

    children.forEach((child, i) => {
      const angle = i * angleStep - Math.PI / 2;
      child.position = {
        x: Math.cos(angle) * radius,
        y: 0,
        z: Math.sin(angle) * radius,
      };

      // Layout grandchildren in smaller circles
      if (child.children.length > 0) {
        const grandRadius = 1.5;
        const grandAngleStep = (2 * Math.PI) / child.children.length;

        child.children.forEach((grandchild, j) => {
          const grandAngle = j * grandAngleStep;
          grandchild.position = {
            x: child.position.x + Math.cos(grandAngle) * grandRadius,
            y: -1.5, // Lower level
            z: child.position.z + Math.sin(grandAngle) * grandRadius,
          };
        });
      }
    });
  }
}

// Export to window
if (typeof window !== 'undefined') {
  window.CodebaseGraphData = CodebaseGraphData;
  window.CodebaseNode = CodebaseNode;
  window.CodebaseNodeType = CodebaseNodeType;
  window.CodebaseColors = CodebaseColors;
}
