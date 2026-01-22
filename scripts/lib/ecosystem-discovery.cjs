/**
 * $ASDFASDFA Ecosystem Discovery
 *
 * DYNAMIC ecosystem schema that discovers and learns relationships.
 * Unlike static schema, this evolves with the ecosystem.
 *
 * "Truth is discovered, not declared" - κυνικός
 *
 * @module @cynic/scripts/ecosystem-discovery
 */

'use strict';

const path = require('path');
const fs = require('fs');
const { execFileSync, spawnSync } = require('child_process');

// Import static schema for base knowledge
const staticSchema = require('./ecosystem-schema.cjs');
const contributorDiscovery = require('./contributor-discovery.cjs');

const { PHI, PHI_POWERS } = staticSchema;

// Discovery cache
let discoveryCache = null;
let lastDiscoveryTime = 0;
const DISCOVERY_TTL = 5 * 60 * 1000; // 5 minutes

// ═══════════════════════════════════════════════════════════════════════════
// SAFE COMMAND EXECUTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Safe git command execution (no shell injection)
 */
function safeGit(args, cwd) {
  try {
    const result = spawnSync('git', args, {
      cwd,
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
      timeout: 10000,
    });
    return result.stdout?.trim() || '';
  } catch (e) {
    return '';
  }
}

/**
 * Safe grep for imports (no shell injection)
 */
function safeGrepImports(dirPath) {
  const imports = new Map();

  // Use Node.js to read files instead of grep
  const srcPath = path.join(dirPath, 'src');
  if (!fs.existsSync(srcPath)) return imports;

  function scanDir(dir) {
    try {
      const entries = fs.readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);
        if (entry.isDirectory() && !entry.name.startsWith('.') && entry.name !== 'node_modules') {
          scanDir(fullPath);
        } else if (entry.isFile() && /\.(js|ts|cjs|mjs)$/.test(entry.name)) {
          try {
            const content = fs.readFileSync(fullPath, 'utf8');
            const matches = content.matchAll(/from ['"]([^'"]+)['"]/g);
            for (const match of matches) {
              const dep = match[1];
              imports.set(dep, (imports.get(dep) || 0) + 1);
            }
          } catch (e) {
            // Ignore read errors
          }
        }
      }
    } catch (e) {
      // Ignore dir errors
    }
  }

  scanDir(srcPath);
  return imports;
}

// ═══════════════════════════════════════════════════════════════════════════
// DYNAMIC DISCOVERY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Discover all repos in workspace
 */
function discoverRepos(workspacePath = '/workspaces') {
  return contributorDiscovery.discoverRepos(workspacePath);
}

/**
 * Get repo ownership info
 */
function getRepoOwnership(repoPath) {
  return contributorDiscovery.getRepoOwnership(repoPath);
}

/**
 * Discover package.json dependencies
 */
function discoverDependencies(repoPath) {
  const deps = {
    production: {},
    development: {},
    peer: {},
  };

  const pkgPath = path.join(repoPath, 'package.json');
  if (!fs.existsSync(pkgPath)) return deps;

  try {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
    deps.production = pkg.dependencies || {};
    deps.development = pkg.devDependencies || {};
    deps.peer = pkg.peerDependencies || {};
  } catch (e) {
    // Ignore parse errors
  }

  return deps;
}

/**
 * Discover imports/requires in code (using safe file scanning)
 */
function discoverCodeDependencies(repoPath) {
  return safeGrepImports(repoPath);
}

/**
 * Discover integration points between repos
 */
function discoverIntegrations(repos) {
  const integrations = [];

  for (const repo of repos) {
    const ownership = getRepoOwnership(repo);
    if (!ownership.isEcosystem) continue;

    const name = path.basename(repo);
    const deps = discoverDependencies(repo);
    const codeDeps = discoverCodeDependencies(repo);

    // Check for known ecosystem integrations
    const allDeps = Object.keys({
      ...deps.production,
      ...deps.development,
    });

    // HolDex integration
    if (codeDeps.has('../services/holdex') || codeDeps.has('./holdex') || allDeps.includes('@cynic/holdex')) {
      integrations.push({
        from: name,
        to: 'HolDex',
        type: 'INTEGRATES',
        weight: PHI_POWERS['2'],
        evidence: 'Code import detected',
      });
    }

    // GASdf integration
    if (codeDeps.has('../services/gasdf') || codeDeps.has('./gasdf') || allDeps.includes('@cynic/gasdf')) {
      integrations.push({
        from: name,
        to: 'GASdf',
        type: 'INTEGRATES',
        weight: PHI_POWERS['2'],
        evidence: 'Code import detected',
      });
    }

    // Solana dependencies
    for (const dep of allDeps) {
      if (dep.startsWith('@solana/')) {
        integrations.push({
          from: name,
          to: dep,
          type: 'DEPENDS_ON',
          weight: PHI_POWERS['1'],
          evidence: 'package.json dependency',
        });
      }
    }
  }

  return integrations;
}

/**
 * Discover project layer based on content
 */
function discoverProjectLayer(repoPath) {
  const name = path.basename(repoPath);

  try {
    const files = fs.readdirSync(repoPath).join(' ');

    // Check for known patterns
    if (name.toLowerCase().includes('oracle') || files.includes('harmonyEngine')) {
      return 'INTELLIGENCE';
    }
    if (name.toLowerCase().includes('gas') || files.includes('gasless')) {
      return 'INFRASTRUCTURE';
    }
    if (files.includes('components') && files.includes('pages')) {
      return 'CONSUMER_APPS';
    }
    if (files.includes('docs') || name.includes('manifesto')) {
      return 'DOCUMENTATION';
    }

    // Check for specific integrations
    const deps = discoverDependencies(repoPath);
    const allDeps = Object.keys(deps.production || {});

    if (allDeps.some(d => d.includes('react') || d.includes('next'))) {
      return 'CONSUMER_APPS';
    }
    if (allDeps.some(d => d.includes('@solana/web3'))) {
      return 'INFRASTRUCTURE';
    }
  } catch (e) {
    // Ignore errors
  }

  return 'UNKNOWN';
}

/**
 * Discover contributor activity (using safe git commands)
 */
function discoverContributorActivity(repoPath) {
  const activity = [];

  const result = safeGit(
    ['log', '--pretty=format:%ae|%s', '--since=30 days ago', '-n', '50'],
    repoPath
  );

  for (const line of result.split('\n')) {
    if (!line.includes('|')) continue;
    const [email, message] = line.split('|');
    if (email && message) {
      activity.push({ email, message, repo: path.basename(repoPath) });
    }
  }

  return activity;
}

// ═══════════════════════════════════════════════════════════════════════════
// ECOSYSTEM STATE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Build complete ecosystem state (cached)
 */
function discoverEcosystem(workspacePath = '/workspaces', forceRefresh = false) {
  const now = Date.now();

  // Return cached if valid
  if (!forceRefresh && discoveryCache && (now - lastDiscoveryTime) < DISCOVERY_TTL) {
    return discoveryCache;
  }

  const repos = discoverRepos(workspacePath);

  const ecosystem = {
    timestamp: now,
    version: 1,

    // Projects discovered
    projects: [],
    externalDeps: [],

    // Relationships discovered
    integrations: [],
    dependencies: [],
    contributions: [],

    // Contributors discovered
    contributors: [],

    // Layers
    layers: {
      CONSUMER_APPS: [],
      INTELLIGENCE: [],
      INFRASTRUCTURE: [],
      DOCUMENTATION: [],
      UNKNOWN: [],
    },

    // Health metrics
    health: {
      totalProjects: 0,
      ecosystemProjects: 0,
      externalProjects: 0,
      totalContributors: 0,
      recentActivity: 0,
    },
  };

  // Discover each repo
  for (const repoPath of repos) {
    const ownership = getRepoOwnership(repoPath);
    const name = path.basename(repoPath);
    const layer = discoverProjectLayer(repoPath);
    const deps = discoverDependencies(repoPath);

    const project = {
      name,
      path: repoPath,
      owner: ownership.owner,
      isEcosystem: ownership.isEcosystem,
      layer,
      origin: ownership.origin,
      dependencies: Object.keys(deps.production || {}),
      devDependencies: Object.keys(deps.development || {}),
    };

    if (ownership.isEcosystem) {
      ecosystem.projects.push(project);
      ecosystem.layers[layer].push(name);
      ecosystem.health.ecosystemProjects++;
    } else {
      ecosystem.externalDeps.push(project);
      ecosystem.health.externalProjects++;
    }

    ecosystem.health.totalProjects++;

    // Discover activity
    const activity = discoverContributorActivity(repoPath);
    ecosystem.contributions.push(...activity);
    ecosystem.health.recentActivity += activity.length;
  }

  // Discover integrations
  ecosystem.integrations = discoverIntegrations(repos);

  // Discover contributors from cache
  try {
    const contributorsFile = path.join(process.env.HOME, '.cynic/learning/contributors.json');
    if (fs.existsSync(contributorsFile)) {
      const data = JSON.parse(fs.readFileSync(contributorsFile, 'utf8'));
      for (const [email, info] of Object.entries(data.contributors || {})) {
        // Only count ecosystem contributors
        const ecosystemCommits = Object.entries(info.repos || {})
          .filter(([repo]) => ecosystem.projects.some(p => p.name === repo))
          .reduce((sum, [, count]) => sum + count, 0);

        if (ecosystemCommits > 0) {
          ecosystem.contributors.push({
            email,
            name: info.primaryName || email.split('@')[0],
            totalCommits: info.totalCommits,
            ecosystemCommits,
            repos: Object.keys(info.repos || {}),
            isBot: email.includes('[bot]'),
          });
          ecosystem.health.totalContributors++;
        }
      }
    }
  } catch (e) {
    // Ignore
  }

  // Sort contributors by ecosystem commits
  ecosystem.contributors.sort((a, b) => b.ecosystemCommits - a.ecosystemCommits);

  // Cache result
  discoveryCache = ecosystem;
  lastDiscoveryTime = now;

  return ecosystem;
}

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Generate graph from discovered ecosystem
 */
function buildDiscoveredGraph(ecosystem = null) {
  if (!ecosystem) {
    ecosystem = discoverEcosystem();
  }

  const nodes = [];
  const edges = [];

  // Add project nodes
  for (const project of ecosystem.projects) {
    nodes.push({
      id: `project:${project.name}`,
      type: 'project',
      identifier: project.name,
      attributes: {
        name: project.name,
        layer: project.layer,
        owner: project.owner,
        isEcosystem: true,
      },
    });
  }

  // Add external dependency nodes
  for (const dep of ecosystem.externalDeps) {
    nodes.push({
      id: `external:${dep.name}`,
      type: 'external',
      identifier: dep.name,
      attributes: {
        name: dep.name,
        owner: dep.owner,
        isEcosystem: false,
      },
    });
  }

  // Add contributor nodes
  for (const contributor of ecosystem.contributors) {
    nodes.push({
      id: `user:${contributor.email}`,
      type: 'user',
      identifier: contributor.email,
      attributes: {
        name: contributor.name,
        email: contributor.email,
        commits: contributor.ecosystemCommits,
        isBot: contributor.isBot,
      },
    });
  }

  // Add integration edges
  for (const integration of ecosystem.integrations) {
    edges.push({
      type: integration.type,
      sourceId: `project:${integration.from}`,
      targetId: integration.to.startsWith('@')
        ? `external:${integration.to}`
        : `project:${integration.to}`,
      weight: integration.weight,
      attributes: {
        evidence: integration.evidence,
      },
    });
  }

  // Add contribution edges (aggregated by contributor)
  const contributionsByUser = new Map();
  for (const contribution of ecosystem.contributions) {
    const key = `${contribution.email}:${contribution.repo}`;
    contributionsByUser.set(key, (contributionsByUser.get(key) || 0) + 1);
  }

  for (const [key, count] of contributionsByUser) {
    const [email, repo] = key.split(':');
    edges.push({
      type: 'CONTRIBUTES',
      sourceId: `user:${email}`,
      targetId: `project:${repo}`,
      weight: PHI_POWERS['1'],
      attributes: {
        commits: count,
        period: '30d',
      },
    });
  }

  return { nodes, edges, ecosystem };
}

// ═══════════════════════════════════════════════════════════════════════════
// DIFF / CHANGE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Compare two ecosystem states
 */
function diffEcosystems(previous, current) {
  const diff = {
    newProjects: [],
    removedProjects: [],
    newContributors: [],
    newIntegrations: [],
    changedLayers: [],
  };

  if (!previous) return diff;

  const prevProjectNames = new Set(previous.projects.map(p => p.name));
  const currProjectNames = new Set(current.projects.map(p => p.name));

  // New projects
  for (const name of currProjectNames) {
    if (!prevProjectNames.has(name)) {
      diff.newProjects.push(name);
    }
  }

  // Removed projects
  for (const name of prevProjectNames) {
    if (!currProjectNames.has(name)) {
      diff.removedProjects.push(name);
    }
  }

  // New contributors
  const prevContributorEmails = new Set(previous.contributors.map(c => c.email));
  for (const contributor of current.contributors) {
    if (!prevContributorEmails.has(contributor.email)) {
      diff.newContributors.push(contributor);
    }
  }

  // New integrations
  const prevIntegrations = new Set(previous.integrations.map(i => `${i.from}→${i.to}`));
  for (const integration of current.integrations) {
    const key = `${integration.from}→${integration.to}`;
    if (!prevIntegrations.has(key)) {
      diff.newIntegrations.push(integration);
    }
  }

  // Layer changes
  for (const project of current.projects) {
    const prev = previous.projects.find(p => p.name === project.name);
    if (prev && prev.layer !== project.layer) {
      diff.changedLayers.push({
        project: project.name,
        from: prev.layer,
        to: project.layer,
      });
    }
  }

  return diff;
}

// ═══════════════════════════════════════════════════════════════════════════
// PRINT / DISPLAY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Print discovered ecosystem summary
 */
function printDiscoveredSummary(ecosystem = null) {
  if (!ecosystem) {
    ecosystem = discoverEcosystem();
  }

  console.log('═══════════════════════════════════════════════════════════════════');
  console.log('🐕 $ASDFASDFA ECOSYSTEM - DISCOVERED STATE');
  console.log('═══════════════════════════════════════════════════════════════════\n');

  console.log('LAYERS (Discovered):');
  for (const [layer, projects] of Object.entries(ecosystem.layers)) {
    if (projects.length === 0) continue;
    console.log(`  ${layer}:`);
    for (const name of projects) {
      const project = ecosystem.projects.find(p => p.name === name);
      console.log(`    └─ ${name} (${project?.owner || 'unknown'})`);
    }
  }

  console.log('\nINTEGRATIONS (Discovered):');
  for (const integration of ecosystem.integrations.slice(0, 10)) {
    console.log(`  ${integration.from} ──${integration.type}──> ${integration.to}`);
  }
  if (ecosystem.integrations.length > 10) {
    console.log(`  ... and ${ecosystem.integrations.length - 10} more`);
  }

  console.log('\nTOP CONTRIBUTORS (Ecosystem):');
  for (const contributor of ecosystem.contributors.slice(0, 5)) {
    const icon = contributor.isBot ? '🤖' : '👤';
    console.log(`  ${icon} ${contributor.name.padEnd(20)} ${String(contributor.ecosystemCommits).padStart(4)} commits`);
  }

  console.log('\nHEALTH:');
  console.log(`  Total Projects:    ${ecosystem.health.totalProjects}`);
  console.log(`  Ecosystem:         ${ecosystem.health.ecosystemProjects}`);
  console.log(`  External:          ${ecosystem.health.externalProjects}`);
  console.log(`  Contributors:      ${ecosystem.health.totalContributors}`);
  console.log(`  Recent Activity:   ${ecosystem.health.recentActivity} commits (30d)`);

  console.log('\n═══════════════════════════════════════════════════════════════════');
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Discovery
  discoverEcosystem,
  discoverRepos,
  getRepoOwnership,
  discoverDependencies,
  discoverIntegrations,
  discoverProjectLayer,
  discoverContributorActivity,

  // Graph
  buildDiscoveredGraph,

  // Diff
  diffEcosystems,

  // Display
  printDiscoveredSummary,

  // Cache control
  clearCache: () => {
    discoveryCache = null;
    lastDiscoveryTime = 0;
  },

  // Constants from static schema
  PHI,
  PHI_POWERS,
  staticSchema,
};
