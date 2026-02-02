#!/usr/bin/env node
/**
 * RALPH LOOP: Burn Cartographer
 *
 * "Ralph never lets go of an idea"
 *
 * Autonomous loop that:
 * 1. Cartographs the entire codebase (files, deps, structure)
 * 2. Judges each module/file harmoniously
 * 3. Produces actionable burn recommendations
 * 4. Iterates until complete
 *
 * φ-aligned: Max confidence 61.8%, skeptical by design
 *
 * @module scripts/ralph-loops/burn-cartographer
 */

import fs from 'fs';
import path from 'path';

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const FIB = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];

// Verdicts
const Verdict = {
  DELETE: 'DELETE',
  MERGE: 'MERGE',
  SPLIT: 'SPLIT',
  SIMPLIFY: 'SIMPLIFY',
  KEEP: 'KEEP',
  REVIEW: 'REVIEW',
};

// Verdict colors for display
const VerdictStyle = {
  DELETE: '\x1b[31m',    // Red
  MERGE: '\x1b[35m',     // Magenta
  SPLIT: '\x1b[33m',     // Yellow
  SIMPLIFY: '\x1b[36m',  // Cyan
  KEEP: '\x1b[32m',      // Green
  REVIEW: '\x1b[37m',    // White
};
const RESET = '\x1b[0m';

/**
 * Ralph Loop State
 */
class RalphState {
  constructor() {
    this.iteration = 0;
    this.startTime = Date.now();
    this.files = new Map();       // path -> FileNode
    this.packages = new Map();    // name -> PackageNode
    this.judgments = new Map();   // path -> Judgment
    this.dependencies = new Map(); // path -> Set<path>
    this.reverseDeps = new Map(); // path -> Set<path> (who imports me)
    this.issues = [];
    this.recommendations = [];
  }

  elapsed() {
    return ((Date.now() - this.startTime) / 1000).toFixed(1);
  }
}

/**
 * File analysis node
 */
class FileNode {
  constructor(filePath, rootDir) {
    this.path = path.relative(rootDir, filePath);
    this.fullPath = filePath;
    this.lines = 0;
    this.size = 0;
    this.imports = [];
    this.exports = [];
    this.package = null;
    this.isTest = false;
    this.isIndex = false;
    this.isExample = false;
    this.metrics = {};
  }
}

/**
 * Judgment result
 */
class Judgment {
  constructor(filePath) {
    this.path = filePath;
    this.verdict = Verdict.REVIEW;
    this.confidence = 0;
    this.reasons = [];
    this.score = 0;
    this.dimensions = {};
  }

  addReason(reason, weight = 1) {
    this.reasons.push({ text: reason, weight });
  }
}

/**
 * Ralph Loop - Burn Cartographer
 */
class BurnCartographer {
  constructor(options = {}) {
    this.rootDir = options.rootDir || process.cwd();
    this.state = new RalphState();
    this.verbose = options.verbose || false;
    this.maxIterations = options.maxIterations || 10;
    this.outputPath = options.outputPath || './ralph-burn-report.json';
  }

  /**
   * Main loop entry
   */
  async run() {
    this.banner();

    try {
      // Phase 1: Cartograph
      await this.phase1_cartograph();

      // Phase 2: Analyze relationships
      await this.phase2_relationships();

      // Phase 3: Judge each file
      await this.phase3_judge();

      // Phase 4: Synthesize recommendations
      await this.phase4_synthesize();

      // Phase 5: Output report
      await this.phase5_report();

    } catch (e) {
      console.error('\n*GROWL* Ralph encountered an error:', e.message);
      throw e;
    }

    this.summary();
  }

  banner() {
    console.log('');
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('🐕 RALPH LOOP: Burn Cartographer');
    console.log('   "Ralph never lets go" - Autonomous codebase judgment');
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('');
  }

  log(msg) {
    if (this.verbose) console.log('   ' + msg);
  }

  /**
   * Phase 1: Cartograph the codebase
   */
  async phase1_cartograph() {
    console.log('── PHASE 1: Cartographing ─────────────────────────────────────');

    const walk = (dir, depth = 0) => {
      if (depth > 10) return;

      try {
        const items = fs.readdirSync(dir);

        for (const item of items) {
          if (item.startsWith('.') || item === 'node_modules' || item === 'dist' || item === 'coverage') continue;

          const fullPath = path.join(dir, item);
          const stat = fs.statSync(fullPath);

          if (stat.isDirectory()) {
            walk(fullPath, depth + 1);
          } else if (item.endsWith('.js') || item.endsWith('.mjs')) {
            const node = this.analyzeFile(fullPath);
            this.state.files.set(node.path, node);
          }
        }
      } catch (e) {
        // Ignore permission errors
      }
    };

    walk(this.rootDir);

    // Identify packages
    this.identifyPackages();

    console.log(`   ✓ ${this.state.files.size} files mapped`);
    console.log(`   ✓ ${this.state.packages.size} packages identified`);
    console.log('');
  }

  /**
   * Analyze a single file
   */
  analyzeFile(filePath) {
    const node = new FileNode(filePath, this.rootDir);

    try {
      const content = fs.readFileSync(filePath, 'utf8');
      const stat = fs.statSync(filePath);

      node.lines = content.split('\n').length;
      node.size = stat.size;
      node.imports = this.extractImports(content);
      node.exports = this.extractExports(content);
      node.isTest = node.path.includes('test') || node.path.includes('.test.');
      node.isIndex = node.path.endsWith('index.js');
      node.isExample = node.path.includes('examples');

      // Extract package name
      const pkgMatch = node.path.match(/^packages\/([^\/]+)/);
      if (pkgMatch) node.package = pkgMatch[1];

      // Compute metrics
      node.metrics = {
        complexity: this.estimateComplexity(content),
        importCount: node.imports.length,
        exportCount: node.exports.length,
        commentRatio: this.commentRatio(content),
      };

    } catch (e) {
      // File read error
    }

    return node;
  }

  extractImports(content) {
    const imports = [];
    const importRegex = /import\s+(?:(?:\{[^}]+\}|\*\s+as\s+\w+|\w+)\s+from\s+)?['"]([^'"]+)['"]/g;
    const requireRegex = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g;

    let match;
    while ((match = importRegex.exec(content)) !== null) imports.push(match[1]);
    while ((match = requireRegex.exec(content)) !== null) {
      if (!imports.includes(match[1])) imports.push(match[1]);
    }

    return imports;
  }

  extractExports(content) {
    const exports = [];
    const exportRegex = /export\s+(?:const|let|var|function|class|async\s+function)\s+(\w+)/g;

    let match;
    while ((match = exportRegex.exec(content)) !== null) exports.push(match[1]);
    if (/export\s+default/.test(content)) exports.push('default');

    return exports;
  }

  estimateComplexity(content) {
    // Simple cyclomatic complexity estimate
    const patterns = [
      /\bif\b/g, /\belse\b/g, /\bfor\b/g, /\bwhile\b/g,
      /\bswitch\b/g, /\bcase\b/g, /\bcatch\b/g, /\?\?/g, /\?\./g,
      /&&/g, /\|\|/g, /\?[^:]/g
    ];

    let complexity = 1;
    for (const pattern of patterns) {
      const matches = content.match(pattern);
      if (matches) complexity += matches.length;
    }

    return complexity;
  }

  commentRatio(content) {
    const lines = content.split('\n');
    const commentLines = lines.filter(l => l.trim().startsWith('//') || l.trim().startsWith('*')).length;
    return lines.length > 0 ? commentLines / lines.length : 0;
  }

  identifyPackages() {
    const packages = new Set();

    for (const [filePath, node] of this.state.files) {
      if (node.package) packages.add(node.package);
    }

    for (const pkg of packages) {
      const pkgFiles = [...this.state.files.values()].filter(f => f.package === pkg);
      const totalLines = pkgFiles.reduce((sum, f) => sum + f.lines, 0);

      this.state.packages.set(pkg, {
        name: pkg,
        files: pkgFiles.length,
        lines: totalLines,
      });
    }
  }

  /**
   * Phase 2: Analyze relationships
   */
  async phase2_relationships() {
    console.log('── PHASE 2: Mapping Relationships ─────────────────────────────');

    // Build dependency graph
    for (const [filePath, node] of this.state.files) {
      const deps = new Set();

      for (const imp of node.imports) {
        // Resolve relative imports
        if (imp.startsWith('.')) {
          const dir = path.dirname(node.fullPath);
          let resolved = path.relative(this.rootDir, path.resolve(dir, imp));
          if (!resolved.endsWith('.js')) resolved += '.js';
          deps.add(resolved);
        }
      }

      this.state.dependencies.set(filePath, deps);

      // Build reverse deps
      for (const dep of deps) {
        if (!this.state.reverseDeps.has(dep)) {
          this.state.reverseDeps.set(dep, new Set());
        }
        this.state.reverseDeps.get(dep).add(filePath);
      }
    }

    // Find orphans (no reverse deps)
    let orphans = 0;
    for (const [filePath, node] of this.state.files) {
      const importers = this.state.reverseDeps.get(filePath);
      if (!importers || importers.size === 0) {
        if (!node.isIndex && !node.isTest && !node.isExample &&
            !filePath.includes('cli') && !filePath.includes('server')) {
          orphans++;
        }
      }
    }

    // Find circular deps
    const cycles = this.findCycles();

    console.log(`   ✓ ${this.state.dependencies.size} dependency links`);
    console.log(`   ✓ ${orphans} potential orphans`);
    console.log(`   ✓ ${cycles.length} circular dependencies`);
    console.log('');

    if (cycles.length > 0) {
      this.state.issues.push({
        type: 'CIRCULAR_DEPS',
        count: cycles.length,
        items: cycles.slice(0, 5),
      });
    }
  }

  findCycles() {
    const cycles = [];
    const visited = new Set();
    const stack = new Set();

    const dfs = (node, path) => {
      if (stack.has(node)) {
        cycles.push([...path, node]);
        return;
      }
      if (visited.has(node)) return;

      visited.add(node);
      stack.add(node);

      const deps = this.state.dependencies.get(node);
      if (deps) {
        for (const dep of deps) {
          dfs(dep, [...path, node]);
        }
      }

      stack.delete(node);
    };

    for (const filePath of this.state.files.keys()) {
      dfs(filePath, []);
    }

    return cycles;
  }

  /**
   * Phase 3: Judge each file
   */
  async phase3_judge() {
    console.log('── PHASE 3: Judging Files ─────────────────────────────────────');

    let judged = 0;

    for (const [filePath, node] of this.state.files) {
      const judgment = this.judgeFile(node);
      this.state.judgments.set(filePath, judgment);
      judged++;

      if (judged % 100 === 0) {
        process.stdout.write(`   Judging... ${judged}/${this.state.files.size}\r`);
      }
    }

    console.log(`   ✓ ${judged} files judged                    `);

    // Count by verdict
    const counts = {};
    for (const j of this.state.judgments.values()) {
      counts[j.verdict] = (counts[j.verdict] || 0) + 1;
    }

    for (const [verdict, count] of Object.entries(counts).sort((a, b) => b[1] - a[1])) {
      const style = VerdictStyle[verdict] || '';
      console.log(`   ${style}${verdict}${RESET}: ${count}`);
    }
    console.log('');
  }

  /**
   * Judge a single file
   */
  judgeFile(node) {
    const j = new Judgment(node.path);

    // Gather signals
    const importers = this.state.reverseDeps.get(node.path);
    const importerCount = importers ? importers.size : 0;

    // === DIMENSION 1: Utility (is it used?) ===
    if (importerCount === 0 && !node.isIndex && !node.isTest && !node.isExample) {
      j.addReason('Zero importers - orphan candidate', 3);
      j.dimensions.utility = 0.2;
    } else if (importerCount < 2) {
      j.dimensions.utility = 0.5;
    } else {
      j.dimensions.utility = Math.min(1, importerCount / 10);
    }

    // === DIMENSION 2: Size (is it right-sized?) ===
    if (node.lines > 1000) {
      j.addReason(`Giant file: ${node.lines} lines`, 3);
      j.dimensions.size = 0.2;
    } else if (node.lines > 500) {
      j.addReason(`Large file: ${node.lines} lines`, 2);
      j.dimensions.size = 0.4;
    } else if (node.lines < 20) {
      j.addReason('Very small file', 1);
      j.dimensions.size = 0.7;
    } else {
      j.dimensions.size = 1;
    }

    // === DIMENSION 3: Complexity ===
    const complexity = node.metrics.complexity;
    if (complexity > 100) {
      j.addReason(`High complexity: ${complexity}`, 2);
      j.dimensions.complexity = 0.3;
    } else if (complexity > 50) {
      j.dimensions.complexity = 0.6;
    } else {
      j.dimensions.complexity = 1;
    }

    // === DIMENSION 4: Coupling (too many deps?) ===
    if (node.imports.length > 20) {
      j.addReason(`Hotspot: ${node.imports.length} imports`, 3);
      j.dimensions.coupling = 0.2;
    } else if (node.imports.length > 13) {
      j.addReason(`Many imports: ${node.imports.length}`, 1);
      j.dimensions.coupling = 0.5;
    } else {
      j.dimensions.coupling = 1;
    }

    // === DIMENSION 5: Category ===
    if (node.isTest) {
      j.dimensions.category = 0.8; // Tests are valuable
    } else if (node.isExample) {
      j.dimensions.category = 0.4; // Examples are optional
      j.addReason('Example file - consider moving to docs', 1);
    } else if (node.isIndex) {
      j.dimensions.category = 1; // Index files are structural
    } else {
      j.dimensions.category = 0.7;
    }

    // === COMPUTE VERDICT ===
    const avgScore = Object.values(j.dimensions).reduce((a, b) => a + b, 0) / 5;
    j.score = avgScore;

    // Confidence capped at φ⁻¹
    j.confidence = Math.min(PHI_INV, avgScore);

    // Determine verdict based on signals
    if (j.dimensions.utility < 0.3 && !node.isTest) {
      j.verdict = Verdict.DELETE;
    } else if (j.dimensions.size < 0.3) {
      j.verdict = Verdict.SPLIT;
    } else if (j.dimensions.coupling < 0.3) {
      j.verdict = Verdict.SIMPLIFY;
    } else if (node.isExample) {
      j.verdict = Verdict.REVIEW;
    } else if (avgScore > 0.8) {
      j.verdict = Verdict.KEEP;
    } else {
      j.verdict = Verdict.REVIEW;
    }

    return j;
  }

  /**
   * Phase 4: Synthesize recommendations
   */
  async phase4_synthesize() {
    console.log('── PHASE 4: Synthesizing Recommendations ──────────────────────');

    // Group by verdict and sort by confidence
    const byVerdict = {};
    for (const [path, j] of this.state.judgments) {
      if (!byVerdict[j.verdict]) byVerdict[j.verdict] = [];
      byVerdict[j.verdict].push({ path, ...j });
    }

    // Generate recommendations
    const recommendations = [];

    // DELETE candidates (high priority)
    if (byVerdict[Verdict.DELETE]) {
      const candidates = byVerdict[Verdict.DELETE]
        .sort((a, b) => b.confidence - a.confidence)
        .slice(0, 20);

      const totalLines = candidates.reduce((sum, c) => {
        const node = this.state.files.get(c.path);
        return sum + (node?.lines || 0);
      }, 0);

      recommendations.push({
        action: 'DELETE',
        priority: 'HIGH',
        candidates: candidates.map(c => ({
          path: c.path,
          lines: this.state.files.get(c.path)?.lines || 0,
          reasons: c.reasons.map(r => r.text),
          confidence: (c.confidence * 100).toFixed(0) + '%',
        })),
        impact: `${totalLines} lines removable`,
      });
    }

    // SPLIT candidates
    if (byVerdict[Verdict.SPLIT]) {
      recommendations.push({
        action: 'SPLIT',
        priority: 'MEDIUM',
        candidates: byVerdict[Verdict.SPLIT]
          .sort((a, b) => {
            const nodeA = this.state.files.get(a.path);
            const nodeB = this.state.files.get(b.path);
            return (nodeB?.lines || 0) - (nodeA?.lines || 0);
          })
          .slice(0, 10)
          .map(c => ({
            path: c.path,
            lines: this.state.files.get(c.path)?.lines || 0,
            reasons: c.reasons.map(r => r.text),
          })),
        impact: 'Improved maintainability',
      });
    }

    // SIMPLIFY candidates
    if (byVerdict[Verdict.SIMPLIFY]) {
      recommendations.push({
        action: 'SIMPLIFY',
        priority: 'MEDIUM',
        candidates: byVerdict[Verdict.SIMPLIFY]
          .slice(0, 10)
          .map(c => ({
            path: c.path,
            imports: this.state.files.get(c.path)?.imports.length || 0,
            reasons: c.reasons.map(r => r.text),
          })),
        impact: 'Reduced coupling',
      });
    }

    this.state.recommendations = recommendations;

    console.log(`   ✓ ${recommendations.length} recommendation groups`);
    console.log('');
  }

  /**
   * Phase 5: Generate report
   */
  async phase5_report() {
    console.log('── PHASE 5: Generating Report ─────────────────────────────────');

    const report = {
      meta: {
        timestamp: new Date().toISOString(),
        duration: this.state.elapsed() + 's',
        rootDir: this.rootDir,
        version: '1.0.0',
      },
      summary: {
        totalFiles: this.state.files.size,
        totalLines: [...this.state.files.values()].reduce((s, f) => s + f.lines, 0),
        totalPackages: this.state.packages.size,
        verdicts: {},
      },
      packages: [...this.state.packages.values()],
      issues: this.state.issues,
      recommendations: this.state.recommendations,
      judgments: {},
    };

    // Count verdicts
    for (const j of this.state.judgments.values()) {
      report.summary.verdicts[j.verdict] = (report.summary.verdicts[j.verdict] || 0) + 1;
    }

    // Include top judgments (not KEEP)
    for (const [path, j] of this.state.judgments) {
      if (j.verdict !== Verdict.KEEP && j.verdict !== Verdict.REVIEW) {
        report.judgments[path] = {
          verdict: j.verdict,
          confidence: j.confidence,
          score: j.score,
          reasons: j.reasons.map(r => r.text),
          dimensions: j.dimensions,
        };
      }
    }

    // Write report
    fs.writeFileSync(this.outputPath, JSON.stringify(report, null, 2));

    console.log(`   ✓ Report saved: ${this.outputPath}`);
    console.log('');
  }

  summary() {
    const report = JSON.parse(fs.readFileSync(this.outputPath, 'utf8'));

    console.log('═══════════════════════════════════════════════════════════════');
    console.log('🐕 RALPH COMPLETE - Burn Cartography Results');
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('');
    console.log(`   Duration: ${report.meta.duration}`);
    console.log(`   Files: ${report.summary.totalFiles}`);
    console.log(`   Lines: ${report.summary.totalLines.toLocaleString()}`);
    console.log('');
    console.log('── VERDICTS ───────────────────────────────────────────────────');

    for (const [verdict, count] of Object.entries(report.summary.verdicts).sort((a, b) => b[1] - a[1])) {
      const style = VerdictStyle[verdict] || '';
      const bar = '█'.repeat(Math.ceil(count / 20));
      console.log(`   ${style}${verdict.padEnd(10)}${RESET} ${bar} ${count}`);
    }

    console.log('');
    console.log('── TOP RECOMMENDATIONS ────────────────────────────────────────');

    for (const rec of report.recommendations) {
      console.log(`   ${VerdictStyle[rec.action] || ''}[${rec.action}]${RESET} ${rec.priority} priority`);
      console.log(`   Impact: ${rec.impact}`);
      console.log(`   Candidates: ${rec.candidates.length}`);
      rec.candidates.slice(0, 3).forEach(c => {
        console.log(`     • ${c.path}`);
      });
      console.log('');
    }

    console.log('═══════════════════════════════════════════════════════════════');
    console.log('   φ max confidence: 61.8% - "φ distrusts φ"');
    console.log('   Review recommendations before executing burns.');
    console.log('═══════════════════════════════════════════════════════════════');
    console.log('');
  }
}

// === MAIN ===
const cartographer = new BurnCartographer({
  rootDir: process.cwd(),
  verbose: process.argv.includes('--verbose'),
  outputPath: './ralph-burn-report.json',
});

cartographer.run().catch(e => {
  console.error('Ralph loop failed:', e);
  process.exit(1);
});
