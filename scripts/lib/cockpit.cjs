/**
 * CYNIC Cockpit Library
 *
 * "Le cockpit qui voit tout" - Continuous ecosystem awareness
 *
 * Provides:
 * - Real-time ecosystem status
 * - Dependency graph tracking
 * - Proactive alerts
 * - Cross-repo monitoring
 *
 * @module cynic/lib/cockpit
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

// Import core library
const cynic = require('./cynic-core.cjs');

// =============================================================================
// CONSTANTS
// =============================================================================

const COCKPIT_VERSION = '1.0.0';
const SCAN_INTERVAL_MS = cynic.HEARTBEAT_MS; // 61.8 seconds (φ-aligned)
const ALERT_TTL_MS = 3600000; // 1 hour
const MAX_ALERTS = 100;

// ANSI color helpers
const ANSI = {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m',
  red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m', white: '\x1b[37m',
  brightRed: '\x1b[91m', brightGreen: '\x1b[92m', brightYellow: '\x1b[93m',
  brightBlue: '\x1b[94m', brightMagenta: '\x1b[95m', brightCyan: '\x1b[96m',
  brightWhite: '\x1b[97m',
};

let useColor = true;
const c = (color, text) => useColor ? `${color}${text}${ANSI.reset}` : text;

// Known ecosystem repos and their roles
// GRANULARITY: Only repos owned by zeyxx are part of the TRUE ecosystem
const ECOSYSTEM_REPOS = {
  // ═══════════════════════════════════════════════════════════════════════════
  // TRUE CYNIC ECOSYSTEM (owned by zeyxx)
  // ═══════════════════════════════════════════════════════════════════════════
  'CYNIC-new': { role: 'core', critical: true, owner: 'zeyxx', ecosystem: true },
  'GASdf': { role: 'gasless', critical: true, owner: 'zeyxx', ecosystem: true },
  'HolDex': { role: 'kscore', critical: true, owner: 'zeyxx', ecosystem: true },
  'asdf-brain': { role: 'legacy-proto', critical: false, owner: 'zeyxx', ecosystem: true },
  'asdf-manifesto': { role: 'docs', critical: false, owner: 'zeyxx', ecosystem: true },

  // ═══════════════════════════════════════════════════════════════════════════
  // EXTERNAL TOOLS (not part of ecosystem - just in workspace)
  // ═══════════════════════════════════════════════════════════════════════════
  'claude-mem': { role: 'external-tool', critical: false, owner: 'thedotmack', ecosystem: false },
  'framework-kit': { role: 'external-tool', critical: false, owner: 'solana-foundation', ecosystem: false },
  'solana-dev-skill': { role: 'external-tool', critical: false, owner: 'GuiBibeau', ecosystem: false },
};

// Dependency map (who depends on whom)
const KNOWN_DEPENDENCIES = {
  'CYNIC-new': ['GASdf', 'HolDex'],      // CYNIC consumes these APIs
  'GASdf': ['HolDex'],                    // GASdf uses K-Score oracle
  'HolDex': [],                           // Independent
  'asdf-brain': ['CYNIC-new'],            // Legacy depends on new
};

// =============================================================================
// PATHS
// =============================================================================

function getCockpitDir() {
  const dir = path.join(cynic.getDataDir(), 'cockpit');
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  return dir;
}

function getStatusPath() {
  return path.join(getCockpitDir(), 'status.json');
}

function getAlertsPath() {
  return path.join(getCockpitDir(), 'alerts.json');
}

function getDepsPath() {
  return path.join(getCockpitDir(), 'dependencies.json');
}

function getHistoryPath() {
  return path.join(getCockpitDir(), 'history.json');
}

// =============================================================================
// SAFE EXECUTION
// =============================================================================

function safeExec(command, args, options = {}) {
  try {
    return execFileSync(command, args, {
      encoding: 'utf-8',
      timeout: 5000,
      ...options
    }).trim();
  } catch (e) {
    return null;
  }
}

// =============================================================================
// ECOSYSTEM SCANNING
// =============================================================================

/**
 * Deep scan of all ecosystem repos
 * @returns {Object} Full ecosystem status
 */
function scanEcosystem() {
  const workspacesDir = '/workspaces';
  const status = {
    version: COCKPIT_VERSION,
    timestamp: new Date().toISOString(),
    repos: {},
    summary: {
      total: 0,
      healthy: 0,
      warnings: 0,
      critical: 0,
    },
  };

  if (!fs.existsSync(workspacesDir)) {
    return status;
  }

  const repos = fs.readdirSync(workspacesDir);

  for (const repoName of repos) {
    const repoPath = path.join(workspacesDir, repoName);

    // Skip non-directories and hidden dirs
    if (!fs.statSync(repoPath).isDirectory() || repoName.startsWith('.')) {
      continue;
    }

    const repoStatus = scanRepo(repoPath, repoName);
    if (repoStatus) {
      status.repos[repoName] = repoStatus;
      status.summary.total++;

      if (repoStatus.health === 'healthy') status.summary.healthy++;
      else if (repoStatus.health === 'warning') status.summary.warnings++;
      else if (repoStatus.health === 'critical') status.summary.critical++;
    }
  }

  return status;
}

/**
 * Scan a single repository
 * @param {string} repoPath - Full path to repo
 * @param {string} repoName - Repository name
 * @returns {Object|null} Repo status or null if not a valid repo
 */
function scanRepo(repoPath, repoName) {
  // Must have .git directory
  if (!fs.existsSync(path.join(repoPath, '.git'))) {
    return null;
  }

  const repoConfig = ECOSYSTEM_REPOS[repoName] || { role: 'unknown', critical: false };

  // Get git state
  const gitState = getDetailedGitState(repoPath);

  // Detect project type
  const projectType = detectProjectType(repoPath);

  // Get package info if available
  const packageInfo = getPackageInfo(repoPath);

  // Calculate health
  const health = calculateRepoHealth(gitState, repoConfig);

  // Get recent activity
  const recentCommits = getRecentCommits(repoPath, 5);

  // Get contributor summary
  const contributors = getRepoContributors(repoPath);

  return {
    name: repoName,
    path: repoPath,
    role: repoConfig.role,
    critical: repoConfig.critical,
    type: projectType,
    version: packageInfo?.version || null,
    health,
    git: gitState,
    recentCommits,
    contributors: contributors ? { top: contributors, count: contributors.length } : null,
    lastScanned: new Date().toISOString(),
  };
}

/**
 * Get detailed git state for a repo
 */
function getDetailedGitState(repoPath) {
  const branch = safeExec('git', ['branch', '--show-current'], { cwd: repoPath });
  if (!branch) return null;

  const status = safeExec('git', ['status', '--porcelain'], { cwd: repoPath }) || '';
  const lines = status.split('\n').filter(l => l.length > 0);

  // Parse status
  const modified = lines.filter(l => l.startsWith(' M') || l.startsWith('M ')).length;
  const untracked = lines.filter(l => l.startsWith('??')).length;
  const staged = lines.filter(l => l.match(/^[MADRC] /)).length;
  const deleted = lines.filter(l => l.includes('D')).length;

  // Check if ahead/behind remote
  const trackingInfo = safeExec('git', ['rev-list', '--left-right', '--count', `origin/${branch}...HEAD`], { cwd: repoPath });
  let ahead = 0, behind = 0;
  if (trackingInfo) {
    const parts = trackingInfo.split(/\s+/);
    behind = parseInt(parts[0]) || 0;
    ahead = parseInt(parts[1]) || 0;
  }

  // Get last commit info
  const lastCommit = safeExec('git', ['log', '-1', '--format=%H|%s|%ar'], { cwd: repoPath });
  let lastCommitInfo = null;
  if (lastCommit) {
    const [hash, message, age] = lastCommit.split('|');
    lastCommitInfo = { hash: hash?.slice(0, 7), message, age };
  }

  return {
    branch,
    clean: status === '',
    modified,
    untracked,
    staged,
    deleted,
    ahead,
    behind,
    lastCommit: lastCommitInfo,
  };
}

/**
 * Get recent commits
 */
function getRecentCommits(repoPath, count = 5) {
  const log = safeExec('git', ['log', `-${count}`, '--format=%H|%s|%an|%ar'], { cwd: repoPath });
  if (!log) return [];

  return log.split('\n').filter(l => l).map(line => {
    const [hash, message, author, age] = line.split('|');
    return { hash: hash?.slice(0, 7), message, author, age };
  });
}

/**
 * Detect project type from files
 */
function detectProjectType(repoPath) {
  if (fs.existsSync(path.join(repoPath, 'package.json'))) return 'node';
  if (fs.existsSync(path.join(repoPath, 'Cargo.toml'))) return 'rust';
  if (fs.existsSync(path.join(repoPath, 'go.mod'))) return 'go';
  if (fs.existsSync(path.join(repoPath, 'pyproject.toml'))) return 'python';
  if (fs.existsSync(path.join(repoPath, 'Anchor.toml'))) return 'anchor';
  return 'unknown';
}

/**
 * Get package.json info
 */
function getPackageInfo(repoPath) {
  const pkgPath = path.join(repoPath, 'package.json');
  if (!fs.existsSync(pkgPath)) return null;

  try {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));
    return {
      name: pkg.name,
      version: pkg.version,
      dependencies: Object.keys(pkg.dependencies || {}),
      devDependencies: Object.keys(pkg.devDependencies || {}),
    };
  } catch (e) {
    return null;
  }
}

/**
 * Calculate repo health based on git state
 */
function calculateRepoHealth(gitState, repoConfig) {
  if (!gitState) return 'unknown';

  // Critical repos have stricter thresholds
  const isCritical = repoConfig.critical;

  // Critical conditions
  if (gitState.behind > 10) return 'critical';
  if (isCritical && gitState.modified > 20) return 'critical';

  // Warning conditions
  if (gitState.behind > 0) return 'warning';
  if (gitState.modified > 5) return 'warning';
  if (gitState.untracked > 10) return 'warning';
  if (!gitState.clean) return 'warning';

  return 'healthy';
}

// =============================================================================
// DEPENDENCY TRACKING
// =============================================================================

/**
 * Build dependency graph from package.json files
 * @returns {Object} Dependency graph
 */
function buildDependencyGraph() {
  const graph = {
    timestamp: new Date().toISOString(),
    nodes: {},
    edges: [],
    external: {},
  };

  const workspacesDir = '/workspaces';
  if (!fs.existsSync(workspacesDir)) return graph;

  const repos = fs.readdirSync(workspacesDir);

  // First pass: collect all internal packages
  const internalPackages = new Map(); // package name -> repo name

  for (const repoName of repos) {
    const repoPath = path.join(workspacesDir, repoName);
    if (!fs.statSync(repoPath).isDirectory()) continue;

    const pkgInfo = getPackageInfo(repoPath);
    if (pkgInfo?.name) {
      internalPackages.set(pkgInfo.name, repoName);
      graph.nodes[repoName] = {
        package: pkgInfo.name,
        version: pkgInfo.version,
        type: detectProjectType(repoPath),
      };
    }

    // Also check for monorepo packages
    const packagesDir = path.join(repoPath, 'packages');
    if (fs.existsSync(packagesDir)) {
      try {
        const subPkgs = fs.readdirSync(packagesDir);
        for (const subPkg of subPkgs) {
          const subPkgPath = path.join(packagesDir, subPkg);
          if (fs.statSync(subPkgPath).isDirectory()) {
            const subPkgInfo = getPackageInfo(subPkgPath);
            if (subPkgInfo?.name) {
              internalPackages.set(subPkgInfo.name, `${repoName}/${subPkg}`);
            }
          }
        }
      } catch (e) { /* ignore */ }
    }
  }

  // Second pass: find dependencies
  for (const repoName of repos) {
    const repoPath = path.join(workspacesDir, repoName);
    if (!fs.statSync(repoPath).isDirectory()) continue;

    const pkgInfo = getPackageInfo(repoPath);
    if (!pkgInfo) continue;

    const allDeps = [...(pkgInfo.dependencies || []), ...(pkgInfo.devDependencies || [])];

    for (const dep of allDeps) {
      if (internalPackages.has(dep)) {
        // Internal dependency
        graph.edges.push({
          from: repoName,
          to: internalPackages.get(dep),
          type: 'internal',
        });
      } else if (dep.startsWith('@cynic/') || dep.startsWith('@asdf/')) {
        // Ecosystem package (might be internal)
        graph.edges.push({
          from: repoName,
          to: dep,
          type: 'ecosystem',
        });
      }
    }
  }

  // Add known API dependencies
  for (const [from, deps] of Object.entries(KNOWN_DEPENDENCIES)) {
    for (const to of deps) {
      // Check if edge doesn't already exist
      const exists = graph.edges.some(e => e.from === from && e.to === to);
      if (!exists) {
        graph.edges.push({ from, to, type: 'api' });
      }
    }
  }

  return graph;
}

/**
 * Detect dependency conflicts or drift
 */
function detectDependencyIssues(graph) {
  const issues = [];

  // Check for circular dependencies
  const visited = new Set();
  const stack = new Set();

  function hasCycle(node, path = []) {
    if (stack.has(node)) {
      issues.push({
        type: 'circular',
        severity: 'warning',
        message: `Circular dependency detected: ${[...path, node].join(' → ')}`,
        nodes: [...path, node],
      });
      return true;
    }
    if (visited.has(node)) return false;

    visited.add(node);
    stack.add(node);

    const edges = graph.edges.filter(e => e.from === node);
    for (const edge of edges) {
      hasCycle(edge.to, [...path, node]);
    }

    stack.delete(node);
    return false;
  }

  for (const node of Object.keys(graph.nodes)) {
    hasCycle(node);
  }

  return issues;
}

// =============================================================================
// ALERT SYSTEM
// =============================================================================

/**
 * Load current alerts
 */
function loadAlerts() {
  const alertsPath = getAlertsPath();
  if (fs.existsSync(alertsPath)) {
    try {
      const data = JSON.parse(fs.readFileSync(alertsPath, 'utf-8'));
      // Filter out expired alerts
      const now = Date.now();
      data.alerts = (data.alerts || []).filter(a =>
        now - new Date(a.timestamp).getTime() < ALERT_TTL_MS
      );
      return data;
    } catch (e) {
      return { alerts: [] };
    }
  }
  return { alerts: [] };
}

/**
 * Save alerts
 */
function saveAlerts(data) {
  const alertsPath = getAlertsPath();
  // Keep only recent alerts
  data.alerts = (data.alerts || []).slice(-MAX_ALERTS);
  data.lastUpdated = new Date().toISOString();
  fs.writeFileSync(alertsPath, JSON.stringify(data, null, 2));
}

/**
 * Add a new alert
 */
function addAlert(alert) {
  const data = loadAlerts();

  // Check for duplicate (same type + repo within last 5 minutes)
  const isDuplicate = data.alerts.some(a =>
    a.type === alert.type &&
    a.repo === alert.repo &&
    Date.now() - new Date(a.timestamp).getTime() < 300000
  );

  if (!isDuplicate) {
    data.alerts.push({
      id: `alert_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      ...alert,
      timestamp: new Date().toISOString(),
      acknowledged: false,
    });
    saveAlerts(data);
    return true;
  }
  return false;
}

/**
 * Generate alerts from ecosystem status
 */
function generateAlerts(status) {
  const alerts = [];

  for (const [repoName, repo] of Object.entries(status.repos)) {
    // Critical health
    if (repo.health === 'critical') {
      alerts.push({
        type: 'health_critical',
        severity: 'critical',
        repo: repoName,
        message: `${repoName} is in critical state`,
        details: repo.git,
      });
    }

    // Uncommitted changes in critical repos
    if (repo.critical && repo.git && !repo.git.clean) {
      alerts.push({
        type: 'uncommitted_changes',
        severity: repo.git.modified > 10 ? 'warning' : 'info',
        repo: repoName,
        message: `${repoName} has ${repo.git.modified} modified, ${repo.git.untracked} untracked files`,
      });
    }

    // Behind remote
    if (repo.git?.behind > 0) {
      alerts.push({
        type: 'behind_remote',
        severity: repo.git.behind > 5 ? 'warning' : 'info',
        repo: repoName,
        message: `${repoName} is ${repo.git.behind} commits behind remote`,
      });
    }

    // Non-main branch on critical repos
    if (repo.critical && repo.git?.branch && repo.git.branch !== 'main') {
      alerts.push({
        type: 'non_main_branch',
        severity: 'info',
        repo: repoName,
        message: `${repoName} is on branch '${repo.git.branch}' (not main)`,
      });
    }
  }

  return alerts;
}

// =============================================================================
// STATUS PERSISTENCE
// =============================================================================

/**
 * Save ecosystem status
 */
function saveStatus(status) {
  const statusPath = getStatusPath();
  fs.writeFileSync(statusPath, JSON.stringify(status, null, 2));
}

/**
 * Load last ecosystem status
 */
function loadStatus() {
  const statusPath = getStatusPath();
  if (fs.existsSync(statusPath)) {
    try {
      return JSON.parse(fs.readFileSync(statusPath, 'utf-8'));
    } catch (e) {
      return null;
    }
  }
  return null;
}

/**
 * Save dependency graph
 */
function saveDependencyGraph(graph) {
  const depsPath = getDepsPath();
  fs.writeFileSync(depsPath, JSON.stringify(graph, null, 2));
}

/**
 * Load dependency graph
 */
function loadDependencyGraph() {
  const depsPath = getDepsPath();
  if (fs.existsSync(depsPath)) {
    try {
      return JSON.parse(fs.readFileSync(depsPath, 'utf-8'));
    } catch (e) {
      return null;
    }
  }
  return null;
}

// =============================================================================
// COCKPIT API
// =============================================================================

/**
 * Full cockpit scan - updates status, deps, and alerts
 * @returns {Object} Cockpit state
 */
function fullScan() {
  // Scan ecosystem
  const status = scanEcosystem();
  saveStatus(status);

  // Build dependency graph
  const deps = buildDependencyGraph();
  const depIssues = detectDependencyIssues(deps);
  saveDependencyGraph(deps);

  // Generate and save alerts
  const newAlerts = generateAlerts(status);
  for (const alert of newAlerts) {
    addAlert(alert);
  }

  // Add dependency issues as alerts
  for (const issue of depIssues) {
    addAlert({
      type: 'dependency_issue',
      severity: issue.severity,
      repo: issue.nodes?.[0] || 'ecosystem',
      message: issue.message,
    });
  }

  return {
    status,
    dependencies: deps,
    alerts: loadAlerts(),
  };
}

/**
 * Get current cockpit state (without scanning)
 */
function getCockpitState() {
  return {
    status: loadStatus(),
    dependencies: loadDependencyGraph(),
    alerts: loadAlerts(),
  };
}

/**
 * Format cockpit status for display
 * @param {Object} state - Cockpit state
 * @param {boolean} enableColor - Whether to use ANSI colors
 */
function formatCockpitStatus(state, enableColor = true) {
  useColor = enableColor;
  const lines = [];

  const header = '══════════════════════════════════════════════════════════════';
  lines.push(c(ANSI.magenta, '╔' + header + '╗'));
  lines.push(c(ANSI.magenta, '║') + c(ANSI.bold + ANSI.brightCyan, '                    🛩️  CYNIC COCKPIT                          ') + c(ANSI.magenta, '║'));
  lines.push(c(ANSI.magenta, '╠' + header + '╣'));

  if (!state.status) {
    lines.push(c(ANSI.magenta, '║') + c(ANSI.yellow, '  ⚠️  No scan data. Run fullScan() first.                     ') + c(ANSI.magenta, '║'));
    lines.push(c(ANSI.magenta, '╚' + header + '╝'));
    return lines.join('\n');
  }

  const s = state.status.summary;
  const healthyColor = s.healthy === s.total ? ANSI.brightGreen : ANSI.yellow;
  const summaryLine = `  ${c(ANSI.brightWhite, 'Repos:')} ${c(ANSI.brightCyan, s.total)} total │ ${c(ANSI.brightGreen, '✅ ' + s.healthy)} │ ${c(ANSI.yellow, '⚠️ ' + s.warnings)} │ ${c(ANSI.brightRed, '🔴 ' + s.critical)}`;
  lines.push(c(ANSI.magenta, '║') + summaryLine.padEnd(75) + c(ANSI.magenta, '║'));
  lines.push(c(ANSI.magenta, '╠' + header + '╣'));

  // Repo list
  for (const [name, repo] of Object.entries(state.status.repos)) {
    const healthIcon = repo.health === 'healthy' ? c(ANSI.brightGreen, '✅') :
                       repo.health === 'warning' ? c(ANSI.yellow, '⚠️') :
                       c(ANSI.brightRed, '🔴');
    const branch = repo.git?.branch || '?';
    const branchColor = branch === 'main' || branch === 'master' ? ANSI.brightGreen : ANSI.yellow;
    const changes = repo.git?.clean ? '' : c(ANSI.dim, ` (${repo.git?.modified || 0}M/${repo.git?.untracked || 0}U)`);
    const nameColor = repo.critical ? ANSI.brightWhite : ANSI.white;
    const criticalMark = repo.critical ? c(ANSI.brightYellow, '*') : ' ';

    const repoLine = `  ${healthIcon}${criticalMark}${c(nameColor, name.padEnd(19))} ${c(branchColor, branch.padEnd(15))}${changes}`;
    lines.push(c(ANSI.magenta, '║') + repoLine.padEnd(75) + c(ANSI.magenta, '║'));
  }

  // Alerts
  const alerts = state.alerts?.alerts || [];
  const activeAlerts = alerts.filter(a => !a.acknowledged);
  if (activeAlerts.length > 0) {
    lines.push(c(ANSI.magenta, '╠' + header + '╣'));
    lines.push(c(ANSI.magenta, '║') + c(ANSI.brightWhite, '  ALERTS:').padEnd(65) + c(ANSI.magenta, '║'));
    for (const alert of activeAlerts.slice(0, 5)) {
      const severityIcon = alert.severity === 'critical' ? c(ANSI.brightRed, '🔴') :
                           alert.severity === 'warning' ? c(ANSI.yellow, '⚠️') :
                           c(ANSI.cyan, 'ℹ️');
      const msgColor = alert.severity === 'critical' ? ANSI.brightRed :
                       alert.severity === 'warning' ? ANSI.yellow : ANSI.dim;
      const alertLine = `  ${severityIcon} ${c(msgColor, alert.message.slice(0, 55))}`;
      lines.push(c(ANSI.magenta, '║') + alertLine.padEnd(75) + c(ANSI.magenta, '║'));
    }
    if (activeAlerts.length > 5) {
      lines.push(c(ANSI.magenta, '║') + c(ANSI.dim, `  ... and ${activeAlerts.length - 5} more alerts`).padEnd(65) + c(ANSI.magenta, '║'));
    }
  }

  lines.push(c(ANSI.magenta, '╠' + header + '╣'));
  const scanTime = state.status.timestamp ? new Date(state.status.timestamp).toLocaleTimeString() : 'never';
  lines.push(c(ANSI.magenta, '║') + c(ANSI.dim, `  Last scan: ${scanTime}  │  * = critical repo`).padEnd(65) + c(ANSI.magenta, '║'));
  lines.push(c(ANSI.magenta, '╚' + header + '╝'));

  return lines.join('\n');
}

// =============================================================================
// CONTRIBUTOR CONTEXT - "Les rails dans le cerveau"
// =============================================================================

// Lazy load contributor discovery
let _contributorDiscovery = null;

function getContributorDiscovery() {
  if (!_contributorDiscovery) {
    try {
      _contributorDiscovery = require('./contributor-discovery.cjs');
    } catch (e) {
      return null;
    }
  }
  return _contributorDiscovery;
}

/**
 * Get top contributors for a repo (from recent commits)
 * @param {string} repoPath - Path to repo
 * @returns {Object} Top contributors with counts
 */
function getRepoContributors(repoPath) {
  const log = safeExec('git', ['log', '-100', '--format=%ae'], { cwd: repoPath });
  if (!log) return null;

  const counts = {};
  log.split('\n').filter(l => l).forEach(email => {
    counts[email] = (counts[email] || 0) + 1;
  });

  // Sort by count
  const sorted = Object.entries(counts)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([email, count]) => ({ email, count }));

  return sorted;
}

/**
 * Enrich repo status with contributor context
 * @param {Object} repoStatus - Repo status from scanRepo
 * @param {string} repoPath - Path to repo
 * @returns {Object} Enriched status
 */
function enrichWithContributors(repoStatus, repoPath) {
  const contributors = getRepoContributors(repoPath);
  if (!contributors) return repoStatus;

  return {
    ...repoStatus,
    contributors: {
      top: contributors,
      count: contributors.length,
    },
  };
}

/**
 * Get full contributor profiles for ecosystem (async)
 * @returns {Promise<Object>} Contributor profiles with φ-scores
 */
async function getEcosystemContributors() {
  const discovery = getContributorDiscovery();
  if (!discovery) return null;

  try {
    const result = await discovery.fullEcosystemScan();
    return result;
  } catch (e) {
    return null;
  }
}

/**
 * Generate contributor-aware alerts
 * Alerts when key contributors haven't been active
 */
function generateContributorAlerts(status, contributors) {
  const alerts = [];

  if (!contributors?.contributors) return alerts;

  // Find repos with single contributor (bus factor = 1)
  for (const [repoName, repo] of Object.entries(status.repos)) {
    const repoContribs = repo.contributors?.top || [];
    if (repoContribs.length === 1 && repo.critical) {
      alerts.push({
        type: 'bus_factor_low',
        severity: 'warning',
        repo: repoName,
        message: `${repoName} has only 1 active contributor (bus factor risk)`,
        contributor: repoContribs[0]?.email,
      });
    }
  }

  return alerts;
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  COCKPIT_VERSION,
  SCAN_INTERVAL_MS,
  ECOSYSTEM_REPOS,
  KNOWN_DEPENDENCIES,

  // Scanning
  scanEcosystem,
  scanRepo,
  fullScan,

  // Dependencies
  buildDependencyGraph,
  detectDependencyIssues,
  loadDependencyGraph,
  saveDependencyGraph,

  // Alerts
  loadAlerts,
  addAlert,
  generateAlerts,

  // Status
  loadStatus,
  saveStatus,
  getCockpitState,

  // Formatting
  formatCockpitStatus,

  // Paths
  getCockpitDir,

  // Contributor Context (les rails dans le cerveau)
  getRepoContributors,
  enrichWithContributors,
  getEcosystemContributors,
  generateContributorAlerts,
  getContributorDiscovery,
};
