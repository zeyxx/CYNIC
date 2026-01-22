/**
 * $ASDFASDFA Ecosystem Schema
 *
 * Complete mapping of the ecosystem architecture, relationships, and φ-economics.
 * CYNIC uses this schema to understand all dimensions of the ecosystem.
 *
 * "Relationships define truth" - κυνικός
 *
 * @module @cynic/scripts/ecosystem-schema
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// φ (PHI) CONSTANTS - The Golden Ratio governs ALL ratios
// ═══════════════════════════════════════════════════════════════════════════
const PHI = 1.618033988749895;
const PHI_POWERS = {
  '-3': 1 / (PHI ** 3),  // 0.236 - Treasury ratio
  '-2': 1 / (PHI ** 2),  // 0.382 - Max ecosystem burn
  '-1': 1 / PHI,         // 0.618 - Max confidence, damping factor
  '0': 1.0,              // Base
  '1': PHI,              // 1.618
  '2': PHI ** 2,         // 2.618
  '3': PHI ** 3,         // 4.236
};

// ═══════════════════════════════════════════════════════════════════════════
// ECOSYSTEM LAYERS (Stack Architecture)
// ═══════════════════════════════════════════════════════════════════════════
const ECOSYSTEM_LAYERS = {
  // Layer 4: Consumer-facing applications
  CONSUMER_APPS: {
    level: 4,
    name: 'Consumer Apps',
    description: 'User-facing applications that consume ecosystem services',
    projects: [
      { name: 'Ignition', status: 'production', developer: 'Sollama58', role: 'token-launch' },
      { name: 'ASDForecast', status: 'production', developer: 'Sollama58', role: 'analytics' },
    ],
    dependencies: ['INTELLIGENCE', 'INFRASTRUCTURE'],
    indirectDeps: ['framework-kit'], // Solana Foundation React hooks
  },

  // Layer 3: Intelligence/Oracle layer
  INTELLIGENCE: {
    level: 3,
    name: 'Intelligence',
    description: 'Oracle and scoring systems - source of truth for ecosystem',
    projects: [
      {
        name: 'HolDex',
        status: 'production',
        role: 'k-score-oracle',
        metrics: {
          'K-Score': 'Token quality: 100 × ∛(D × O × L)',
          'E-Score': 'Contribution weight: Σ(dimension × φ_weight)',
          'Metal Ranks': 'Diamond(90+), Platinum(80+), Gold(70+), Silver(60+), Bronze(50+)',
          'Credit Rating': 'A1-D grades based on K-Score + trajectory',
        },
        integrations: ['GASdf', 'Consumer Apps'],
      },
    ],
    dependencies: ['INFRASTRUCTURE'],
  },

  // Layer 2: Infrastructure
  INFRASTRUCTURE: {
    level: 2,
    name: 'Infrastructure',
    description: 'Core infrastructure for gasless transactions and burns',
    projects: [
      {
        name: 'GASdf',
        status: 'production',
        role: 'gasless-transactions',
        mechanics: {
          'Acceptance': 'K-Score >= 50 (Bronze+) for gasless',
          'Dual-burn Flywheel': {
            'Treasury': '23.6% (1/φ³)',
            'Ecosystem Burn': '0-38.2% (based on token burn rate)',
            'ASDF Burn': '38.2-76.4%',
          },
          'Formula': 'ecosystemBurn = (1/φ²) × (1 - φ^(-burnPct/30))',
        },
        holDexIntegration: {
          file: 'src/services/holdex.js',
          purpose: 'Token verification & K-score oracle',
        },
      },
    ],
    dependencies: ['SOLANA'],
  },

  // Layer 1: Base blockchain
  SOLANA: {
    level: 1,
    name: 'Solana',
    description: 'Base layer blockchain',
    projects: [],
    isExternal: true,
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// CYNIC'S ROLE IN THE ECOSYSTEM
// ═══════════════════════════════════════════════════════════════════════════
const CYNIC_ROLE = {
  name: 'CYNIC',
  fullName: 'κυνικός - The Cynical Dog',
  description: 'Collective intelligence that monitors, validates, and governs the ecosystem',

  replaces: 'asdf-brain', // Legacy prototype

  responsibilities: [
    'Monitor governance proposals',
    'Validate E-Score calculations',
    'Auto-merge Tier 1 changes (low-risk, well-tested)',
    'Anchor decisions to Merkle roots (PoJ Chain)',
    'Detect anomalies and patterns',
    'Learn from contributors and context',
  ],

  memory: {
    'PoJ Chain': 'Proof of Judgment blockchain - immutable judgment history',
    'Merkle DAG': 'Content-addressable storage with HAMT indices',
    'Graph Overlay': 'Relationship graph with φ-weighted PageRank',
    'PostgreSQL': 'FTS and knowledge persistence',
    'Redis': 'Session caching and hot data',
  },

  timing: {
    'Heartbeat': '61.8 seconds (φ × 38.2)',
    'Epoch': '32 slots',
    'Quorum': '61.8% (φ⁻¹)',
  },

  packages: [
    'anchor',      // Solana anchor integration
    'burns',       // Burn tracking and analytics
    'core',        // Core CYNIC logic
    'emergence',   // Pattern emergence detection
    'gasdf',       // GASdf integration
    'holdex',      // HolDex integration
    'identity',    // Identity verification
    'mcp',         // Model Context Protocol server
    'node',        // CYNIC node daemon
    'persistence', // Storage (PoJ, DAG, Graph)
    'protocol',    // Protocol definitions
    'zk',          // Zero-knowledge proofs
  ],
};

// ═══════════════════════════════════════════════════════════════════════════
// ECOSYSTEM REPOSITORIES (TRUE ECOSYSTEM = zeyxx owned)
// ═══════════════════════════════════════════════════════════════════════════
const ECOSYSTEM_REPOS = {
  // TRUE ECOSYSTEM (owned by zeyxx)
  'CYNIC-new': {
    owner: 'zeyxx',
    isEcosystem: true,
    layer: null, // Meta - CYNIC is the brain
    role: 'collective-intelligence',
    description: 'CYNIC implementation - the ecosystem brain',
    critical: true,
  },
  'GASdf': {
    owner: 'zeyxx',
    isEcosystem: true,
    layer: 'INFRASTRUCTURE',
    role: 'gasless-transactions',
    description: 'Gasless transactions with φ-based dual-burn',
    critical: true,
  },
  'HolDex': {
    owner: 'zeyxx',
    isEcosystem: true,
    layer: 'INTELLIGENCE',
    role: 'k-score-oracle',
    description: 'Token quality scoring and E-Score calculations',
    critical: true,
  },
  'asdf-brain': {
    owner: 'zeyxx',
    isEcosystem: true,
    layer: null,
    role: 'legacy-prototype',
    description: 'Legacy CYNIC prototype - being replaced by CYNIC-new',
    critical: false,
    status: 'deprecated',
  },
  'asdf-manifesto': {
    owner: 'zeyxx',
    isEcosystem: true,
    layer: null,
    role: 'philosophy',
    description: 'Ecosystem philosophy and documentation',
    critical: false,
  },

  // Sollama58 Forks (Production Apps)
  'Ignition': {
    owner: 'Sollama58',
    isEcosystem: true, // Part of ecosystem via fork relationship
    layer: 'CONSUMER_APPS',
    role: 'token-launch',
    description: 'Token launch platform',
    critical: false,
    relationship: 'fork', // Forked from zeyxx
  },
  'ASDForecast': {
    owner: 'Sollama58',
    isEcosystem: true,
    layer: 'CONSUMER_APPS',
    role: 'analytics',
    description: 'Analytics and forecasting',
    critical: false,
    relationship: 'fork',
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// EXTERNAL DEPENDENCIES (Not part of ecosystem, but used)
// ═══════════════════════════════════════════════════════════════════════════
const EXTERNAL_DEPS = {
  'framework-kit': {
    owner: 'solana-foundation',
    isEcosystem: false,
    role: 'react-hooks',
    description: 'Solana React hooks (@solana/client, @solana/react-hooks)',
    usedBy: ['Consumer Apps'],
    relationship: 'dependency',
  },
  'claude-mem': {
    owner: 'thedotmack',
    isEcosystem: false,
    role: 'inspiration',
    description: 'Memory system for Claude - inspired CYNIC memory design',
    relationship: 'inspiration', // Not directly used
  },
  'solana-dev-skill': {
    owner: 'GuiBibeau',
    isEcosystem: false,
    role: 'development-tool',
    description: 'Solana development skill',
    relationship: 'tool',
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// CONTRIBUTOR ROLES
// ═══════════════════════════════════════════════════════════════════════════
const CONTRIBUTOR_ROLES = {
  'zeyxx': {
    role: 'founder',
    responsibility: 'Core architecture, philosophy, primary development',
    repos: ['CYNIC-new', 'GASdf', 'HolDex', 'asdf-brain', 'asdf-manifesto'],
    isPrimary: true,
  },
  'Sollama58': {
    role: 'developer',
    responsibility: 'Consumer apps development (Ignition, ASDForecast)',
    repos: ['HolDex', 'Ignition', 'ASDForecast'],
    isPrimary: false,
    commits: 435, // On HolDex
  },
  'asdf-brain[bot]': {
    role: 'automation',
    responsibility: 'Automated commits, CI/CD, maintenance',
    isBot: true,
    isPrimary: false,
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// E-SCORE DIMENSIONS (from asdf-manifesto)
// ═══════════════════════════════════════════════════════════════════════════
const E_SCORE_DIMENSIONS = {
  HOLD: { weight: 1.0, description: 'Token holdings' },
  BURN: { weight: PHI, description: 'Tokens burned' },
  USE: { weight: 1.0, description: 'API/service usage' },
  BUILD: { weight: PHI ** 2, description: 'Apps built on ecosystem' },
  RUN: { weight: PHI ** 2, description: 'Nodes operated' },
  REFER: { weight: PHI, description: 'Referrals made' },
  TIME: { weight: 1 / PHI, description: 'Time in ecosystem' },
};

// ═══════════════════════════════════════════════════════════════════════════
// K-SCORE FORMULA (from HolDex)
// ═══════════════════════════════════════════════════════════════════════════
const K_SCORE = {
  formula: 'K = 100 × ∛(D × O × L)',
  components: {
    D: 'Diamond Hands - holding behavior',
    O: 'Organic Growth - natural acquisition',
    L: 'Longevity - time factor',
  },
  ranks: {
    Diamond: { min: 90, max: 99, icon: '💎', level: 8 },
    Platinum: { min: 80, max: 89, icon: '💠', level: 7 },
    Gold: { min: 70, max: 79, icon: '🥇', level: 6 },
    Silver: { min: 60, max: 69, icon: '🥈', level: 5 },
    Bronze: { min: 50, max: 59, icon: '🥉', level: 4 },
    Copper: { min: 40, max: 49, icon: '🟤', level: 3 },
    Iron: { min: 20, max: 39, icon: '⚫', level: 2 },
    Rust: { min: 0, max: 19, icon: '🔩', level: 1 },
  },
  // K-Score 100 is reserved for native tokens (SOL)
  acceptance: 50, // Bronze+ for gasless
};

// ═══════════════════════════════════════════════════════════════════════════
// RELATIONSHIP TYPES (for Graph Overlay)
// ═══════════════════════════════════════════════════════════════════════════
const RELATIONSHIPS = {
  // Direct ecosystem relationships
  OWNS: { weight: PHI ** 2, description: 'Project owns token' },
  DEVELOPS: { weight: PHI, description: 'Project develops repo' },
  CONTRIBUTES: { weight: PHI, description: 'User contributes to repo' },
  INTEGRATES: { weight: PHI ** 2, description: 'Project integrates with project' },

  // Token relationships
  HOLDS: { weight: PHI ** 2, description: 'Wallet holds token' },
  BURNS: { weight: PHI, description: 'Wallet burns token' },
  CREATED: { weight: PHI ** 3, description: 'Wallet created token' },

  // Indirect relationships
  DEPENDS_ON: { weight: PHI, description: 'Uses as dependency' },
  INSPIRED_BY: { weight: 1.0, description: 'Design inspiration' },
  FORKED_FROM: { weight: PHI, description: 'Forked repository' },
};

// ═══════════════════════════════════════════════════════════════════════════
// ECOSYSTEM GRAPH (Simplified representation)
// ═══════════════════════════════════════════════════════════════════════════
function buildEcosystemGraph() {
  const nodes = [];
  const edges = [];

  // Add project nodes
  for (const [name, repo] of Object.entries(ECOSYSTEM_REPOS)) {
    nodes.push({
      id: `project:${name}`,
      type: 'project',
      name,
      ...repo,
    });
  }

  // Add external dependency nodes
  for (const [name, dep] of Object.entries(EXTERNAL_DEPS)) {
    nodes.push({
      id: `external:${name}`,
      type: 'external',
      name,
      ...dep,
    });
  }

  // Add contributor nodes
  for (const [handle, contributor] of Object.entries(CONTRIBUTOR_ROLES)) {
    nodes.push({
      id: `user:${handle}`,
      type: 'user',
      handle,
      ...contributor,
    });

    // Add CONTRIBUTES edges
    for (const repo of contributor.repos) {
      edges.push({
        type: 'CONTRIBUTES',
        source: `user:${handle}`,
        target: `project:${repo}`,
        weight: RELATIONSHIPS.CONTRIBUTES.weight,
      });
    }
  }

  // Add integration edges
  edges.push({
    type: 'INTEGRATES',
    source: 'project:GASdf',
    target: 'project:HolDex',
    weight: RELATIONSHIPS.INTEGRATES.weight,
    description: 'GASdf uses HolDex for K-Score verification',
  });

  // Add dependency edges
  edges.push({
    type: 'DEPENDS_ON',
    source: 'project:Ignition',
    target: 'external:framework-kit',
    weight: RELATIONSHIPS.DEPENDS_ON.weight,
  });
  edges.push({
    type: 'DEPENDS_ON',
    source: 'project:ASDForecast',
    target: 'external:framework-kit',
    weight: RELATIONSHIPS.DEPENDS_ON.weight,
  });

  // Add inspiration edge
  edges.push({
    type: 'INSPIRED_BY',
    source: 'project:CYNIC-new',
    target: 'external:claude-mem',
    weight: RELATIONSHIPS.INSPIRED_BY.weight,
  });

  // Add replacement edge
  edges.push({
    type: 'REPLACES',
    source: 'project:CYNIC-new',
    target: 'project:asdf-brain',
    weight: PHI ** 2,
  });

  return { nodes, edges };
}

// ═══════════════════════════════════════════════════════════════════════════
// ANTI-EXTRACTION MECHANISMS (from asdf-manifesto)
// ═══════════════════════════════════════════════════════════════════════════
const ANTI_EXTRACTION = {
  philosophy: "Don't trust. Verify. Don't extract. Burn.",

  mechanisms: {
    burn: {
      description: '100% of all fees → $asdfasdfa purchase → burn',
      formula: 'ecosystemBurn = (1/φ²) × (1 - φ^(-burnPct/30))',
      effect: 'Deflationary pressure, no extraction',
    },
    sybil_resistance: {
      description: 'E-Score requires diverse contribution',
      dimensions: Object.keys(E_SCORE_DIMENSIONS).length,
    },
    whale_resistance: {
      description: 'Diminishing returns via φ-curves',
      mechanism: 'Square root scaling, φ-weights',
    },
    velocity_resistance: {
      description: 'Time-weighted contributions',
      mechanism: 'TIME dimension in E-Score',
    },
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Get the ecosystem layer for a project
 */
function getProjectLayer(projectName) {
  const repo = ECOSYSTEM_REPOS[projectName];
  if (!repo) return null;
  return ECOSYSTEM_LAYERS[repo.layer] || null;
}

/**
 * Check if a project is part of the ecosystem
 */
function isEcosystemProject(projectName) {
  return ECOSYSTEM_REPOS[projectName]?.isEcosystem === true;
}

/**
 * Check if a contributor is primary (owner)
 */
function isPrimaryContributor(handle) {
  return CONTRIBUTOR_ROLES[handle]?.isPrimary === true;
}

/**
 * Get all projects in a layer
 */
function getProjectsInLayer(layerName) {
  return Object.entries(ECOSYSTEM_REPOS)
    .filter(([, repo]) => repo.layer === layerName)
    .map(([name, repo]) => ({ name, ...repo }));
}

/**
 * Calculate E-Score from dimensions
 */
function calculateEScore(dimensions) {
  let total = 0;
  for (const [dim, value] of Object.entries(dimensions)) {
    const weight = E_SCORE_DIMENSIONS[dim]?.weight || 1.0;
    total += value * weight;
  }
  return total;
}

/**
 * Get K-Score rank info
 */
function getKRank(score) {
  for (const [name, rank] of Object.entries(K_SCORE.ranks)) {
    if (score >= rank.min && score <= rank.max) {
      return { name, ...rank };
    }
  }
  return { name: 'Rust', ...K_SCORE.ranks.Rust };
}

/**
 * Print ecosystem summary
 */
function printSummary() {
  console.log('═══════════════════════════════════════════════════════════════════');
  console.log('🐕 $ASDFASDFA ECOSYSTEM SCHEMA');
  console.log('═══════════════════════════════════════════════════════════════════\n');

  console.log('LAYERS:');
  for (const [name, layer] of Object.entries(ECOSYSTEM_LAYERS).sort((a, b) => b[1].level - a[1].level)) {
    if (layer.isExternal) continue;
    console.log(`  L${layer.level}: ${layer.name}`);
    for (const project of layer.projects) {
      console.log(`      └─ ${project.name} (${project.status}) - ${project.role}`);
    }
  }

  console.log('\nCYNIC ROLE:');
  console.log(`  ${CYNIC_ROLE.name}: ${CYNIC_ROLE.description}`);
  console.log(`  Packages: ${CYNIC_ROLE.packages.join(', ')}`);

  console.log('\nTRUE ECOSYSTEM REPOS (zeyxx owned):');
  for (const [name, repo] of Object.entries(ECOSYSTEM_REPOS)) {
    if (repo.isEcosystem && repo.owner === 'zeyxx') {
      console.log(`  ✓ ${name.padEnd(15)} - ${repo.role}`);
    }
  }

  console.log('\nCONTRIBUTORS:');
  for (const [handle, contributor] of Object.entries(CONTRIBUTOR_ROLES)) {
    const icon = contributor.isBot ? '🤖' : (contributor.isPrimary ? '👑' : '👤');
    console.log(`  ${icon} ${handle.padEnd(15)} - ${contributor.role}`);
  }

  console.log('\nφ-ECONOMICS:');
  console.log(`  Treasury:      ${(PHI_POWERS['-3'] * 100).toFixed(1)}% (1/φ³)`);
  console.log(`  Max Eco Burn:  ${(PHI_POWERS['-2'] * 100).toFixed(1)}% (1/φ²)`);
  console.log(`  Max Confidence:${(PHI_POWERS['-1'] * 100).toFixed(1)}% (1/φ)`);

  console.log('\n═══════════════════════════════════════════════════════════════════');
}

module.exports = {
  // Constants
  PHI,
  PHI_POWERS,

  // Schema
  ECOSYSTEM_LAYERS,
  CYNIC_ROLE,
  ECOSYSTEM_REPOS,
  EXTERNAL_DEPS,
  CONTRIBUTOR_ROLES,
  E_SCORE_DIMENSIONS,
  K_SCORE,
  RELATIONSHIPS,
  ANTI_EXTRACTION,

  // Functions
  buildEcosystemGraph,
  getProjectLayer,
  isEcosystemProject,
  isPrimaryContributor,
  getProjectsInLayer,
  calculateEScore,
  getKRank,
  printSummary,
};
