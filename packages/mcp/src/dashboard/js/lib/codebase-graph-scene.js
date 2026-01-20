/**
 * CYNIC Dashboard - Codebase Graph 3D Scene
 * Three.js scene for hierarchical codebase visualization with semantic zoom
 */

import {
  CodebaseNodeType,
  CodebaseColors,
  CodebaseShapes,
} from './codebase-graph-data.js';

// PHI for proportions
const PHI = 1.618033988749895;

/**
 * 3D Scene manager for codebase visualization
 */
export class CodebaseGraphScene {
  constructor(containerId) {
    this.containerId = containerId;
    this.container = null;
    this.scene = null;
    this.camera = null;
    this.renderer = null;
    this.data = null;

    // Three.js objects
    this.meshes = new Map();      // nodeId -> mesh
    this.labels = new Map();      // nodeId -> label sprite
    this.lines = [];              // edge lines
    this.geometries = {};         // cached geometries by shape

    // State
    this.isInitialized = false;
    this.selectedNodeId = null;
    this.hoveredNodeId = null;
    this.animationId = null;

    // Camera animation state
    this.cameraTarget = { x: 0, y: 0, z: 0 };
    this.cameraLookAt = { x: 0, y: 0, z: 0 };
    this.isAnimating = false;

    // Event callbacks
    this.onNodeClick = null;
    this.onNodeDoubleClick = null;
    this.onNodeHover = null;
  }

  /**
   * Initialize the 3D scene
   */
  init() {
    this.container = document.getElementById(this.containerId);
    if (!this.container) {
      console.error(`Container #${this.containerId} not found`);
      return false;
    }

    const width = this.container.clientWidth;
    const height = this.container.clientHeight;

    // Scene
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x0a0a0f);
    this.scene.fog = new THREE.Fog(0x0a0a0f, 15, 50);

    // Camera
    this.camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 1000);
    this.camera.position.set(8, 5, 8);
    this.camera.lookAt(0, 0, 0);

    // Renderer
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setSize(width, height);
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.container.appendChild(this.renderer.domElement);

    // Lights
    this._setupLights();

    // Create cached geometries
    this._createGeometries();

    // Grid helper
    const gridHelper = new THREE.GridHelper(20, 20, 0x222233, 0x111122);
    gridHelper.position.y = -3;
    this.scene.add(gridHelper);

    // Setup controls
    this._setupControls();

    // Handle resize
    window.addEventListener('resize', () => this._onResize());

    // Start animation loop
    this._animate();

    this.isInitialized = true;
    return true;
  }

  /**
   * Setup lighting
   */
  _setupLights() {
    // Ambient light
    const ambient = new THREE.AmbientLight(0x404050, 0.6);
    this.scene.add(ambient);

    // Main directional light
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(10, 15, 10);
    this.scene.add(dirLight);

    // Fill light
    const fillLight = new THREE.DirectionalLight(0x4ecdc4, 0.3);
    fillLight.position.set(-10, 5, -10);
    this.scene.add(fillLight);

    // Point light at center for glow effect
    const centerLight = new THREE.PointLight(0x667eea, 0.5, 20);
    centerLight.position.set(0, 2, 0);
    this.scene.add(centerLight);
  }

  /**
   * Create cached geometries for each node type
   */
  _createGeometries() {
    this.geometries = {
      sphere: new THREE.SphereGeometry(1, 24, 24),
      box: new THREE.BoxGeometry(1, 1, 1),
      dodecahedron: new THREE.DodecahedronGeometry(1, 0),
      octahedron: new THREE.OctahedronGeometry(1, 0),
      tetrahedron: new THREE.TetrahedronGeometry(1, 0),
    };
  }

  /**
   * Setup mouse/touch controls
   */
  _setupControls() {
    let isDragging = false;
    let previousMouse = { x: 0, y: 0 };
    let lastClickTime = 0;

    // Mouse down
    this.container.addEventListener('mousedown', (e) => {
      isDragging = true;
      previousMouse = { x: e.clientX, y: e.clientY };
    });

    // Mouse move
    this.container.addEventListener('mousemove', (e) => {
      this._handleHover(e);

      if (!isDragging) return;

      const deltaX = e.clientX - previousMouse.x;
      const deltaY = e.clientY - previousMouse.y;

      // Orbit camera around look-at point
      const theta = deltaX * 0.01;
      const phi = deltaY * 0.01;

      const pos = this.camera.position.clone().sub(
        new THREE.Vector3(this.cameraLookAt.x, this.cameraLookAt.y, this.cameraLookAt.z)
      );
      const radius = pos.length();

      const currentTheta = Math.atan2(pos.z, pos.x);
      const currentPhi = Math.acos(Math.max(-1, Math.min(1, pos.y / radius)));

      const newTheta = currentTheta + theta;
      const newPhi = Math.max(0.1, Math.min(Math.PI - 0.1, currentPhi + phi));

      this.camera.position.set(
        this.cameraLookAt.x + radius * Math.sin(newPhi) * Math.cos(newTheta),
        this.cameraLookAt.y + radius * Math.cos(newPhi),
        this.cameraLookAt.z + radius * Math.sin(newPhi) * Math.sin(newTheta)
      );

      this.camera.lookAt(this.cameraLookAt.x, this.cameraLookAt.y, this.cameraLookAt.z);
      previousMouse = { x: e.clientX, y: e.clientY };
    });

    // Mouse up
    this.container.addEventListener('mouseup', () => {
      isDragging = false;
    });
    this.container.addEventListener('mouseleave', () => {
      isDragging = false;
      this.hoveredNodeId = null;
      this.container.style.cursor = 'grab';
    });

    // Wheel zoom
    this.container.addEventListener('wheel', (e) => {
      e.preventDefault();
      const factor = e.deltaY > 0 ? 1.1 : 0.9;

      const dir = this.camera.position.clone()
        .sub(new THREE.Vector3(this.cameraLookAt.x, this.cameraLookAt.y, this.cameraLookAt.z))
        .normalize();

      const distance = this.camera.position.distanceTo(
        new THREE.Vector3(this.cameraLookAt.x, this.cameraLookAt.y, this.cameraLookAt.z)
      );
      const newDistance = Math.max(3, Math.min(30, distance * factor));

      this.camera.position.copy(
        new THREE.Vector3(this.cameraLookAt.x, this.cameraLookAt.y, this.cameraLookAt.z)
          .add(dir.multiplyScalar(newDistance))
      );
    });

    // Click
    this.container.addEventListener('click', (e) => {
      const now = Date.now();
      const isDoubleClick = now - lastClickTime < 300;
      lastClickTime = now;

      const intersected = this._raycast(e);
      if (intersected) {
        if (isDoubleClick) {
          this.onNodeDoubleClick?.(intersected.userData.nodeId);
        } else {
          this._selectNode(intersected.userData.nodeId);
          this.onNodeClick?.(intersected.userData.nodeId);
        }
      }
    });
  }

  /**
   * Handle hover
   */
  _handleHover(event) {
    const intersected = this._raycast(event);

    if (intersected) {
      this.container.style.cursor = 'pointer';
      const nodeId = intersected.userData.nodeId;

      if (nodeId !== this.hoveredNodeId) {
        this.hoveredNodeId = nodeId;
        this.onNodeHover?.(nodeId);
        this._highlightNode(nodeId);
      }
    } else {
      this.container.style.cursor = 'grab';
      if (this.hoveredNodeId) {
        this.hoveredNodeId = null;
        this._clearHighlight();
        this.onNodeHover?.(null);
      }
    }
  }

  /**
   * Raycast from mouse position
   */
  _raycast(event) {
    const rect = this.container.getBoundingClientRect();
    const mouse = new THREE.Vector2(
      ((event.clientX - rect.left) / this.container.clientWidth) * 2 - 1,
      -((event.clientY - rect.top) / this.container.clientHeight) * 2 + 1
    );

    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(mouse, this.camera);

    const meshArray = Array.from(this.meshes.values());
    const intersects = raycaster.intersectObjects(meshArray);

    return intersects.length > 0 ? intersects[0].object : null;
  }

  /**
   * Set data source
   */
  setData(codebaseGraphData) {
    this.data = codebaseGraphData;

    // Subscribe to data changes
    this.data.onFocusChanged = (node, previous) => {
      this._onFocusChanged(node, previous);
    };
  }

  /**
   * Render the graph
   */
  render() {
    if (!this.data) return;

    this._clear();

    // Layout nodes
    this.data.layoutRadial();

    // Get visible nodes and edges
    const visibleNodes = this.data.getVisibleNodes();
    const visibleEdges = this.data.getVisibleEdges();

    // Create meshes for nodes
    for (const node of visibleNodes) {
      this._createNodeMesh(node);
    }

    // Create lines for edges
    for (const edge of visibleEdges) {
      this._createEdgeLine(edge);
    }

    // Position camera to see everything
    this._fitCameraToScene();
  }

  /**
   * Create mesh for a node
   */
  _createNodeMesh(node) {
    const geometry = this.geometries[node.shape] || this.geometries.sphere;
    const color = new THREE.Color(node.color);

    const material = new THREE.MeshPhongMaterial({
      color: color,
      transparent: true,
      opacity: 0.85,
      emissive: color,
      emissiveIntensity: node.highlighted ? 0.4 : 0.1,
      shininess: 30,
    });

    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(node.position.x, node.position.y, node.position.z);
    mesh.scale.setScalar(node.size);
    mesh.userData = { nodeId: node.id, type: node.type };

    this.scene.add(mesh);
    this.meshes.set(node.id, mesh);

    // Create label
    this._createLabel(node);
  }

  /**
   * Create text label for node
   */
  _createLabel(node) {
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    canvas.width = 512;
    canvas.height = 128;

    // Background with rounded corners
    context.fillStyle = 'rgba(10, 10, 15, 0.85)';
    this._roundRect(context, 0, 0, canvas.width, canvas.height, 16);
    context.fill();

    // Border
    context.strokeStyle = '#' + node.color.toString(16).padStart(6, '0');
    context.lineWidth = 3;
    this._roundRect(context, 2, 2, canvas.width - 4, canvas.height - 4, 14);
    context.stroke();

    // Text
    context.font = 'bold 36px JetBrains Mono, monospace';
    context.fillStyle = '#ffffff';
    context.textAlign = 'center';
    context.textBaseline = 'middle';

    const displayName = node.name.length > 18 ? node.name.slice(0, 15) + '...' : node.name;
    context.fillText(displayName, canvas.width / 2, canvas.height / 2 - 10);

    // Type badge
    context.font = '22px JetBrains Mono, monospace';
    context.fillStyle = '#' + node.color.toString(16).padStart(6, '0');
    context.fillText(node.type, canvas.width / 2, canvas.height / 2 + 25);

    const texture = new THREE.CanvasTexture(canvas);
    texture.minFilter = THREE.LinearFilter;

    const spriteMaterial = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
    });

    const sprite = new THREE.Sprite(spriteMaterial);
    sprite.position.set(
      node.position.x,
      node.position.y - node.size - 0.5,
      node.position.z
    );
    sprite.scale.set(2.5, 0.625, 1);

    this.scene.add(sprite);
    this.labels.set(node.id, sprite);
  }

  /**
   * Helper to draw rounded rectangle
   */
  _roundRect(ctx, x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + r);
    ctx.lineTo(x + w, y + h - r);
    ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    ctx.lineTo(x + r, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
  }

  /**
   * Create line for edge
   */
  _createEdgeLine(edge) {
    const sourcePos = edge.source.position;
    const targetPos = edge.target.position;

    const material = new THREE.LineBasicMaterial({
      color: edge.highlighted ? 0x4ecdc4 : edge.getColor(),
      transparent: true,
      opacity: edge.highlighted ? 0.8 : 0.3,
      linewidth: edge.highlighted ? 2 : 1,
    });

    const points = [
      new THREE.Vector3(sourcePos.x, sourcePos.y, sourcePos.z),
      new THREE.Vector3(targetPos.x, targetPos.y, targetPos.z),
    ];

    const geometry = new THREE.BufferGeometry().setFromPoints(points);
    const line = new THREE.Line(geometry, material);

    this.scene.add(line);
    this.lines.push(line);
  }

  /**
   * Clear all rendered objects
   */
  _clear() {
    // Remove meshes
    for (const mesh of this.meshes.values()) {
      this.scene.remove(mesh);
      mesh.geometry?.dispose();
      mesh.material?.dispose();
    }
    this.meshes.clear();

    // Remove labels
    for (const label of this.labels.values()) {
      this.scene.remove(label);
      label.material?.map?.dispose();
      label.material?.dispose();
    }
    this.labels.clear();

    // Remove lines
    for (const line of this.lines) {
      this.scene.remove(line);
      line.geometry?.dispose();
      line.material?.dispose();
    }
    this.lines = [];
  }

  /**
   * Select a node
   */
  _selectNode(nodeId) {
    // Reset previous selection
    if (this.selectedNodeId) {
      const prevMesh = this.meshes.get(this.selectedNodeId);
      if (prevMesh) {
        prevMesh.scale.setScalar(this.data.getNode(this.selectedNodeId)?.size || 1);
        prevMesh.material.emissiveIntensity = 0.1;
      }
    }

    this.selectedNodeId = nodeId;

    // Highlight new selection
    const mesh = this.meshes.get(nodeId);
    if (mesh) {
      mesh.scale.multiplyScalar(1.3);
      mesh.material.emissiveIntensity = 0.5;
    }
  }

  /**
   * Highlight node and connections
   */
  _highlightNode(nodeId) {
    if (!this.data) return;

    this.data.highlightNode(nodeId);

    // Update mesh appearances
    for (const [id, mesh] of this.meshes) {
      const node = this.data.getNode(id);
      if (node) {
        mesh.material.emissiveIntensity = node.highlighted ? 0.4 : 0.1;
        mesh.material.opacity = node.highlighted ? 1.0 : 0.6;
      }
    }

    // Update edge appearances
    this.lines.forEach((line, index) => {
      const edge = this.data.getVisibleEdges()[index];
      if (edge) {
        line.material.opacity = edge.highlighted ? 0.8 : 0.2;
        line.material.color.setHex(edge.highlighted ? 0x4ecdc4 : edge.getColor());
      }
    });
  }

  /**
   * Clear all highlights
   */
  _clearHighlight() {
    if (!this.data) return;

    this.data.clearHighlights();

    // Reset mesh appearances
    for (const [id, mesh] of this.meshes) {
      const node = this.data.getNode(id);
      if (node) {
        mesh.material.emissiveIntensity = id === this.selectedNodeId ? 0.5 : 0.1;
        mesh.material.opacity = 0.85;
      }
    }

    // Reset edge appearances
    this.lines.forEach((line, index) => {
      const edge = this.data.getVisibleEdges()[index];
      if (edge) {
        line.material.opacity = 0.3;
        line.material.color.setHex(edge.getColor());
      }
    });
  }

  /**
   * Handle focus change (semantic zoom)
   */
  _onFocusChanged(node, previous) {
    // Re-render with new focus
    this.render();

    // Animate camera to new position
    this._animateCameraTo(node);
  }

  /**
   * Fit camera to see all visible nodes
   */
  _fitCameraToScene() {
    if (!this.data) return;

    const visibleNodes = this.data.getVisibleNodes();
    if (visibleNodes.length === 0) return;

    // Calculate bounding box
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;
    let minZ = Infinity, maxZ = -Infinity;

    for (const node of visibleNodes) {
      minX = Math.min(minX, node.position.x);
      maxX = Math.max(maxX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxY = Math.max(maxY, node.position.y);
      minZ = Math.min(minZ, node.position.z);
      maxZ = Math.max(maxZ, node.position.z);
    }

    // Center point
    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;
    const centerZ = (minZ + maxZ) / 2;

    this.cameraLookAt = { x: centerX, y: centerY, z: centerZ };

    // Distance based on extent
    const extentX = maxX - minX;
    const extentZ = maxZ - minZ;
    const maxExtent = Math.max(extentX, extentZ, 5);
    const distance = maxExtent * PHI;

    this.camera.position.set(
      centerX + distance * 0.6,
      centerY + distance * 0.4,
      centerZ + distance * 0.6
    );
    this.camera.lookAt(centerX, centerY, centerZ);
  }

  /**
   * Animate camera to focus on node
   */
  _animateCameraTo(node) {
    if (!node) return;

    const targetLookAt = {
      x: node.position.x,
      y: node.position.y,
      z: node.position.z,
    };

    // Calculate camera position
    const distance = 6 * PHI;
    const targetPosition = {
      x: node.position.x + distance * 0.5,
      y: node.position.y + distance * 0.3,
      z: node.position.z + distance * 0.5,
    };

    // Animate
    this._animateCamera(targetPosition, targetLookAt, 500);
  }

  /**
   * Smooth camera animation
   */
  _animateCamera(targetPosition, targetLookAt, duration) {
    const startPosition = this.camera.position.clone();
    const startLookAt = { ...this.cameraLookAt };
    const startTime = Date.now();

    this.isAnimating = true;

    const animate = () => {
      const elapsed = Date.now() - startTime;
      const t = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - t, 3); // Ease out cubic

      // Interpolate position
      this.camera.position.set(
        startPosition.x + (targetPosition.x - startPosition.x) * eased,
        startPosition.y + (targetPosition.y - startPosition.y) * eased,
        startPosition.z + (targetPosition.z - startPosition.z) * eased
      );

      // Interpolate look-at
      this.cameraLookAt = {
        x: startLookAt.x + (targetLookAt.x - startLookAt.x) * eased,
        y: startLookAt.y + (targetLookAt.y - startLookAt.y) * eased,
        z: startLookAt.z + (targetLookAt.z - startLookAt.z) * eased,
      };

      this.camera.lookAt(this.cameraLookAt.x, this.cameraLookAt.y, this.cameraLookAt.z);

      if (t < 1) {
        requestAnimationFrame(animate);
      } else {
        this.isAnimating = false;
      }
    };

    animate();
  }

  /**
   * Animation loop
   */
  _animate() {
    this.animationId = requestAnimationFrame(() => this._animate());

    // Gentle rotation for meshes
    for (const mesh of this.meshes.values()) {
      if (!this.isAnimating) {
        mesh.rotation.y += 0.002;
      }
    }

    this.renderer.render(this.scene, this.camera);
  }

  /**
   * Handle window resize
   */
  _onResize() {
    if (!this.container) return;

    const width = this.container.clientWidth;
    const height = this.container.clientHeight;

    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height);
  }

  /**
   * Dispose scene
   */
  dispose() {
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
    }

    this._clear();

    // Dispose cached geometries
    for (const geometry of Object.values(this.geometries)) {
      geometry.dispose();
    }

    if (this.renderer) {
      this.renderer.dispose();
      this.container?.removeChild(this.renderer.domElement);
    }

    this.isInitialized = false;
  }
}

// Export to window
if (typeof window !== 'undefined') {
  window.CodebaseGraphScene = CodebaseGraphScene;
}
