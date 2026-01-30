/**
 * CYNIC Self-Monitor
 *
 * "Connais-toi toi-même, puis vérifie" - κυνικός
 *
 * Auto-tracks CYNIC's own development state:
 * - Package test status
 * - Feature completion (derived from tests)
 * - Integration health (hooks, skills, MCP)
 * - Roadmap auto-generation
 *
 * Unlike static ROADMAP.md, this is live truth.
 *
 * @module @cynic/scripts/self-monitor
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');
const { execFileSync, spawnSync } = require('child_process');

// Import centralized color system
let colors;
try {
  colors = require('./colors.cjs');
} catch {
  colors = null;
}

// φ Constants
const PHI = 1.618033988749895;
const PHI_INV = 1 / PHI; // 0.618

// Use centralized ANSI or fallback
const ANSI = colors?.ANSI || {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
  red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m', white: '\x1b[37m',
  brightRed: '\x1b[91m', brightGreen: '\x1b[92m', brightYellow: '\x1b[93m',
  brightBlue: '\x1b[94m', brightMagenta: '\x1b[95m', brightCyan: '\x1b[96m',
  brightWhite: '\x1b[97m',
};

let useColor = true;
const c = colors?.colorize || ((color, text) => useColor ? `${color}${text}${ANSI.reset}` : text);

// Paths
const CYNIC_DIR = path.join(os.homedir(), '.cynic');
const SELF_DIR = path.join(CYNIC_DIR, 'self');
const PROJECT_ROOT = path.join(__dirname, '..', '..');

// Package definitions with expected capabilities
const PACKAGES = {
  core: {
    description: 'Constants, axioms, φ timing',
    critical: true,
    features: ['phi-constants', 'axioms', 'timing', 'worlds', 'identity-module']
  },
  protocol: {
    description: 'PoJ, Merkle, gossip, consensus',
    critical: true,
    features: ['poj', 'merkle', 'gossip', 'consensus', 'validation']
  },
  persistence: {
    description: 'PostgreSQL, Redis, DAG',
    critical: true,
    features: ['postgres', 'redis', 'dag', 'repositories', 'graph']
  },
  anchor: {
    description: 'Solana block anchoring',
    critical: false,
    features: ['anchoring', 'queue', 'merkle-proofs', 'devnet']
  },
  burns: {
    description: 'Token burn verification',
    critical: false,
    features: ['verification', 'on-chain', 'tracking']
  },
  identity: {
    description: 'E-Score, keys, reputation',
    critical: true,
    features: ['e-score-7d', 'key-manager', 'node-identity', 'reputation-graph']
  },
  emergence: {
    description: 'Consciousness, patterns',
    critical: false,
    features: ['consciousness', 'patterns', 'dimensions']
  },
  node: {
    description: 'Full node implementation',
    critical: true,
    features: ['judge-25d', 'brain', 'cli', 'services']
  },
  mcp: {
    description: 'MCP server for Claude',
    critical: true,
    features: ['brain-tools', 'memory-tools', 'graph-tools', 'poj-tools']
  },
  holdex: {
    description: 'Token K-Score integration',
    critical: false,
    features: ['k-score', 'token-analysis']
  },
  gasdf: {
    description: 'Gasless burns',
    critical: false,
    features: ['delegation', 'gasless']
  },
  zk: {
    description: 'Zero-knowledge proofs',
    critical: false,
    features: ['noir-circuits', 'proofs']
  }
};

// ═══════════════════════════════════════════════════════════════════════════
// SAFE EXECUTION (no shell injection)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Run tests for a package using node --test directly
 * (npm test --workspace doesn't capture output properly on Windows)
 */
function runPackageTest(pkgName) {
  // Validate package name (alphanumeric only)
  if (!/^[a-z0-9-]+$/.test(pkgName)) {
    throw new Error(`Invalid package name: ${pkgName}`);
  }

  const pkgPath = path.join(PROJECT_ROOT, 'packages', pkgName);
  const testDir = path.join(pkgPath, 'test');

  // Check if test directory exists
  if (!fs.existsSync(testDir)) {
    return { stdout: '', stderr: 'No test directory', status: 1 };
  }

  // Find test files (use relative paths since cwd is set to pkgPath)
  let testFiles = fs.readdirSync(testDir)
    .filter(f => f.endsWith('.test.js'))
    .map(f => path.join('test', f));

  if (testFiles.length === 0) {
    return { stdout: '', stderr: 'No test files', status: 1 };
  }

  // For large packages with many test files, sample to avoid timeout
  // node package has 23 files with 600+ tests - even 10 files can timeout
  const MAX_TEST_FILES = 5;
  let sampled = false;
  let originalCount = testFiles.length;

  if (testFiles.length > MAX_TEST_FILES) {
    // Sample evenly distributed files for representative coverage
    const step = Math.floor(testFiles.length / MAX_TEST_FILES);
    testFiles = testFiles.filter((_, i) => i % step === 0).slice(0, MAX_TEST_FILES);
    sampled = true;
  }

  // Run node --test directly
  // Use 180s timeout for large packages
  const timeout = ['node', 'mcp'].includes(pkgName) ? 180000 : 120000;

  const result = spawnSync('node', ['--test', ...testFiles], {
    cwd: pkgPath,
    timeout,
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, NODE_OPTIONS: '' }, // Clear NODE_OPTIONS to avoid conflicts
  });

  // If we sampled, extrapolate the test counts
  if (sampled && result.status === 0) {
    const output = result.stdout + result.stderr;
    const testsMatch = output.match(/ℹ tests (\d+)/);
    const passMatch = output.match(/ℹ pass (\d+)/);

    if (testsMatch && passMatch) {
      const sampledTests = parseInt(testsMatch[1], 10);
      const sampledPass = parseInt(passMatch[1], 10);
      const ratio = originalCount / testFiles.length;

      // Extrapolate and mark with ~ to indicate estimate
      result._extrapolated = true;
      result._originalFileCount = originalCount;
      result._sampledFileCount = testFiles.length;
      result._estimatedTests = Math.round(sampledTests * ratio);
      result._estimatedPass = Math.round(sampledPass * ratio);
    }
  }

  return {
    stdout: result.stdout || '',
    stderr: result.stderr || '',
    status: result.status,
    _extrapolated: result._extrapolated,
    _originalFileCount: result._originalFileCount,
    _sampledFileCount: result._sampledFileCount,
    _estimatedTests: result._estimatedTests,
    _estimatedPass: result._estimatedPass,
  };
}

/**
 * Check MCP health safely
 */
function checkMcpHealth() {
  const result = spawnSync('curl', [
    '-s',
    '--max-time', '3',
    'https://cynic-mcp.onrender.com/health'
  ], {
    encoding: 'utf8',
    timeout: 5000,
  });

  if (result.status !== 0) return 'unreachable';

  try {
    const data = JSON.parse(result.stdout);
    return data.status || 'healthy';
  } catch {
    return 'unknown';
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// PACKAGE SCANNING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Scan a single package for test status
 */
function scanPackage(pkgName) {
  const pkgPath = path.join(PROJECT_ROOT, 'packages', pkgName);

  if (!fs.existsSync(pkgPath)) {
    return { name: pkgName, exists: false, tests: 0, pass: 0, fail: 0 };
  }

  try {
    const result = runPackageTest(pkgName);
    const output = result.stdout + result.stderr;

    // Parse node --test output format
    const testsMatch = output.match(/ℹ tests (\d+)/);
    const passMatch = output.match(/ℹ pass (\d+)/);
    const failMatch = output.match(/ℹ fail (\d+)/);

    // Use extrapolated values if available (for sampled large packages)
    let tests = result._estimatedTests || parseInt(testsMatch?.[1] || '0', 10);
    let pass = result._estimatedPass || parseInt(passMatch?.[1] || '0', 10);
    const fail = parseInt(failMatch?.[1] || '0', 10);

    // Package is healthy if: has tests, all pass, no failures, exit code 0
    const healthy = tests > 0 && fail === 0 && result.status === 0;

    return {
      name: pkgName,
      exists: true,
      tests,
      pass,
      fail,
      description: PACKAGES[pkgName]?.description || '',
      critical: PACKAGES[pkgName]?.critical || false,
      healthy,
      sampled: result._extrapolated || false,
    };
  } catch (error) {
    return {
      name: pkgName,
      exists: true,
      tests: 0,
      pass: 0,
      fail: 1,
      error: String(error.message || error).slice(0, 100),
      critical: PACKAGES[pkgName]?.critical || false,
      healthy: false,
    };
  }
}

/**
 * Scan all packages
 */
function scanAllPackages() {
  const packages = {};
  let totalTests = 0;
  let totalPass = 0;
  let totalFail = 0;

  for (const pkgName of Object.keys(PACKAGES)) {
    const result = scanPackage(pkgName);
    packages[pkgName] = result;
    totalTests += result.tests;
    totalPass += result.pass;
    totalFail += result.fail;
  }

  return {
    packages,
    summary: {
      total: Object.keys(packages).length,
      healthy: Object.values(packages).filter(p => p.healthy).length,
      tests: totalTests,
      pass: totalPass,
      fail: totalFail,
      coverage: totalTests > 0 ? totalPass / totalTests : 0,
    },
    scannedAt: Date.now(),
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION SCANNING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Scan Claude Code integration (hooks, skills, agents)
 */
function scanIntegrations() {
  const claudeDir = path.join(PROJECT_ROOT, '.claude');

  // Count hooks (both .js and .cjs)
  const hooksDir = path.join(PROJECT_ROOT, 'scripts', 'hooks');
  const hooks = fs.existsSync(hooksDir)
    ? fs.readdirSync(hooksDir).filter(f => f.endsWith('.js') || f.endsWith('.cjs'))
    : [];

  // Count lib modules
  const libDir = path.join(PROJECT_ROOT, 'scripts', 'lib');
  const libModules = fs.existsSync(libDir)
    ? fs.readdirSync(libDir).filter(f => f.endsWith('.cjs'))
    : [];

  // Count skills
  const skillsDir = path.join(claudeDir, 'skills');
  const skills = fs.existsSync(skillsDir)
    ? fs.readdirSync(skillsDir).filter(f => {
        const skillPath = path.join(skillsDir, f, 'SKILL.md');
        return fs.existsSync(skillPath);
      })
    : [];

  // Count agents
  const agentsDir = path.join(claudeDir, 'agents');
  const agents = fs.existsSync(agentsDir)
    ? fs.readdirSync(agentsDir).filter(f => f.endsWith('.md'))
    : [];

  // Check MCP connection
  const mcpStatus = checkMcpHealth();

  return {
    hooks: {
      count: hooks.length,
      list: hooks.map(h => h.replace(/\.(c?js)$/, '')),
    },
    libModules: {
      count: libModules.length,
    },
    skills: {
      count: skills.length,
      list: skills,
    },
    agents: {
      count: agents.length,
      list: agents.map(a => a.replace('.md', '')),
    },
    mcp: {
      status: mcpStatus,
    },
    scannedAt: Date.now(),
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FEATURE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Detect implemented features from test names
 */
function detectFeatures(packages) {
  const features = {
    implemented: [],
    partial: [],
    missing: [],
  };

  for (const [pkgName, pkg] of Object.entries(packages)) {
    const expected = PACKAGES[pkgName]?.features || [];

    for (const feature of expected) {
      if (pkg.healthy && pkg.pass > 0) {
        features.implemented.push(`${pkgName}:${feature}`);
      } else if (pkg.pass > 0) {
        features.partial.push(`${pkgName}:${feature}`);
      } else {
        features.missing.push(`${pkgName}:${feature}`);
      }
    }
  }

  return features;
}

// ═══════════════════════════════════════════════════════════════════════════
// ROADMAP GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Generate roadmap status from scan results
 */
function generateRoadmap(packageScan, integrations) {
  const phases = {
    'Phase 1: Core Foundation': {
      status: 'complete',
      items: []
    },
    'Phase 2: Integration': {
      status: 'in_progress',
      items: []
    },
    'Phase 3: External': {
      status: 'planned',
      items: []
    }
  };

  // Phase 1: Core packages
  const corePackages = ['core', 'protocol', 'persistence', 'identity', 'node', 'mcp'];
  for (const pkg of corePackages) {
    const data = packageScan.packages[pkg];
    phases['Phase 1: Core Foundation'].items.push({
      name: `@cynic/${pkg}`,
      status: data?.healthy ? 'complete' : 'in_progress',
      tests: `${data?.pass || 0}/${data?.tests || 0}`,
    });
  }

  // Check if Phase 1 complete
  const phase1Complete = corePackages.every(p => packageScan.packages[p]?.healthy);
  phases['Phase 1: Core Foundation'].status = phase1Complete ? 'complete' : 'in_progress';

  // Phase 2: Integrations
  phases['Phase 2: Integration'].items = [
    {
      name: 'Hooks',
      status: integrations.hooks.count >= 5 ? 'complete' : 'in_progress',
      count: integrations.hooks.count,
    },
    {
      name: 'Skills',
      status: integrations.skills.count >= 10 ? 'complete' : 'in_progress',
      count: integrations.skills.count,
    },
    {
      name: 'Agents',
      status: integrations.agents.count >= 10 ? 'complete' : 'in_progress',
      count: integrations.agents.count,
    },
    {
      name: 'MCP Server',
      status: integrations.mcp.status === 'healthy' ? 'complete' : 'warning',
      mcpStatus: integrations.mcp.status,
    },
  ];

  // Check if Phase 2 complete
  const phase2Complete = phases['Phase 2: Integration'].items.every(
    item => item.status === 'complete'
  );
  phases['Phase 2: Integration'].status = phase2Complete ? 'complete' : 'in_progress';

  // Phase 3: External
  const externalPackages = ['anchor', 'burns', 'holdex', 'gasdf', 'zk'];
  for (const pkg of externalPackages) {
    const data = packageScan.packages[pkg];
    phases['Phase 3: External'].items.push({
      name: `@cynic/${pkg}`,
      status: data?.healthy ? 'ready' : 'in_progress',
      tests: `${data?.pass || 0}/${data?.tests || 0}`,
    });
  }

  return {
    phases,
    generatedAt: Date.now(),
    summary: {
      phase1: phases['Phase 1: Core Foundation'].status,
      phase2: phases['Phase 2: Integration'].status,
      phase3: phases['Phase 3: External'].status,
    }
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Ensure self-monitoring directory exists
 */
function ensureSelfDir() {
  if (!fs.existsSync(SELF_DIR)) {
    fs.mkdirSync(SELF_DIR, { recursive: true });
  }
}

/**
 * Save state to file
 */
function saveState(name, data) {
  ensureSelfDir();
  const filepath = path.join(SELF_DIR, `${name}.json`);
  fs.writeFileSync(filepath, JSON.stringify(data, null, 2));
}

/**
 * Load state from file
 */
function loadState(name) {
  const filepath = path.join(SELF_DIR, `${name}.json`);
  if (!fs.existsSync(filepath)) return null;

  try {
    return JSON.parse(fs.readFileSync(filepath, 'utf8'));
  } catch {
    return null;
  }
}

/**
 * Run full self-scan
 */
function fullScan(runTests = true) {
  const result = {
    timestamp: Date.now(),
    packages: null,
    integrations: null,
    features: null,
    roadmap: null,
  };

  // Package scan (optionally skip tests for speed)
  if (runTests) {
    result.packages = scanAllPackages();
    saveState('packages', result.packages);
  } else {
    result.packages = loadState('packages') || { packages: {}, summary: {} };
  }

  // Integration scan
  result.integrations = scanIntegrations();
  saveState('integrations', result.integrations);

  // Feature detection
  result.features = detectFeatures(result.packages.packages);
  saveState('features', result.features);

  // Roadmap generation
  result.roadmap = generateRoadmap(result.packages, result.integrations);
  saveState('roadmap', result.roadmap);

  return result;
}

// ═══════════════════════════════════════════════════════════════════════════
// DISPLAY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Format self-status for display
 * @param {Object} scan - Scan results
 * @param {boolean} enableColor - Whether to use ANSI colors
 */
function formatStatus(scan, enableColor = true) {
  useColor = enableColor;
  const lines = [];

  const header = '═══════════════════════════════════════════════════════════════════';
  lines.push('');
  lines.push(c(ANSI.cyan, '╔' + header + '╗'));
  lines.push(c(ANSI.cyan, '║') + c(ANSI.bold + ANSI.brightCyan, '            🐕 CYNIC SELF-STATUS (Auto-generated)                  ') + c(ANSI.cyan, '║'));
  lines.push(c(ANSI.cyan, '╠' + header + '╣'));

  // Package summary
  const pkg = scan.packages?.summary || {};
  const pkgHealthy = (pkg.healthy || 0) === (pkg.total || 0);
  const pkgColor = pkgHealthy ? ANSI.brightGreen : ANSI.brightYellow;
  const coveragePct = ((pkg.coverage || 0) * 100).toFixed(1);
  const coverageColor = coveragePct >= 90 ? ANSI.brightGreen : (coveragePct >= 70 ? ANSI.yellow : ANSI.brightRed);

  lines.push(c(ANSI.cyan, '║') + '                                                                   ' + c(ANSI.cyan, '║'));
  lines.push(c(ANSI.cyan, '║') + `  ${c(ANSI.brightWhite, 'PACKAGES:')} ${c(pkgColor, `${pkg.healthy || 0}/${pkg.total || 0}`)} healthy                                          `);
  lines.push(c(ANSI.cyan, '║') + `  ${c(ANSI.brightWhite, 'TESTS:')} ${c(ANSI.brightCyan, `${pkg.pass || 0}/${pkg.tests || 0}`)} passing (${c(coverageColor, coveragePct + '%')})                              `);
  lines.push(c(ANSI.cyan, '║') + '                                                                   ' + c(ANSI.cyan, '║'));

  // Package details
  for (const [name, data] of Object.entries(scan.packages?.packages || {})) {
    const icon = data.healthy ? c(ANSI.brightGreen, '✅') : (data.fail > 0 ? c(ANSI.brightRed, '❌') : c(ANSI.dim, '⚪'));
    const critical = data.critical ? c(ANSI.brightYellow, '*') : ' ';
    const sampled = data.sampled ? c(ANSI.dim, '~') : ' ';
    const testColor = data.healthy ? ANSI.brightGreen : (data.fail > 0 ? ANSI.brightRed : ANSI.dim);
    const nameColor = data.critical ? ANSI.brightWhite : ANSI.white;
    lines.push(c(ANSI.cyan, '║') + `  ${icon}${critical} ${c(nameColor, name.padEnd(12))} ${sampled}${c(testColor, String(data.pass || 0).padStart(3) + '/' + String(data.tests || 0).padStart(3))} tests           `);
  }

  lines.push(c(ANSI.cyan, '║') + '                                                                   ' + c(ANSI.cyan, '║'));
  lines.push(c(ANSI.cyan, '╠' + header + '╣'));

  // Integrations
  const int = scan.integrations || {};
  const hooksOk = (int.hooks?.count || 0) >= 5;
  const skillsOk = (int.skills?.count || 0) >= 10;
  const agentsOk = (int.agents?.count || 0) >= 10;
  const mcpOk = int.mcp?.status === 'healthy';

  lines.push(c(ANSI.cyan, '║') + `  ${c(ANSI.brightWhite, 'HOOKS:')} ${c(hooksOk ? ANSI.brightGreen : ANSI.yellow, int.hooks?.count || 0)}   ${c(ANSI.brightWhite, 'SKILLS:')} ${c(skillsOk ? ANSI.brightGreen : ANSI.yellow, int.skills?.count || 0)}   ${c(ANSI.brightWhite, 'AGENTS:')} ${c(agentsOk ? ANSI.brightGreen : ANSI.yellow, int.agents?.count || 0)}   ${c(ANSI.brightWhite, 'LIB:')} ${c(ANSI.brightCyan, int.libModules?.count || 0)}         `);
  lines.push(c(ANSI.cyan, '║') + `  ${c(ANSI.brightWhite, 'MCP:')} ${c(mcpOk ? ANSI.brightGreen : ANSI.yellow, int.mcp?.status || 'unknown')}                                                    `);
  lines.push(c(ANSI.cyan, '║') + '                                                                   ' + c(ANSI.cyan, '║'));

  // Roadmap
  const roadmap = scan.roadmap?.summary || {};
  const p1 = roadmap.phase1 === 'complete' ? c(ANSI.brightGreen, '✅') : c(ANSI.yellow, '🔄');
  const p2 = roadmap.phase2 === 'complete' ? c(ANSI.brightGreen, '✅') : c(ANSI.yellow, '🔄');
  const p3 = roadmap.phase3 === 'complete' ? c(ANSI.brightGreen, '✅') : c(ANSI.dim, '📋');
  lines.push(c(ANSI.cyan, '║') + `  ${c(ANSI.brightWhite, 'ROADMAP:')} ${p1} ${c(ANSI.white, 'Core')}  ${p2} ${c(ANSI.white, 'Integration')}  ${p3} ${c(ANSI.white, 'External')}              `);
  lines.push(c(ANSI.cyan, '║') + '                                                                   ' + c(ANSI.cyan, '║'));

  lines.push(c(ANSI.cyan, '╠' + header + '╣'));
  lines.push(c(ANSI.cyan, '║') + c(ANSI.dim, '  * = critical package   φ⁻¹ = 61.8% max confidence               ') + c(ANSI.cyan, '║'));
  lines.push(c(ANSI.cyan, '╚' + header + '╝'));
  lines.push('');

  return lines.join('\n');
}

/**
 * Get compact status line
 */
function getStatusLine() {
  const packages = loadState('packages');
  const integrations = loadState('integrations');

  if (!packages) return '🐕 CYNIC: No self-scan yet (run /status)';

  const pkg = packages.summary;
  const icon = pkg.healthy === pkg.total ? '✅' : '⚠️';

  return `${icon} CYNIC | Pkgs: ${pkg.healthy}/${pkg.total} | Tests: ${pkg.pass}/${pkg.tests} | MCP: ${integrations?.mcp?.status || '?'}`;
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Scanning
  scanPackage,
  scanAllPackages,
  scanIntegrations,
  detectFeatures,
  generateRoadmap,
  fullScan,

  // State
  saveState,
  loadState,
  ensureSelfDir,

  // Display
  formatStatus,
  getStatusLine,

  // Constants
  PACKAGES,
  SELF_DIR,
  PROJECT_ROOT,
  PHI,
  PHI_INV,
};

// CLI execution
if (require.main === module) {
  const args = process.argv.slice(2);
  const enableColor = !args.includes('--no-color');

  if (args.includes('--quick') || args.includes('-q')) {
    // Quick scan (no tests)
    const scan = fullScan(false);
    console.log(formatStatus(scan, enableColor));
  } else if (args.includes('--status') || args.includes('-s')) {
    console.log(getStatusLine());
  } else if (args.includes('--json') || args.includes('-j')) {
    const scan = fullScan(true);
    console.log(JSON.stringify(scan, null, 2));
  } else {
    // Full scan with tests
    console.log(c(ANSI.dim, '🐕 Running full self-scan (tests included)...'));
    const scan = fullScan(true);
    console.log(formatStatus(scan, enableColor));
  }
}
