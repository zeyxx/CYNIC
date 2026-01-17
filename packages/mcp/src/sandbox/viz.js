/**
 * CYNIC Visualizations - Three.js 3D + D3.js graphs
 * Enhanced with hierarchical codebase navigation
 */

const CYNICViz = {
  // Three.js components
  scene: null,
  camera: null,
  renderer: null,
  controls: null,
  packages: {},
  dogs: {},
  connections: [],

  // D3 components
  graphSvg: null,
  graphSimulation: null,

  // Codebase visualization state
  codebase: {
    data: null,          // Full codebase tree from API
    currentLevel: 'packages', // 'packages', 'modules', 'classes', 'methods'
    currentPackage: null,
    currentModule: null,
    currentClass: null,
    breadcrumb: [],      // Navigation trail
    objects: [],         // Current 3D objects
    labels: [],          // Current labels
  },

  // Geometry presets for each level
  GEOMETRY_CONFIG: {
    package: { type: 'sphere', size: 0.6, segments: 16 },
    module: { type: 'box', size: 0.4 },
    class: { type: 'dodecahedron', size: 0.35 },
    method: { type: 'tetrahedron', size: 0.2 },
    function: { type: 'octahedron', size: 0.25 },
  },

  // Color palette for packages
  PACKAGE_COLORS: {
    core: 0x00d4aa,
    protocol: 0xd4aa00,
    persistence: 0x00aad4,
    node: 0xaa00d4,
    mcp: 0xd400aa,
    client: 0xaad400,
    default: 0x888888,
  },

  // Architecture data (static fallback)
  ARCHITECTURE: {
    packages: [
      { id: 'core', name: 'Core', position: { x: 0, y: 0, z: 0 }, color: 0x00d4aa, status: 'active' },
      { id: 'protocol', name: 'Protocol', position: { x: -2, y: 1, z: 0 }, color: 0xd4aa00, status: 'active' },
      { id: 'persistence', name: 'Persistence', position: { x: -2, y: -1, z: 0 }, color: 0x00aad4, status: 'active' },
      { id: 'node', name: 'Node', position: { x: 2, y: 1, z: 0 }, color: 0xaa00d4, status: 'active' },
      { id: 'mcp', name: 'MCP', position: { x: 0, y: 0, z: 2 }, color: 0xd400aa, status: 'active' },
      { id: 'client', name: 'Client', position: { x: 2, y: -1, z: 0 }, color: 0xaad400, status: 'planned' }
    ],
    connections: [
      { from: 'core', to: 'protocol' },
      { from: 'core', to: 'persistence' },
      { from: 'core', to: 'node' },
      { from: 'core', to: 'mcp' },
      { from: 'protocol', to: 'node' },
      { from: 'persistence', to: 'node' },
      { from: 'mcp', to: 'node' },
      { from: 'node', to: 'client' }
    ],
    dogs: [
      { id: 'observer', name: 'Observer', active: true, position: { x: -3, y: 2, z: 1 } },
      { id: 'digester', name: 'Digester', active: true, position: { x: -1, y: 2, z: 1 } },
      { id: 'guardian', name: 'Guardian', active: true, position: { x: 1, y: 2, z: 1 } },
      { id: 'mentor', name: 'Mentor', active: true, position: { x: 3, y: 2, z: 1 } },
      { id: 'tracker', name: 'Tracker', active: false, position: { x: -3, y: -2, z: 1 } },
      { id: 'auditor', name: 'Auditor', active: false, position: { x: -1, y: -2, z: 1 } },
      { id: 'librarian', name: 'Librarian', active: false, position: { x: 1, y: -2, z: 1 } },
      { id: 'economist', name: 'Economist', active: false, position: { x: 3, y: -2, z: 1 } },
      { id: 'diplomat', name: 'Diplomat', active: false, position: { x: 0, y: -3, z: 1 } },
      { id: 'archivist', name: 'Archivist', active: false, position: { x: 0, y: 3, z: 1 } }
    ]
  },

  /**
   * Initialize 3D visualization
   */
  init3D(containerId) {
    const container = document.getElementById(containerId);
    if (!container) return;

    const width = container.clientWidth;
    const height = container.clientHeight;

    // Scene
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x000000);

    // Camera
    this.camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 1000);
    this.camera.position.set(5, 3, 5);
    this.camera.lookAt(0, 0, 0);

    // Renderer
    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setSize(width, height);
    this.renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(this.renderer.domElement);

    // Lights
    const ambientLight = new THREE.AmbientLight(0x404040, 0.5);
    this.scene.add(ambientLight);

    const pointLight = new THREE.PointLight(0xffffff, 1);
    pointLight.position.set(10, 10, 10);
    this.scene.add(pointLight);

    // Grid helper
    const gridHelper = new THREE.GridHelper(10, 10, 0x222233, 0x111122);
    this.scene.add(gridHelper);

    // Create architecture
    this.createPackages();
    this.createDogs();
    this.createConnections();

    // Mouse controls
    this.setupControls(container);

    // Animation loop
    this.animate();

    // Handle resize
    window.addEventListener('resize', () => this.onResize(container));
  },

  /**
   * Create package nodes as icosahedrons
   */
  createPackages() {
    const geometry = new THREE.IcosahedronGeometry(0.4, 0);

    for (const pkg of this.ARCHITECTURE.packages) {
      const material = new THREE.MeshPhongMaterial({
        color: pkg.color,
        transparent: true,
        opacity: pkg.status === 'active' ? 0.9 : 0.4,
        wireframe: pkg.status !== 'active'
      });

      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(pkg.position.x, pkg.position.y, pkg.position.z);
      mesh.userData = { type: 'package', data: pkg };

      this.packages[pkg.id] = mesh;
      this.scene.add(mesh);

      // Label
      this.addLabel(pkg.name, mesh.position, pkg.color);
    }
  },

  /**
   * Create dog agents as tetrahedrons
   */
  createDogs() {
    const geometry = new THREE.TetrahedronGeometry(0.25, 0);

    for (const dog of this.ARCHITECTURE.dogs) {
      const color = dog.active ? 0x00d4aa : 0xff4444;
      const material = new THREE.MeshPhongMaterial({
        color,
        transparent: true,
        opacity: dog.active ? 0.9 : 0.3,
        wireframe: !dog.active
      });

      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(dog.position.x, dog.position.y, dog.position.z);
      mesh.userData = { type: 'dog', data: dog };

      this.dogs[dog.id] = mesh;
      this.scene.add(mesh);
    }
  },

  /**
   * Create connection lines between packages
   */
  createConnections() {
    const material = new THREE.LineBasicMaterial({
      color: 0x2a2a3a,
      transparent: true,
      opacity: 0.5
    });

    for (const conn of this.ARCHITECTURE.connections) {
      const from = this.packages[conn.from];
      const to = this.packages[conn.to];

      if (from && to) {
        const points = [from.position.clone(), to.position.clone()];
        const geometry = new THREE.BufferGeometry().setFromPoints(points);
        const line = new THREE.Line(geometry, material);

        this.connections.push(line);
        this.scene.add(line);
      }
    }
  },

  /**
   * Add text label (using sprite)
   */
  addLabel(text, position, color) {
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    canvas.width = 128;
    canvas.height = 32;

    context.fillStyle = '#000000';
    context.fillRect(0, 0, canvas.width, canvas.height);

    context.font = '14px JetBrains Mono, monospace';
    context.fillStyle = '#' + color.toString(16).padStart(6, '0');
    context.textAlign = 'center';
    context.fillText(text, canvas.width / 2, canvas.height / 2 + 5);

    const texture = new THREE.CanvasTexture(canvas);
    const material = new THREE.SpriteMaterial({ map: texture, transparent: true });
    const sprite = new THREE.Sprite(material);

    sprite.position.copy(position);
    sprite.position.y -= 0.6;
    sprite.scale.set(1, 0.25, 1);

    this.scene.add(sprite);
  },

  /**
   * Setup mouse controls for rotation/zoom
   */
  setupControls(container) {
    let isDragging = false;
    let previousMousePosition = { x: 0, y: 0 };

    container.addEventListener('mousedown', (e) => {
      isDragging = true;
      previousMousePosition = { x: e.clientX, y: e.clientY };
    });

    container.addEventListener('mousemove', (e) => {
      if (!isDragging) return;

      const deltaX = e.clientX - previousMousePosition.x;
      const deltaY = e.clientY - previousMousePosition.y;

      // Rotate camera around origin
      const theta = deltaX * 0.01;
      const phi = deltaY * 0.01;

      const pos = this.camera.position;
      const radius = Math.sqrt(pos.x * pos.x + pos.y * pos.y + pos.z * pos.z);

      const currentTheta = Math.atan2(pos.z, pos.x);
      const currentPhi = Math.acos(pos.y / radius);

      const newTheta = currentTheta + theta;
      const newPhi = Math.max(0.1, Math.min(Math.PI - 0.1, currentPhi + phi));

      this.camera.position.x = radius * Math.sin(newPhi) * Math.cos(newTheta);
      this.camera.position.y = radius * Math.cos(newPhi);
      this.camera.position.z = radius * Math.sin(newPhi) * Math.sin(newTheta);

      this.camera.lookAt(0, 0, 0);
      previousMousePosition = { x: e.clientX, y: e.clientY };
    });

    container.addEventListener('mouseup', () => isDragging = false);
    container.addEventListener('mouseleave', () => isDragging = false);

    // Zoom with wheel
    container.addEventListener('wheel', (e) => {
      e.preventDefault();
      const factor = e.deltaY > 0 ? 1.1 : 0.9;
      this.camera.position.multiplyScalar(factor);
    });

    // Click to select
    container.addEventListener('click', (e) => this.onObjectClick(e, container));
  },

  /**
   * Handle object click
   */
  onObjectClick(event, container) {
    const rect = container.getBoundingClientRect();
    const mouse = new THREE.Vector2(
      ((event.clientX - rect.left) / container.clientWidth) * 2 - 1,
      -((event.clientY - rect.top) / container.clientHeight) * 2 + 1
    );

    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(mouse, this.camera);

    const objects = [...Object.values(this.packages), ...Object.values(this.dogs)];
    const intersects = raycaster.intersectObjects(objects);

    if (intersects.length > 0) {
      const selected = intersects[0].object;
      const data = selected.userData;

      // Dispatch custom event
      container.dispatchEvent(new CustomEvent('objectSelected', {
        detail: data
      }));

      // Visual feedback
      this.highlightObject(selected);
    }
  },

  /**
   * Highlight selected object
   */
  highlightObject(object) {
    // Reset all
    Object.values(this.packages).forEach(p => p.scale.set(1, 1, 1));
    Object.values(this.dogs).forEach(d => d.scale.set(1, 1, 1));

    // Scale up selected
    object.scale.set(1.3, 1.3, 1.3);
  },

  /**
   * Animation loop
   */
  animate() {
    requestAnimationFrame(() => this.animate());

    // Rotate packages slowly
    Object.values(this.packages).forEach(pkg => {
      pkg.rotation.x += 0.002;
      pkg.rotation.y += 0.003;
    });

    // Rotate dogs
    Object.values(this.dogs).forEach(dog => {
      if (dog.userData.data.active) {
        dog.rotation.y += 0.01;
      }
    });

    this.renderer.render(this.scene, this.camera);
  },

  /**
   * Handle window resize
   */
  onResize(container) {
    const width = container.clientWidth;
    const height = container.clientHeight;

    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height);
  },

  /**
   * Initialize Knowledge Graph with D3
   */
  initGraph(containerId) {
    const container = document.getElementById(containerId);
    if (!container) return;

    const width = container.clientWidth;
    const height = container.clientHeight;

    // Sample graph data
    const nodes = [
      { id: 'user-1', type: 'user', label: 'User A' },
      { id: 'judgment-1', type: 'judgment', label: 'Judgment X' },
      { id: 'token-1', type: 'token', label: 'Token Y' },
      { id: 'pattern-1', type: 'pattern', label: 'Pattern Z' }
    ];

    const links = [
      { source: 'user-1', target: 'judgment-1', weight: 0.8 },
      { source: 'judgment-1', target: 'token-1', weight: 0.9 },
      { source: 'token-1', target: 'pattern-1', weight: 0.7 },
      { source: 'user-1', target: 'token-1', weight: 0.6 }
    ];

    // Create SVG
    this.graphSvg = d3.select(container)
      .append('svg')
      .attr('width', width)
      .attr('height', height);

    // Create simulation
    this.graphSimulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(links).id(d => d.id).distance(80))
      .force('charge', d3.forceManyBody().strength(-200))
      .force('center', d3.forceCenter(width / 2, height / 2));

    // Draw links
    const link = this.graphSvg.append('g')
      .selectAll('line')
      .data(links)
      .enter()
      .append('line')
      .attr('stroke', '#2a2a3a')
      .attr('stroke-width', d => d.weight * 3);

    // Draw link labels
    const linkLabel = this.graphSvg.append('g')
      .selectAll('text')
      .data(links)
      .enter()
      .append('text')
      .attr('font-size', '10px')
      .attr('fill', '#666')
      .text(d => d.weight.toFixed(2));

    // Draw nodes
    const node = this.graphSvg.append('g')
      .selectAll('circle')
      .data(nodes)
      .enter()
      .append('circle')
      .attr('r', 12)
      .attr('fill', d => this.getNodeColor(d.type))
      .call(d3.drag()
        .on('start', (event, d) => this.dragStarted(event, d))
        .on('drag', (event, d) => this.dragged(event, d))
        .on('end', (event, d) => this.dragEnded(event, d)));

    // Draw node labels
    const nodeLabel = this.graphSvg.append('g')
      .selectAll('text')
      .data(nodes)
      .enter()
      .append('text')
      .attr('font-size', '11px')
      .attr('fill', '#e0e0e0')
      .attr('text-anchor', 'middle')
      .attr('dy', 25)
      .text(d => d.label);

    // Update positions on tick
    this.graphSimulation.on('tick', () => {
      link
        .attr('x1', d => d.source.x)
        .attr('y1', d => d.source.y)
        .attr('x2', d => d.target.x)
        .attr('y2', d => d.target.y);

      linkLabel
        .attr('x', d => (d.source.x + d.target.x) / 2)
        .attr('y', d => (d.source.y + d.target.y) / 2);

      node
        .attr('cx', d => d.x)
        .attr('cy', d => d.y);

      nodeLabel
        .attr('x', d => d.x)
        .attr('y', d => d.y);
    });
  },

  getNodeColor(type) {
    const colors = {
      user: '#00d4aa',
      judgment: '#d4aa00',
      token: '#aa00d4',
      pattern: '#00aad4'
    };
    return colors[type] || '#888';
  },

  dragStarted(event, d) {
    if (!event.active) this.graphSimulation.alphaTarget(0.3).restart();
    d.fx = d.x;
    d.fy = d.y;
  },

  dragged(event, d) {
    d.fx = event.x;
    d.fy = event.y;
  },

  dragEnded(event, d) {
    if (!event.active) this.graphSimulation.alphaTarget(0);
    d.fx = null;
    d.fy = null;
  },

  /**
   * Update graph with new data
   */
  updateGraph(nodes, links) {
    // TODO: Implement dynamic graph updates
  },

  /**
   * Render PoJ Chain visualization (safe DOM methods)
   */
  renderChain(containerId, blocks) {
    const container = document.getElementById(containerId);
    if (!container) return;

    // Clear container safely
    while (container.firstChild) {
      container.removeChild(container.firstChild);
    }

    blocks.forEach((block, i) => {
      // Block element
      const blockEl = document.createElement('div');
      blockEl.className = 'chain-block' + (block.pending ? ' pending' : '');

      const numberSpan = document.createElement('span');
      numberSpan.className = 'number';
      numberSpan.textContent = 'B#' + String(block.number);

      const countSpan = document.createElement('span');
      countSpan.className = 'count';
      countSpan.textContent = String(block.judgments) + ' jdg';

      blockEl.appendChild(numberSpan);
      blockEl.appendChild(countSpan);
      blockEl.onclick = () => this.onBlockClick(block);
      container.appendChild(blockEl);

      // Connector (except last)
      if (i < blocks.length - 1) {
        const connector = document.createElement('span');
        connector.className = 'chain-connector';
        connector.textContent = '──';
        container.appendChild(connector);
      }
    });
  },

  onBlockClick(block) {
    console.log('Block clicked:', block);
    // Dispatch event for console to handle
    document.dispatchEvent(new CustomEvent('blockSelected', { detail: block }));
  },

  /**
   * Render matrix visualization (safe DOM methods)
   */
  renderMatrix(containerId, scores) {
    const container = document.getElementById(containerId);
    if (!container) return;

    // Clear container safely
    while (container.firstChild) {
      container.removeChild(container.firstChild);
    }

    const axioms = ['PHI', 'VERIFY', 'CULTURE', 'BURN'];

    for (const axiom of axioms) {
      const bar = document.createElement('div');
      bar.className = 'matrix-bar';

      const avgScore = this.getAxiomAverage(axiom, scores);

      const labelSpan = document.createElement('span');
      labelSpan.className = 'label';
      labelSpan.textContent = axiom;

      const barDiv = document.createElement('div');
      barDiv.className = 'bar';

      const fillDiv = document.createElement('div');
      fillDiv.className = 'fill';
      fillDiv.style.width = (avgScore * 100) + '%';
      fillDiv.style.background = this.getAxiomColor(axiom);

      barDiv.appendChild(fillDiv);
      bar.appendChild(labelSpan);
      bar.appendChild(barDiv);
      container.appendChild(bar);
    }
  },

  getAxiomAverage(axiom, scores) {
    const dims = CYNICFormulas.DIMENSIONS[axiom] || [];
    if (dims.length === 0) return 0;

    let sum = 0;
    for (const dim of dims) {
      sum += scores[dim.id] ?? 0.5;
    }
    return sum / dims.length;
  },

  getAxiomColor(axiom) {
    const colors = {
      PHI: '#d4aa00',
      VERIFY: '#00d4aa',
      CULTURE: '#aa00d4',
      BURN: '#d44400'
    };
    return colors[axiom] || '#888';
  },

  // ═══════════════════════════════════════════════════════════
  // CODEBASE VISUALIZATION - Hierarchical 3D Navigation
  // ═══════════════════════════════════════════════════════════

  /**
   * Load codebase data and render packages
   * @param {Object} data - Codebase tree from brain_codebase API
   */
  loadCodebase(data) {
    this.codebase.data = data;
    this.codebase.currentLevel = 'packages';
    this.codebase.currentPackage = null;
    this.codebase.currentModule = null;
    this.codebase.currentClass = null;
    this.codebase.breadcrumb = [{ level: 'packages', name: 'CYNIC', data: data }];

    // Clear existing architecture view
    this.clearCodebaseObjects();

    // Render packages
    this.renderPackages(data.packages);

    // Update breadcrumb UI
    this.updateBreadcrumb();

    // Dispatch event
    this.dispatchCodebaseEvent('levelChanged', { level: 'packages' });
  },

  /**
   * Clear codebase 3D objects
   */
  clearCodebaseObjects() {
    // Remove objects
    for (const obj of this.codebase.objects) {
      this.scene.remove(obj);
      if (obj.geometry) obj.geometry.dispose();
      if (obj.material) obj.material.dispose();
    }
    this.codebase.objects = [];

    // Remove labels
    for (const label of this.codebase.labels) {
      this.scene.remove(label);
      if (label.material?.map) label.material.map.dispose();
      if (label.material) label.material.dispose();
    }
    this.codebase.labels = [];

    // Clear connections
    for (const conn of this.connections) {
      this.scene.remove(conn);
      if (conn.geometry) conn.geometry.dispose();
      if (conn.material) conn.material.dispose();
    }
    this.connections = [];
  },

  /**
   * Render packages as spheres
   */
  renderPackages(packages) {
    const config = this.GEOMETRY_CONFIG.package;
    const radius = 3; // Circle radius for layout

    packages.forEach((pkg, i) => {
      const angle = (i / packages.length) * Math.PI * 2;
      const x = Math.cos(angle) * radius;
      const z = Math.sin(angle) * radius;

      const geometry = new THREE.SphereGeometry(config.size, config.segments, config.segments);
      const color = pkg.color || this.PACKAGE_COLORS[pkg.shortName] || this.PACKAGE_COLORS.default;

      const material = new THREE.MeshPhongMaterial({
        color,
        transparent: true,
        opacity: 0.85,
        emissive: color,
        emissiveIntensity: 0.1,
      });

      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(x, 0, z);
      mesh.userData = {
        type: 'package',
        level: 'packages',
        data: pkg,
        name: pkg.shortName || pkg.name,
      };

      this.codebase.objects.push(mesh);
      this.scene.add(mesh);

      // Add label
      const label = this.addCodeLabel(pkg.shortName || pkg.name, mesh.position, color);
      this.codebase.labels.push(label);
    });

    // Reset camera to see all packages
    this.animateCameraTo({ x: 5, y: 4, z: 5 }, { x: 0, y: 0, z: 0 });
  },

  /**
   * Render modules for a package
   */
  renderModules(pkg) {
    const modules = pkg.modules || [];
    const config = this.GEOMETRY_CONFIG.module;
    const color = pkg.color || this.PACKAGE_COLORS[pkg.shortName] || this.PACKAGE_COLORS.default;

    // Layout in grid or spiral
    const cols = Math.ceil(Math.sqrt(modules.length));
    const spacing = 1.2;

    modules.forEach((mod, i) => {
      const row = Math.floor(i / cols);
      const col = i % cols;
      const x = (col - cols / 2 + 0.5) * spacing;
      const z = (row - Math.ceil(modules.length / cols) / 2 + 0.5) * spacing;

      const geometry = new THREE.BoxGeometry(config.size, config.size, config.size);
      const material = new THREE.MeshPhongMaterial({
        color,
        transparent: true,
        opacity: 0.8,
      });

      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(x, 0, z);
      mesh.userData = {
        type: 'module',
        level: 'modules',
        data: mod,
        name: mod.name,
        parentPackage: pkg,
      };

      this.codebase.objects.push(mesh);
      this.scene.add(mesh);

      // Add label
      const label = this.addCodeLabel(mod.name, mesh.position, color, 0.8);
      this.codebase.labels.push(label);
    });

    // Zoom camera closer
    this.animateCameraTo({ x: 3, y: 3, z: 3 }, { x: 0, y: 0, z: 0 });
  },

  /**
   * Render classes for a module
   */
  renderClasses(mod, pkg) {
    const classes = mod.classes || [];
    const functions = mod.functions || [];
    const all = [...classes.map(c => ({ ...c, isClass: true })), ...functions.map(f => ({ ...f, isFunction: true }))];

    if (all.length === 0) {
      // Empty module - show message
      console.log('Module has no classes or functions');
      return;
    }

    const color = pkg.color || this.PACKAGE_COLORS.default;
    const spacing = 1.0;
    const cols = Math.ceil(Math.sqrt(all.length));

    all.forEach((item, i) => {
      const row = Math.floor(i / cols);
      const col = i % cols;
      const x = (col - cols / 2 + 0.5) * spacing;
      const z = (row - Math.ceil(all.length / cols) / 2 + 0.5) * spacing;

      let geometry, meshColor;

      if (item.isClass) {
        const config = this.GEOMETRY_CONFIG.class;
        geometry = new THREE.DodecahedronGeometry(config.size, 0);
        meshColor = color;
      } else {
        const config = this.GEOMETRY_CONFIG.function;
        geometry = new THREE.OctahedronGeometry(config.size, 0);
        meshColor = 0x888888; // Functions are gray
      }

      const material = new THREE.MeshPhongMaterial({
        color: meshColor,
        transparent: true,
        opacity: 0.85,
      });

      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(x, 0, z);
      mesh.userData = {
        type: item.isClass ? 'class' : 'function',
        level: 'classes',
        data: item,
        name: item.name,
        parentModule: mod,
        parentPackage: pkg,
      };

      this.codebase.objects.push(mesh);
      this.scene.add(mesh);

      // Add label
      const label = this.addCodeLabel(item.name, mesh.position, meshColor, 0.6);
      this.codebase.labels.push(label);
    });

    this.animateCameraTo({ x: 2.5, y: 2.5, z: 2.5 }, { x: 0, y: 0, z: 0 });
  },

  /**
   * Render methods for a class
   */
  renderMethods(cls, mod, pkg) {
    const methods = cls.methods || [];

    if (methods.length === 0) {
      console.log('Class has no methods');
      return;
    }

    const color = pkg.color || this.PACKAGE_COLORS.default;
    const config = this.GEOMETRY_CONFIG.method;
    const spacing = 0.6;
    const cols = Math.ceil(Math.sqrt(methods.length));

    methods.forEach((method, i) => {
      const row = Math.floor(i / cols);
      const col = i % cols;
      const x = (col - cols / 2 + 0.5) * spacing;
      const z = (row - Math.ceil(methods.length / cols) / 2 + 0.5) * spacing;

      const geometry = new THREE.TetrahedronGeometry(config.size, 0);

      // Color based on visibility
      const isPrivate = method.visibility === 'private';
      const meshColor = isPrivate ? 0x666666 : color;

      const material = new THREE.MeshPhongMaterial({
        color: meshColor,
        transparent: true,
        opacity: isPrivate ? 0.5 : 0.85,
        wireframe: isPrivate,
      });

      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(x, 0, z);
      mesh.userData = {
        type: 'method',
        level: 'methods',
        data: method,
        name: method.name,
        parentClass: cls,
        parentModule: mod,
        parentPackage: pkg,
      };

      this.codebase.objects.push(mesh);
      this.scene.add(mesh);

      // Add label for public methods
      if (!isPrivate) {
        const label = this.addCodeLabel(method.name, mesh.position, meshColor, 0.5);
        this.codebase.labels.push(label);
      }
    });

    this.animateCameraTo({ x: 2, y: 2, z: 2 }, { x: 0, y: 0, z: 0 });
  },

  /**
   * Add label for codebase items
   */
  addCodeLabel(text, position, color, scale = 1) {
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    canvas.width = 256;
    canvas.height = 64;

    // Background
    context.fillStyle = 'rgba(0, 0, 0, 0.7)';
    context.fillRect(0, 0, canvas.width, canvas.height);

    // Text
    context.font = '24px JetBrains Mono, monospace';
    context.fillStyle = '#' + (typeof color === 'number' ? color.toString(16).padStart(6, '0') : color.replace('#', ''));
    context.textAlign = 'center';
    context.textBaseline = 'middle';
    context.fillText(text.slice(0, 20), canvas.width / 2, canvas.height / 2);

    const texture = new THREE.CanvasTexture(canvas);
    const material = new THREE.SpriteMaterial({ map: texture, transparent: true });
    const sprite = new THREE.Sprite(material);

    sprite.position.copy(position);
    sprite.position.y -= 0.5;
    sprite.scale.set(scale * 1.5, scale * 0.4, 1);

    this.scene.add(sprite);
    return sprite;
  },

  /**
   * Navigate into an item (drill down)
   */
  navigateInto(item) {
    const { type, data, parentPackage, parentModule, parentClass } = item.userData;

    this.clearCodebaseObjects();

    switch (type) {
      case 'package':
        this.codebase.currentLevel = 'modules';
        this.codebase.currentPackage = data;
        this.codebase.breadcrumb.push({ level: 'modules', name: data.shortName || data.name, data });
        this.renderModules(data);
        break;

      case 'module':
        this.codebase.currentLevel = 'classes';
        this.codebase.currentModule = data;
        this.codebase.breadcrumb.push({ level: 'classes', name: data.name, data });
        this.renderClasses(data, parentPackage);
        break;

      case 'class':
        this.codebase.currentLevel = 'methods';
        this.codebase.currentClass = data;
        this.codebase.breadcrumb.push({ level: 'methods', name: data.name, data });
        this.renderMethods(data, parentModule, parentPackage);
        break;

      case 'method':
      case 'function':
        // Can't go deeper - dispatch selection event
        this.dispatchCodebaseEvent('symbolSelected', {
          type,
          name: data.name,
          line: data.line,
          params: data.params,
          module: parentModule?.name,
          package: parentPackage?.shortName,
        });
        return;
    }

    this.updateBreadcrumb();
    this.dispatchCodebaseEvent('levelChanged', { level: this.codebase.currentLevel, item: data });
  },

  /**
   * Navigate back one level
   */
  navigateBack() {
    if (this.codebase.breadcrumb.length <= 1) return;

    // Pop current level
    this.codebase.breadcrumb.pop();

    // Get parent level
    const parent = this.codebase.breadcrumb[this.codebase.breadcrumb.length - 1];

    this.clearCodebaseObjects();

    switch (parent.level) {
      case 'packages':
        this.codebase.currentLevel = 'packages';
        this.codebase.currentPackage = null;
        this.codebase.currentModule = null;
        this.codebase.currentClass = null;
        this.renderPackages(this.codebase.data.packages);
        break;

      case 'modules':
        this.codebase.currentLevel = 'modules';
        this.codebase.currentModule = null;
        this.codebase.currentClass = null;
        this.renderModules(parent.data);
        break;

      case 'classes':
        this.codebase.currentLevel = 'classes';
        this.codebase.currentClass = null;
        const pkg = this.codebase.breadcrumb.find(b => b.level === 'modules')?.data;
        this.renderClasses(parent.data, this.codebase.currentPackage || pkg);
        break;
    }

    this.updateBreadcrumb();
    this.dispatchCodebaseEvent('levelChanged', { level: this.codebase.currentLevel });
  },

  /**
   * Navigate to specific breadcrumb level
   */
  navigateTo(index) {
    while (this.codebase.breadcrumb.length > index + 1) {
      this.navigateBack();
    }
  },

  /**
   * Update breadcrumb UI
   */
  updateBreadcrumb() {
    const container = document.getElementById('breadcrumb');
    if (!container) return;

    // Clear
    while (container.firstChild) {
      container.removeChild(container.firstChild);
    }

    this.codebase.breadcrumb.forEach((item, i) => {
      if (i > 0) {
        const sep = document.createElement('span');
        sep.className = 'breadcrumb-separator';
        sep.textContent = ' → ';
        container.appendChild(sep);
      }

      const link = document.createElement('span');
      link.className = 'breadcrumb-item';
      if (i === this.codebase.breadcrumb.length - 1) {
        link.classList.add('active');
      }
      link.textContent = item.name;
      link.onclick = () => this.navigateTo(i);
      container.appendChild(link);
    });
  },

  /**
   * Animate camera to position
   */
  animateCameraTo(targetPos, lookAt) {
    const startPos = this.camera.position.clone();
    const duration = 500;
    const startTime = Date.now();

    const animate = () => {
      const elapsed = Date.now() - startTime;
      const t = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - t, 3); // Ease out cubic

      this.camera.position.x = startPos.x + (targetPos.x - startPos.x) * eased;
      this.camera.position.y = startPos.y + (targetPos.y - startPos.y) * eased;
      this.camera.position.z = startPos.z + (targetPos.z - startPos.z) * eased;

      this.camera.lookAt(lookAt.x, lookAt.y, lookAt.z);

      if (t < 1) {
        requestAnimationFrame(animate);
      }
    };

    animate();
  },

  /**
   * Handle codebase object click
   */
  onCodebaseClick(event, container) {
    const rect = container.getBoundingClientRect();
    const mouse = new THREE.Vector2(
      ((event.clientX - rect.left) / container.clientWidth) * 2 - 1,
      -((event.clientY - rect.top) / container.clientHeight) * 2 + 1
    );

    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(mouse, this.camera);

    const intersects = raycaster.intersectObjects(this.codebase.objects);

    if (intersects.length > 0) {
      const selected = intersects[0].object;
      this.highlightCodebaseObject(selected);

      // Double-click to navigate into
      if (event.detail === 2) {
        this.navigateInto(selected);
      } else {
        // Single click - show details
        this.dispatchCodebaseEvent('itemSelected', selected.userData);
      }
    }
  },

  /**
   * Highlight selected codebase object
   */
  highlightCodebaseObject(object) {
    // Reset all
    this.codebase.objects.forEach(obj => {
      obj.scale.set(1, 1, 1);
      if (obj.material) {
        obj.material.emissiveIntensity = 0.1;
      }
    });

    // Highlight selected
    object.scale.set(1.3, 1.3, 1.3);
    if (object.material) {
      object.material.emissiveIntensity = 0.4;
    }
  },

  /**
   * Dispatch codebase event
   */
  dispatchCodebaseEvent(eventName, detail) {
    document.dispatchEvent(new CustomEvent('codebase:' + eventName, { detail }));
  },

  /**
   * Search and highlight symbols
   */
  highlightSearch(query) {
    if (!query || !this.codebase.objects.length) return;

    const lowerQuery = query.toLowerCase();

    this.codebase.objects.forEach(obj => {
      const name = obj.userData.name?.toLowerCase() || '';
      const matches = name.includes(lowerQuery);

      if (matches) {
        obj.material.opacity = 1;
        obj.material.emissive = obj.material.color;
        obj.material.emissiveIntensity = 0.5;
      } else {
        obj.material.opacity = 0.2;
        obj.material.emissiveIntensity = 0;
      }
    });
  },

  /**
   * Clear search highlighting
   */
  clearSearchHighlight() {
    this.codebase.objects.forEach(obj => {
      obj.material.opacity = 0.85;
      obj.material.emissiveIntensity = 0.1;
    });
  },

  /**
   * Focus on a specific object (for external selection)
   */
  focusObject(id) {
    // Find object by id in current view
    const obj = this.codebase.objects.find(o =>
      o.userData.data?.id === id ||
      o.userData.data?.shortName === id ||
      o.userData.name === id
    );

    if (obj) {
      this.highlightCodebaseObject(obj);
      // Animate camera towards it
      const targetPos = obj.position.clone();
      targetPos.y += 2;
      targetPos.z += 2;
      this.animateCameraTo(targetPos, obj.position);
    }
  }
};

// Export
window.CYNICViz = CYNICViz;
