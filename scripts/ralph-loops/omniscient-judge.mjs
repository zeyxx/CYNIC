#!/usr/bin/env node
/**
 * RALPH LOOP: Omniscient Judge
 *
 * "φ voit tout, φ doute de tout"
 *
 * Deep analysis with:
 * - Package-level understanding
 * - Dynamic import detection
 * - Cross-reference validation
 * - Architectural pattern recognition
 * - Holistic judgment with 25 dimensions
 *
 * @module scripts/ralph-loops/omniscient-judge
 */

import fs from 'fs';
import path from 'path';

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

// Colors
const C = {
  reset: '\x1b[0m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
};

/**
 * File categories
 */
const Category = {
  CORE: 'core',           // Essential infrastructure
  FEATURE: 'feature',     // Feature implementation
  TEST: 'test',           // Test files
  EXAMPLE: 'example',     // Examples/demos
  SCRIPT: 'script',       // CLI/utility scripts
  CONFIG: 'config',       // Configuration
  BENCHMARK: 'benchmark', // Performance tests
  DEPRECATED: 'deprecated', // Old/unused
  INDEX: 'index',         // Barrel exports
  UNKNOWN: 'unknown',
};

/**
 * Health status
 */
const Health = {
  HEALTHY: 'HEALTHY',     // Good shape
  WARNING: 'WARNING',     // Minor issues
  CRITICAL: 'CRITICAL',   // Needs attention
  DEAD: 'DEAD',           // Should be removed
};

/**
 * Omniscient state
 */
class OmniscientState {
  constructor(rootDir) {
    this.rootDir = rootDir;
    this.startTime = Date.now();

    // File-level data
    this.files = new Map();

    // Package-level data
    this.packages = new Map();

    // Dependency graph
    this.imports = new Map();      // file -> [imported files]
    this.importedBy = new Map();   // file -> [files that import it]
    this.packageDeps = new Map();  // package -> [package deps]

    // Dynamic analysis
    this.dynamicImports = new Set();
    this.entryPoints = new Set();
    this.exports = new Map();

    // Judgments
    this.judgments = new Map();

    // Global metrics
    this.metrics = {
      totalFiles: 0,
      totalLines: 0,
      totalPackages: 0,
      healthDistribution: {},
      categoryDistribution: {},
    };

    // Issues found
    this.issues = [];

    // Recommendations
    this.recommendations = [];
  }
}

/**
 * Deep file analysis
 */
class FileAnalysis {
  constructor(filePath, rootDir) {
    this.path = path.relative(rootDir, filePath).replace(/\\/g, '/');
    this.fullPath = filePath;
    this.package = null;
    this.category = Category.UNKNOWN;
    this.health = Health.HEALTHY;

    // Metrics
    this.lines = 0;
    this.bytes = 0;
    this.complexity = 0;
    this.imports = [];
    this.exports = [];
    this.dynamicImports = [];

    // Analysis
    this.isEntryPoint = false;
    this.isBarrel = false;
    this.hasSideEffects = false;
    this.usesRequire = false;
    this.usesESM = false;

    // 25-dimension judgment
    this.dimensions = {};
    this.score = 0;
    this.verdict = null;
    this.reasons = [];
  }
}

/**
 * Package analysis
 */
class PackageAnalysis {
  constructor(name) {
    this.name = name;
    this.path = `packages/${name}`;
    this.files = [];
    this.totalLines = 0;
    this.health = Health.HEALTHY;

    // Package.json data
    this.version = null;
    this.dependencies = [];
    this.peerDependencies = [];
    this.exports = {};

    // Analysis
    this.isUsed = false;
    this.usedBy = [];
    this.cohesion = 0;
    this.coupling = 0;

    // Judgment
    this.score = 0;
    this.verdict = null;
    this.reasons = [];
  }
}

/**
 * Omniscient Judge
 */
class OmniscientJudge {
  constructor(options = {}) {
    this.rootDir = options.rootDir || process.cwd();
    this.state = new OmniscientState(this.rootDir);
    this.verbose = options.verbose || false;
  }

  async run() {
    this.banner();

    console.log(`${C.cyan}── PHASE 1: Deep Scan ─────────────────────────────────────────${C.reset}`);
    await this.phase1_deepScan();

    console.log(`${C.cyan}── PHASE 2: Package Analysis ──────────────────────────────────${C.reset}`);
    await this.phase2_packageAnalysis();

    console.log(`${C.cyan}── PHASE 3: Dependency Graph ──────────────────────────────────${C.reset}`);
    await this.phase3_dependencyGraph();

    console.log(`${C.cyan}── PHASE 4: Entry Point Detection ─────────────────────────────${C.reset}`);
    await this.phase4_entryPoints();

    console.log(`${C.cyan}── PHASE 5: 25-Dimension Judgment ─────────────────────────────${C.reset}`);
    await this.phase5_judgment();

    console.log(`${C.cyan}── PHASE 6: Holistic Synthesis ────────────────────────────────${C.reset}`);
    await this.phase6_synthesis();

    console.log(`${C.cyan}── PHASE 7: Report Generation ─────────────────────────────────${C.reset}`);
    await this.phase7_report();

    this.omniscientSummary();
  }

  banner() {
    console.log('');
    console.log('═══════════════════════════════════════════════════════════════');
    console.log(`${C.magenta}👁️  RALPH OMNISCIENT JUDGE${C.reset}`);
    console.log(`${C.dim}   "φ voit tout, φ doute de tout"${C.reset}`);
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('');
  }

  /**
   * Phase 1: Deep file scan
   */
  async phase1_deepScan() {
    const walk = (dir, depth = 0) => {
      if (depth > 12) return;

      try {
        const items = fs.readdirSync(dir);

        for (const item of items) {
          if (this.shouldSkip(item)) continue;

          const fullPath = path.join(dir, item);
          const stat = fs.statSync(fullPath);

          if (stat.isDirectory()) {
            walk(fullPath, depth + 1);
          } else if (this.isJSFile(item)) {
            const analysis = this.analyzeFile(fullPath);
            this.state.files.set(analysis.path, analysis);
          }
        }
      } catch (e) {
        // Ignore errors
      }
    };

    walk(this.rootDir);

    this.state.metrics.totalFiles = this.state.files.size;
    this.state.metrics.totalLines = [...this.state.files.values()]
      .reduce((sum, f) => sum + f.lines, 0);

    console.log(`   ✓ ${this.state.files.size} files scanned`);
    console.log(`   ✓ ${this.state.metrics.totalLines.toLocaleString()} lines analyzed`);
    console.log('');
  }

  shouldSkip(name) {
    return name.startsWith('.') ||
           name === 'node_modules' ||
           name === 'dist' ||
           name === 'coverage' ||
           name === 'build';
  }

  isJSFile(name) {
    return name.endsWith('.js') || name.endsWith('.mjs') || name.endsWith('.cjs');
  }

  /**
   * Deep file analysis
   */
  analyzeFile(filePath) {
    const analysis = new FileAnalysis(filePath, this.rootDir);

    try {
      const content = fs.readFileSync(filePath, 'utf8');
      const stat = fs.statSync(filePath);

      analysis.bytes = stat.size;
      analysis.lines = content.split('\n').length;

      // Extract package
      const pkgMatch = analysis.path.match(/^packages\/([^\/]+)/);
      if (pkgMatch) analysis.package = pkgMatch[1];

      // Categorize
      analysis.category = this.categorizeFile(analysis.path, content);

      // Extract imports (static)
      analysis.imports = this.extractImports(content);
      analysis.usesESM = /\bimport\s/.test(content);
      analysis.usesRequire = /\brequire\s*\(/.test(content);

      // Extract dynamic imports
      analysis.dynamicImports = this.extractDynamicImports(content);

      // Extract exports
      analysis.exports = this.extractExports(content);
      analysis.isBarrel = this.isBarrelFile(content, analysis.exports);

      // Detect side effects
      analysis.hasSideEffects = this.hasSideEffects(content);

      // Compute complexity
      analysis.complexity = this.computeComplexity(content);

      // Check if entry point
      analysis.isEntryPoint = this.isEntryPoint(analysis.path, content);

    } catch (e) {
      analysis.reasons.push(`Read error: ${e.message}`);
    }

    return analysis;
  }

  categorizeFile(filePath, content) {
    if (filePath.includes('/test/') || filePath.includes('.test.')) return Category.TEST;
    if (filePath.includes('/examples/') || filePath.includes('example')) return Category.EXAMPLE;
    if (filePath.includes('/benchmarks/') || filePath.includes('benchmark')) return Category.BENCHMARK;
    if (filePath.startsWith('scripts/')) return Category.SCRIPT;
    if (filePath.endsWith('index.js')) return Category.INDEX;
    if (filePath.includes('config') || filePath.endsWith('.config.js')) return Category.CONFIG;
    if (content.includes('@deprecated') || content.includes('DEPRECATED')) return Category.DEPRECATED;

    // Core vs Feature
    if (filePath.includes('/core/') || filePath.includes('/lib/')) return Category.CORE;

    return Category.FEATURE;
  }

  extractImports(content) {
    const imports = [];

    // ES6 imports
    const esmRegex = /import\s+(?:(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)\s+from\s+)?['"]([^'"]+)['"]/g;
    let match;
    while ((match = esmRegex.exec(content)) !== null) {
      imports.push({ source: match[1], type: 'esm' });
    }

    // CommonJS requires
    const cjsRegex = /(?:const|let|var)\s+(?:\{[^}]*\}|\w+)\s*=\s*require\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
    while ((match = cjsRegex.exec(content)) !== null) {
      imports.push({ source: match[1], type: 'cjs' });
    }

    return imports;
  }

  extractDynamicImports(content) {
    const dynamicImports = [];

    // Dynamic import()
    const dynamicRegex = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
    let match;
    while ((match = dynamicRegex.exec(content)) !== null) {
      dynamicImports.push(match[1]);
    }

    // Dynamic require with variable
    if (/require\s*\([^'"]+\)/.test(content)) {
      dynamicImports.push('__dynamic_require__');
    }

    return dynamicImports;
  }

  extractExports(content) {
    const exports = [];

    // Named exports
    const namedRegex = /export\s+(?:const|let|var|function|class|async\s+function)\s+(\w+)/g;
    let match;
    while ((match = namedRegex.exec(content)) !== null) {
      exports.push({ name: match[1], type: 'named' });
    }

    // Re-exports
    const reExportRegex = /export\s+(?:\*|\{[^}]*\})\s+from\s+['"]([^'"]+)['"]/g;
    while ((match = reExportRegex.exec(content)) !== null) {
      exports.push({ name: '*', type: 're-export', source: match[1] });
    }

    // Default export
    if (/export\s+default/.test(content)) {
      exports.push({ name: 'default', type: 'default' });
    }

    // module.exports
    if (/module\.exports\s*=/.test(content)) {
      exports.push({ name: 'module.exports', type: 'cjs' });
    }

    return exports;
  }

  isBarrelFile(content, exports) {
    // A barrel file mostly re-exports
    const reExports = exports.filter(e => e.type === 're-export').length;
    const total = exports.length;
    return total > 0 && reExports / total > 0.5;
  }

  hasSideEffects(content) {
    // Check for top-level side effects
    const patterns = [
      /^[^/]*console\.(log|warn|error)/m,
      /^[^/]*process\.(exit|on)/m,
      /^[^/]*fs\.(write|unlink|mkdir)/m,
      /^[^/]*setInterval|setTimeout/m,
    ];

    return patterns.some(p => p.test(content));
  }

  computeComplexity(content) {
    const patterns = [
      [/\bif\b/g, 1],
      [/\belse\s+if\b/g, 1],
      [/\bfor\b/g, 1],
      [/\bwhile\b/g, 1],
      [/\bswitch\b/g, 1],
      [/\bcase\b/g, 0.5],
      [/\bcatch\b/g, 1],
      [/\?\?/g, 0.5],
      [/\?\./g, 0.3],
      [/&&/g, 0.3],
      [/\|\|/g, 0.3],
      [/\?[^:?]/g, 0.5],
      [/\.then\(/g, 0.5],
      [/\.catch\(/g, 0.5],
      [/async\s+function/g, 0.5],
      [/await\s/g, 0.3],
    ];

    let complexity = 1;
    for (const [pattern, weight] of patterns) {
      const matches = content.match(pattern);
      if (matches) complexity += matches.length * weight;
    }

    return Math.round(complexity);
  }

  isEntryPoint(filePath, content) {
    // CLI entry points
    if (content.includes('#!/usr/bin')) return true;
    if (filePath.includes('/cli/') || filePath.includes('/bin/')) return true;

    // Server entry points
    if (filePath.endsWith('server.js') || filePath.endsWith('app.js')) return true;

    // Main entry points
    if (filePath.endsWith('/index.js') && filePath.split('/').length === 2) return true;

    // Script runners
    if (filePath.startsWith('scripts/') && !filePath.includes('/lib/')) return true;

    // Benchmark runners
    if (filePath.includes('benchmark') && filePath.endsWith('.js')) return true;

    return false;
  }

  /**
   * Phase 2: Package-level analysis
   */
  async phase2_packageAnalysis() {
    // Find all packages
    const packagesDir = path.join(this.rootDir, 'packages');

    try {
      const dirs = fs.readdirSync(packagesDir);

      for (const dir of dirs) {
        const pkgPath = path.join(packagesDir, dir);
        const pkgJsonPath = path.join(pkgPath, 'package.json');

        if (fs.statSync(pkgPath).isDirectory() && fs.existsSync(pkgJsonPath)) {
          const analysis = this.analyzePackage(dir, pkgJsonPath);
          this.state.packages.set(dir, analysis);
        }
      }
    } catch (e) {
      // No packages directory
    }

    this.state.metrics.totalPackages = this.state.packages.size;

    console.log(`   ✓ ${this.state.packages.size} packages analyzed`);

    // Show package overview
    for (const [name, pkg] of this.state.packages) {
      const files = [...this.state.files.values()].filter(f => f.package === name);
      pkg.files = files.map(f => f.path);
      pkg.totalLines = files.reduce((sum, f) => sum + f.lines, 0);

      console.log(`   ${C.dim}├─${C.reset} ${name}: ${files.length} files, ${pkg.totalLines.toLocaleString()} lines`);
    }
    console.log('');
  }

  analyzePackage(name, pkgJsonPath) {
    const analysis = new PackageAnalysis(name);

    try {
      const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));

      analysis.version = pkgJson.version;
      analysis.dependencies = Object.keys(pkgJson.dependencies || {});
      analysis.peerDependencies = Object.keys(pkgJson.peerDependencies || {});
      analysis.exports = pkgJson.exports || {};

      // Find internal @cynic dependencies
      const cynicDeps = analysis.dependencies.filter(d => d.startsWith('@cynic/'));
      analysis.internalDeps = cynicDeps.map(d => d.replace('@cynic/', ''));

    } catch (e) {
      analysis.reasons.push(`package.json error: ${e.message}`);
    }

    return analysis;
  }

  /**
   * Phase 3: Build dependency graph
   */
  async phase3_dependencyGraph() {
    let edges = 0;

    for (const [filePath, file] of this.state.files) {
      const resolvedImports = [];

      for (const imp of file.imports) {
        const resolved = this.resolveImport(imp.source, filePath);
        if (resolved) {
          resolvedImports.push(resolved);

          // Build reverse map
          if (!this.state.importedBy.has(resolved)) {
            this.state.importedBy.set(resolved, new Set());
          }
          this.state.importedBy.get(resolved).add(filePath);
          edges++;
        }
      }

      this.state.imports.set(filePath, resolvedImports);
    }

    // Handle dynamic imports
    for (const [filePath, file] of this.state.files) {
      for (const dynImp of file.dynamicImports) {
        if (dynImp !== '__dynamic_require__') {
          const resolved = this.resolveImport(dynImp, filePath);
          if (resolved) {
            this.state.dynamicImports.add(resolved);
          }
        }
      }
    }

    console.log(`   ✓ ${edges} dependency edges mapped`);
    console.log(`   ✓ ${this.state.dynamicImports.size} dynamic imports detected`);

    // Find circular dependencies
    const cycles = this.findCycles();
    if (cycles.length > 0) {
      console.log(`   ${C.yellow}⚠ ${cycles.length} circular dependencies${C.reset}`);
      this.state.issues.push({
        type: 'CIRCULAR_DEPS',
        severity: 'WARNING',
        items: cycles,
      });
    }

    console.log('');
  }

  resolveImport(source, fromFile) {
    // Skip external packages
    if (!source.startsWith('.') && !source.startsWith('@cynic/')) {
      return null;
    }

    // Resolve @cynic/ imports
    if (source.startsWith('@cynic/')) {
      const pkgName = source.replace('@cynic/', '').split('/')[0];
      return `packages/${pkgName}/src/index.js`;
    }

    // Resolve relative imports
    const fromDir = path.dirname(fromFile);
    let resolved = path.join(fromDir, source).replace(/\\/g, '/');

    // Try with .js extension
    if (!resolved.endsWith('.js') && !resolved.endsWith('.mjs')) {
      if (this.state.files.has(resolved + '.js')) {
        resolved = resolved + '.js';
      } else if (this.state.files.has(resolved + '/index.js')) {
        resolved = resolved + '/index.js';
      }
    }

    return this.state.files.has(resolved) ? resolved : null;
  }

  findCycles() {
    const cycles = [];
    const visited = new Set();
    const recursionStack = new Set();

    const dfs = (node, path) => {
      if (recursionStack.has(node)) {
        const cycleStart = path.indexOf(node);
        if (cycleStart !== -1) {
          cycles.push(path.slice(cycleStart));
        }
        return;
      }

      if (visited.has(node)) return;

      visited.add(node);
      recursionStack.add(node);

      const deps = this.state.imports.get(node) || [];
      for (const dep of deps) {
        dfs(dep, [...path, node]);
      }

      recursionStack.delete(node);
    };

    for (const file of this.state.files.keys()) {
      dfs(file, []);
    }

    return cycles.slice(0, 10); // Limit to first 10
  }

  /**
   * Phase 4: Entry point detection
   */
  async phase4_entryPoints() {
    // Collect all entry points
    for (const [filePath, file] of this.state.files) {
      if (file.isEntryPoint) {
        this.state.entryPoints.add(filePath);
      }
    }

    // Mark files reachable from entry points
    const reachable = new Set();

    const markReachable = (file) => {
      if (reachable.has(file)) return;
      reachable.add(file);

      const deps = this.state.imports.get(file) || [];
      for (const dep of deps) {
        markReachable(dep);
      }
    };

    // Start from all entry points
    for (const entry of this.state.entryPoints) {
      markReachable(entry);
    }

    // Also mark dynamically imported files
    for (const dynFile of this.state.dynamicImports) {
      markReachable(dynFile);
    }

    // Also mark index files (they're structural)
    for (const [filePath, file] of this.state.files) {
      if (file.category === Category.INDEX) {
        reachable.add(filePath);
        const deps = this.state.imports.get(filePath) || [];
        for (const dep of deps) markReachable(dep);
      }
    }

    // Find unreachable files (true orphans)
    const orphans = [];
    for (const [filePath, file] of this.state.files) {
      if (!reachable.has(filePath) &&
          file.category !== Category.TEST &&
          file.category !== Category.EXAMPLE &&
          file.category !== Category.BENCHMARK &&
          file.category !== Category.SCRIPT) {
        orphans.push(filePath);
      }
    }

    console.log(`   ✓ ${this.state.entryPoints.size} entry points identified`);
    console.log(`   ✓ ${reachable.size} files reachable from entry points`);
    console.log(`   ${C.red}✗ ${orphans.length} true orphans (unreachable code)${C.reset}`);

    if (orphans.length > 0) {
      this.state.issues.push({
        type: 'ORPHAN_FILES',
        severity: 'HIGH',
        items: orphans,
      });
    }

    // Store reachability
    for (const [filePath, file] of this.state.files) {
      file.isReachable = reachable.has(filePath);
    }

    console.log('');
  }

  /**
   * Phase 5: 25-Dimension Judgment
   */
  async phase5_judgment() {
    let judged = 0;

    for (const [filePath, file] of this.state.files) {
      this.judge25Dimensions(file);
      judged++;

      if (judged % 100 === 0) {
        process.stdout.write(`   Judging... ${judged}/${this.state.files.size}\r`);
      }
    }

    console.log(`   ✓ ${judged} files judged on 25 dimensions                    `);

    // Aggregate by health
    const byHealth = { HEALTHY: 0, WARNING: 0, CRITICAL: 0, DEAD: 0 };
    for (const file of this.state.files.values()) {
      byHealth[file.health] = (byHealth[file.health] || 0) + 1;
    }

    this.state.metrics.healthDistribution = byHealth;

    console.log(`   ${C.green}HEALTHY${C.reset}: ${byHealth.HEALTHY} | ${C.yellow}WARNING${C.reset}: ${byHealth.WARNING} | ${C.red}CRITICAL${C.reset}: ${byHealth.CRITICAL} | ${C.dim}DEAD${C.reset}: ${byHealth.DEAD}`);
    console.log('');
  }

  /**
   * 25-Dimension judgment system
   */
  judge25Dimensions(file) {
    const d = file.dimensions;

    // === EXISTENCE DIMENSIONS (Is it needed?) ===

    // D1: Reachability - Is it reachable from entry points?
    d.reachability = file.isReachable ? 1 : 0;

    // D2: Import count - How many files import this?
    const importers = this.state.importedBy.get(file.path);
    const importerCount = importers ? importers.size : 0;
    d.importCount = Math.min(1, importerCount / 5);

    // D3: Export usage - Are exports used?
    d.exportUsage = file.exports.length > 0 ? Math.min(1, importerCount / file.exports.length) : 0.5;

    // D4: Dynamic import protection - Is it dynamically imported?
    d.dynamicProtection = this.state.dynamicImports.has(file.path) ? 1 : 0;

    // D5: Entry point status
    d.entryPoint = file.isEntryPoint ? 1 : 0;

    // === SIZE DIMENSIONS (Is it right-sized?) ===

    // D6: Line count appropriateness
    if (file.lines > 1000) d.lineCount = 0.2;
    else if (file.lines > 500) d.lineCount = 0.4;
    else if (file.lines > 300) d.lineCount = 0.7;
    else if (file.lines < 10) d.lineCount = 0.5;
    else d.lineCount = 1;

    // D7: Complexity appropriateness
    const complexityPerLine = file.complexity / Math.max(file.lines, 1);
    d.complexity = complexityPerLine < 0.1 ? 1 : complexityPerLine < 0.2 ? 0.7 : 0.4;

    // D8: Export count balance
    d.exportBalance = file.exports.length <= 10 ? 1 : file.exports.length <= 20 ? 0.7 : 0.4;

    // === COUPLING DIMENSIONS (Is it well-connected?) ===

    // D9: Import count (too many = hotspot)
    if (file.imports.length > 20) d.importCoupling = 0.2;
    else if (file.imports.length > 13) d.importCoupling = 0.5;
    else d.importCoupling = 1;

    // D10: Fan-out (how many things does it depend on)
    const fanOut = this.state.imports.get(file.path)?.length || 0;
    d.fanOut = fanOut <= 8 ? 1 : fanOut <= 13 ? 0.7 : 0.4;

    // D11: Fan-in (how many things depend on it)
    d.fanIn = importerCount >= 3 ? 1 : importerCount >= 1 ? 0.7 : 0.3;

    // D12: Package cohesion (imports from same package)
    if (file.package) {
      const samePackageImports = file.imports.filter(i =>
        i.source.includes(file.package) || i.source.startsWith('.')
      ).length;
      d.cohesion = file.imports.length > 0 ? samePackageImports / file.imports.length : 1;
    } else {
      d.cohesion = 0.5;
    }

    // === QUALITY DIMENSIONS ===

    // D13: Category appropriateness
    d.category = file.category !== Category.DEPRECATED ? 0.8 : 0.1;

    // D14: Barrel file (index.js that just re-exports)
    d.barrelPenalty = file.isBarrel ? 0.8 : 1;

    // D15: Mixed module systems (ESM + CJS)
    d.moduleConsistency = (file.usesESM && file.usesRequire) ? 0.5 : 1;

    // D16: Side effects at module level
    d.sideEffects = file.hasSideEffects ? 0.6 : 1;

    // D17: Test coverage proxy (test file exists?)
    const hasTest = this.state.files.has(file.path.replace('.js', '.test.js'));
    d.testCoverage = file.category === Category.TEST ? 1 : (hasTest ? 0.8 : 0.5);

    // === ARCHITECTURAL DIMENSIONS ===

    // D18: Package membership (is it in a package?)
    d.packageMembership = file.package ? 1 : 0.7;

    // D19: Depth in tree (too deep = hidden)
    const depth = file.path.split('/').length;
    d.depth = depth <= 5 ? 1 : depth <= 7 ? 0.8 : 0.5;

    // D20: Naming convention
    const hasGoodName = /^[a-z][a-z0-9-]*\.m?js$/.test(path.basename(file.path));
    d.naming = hasGoodName ? 1 : 0.6;

    // === MAINTENANCE DIMENSIONS ===

    // D21: Documentation (has JSDoc?)
    // Approximated by checking for /** comments
    d.documentation = 0.5; // Default, would need content analysis

    // D22: Error handling (try/catch present for async)
    d.errorHandling = 0.7; // Default

    // D23: Circular dependency involvement
    const inCycle = this.state.issues
      .filter(i => i.type === 'CIRCULAR_DEPS')
      .some(i => i.items.some(cycle => cycle.includes(file.path)));
    d.noCycles = inCycle ? 0.2 : 1;

    // === STRATEGIC DIMENSIONS ===

    // D24: Core vs peripheral
    d.strategic = file.category === Category.CORE ? 1 :
                  file.category === Category.FEATURE ? 0.8 : 0.5;

    // D25: Future potential (based on exports and usage)
    d.potential = (d.exportUsage + d.fanIn) / 2;

    // === COMPUTE FINAL SCORE ===
    const weights = {
      // Existence (35%)
      reachability: 0.12,
      importCount: 0.08,
      exportUsage: 0.05,
      dynamicProtection: 0.05,
      entryPoint: 0.05,

      // Size (15%)
      lineCount: 0.08,
      complexity: 0.04,
      exportBalance: 0.03,

      // Coupling (20%)
      importCoupling: 0.06,
      fanOut: 0.05,
      fanIn: 0.05,
      cohesion: 0.04,

      // Quality (15%)
      category: 0.04,
      barrelPenalty: 0.02,
      moduleConsistency: 0.03,
      sideEffects: 0.03,
      testCoverage: 0.03,

      // Architectural (10%)
      packageMembership: 0.03,
      depth: 0.02,
      naming: 0.02,
      noCycles: 0.03,

      // Maintenance & Strategic (5%)
      documentation: 0.01,
      errorHandling: 0.01,
      strategic: 0.02,
      potential: 0.01,
    };

    let score = 0;
    for (const [dim, weight] of Object.entries(weights)) {
      score += (d[dim] || 0) * weight;
    }

    file.score = Math.min(score, PHI_INV); // Cap at φ⁻¹

    // Determine health
    if (score >= 0.7) file.health = Health.HEALTHY;
    else if (score >= 0.5) file.health = Health.WARNING;
    else if (score >= 0.3) file.health = Health.CRITICAL;
    else file.health = Health.DEAD;

    // Generate reasons
    if (d.reachability < 0.5) file.reasons.push('Unreachable from entry points');
    if (d.lineCount < 0.5) file.reasons.push(`Too large: ${file.lines} lines`);
    if (d.importCoupling < 0.5) file.reasons.push(`Hotspot: ${file.imports.length} imports`);
    if (d.noCycles < 0.5) file.reasons.push('Involved in circular dependency');
    if (d.fanIn < 0.5 && !file.isEntryPoint) file.reasons.push('Low usage');
  }

  /**
   * Phase 6: Holistic synthesis
   */
  async phase6_synthesis() {
    // Generate recommendations based on aggregated analysis

    // 1. Dead files (should delete)
    const deadFiles = [...this.state.files.values()]
      .filter(f => f.health === Health.DEAD)
      .sort((a, b) => a.score - b.score);

    if (deadFiles.length > 0) {
      const totalLines = deadFiles.reduce((sum, f) => sum + f.lines, 0);
      this.state.recommendations.push({
        action: 'DELETE',
        priority: 'HIGH',
        count: deadFiles.length,
        linesImpact: totalLines,
        files: deadFiles.slice(0, 30).map(f => ({
          path: f.path,
          lines: f.lines,
          score: f.score.toFixed(2),
          reasons: f.reasons,
        })),
      });
    }

    // 2. Critical files (need attention)
    const criticalFiles = [...this.state.files.values()]
      .filter(f => f.health === Health.CRITICAL && f.category !== Category.TEST)
      .sort((a, b) => a.score - b.score);

    if (criticalFiles.length > 0) {
      this.state.recommendations.push({
        action: 'REFACTOR',
        priority: 'MEDIUM',
        count: criticalFiles.length,
        files: criticalFiles.slice(0, 20).map(f => ({
          path: f.path,
          lines: f.lines,
          score: f.score.toFixed(2),
          reasons: f.reasons,
        })),
      });
    }

    // 3. Giant files (should split)
    const giants = [...this.state.files.values()]
      .filter(f => f.lines > 500 && f.category !== Category.TEST)
      .sort((a, b) => b.lines - a.lines);

    if (giants.length > 0) {
      this.state.recommendations.push({
        action: 'SPLIT',
        priority: 'MEDIUM',
        count: giants.length,
        files: giants.slice(0, 15).map(f => ({
          path: f.path,
          lines: f.lines,
          complexity: f.complexity,
        })),
      });
    }

    // 4. Hotspots (should simplify)
    const hotspots = [...this.state.files.values()]
      .filter(f => f.imports.length > 13)
      .sort((a, b) => b.imports.length - a.imports.length);

    if (hotspots.length > 0) {
      this.state.recommendations.push({
        action: 'SIMPLIFY',
        priority: 'LOW',
        count: hotspots.length,
        files: hotspots.slice(0, 10).map(f => ({
          path: f.path,
          imports: f.imports.length,
        })),
      });
    }

    // 5. Package health
    for (const [name, pkg] of this.state.packages) {
      const pkgFiles = [...this.state.files.values()].filter(f => f.package === name);
      const avgScore = pkgFiles.reduce((sum, f) => sum + f.score, 0) / pkgFiles.length;
      const deadCount = pkgFiles.filter(f => f.health === Health.DEAD).length;

      pkg.score = avgScore;
      pkg.deadCount = deadCount;

      if (avgScore < 0.4) {
        pkg.health = Health.CRITICAL;
        pkg.reasons.push(`Low average score: ${avgScore.toFixed(2)}`);
      } else if (deadCount > pkgFiles.length * 0.3) {
        pkg.health = Health.WARNING;
        pkg.reasons.push(`${deadCount} dead files (${((deadCount/pkgFiles.length)*100).toFixed(0)}%)`);
      }
    }

    console.log(`   ✓ ${this.state.recommendations.length} recommendation groups generated`);
    console.log('');
  }

  /**
   * Phase 7: Report generation
   */
  async phase7_report() {
    const report = {
      meta: {
        timestamp: new Date().toISOString(),
        duration: ((Date.now() - this.state.startTime) / 1000).toFixed(1) + 's',
        version: '2.0.0-omniscient',
      },
      summary: {
        files: this.state.metrics.totalFiles,
        lines: this.state.metrics.totalLines,
        packages: this.state.metrics.totalPackages,
        health: this.state.metrics.healthDistribution,
      },
      packages: [...this.state.packages.values()].map(p => ({
        name: p.name,
        files: p.files.length,
        lines: p.totalLines,
        score: p.score?.toFixed(2),
        health: p.health,
        deadCount: p.deadCount,
        reasons: p.reasons,
      })),
      issues: this.state.issues,
      recommendations: this.state.recommendations,
      files: {},
    };

    // Include detailed file judgments for non-healthy files
    for (const [filePath, file] of this.state.files) {
      if (file.health !== Health.HEALTHY) {
        report.files[filePath] = {
          category: file.category,
          health: file.health,
          lines: file.lines,
          score: file.score.toFixed(3),
          reasons: file.reasons,
          dimensions: Object.fromEntries(
            Object.entries(file.dimensions).map(([k, v]) => [k, v.toFixed(2)])
          ),
        };
      }
    }

    const outputPath = path.join(this.rootDir, 'ralph-omniscient-report.json');
    fs.writeFileSync(outputPath, JSON.stringify(report, null, 2));

    console.log(`   ✓ Report saved: ralph-omniscient-report.json`);
    console.log('');
  }

  /**
   * Final omniscient summary
   */
  omniscientSummary() {
    const report = JSON.parse(fs.readFileSync(
      path.join(this.rootDir, 'ralph-omniscient-report.json'), 'utf8'
    ));

    console.log('═══════════════════════════════════════════════════════════════');
    console.log(`${C.magenta}👁️  OMNISCIENT VISION COMPLETE${C.reset}`);
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('');

    // Global stats
    console.log(`${C.cyan}── CODEBASE OVERVIEW ──────────────────────────────────────────${C.reset}`);
    console.log(`   Files: ${report.summary.files} | Lines: ${report.summary.lines.toLocaleString()} | Packages: ${report.summary.packages}`);
    console.log('');

    // Health distribution
    console.log(`${C.cyan}── HEALTH DISTRIBUTION ────────────────────────────────────────${C.reset}`);
    const h = report.summary.health;
    const total = h.HEALTHY + h.WARNING + h.CRITICAL + h.DEAD;

    const bar = (count, color) => {
      const width = Math.round((count / total) * 40);
      return color + '█'.repeat(width) + C.reset;
    };

    console.log(`   ${bar(h.HEALTHY, C.green)}${bar(h.WARNING, C.yellow)}${bar(h.CRITICAL, C.red)}${bar(h.DEAD, C.dim)}`);
    console.log(`   ${C.green}HEALTHY ${h.HEALTHY}${C.reset} | ${C.yellow}WARNING ${h.WARNING}${C.reset} | ${C.red}CRITICAL ${h.CRITICAL}${C.reset} | ${C.dim}DEAD ${h.DEAD}${C.reset}`);
    console.log('');

    // Package health
    console.log(`${C.cyan}── PACKAGE HEALTH ─────────────────────────────────────────────${C.reset}`);
    for (const pkg of report.packages.sort((a, b) => (a.score || 0) - (b.score || 0))) {
      const healthColor = pkg.health === 'HEALTHY' ? C.green :
                          pkg.health === 'WARNING' ? C.yellow : C.red;
      const scoreBar = '█'.repeat(Math.round((pkg.score || 0) * 10));
      console.log(`   ${healthColor}${pkg.name.padEnd(15)}${C.reset} [${scoreBar.padEnd(10)}] ${pkg.score || '?'} | ${pkg.files} files | ${pkg.deadCount || 0} dead`);
    }
    console.log('');

    // Issues
    if (report.issues.length > 0) {
      console.log(`${C.cyan}── ISSUES DETECTED ────────────────────────────────────────────${C.reset}`);
      for (const issue of report.issues) {
        console.log(`   ${C.yellow}⚠ ${issue.type}${C.reset}: ${issue.items?.length || 0} occurrences`);
      }
      console.log('');
    }

    // Recommendations
    console.log(`${C.cyan}── RECOMMENDATIONS ────────────────────────────────────────────${C.reset}`);
    for (const rec of report.recommendations) {
      const actionColor = rec.action === 'DELETE' ? C.red :
                          rec.action === 'REFACTOR' ? C.yellow :
                          rec.action === 'SPLIT' ? C.magenta : C.cyan;

      console.log(`   ${actionColor}[${rec.action}]${C.reset} ${rec.priority} priority | ${rec.count} files`);
      if (rec.linesImpact) {
        console.log(`   └─ Impact: ${rec.linesImpact.toLocaleString()} lines removable`);
      }

      // Show top 5 files
      rec.files?.slice(0, 5).forEach(f => {
        console.log(`      ${C.dim}•${C.reset} ${f.path}`);
        if (f.reasons?.length > 0) {
          console.log(`        ${C.dim}${f.reasons[0]}${C.reset}`);
        }
      });
      if (rec.files?.length > 5) {
        console.log(`      ${C.dim}... +${rec.files.length - 5} more${C.reset}`);
      }
      console.log('');
    }

    console.log('═══════════════════════════════════════════════════════════════');
    console.log(`   ${C.dim}φ max confidence: 61.8% - "φ voit tout, φ doute de tout"${C.reset}`);
    console.log(`   ${C.dim}Duration: ${report.meta.duration}${C.reset}`);
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('');
  }
}

// === MAIN ===
const judge = new OmniscientJudge({
  rootDir: process.cwd(),
  verbose: process.argv.includes('--verbose'),
});

judge.run().catch(e => {
  console.error('Omniscient Judge failed:', e);
  process.exit(1);
});
