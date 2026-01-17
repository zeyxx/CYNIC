/**
 * META Dashboard Service
 *
 * CYNIC analyzing itself - self-reflection for guided development.
 *
 * "φ distrusts φ" - Even I doubt myself at 61.8% max confidence
 *
 * @module @cynic/mcp/meta-dashboard
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// CYNIC ARCHITECTURE MAP
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Complete CYNIC architecture definition
 * This is CYNIC's self-knowledge
 */
export const CYNIC_ARCHITECTURE = {
  version: '0.4.0',
  updated: '2026-01-17',

  // ─────────────────────────────────────────────────────────────────────────
  // PACKAGES
  // ─────────────────────────────────────────────────────────────────────────
  packages: {
    core: {
      name: '@cynic/core',
      version: '0.1.0',
      purpose: 'Identity, Axioms, Scoring - φ derives all',
      status: 'COMPLETE',
      completeness: 1.0,
      modules: {
        axioms: { status: 'COMPLETE', exports: ['PHI', 'PHI_INV', 'AXIOMS', 'fib'] },
        qscore: { status: 'COMPLETE', exports: ['calculateQScore', 'getVerdict', 'analyzeWeaknesses'] },
        worlds: { status: 'COMPLETE', exports: ['WorldManager', 'Atzilut', 'Beriah', 'Yetzirah', 'Assiah'] },
        identity: { status: 'COMPLETE', exports: ['IDENTITY', 'TRAITS', 'VOICE', 'VERDICTS', 'THE_DOGS', 'FOUR_DOGS'] },
        config: { status: 'COMPLETE', exports: ['detectEnvironment', 'validateSecrets', 'getDatabaseConfig'] },
      },
    },

    protocol: {
      name: '@cynic/protocol',
      version: '0.1.0',
      purpose: '4-Layer Decentralized Protocol Stack',
      status: 'COMPLETE',
      completeness: 1.0,
      modules: {
        poj: { status: 'COMPLETE', layer: 1, exports: ['PoJChain', 'createJudgmentBlock', 'hashBlock'] },
        merkle: { status: 'COMPLETE', layer: 2, exports: ['MerkleTree', 'KnowledgeTree', 'createPattern'] },
        gossip: { status: 'COMPLETE', layer: 3, exports: ['GossipProtocol', 'PeerManager', 'MessageType'] },
        consensus: { status: 'COMPLETE', layer: 4, exports: ['ConsensusEngine', 'VoterLockout', 'FinalityTracker'] },
        kscore: { status: 'COMPLETE', exports: ['calculateKScore', 'KScoreTier', 'isHealthyKScore'] },
        crypto: { status: 'COMPLETE', exports: ['sha256', 'generateKeypair', 'signBlock', 'verifyBlock'] },
      },
    },

    persistence: {
      name: '@cynic/persistence',
      version: '0.4.0',
      purpose: 'Hybrid Storage - PostgreSQL + Redis + DAG + PoJ + Graph',
      status: 'ACTIVE_DEVELOPMENT',
      completeness: 0.85,
      modules: {
        postgres: { status: 'COMPLETE', exports: ['PostgresClient', '8 Repositories'] },
        redis: { status: 'COMPLETE', exports: ['RedisClient', 'SessionStore'] },
        dag: { status: 'NEW', exports: ['MerkleDAG', 'BlockStore', 'HAMTIndex', 'CID utils'] },
        poj: { status: 'NEW', exports: ['PoJChain', 'PoJBlock', 'computeMerkleRoot'] },
        graph: { status: 'NEW', exports: ['GraphOverlay', 'GraphTraversal', '7 NodeTypes', '12 EdgeTypes'] },
      },
    },

    node: {
      name: '@cynic/node',
      version: '0.1.0',
      purpose: 'Judgment Node - Judge, Operator, Agents',
      status: 'COMPLETE',
      completeness: 0.95,
      modules: {
        judge: { status: 'COMPLETE', exports: ['CYNICJudge', '25 Dimensions', 'ResidualDetector'] },
        operator: { status: 'COMPLETE', exports: ['Operator', 'E-Score', 'Identity'] },
        agents: {
          status: 'COMPLETE',
          exports: ['CollectivePack', 'THE_DOGS (11 Sefirot)'],
          dogs: ['Guardian', 'Analyst', 'Scholar', 'Architect', 'Sage', 'Cynic', 'Janitor', 'Scout', 'Cartographer', 'Oracle', 'Deployer'],
          legacy: ['Observer', 'Digester', 'Guardian', 'Mentor'],
        },
        state: { status: 'COMPLETE', exports: ['StateManager', 'MemoryStorage', 'FileStorage'] },
        transport: { status: 'COMPLETE', exports: ['WebSocketTransport', 'CBOR serialization'] },
        api: { status: 'COMPLETE', exports: ['APIServer'] },
        privacy: { status: 'COMPLETE', exports: ['DifferentialPrivacy', 'LocalStore'] },
      },
    },

    mcp: {
      name: '@cynic/mcp',
      version: '0.1.0',
      purpose: 'Model Context Protocol Server - AI Integration',
      status: 'COMPLETE',
      completeness: 1.0,
      modules: {
        server: { status: 'COMPLETE', exports: ['MCPServer'] },
        tools: {
          status: 'COMPLETE',
          count: 15,
          list: [
            'brain_cynic_judge',
            'brain_cynic_digest',
            'brain_health',
            'brain_search',
            'brain_patterns',
            'brain_cynic_feedback',
            'brain_agents_status',
            'brain_agent_diagnostic',
            'brain_session_start',
            'brain_session_end',
            'brain_docs',
            'brain_ecosystem',
            'brain_poj_chain',
            'brain_integrator',
            'brain_metrics',
          ],
        },
        services: {
          status: 'COMPLETE',
          count: 6,
          list: ['PersistenceManager', 'SessionManager', 'PoJChainManager', 'LibrarianService', 'EcosystemService', 'IntegratorService', 'MetricsService'],
        },
      },
    },

    client: {
      name: '@cynic/client',
      version: '0.1.0',
      purpose: 'HTTP Client for Node API - HolDex Integration',
      status: 'COMPLETE',
      completeness: 1.0,
      modules: {
        client: { status: 'COMPLETE', exports: ['CYNICClient', 'submitKScore', 'createPool', 'createHolDexClient'] },
      },
    },
  },

  // ─────────────────────────────────────────────────────────────────────────
  // THE FOUR DOGS (Agents)
  // ─────────────────────────────────────────────────────────────────────────
  dogs: {
    v1: {
      observer: {
        name: 'Observer',
        trigger: 'PostToolUse',
        blocking: false,
        purpose: 'Silent watcher, pattern detection',
        status: 'ACTIVE',
      },
      digester: {
        name: 'Digester',
        trigger: 'PostConversation',
        blocking: false,
        purpose: 'Archivist, extracts knowledge from chaos',
        status: 'ACTIVE',
      },
      guardian: {
        name: 'Guardian',
        trigger: 'PreToolUse',
        blocking: true,
        purpose: 'Watchdog, prevents destruction',
        status: 'ACTIVE',
      },
      mentor: {
        name: 'Mentor',
        trigger: 'ContextAware',
        blocking: false,
        purpose: 'Wise elder, shares wisdom',
        status: 'ACTIVE',
      },
    },
    v2_collective: {
      // The Five Dogs (Original)
      guardian: { name: 'CollectiveGuardian', sephirah: 'Gevurah', status: 'IMPLEMENTED' },
      analyst: { name: 'CollectiveAnalyst', sephirah: 'Binah', status: 'IMPLEMENTED' },
      scholar: { name: 'CollectiveScholar', sephirah: 'Daat', status: 'IMPLEMENTED' },
      architect: { name: 'CollectiveArchitect', sephirah: 'Chesed', status: 'IMPLEMENTED' },
      sage: { name: 'CollectiveSage', sephirah: 'Chochmah', status: 'IMPLEMENTED' },
      // The Hidden Dog (Keter)
      cynic: { name: 'CollectiveCynic', sephirah: 'Keter', status: 'IMPLEMENTED' },
      // The Additional Dogs (Complete Sefirot Tree)
      janitor: { name: 'CollectiveJanitor', sephirah: 'Yesod', status: 'IMPLEMENTED' },
      scout: { name: 'CollectiveScout', sephirah: 'Netzach', status: 'IMPLEMENTED' },
      cartographer: { name: 'CollectiveCartographer', sephirah: 'Malkhut', status: 'IMPLEMENTED' },
      oracle: { name: 'CollectiveOracle', sephirah: 'Tiferet', status: 'IMPLEMENTED' },
      deployer: { name: 'CollectiveDeployer', sephirah: 'Hod', status: 'IMPLEMENTED' },
    },
    // Legacy names mapped to current implementations (GAP-004 COMPLETE)
    legacy_mapping: {
      hunter: { renamed_to: 'scout', purpose: 'Proactive information retrieval', status: 'RENAMED' },
      shepherd: { renamed_to: 'sage', purpose: 'Multi-session coherence', status: 'RENAMED' },
      herald: { renamed_to: 'oracle', purpose: 'External notifications', status: 'RENAMED' },
      tracker: { renamed_to: 'cartographer', purpose: 'Long-term goal persistence', status: 'RENAMED' },
      sentinel: { renamed_to: 'guardian', purpose: 'Background monitoring', status: 'RENAMED' },
    },
  },

  // ─────────────────────────────────────────────────────────────────────────
  // THE FOUR AXIOMS (Worlds)
  // ─────────────────────────────────────────────────────────────────────────
  axioms: {
    PHI: {
      symbol: 'φ',
      world: 'Atzilut',
      question: 'Does it embody φ?',
      color: '#FFD700',
      dimensions: ['COHERENCE', 'HARMONY', 'STRUCTURE', 'ELEGANCE', 'COMPLETENESS', 'PRECISION'],
    },
    VERIFY: {
      symbol: '✓',
      world: 'Beriah',
      question: 'Can it be verified?',
      color: '#4169E1',
      dimensions: ['ACCURACY', 'CONSISTENCY', 'REPRODUCIBILITY', 'TRANSPARENCY', 'AUDITABILITY', 'FALSIFIABILITY'],
    },
    CULTURE: {
      symbol: '⛩',
      world: 'Yetzirah',
      question: 'Does it respect culture?',
      color: '#228B22',
      dimensions: ['ALIGNMENT', 'ADAPTABILITY', 'INCLUSIVITY', 'SUSTAINABILITY', 'LEGACY', 'EVOLUTION'],
    },
    BURN: {
      symbol: '🔥',
      world: 'Assiah',
      question: 'Does it burn?',
      color: '#FF4500',
      dimensions: ['COMMITMENT', 'SACRIFICE', 'TRANSFORMATION', 'IRREVERSIBILITY', 'AUTHENTICITY', 'NON_EXTRACTION'],
    },
  },

  // ─────────────────────────────────────────────────────────────────────────
  // IDENTIFIED GAPS
  // ─────────────────────────────────────────────────────────────────────────
  gaps: {
    critical: [
      {
        id: 'GAP-001',
        area: 'Integration',
        title: 'HolDex Integration',
        description: 'K-Score submission flow exists but not connected to live HolDex',
        priority: 'HIGH',
        packages: ['client', 'node'],
      },
      {
        id: 'GAP-002',
        area: 'Integration',
        title: 'GASdf Integration',
        description: 'Gasless transaction support specified but not implemented',
        priority: 'HIGH',
        packages: ['protocol', 'node'],
      },
    ],
    important: [
      {
        id: 'GAP-003',
        area: 'Privacy',
        title: 'ZK/Light Protocol',
        description: 'Zero-knowledge proofs for E-Score privacy not implemented',
        priority: 'MEDIUM',
        packages: ['node', 'protocol'],
      },
      {
        id: 'GAP-005',
        area: 'Distributed',
        title: 'Multi-Node Consensus',
        description: 'Consensus engine implemented but not tested in multi-node setup',
        priority: 'MEDIUM',
        packages: ['protocol', 'node'],
      },
    ],
    completed: [
      {
        id: 'GAP-004',
        area: 'Agents',
        title: '11 Dogs Complete',
        description: 'All Sefirot dogs implemented: Guardian, Analyst, Scholar, Architect, Sage, CYNIC, Janitor, Scout, Cartographer, Oracle, Deployer',
        completedAt: '2026-01-17',
        packages: ['node'],
      },
    ],
    wishlist: [
      {
        id: 'GAP-006',
        area: 'UI',
        title: 'Dashboard Web UI',
        description: 'Visual dashboard for node operators',
        priority: 'LOW',
        packages: ['mcp', 'node'],
      },
      {
        id: 'GAP-007',
        area: 'API',
        title: 'Public REST API',
        description: 'Public API for third-party integrations',
        priority: 'LOW',
        packages: ['node', 'client'],
      },
    ],
  },

  // ─────────────────────────────────────────────────────────────────────────
  // ECOSYSTEM PROJECTS
  // ─────────────────────────────────────────────────────────────────────────
  ecosystem: {
    cynic: { path: '/workspaces/CYNIC-new', branch: 'main', status: 'ACTIVE' },
    holdex: { path: '/workspaces/HolDex', branch: 'main', status: 'NEEDS_SYNC' },
    gasdf: { path: '/workspaces/GASdf', branch: 'test/web3-compat-swap', status: 'NEEDS_SYNC' },
    'asdf-brain': { path: '/workspaces/asdf-brain', branch: 'main', status: 'NEEDS_SYNC' },
    'asdf-manifesto': { path: '/workspaces/asdf-manifesto', branch: 'main', status: 'OK' },
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// META DASHBOARD SERVICE
// ═══════════════════════════════════════════════════════════════════════════

export class MetaDashboard {
  constructor() {
    this.architecture = CYNIC_ARCHITECTURE;
  }

  /**
   * Get complete self-analysis
   */
  analyze() {
    return {
      identity: this.getIdentity(),
      packages: this.getPackagesStatus(),
      dogs: this.getDogsStatus(),
      axioms: this.getAxiomsStatus(),
      gaps: this.getGaps(),
      metrics: this.getMetrics(),
      recommendations: this.getRecommendations(),
    };
  }

  /**
   * CYNIC identity summary
   */
  getIdentity() {
    return {
      name: 'CYNIC',
      etymology: 'κυνικός - comme un chien',
      maxConfidence: PHI_INV,
      version: this.architecture.version,
      tagline: 'Loyal to truth, not to comfort',
      axioms: Object.keys(this.architecture.axioms),
    };
  }

  /**
   * Package status overview
   */
  getPackagesStatus() {
    const packages = this.architecture.packages;
    const result = {};

    for (const [name, pkg] of Object.entries(packages)) {
      const moduleCount = Object.keys(pkg.modules).length;
      const completeModules = Object.values(pkg.modules).filter((m) => m.status === 'COMPLETE').length;

      result[name] = {
        name: pkg.name,
        version: pkg.version,
        purpose: pkg.purpose,
        status: pkg.status,
        completeness: pkg.completeness,
        modules: {
          total: moduleCount,
          complete: completeModules,
          ratio: completeModules / moduleCount,
        },
      };
    }

    // Calculate overall
    const allPkgs = Object.values(result);
    const avgCompleteness = allPkgs.reduce((sum, p) => sum + p.completeness, 0) / allPkgs.length;

    return {
      packages: result,
      overall: {
        total: allPkgs.length,
        avgCompleteness: Math.round(avgCompleteness * 100) / 100,
        fullyComplete: allPkgs.filter((p) => p.completeness >= 1.0).length,
      },
    };
  }

  /**
   * Dogs (Agents) status
   */
  getDogsStatus() {
    const dogs = this.architecture.dogs;

    return {
      v1: {
        active: Object.keys(dogs.v1).length,
        list: Object.entries(dogs.v1).map(([key, dog]) => ({
          id: key,
          name: dog.name,
          trigger: dog.trigger,
          blocking: dog.blocking,
          status: dog.status,
        })),
      },
      v2: {
        implemented: Object.keys(dogs.v2_collective).length,
        list: Object.entries(dogs.v2_collective).map(([key, dog]) => ({
          id: key,
          name: dog.name,
          sephirah: dog.sephirah,
          status: dog.status,
        })),
      },
      missing: {
        count: Object.keys(dogs.missing).length,
        list: Object.entries(dogs.missing).map(([key, dog]) => ({
          id: key,
          name: dog.name,
          purpose: dog.purpose,
          status: dog.status,
        })),
      },
      total: {
        implemented: Object.keys(dogs.v1).length + Object.keys(dogs.v2_collective).length,
        planned: Object.keys(dogs.v1).length + Object.keys(dogs.v2_collective).length + Object.keys(dogs.missing).length,
      },
    };
  }

  /**
   * Axioms (Worlds) status
   */
  getAxiomsStatus() {
    const axioms = this.architecture.axioms;

    return Object.entries(axioms).map(([name, axiom]) => ({
      name,
      symbol: axiom.symbol,
      world: axiom.world,
      question: axiom.question,
      color: axiom.color,
      dimensionCount: axiom.dimensions.length,
      dimensions: axiom.dimensions,
    }));
  }

  /**
   * Gap analysis
   */
  getGaps() {
    const gaps = this.architecture.gaps;

    return {
      critical: gaps.critical,
      important: gaps.important,
      wishlist: gaps.wishlist,
      summary: {
        critical: gaps.critical.length,
        important: gaps.important.length,
        wishlist: gaps.wishlist.length,
        total: gaps.critical.length + gaps.important.length + gaps.wishlist.length,
      },
    };
  }

  /**
   * Codebase metrics
   */
  getMetrics() {
    return {
      linesOfCode: '~29,324',
      packages: 6,
      testFiles: 26,
      dimensions: 25,
      mcpTools: 15,
      mcpServices: 7,
      dogsImplemented: 9,
      dogsPlanned: 14,
      consensusThreshold: `${PHI_INV * 100}%`,
      maxConfidence: `${PHI_INV * 100}%`,
    };
  }

  /**
   * Development recommendations
   */
  getRecommendations() {
    const gaps = this.architecture.gaps;

    // Priority based on gaps
    const priorities = [];

    // Critical first
    for (const gap of gaps.critical) {
      priorities.push({
        priority: 1,
        action: `Implement ${gap.title}`,
        reason: gap.description,
        packages: gap.packages,
        effort: 'HIGH',
      });
    }

    // Then important
    for (const gap of gaps.important) {
      priorities.push({
        priority: 2,
        action: `Implement ${gap.title}`,
        reason: gap.description,
        packages: gap.packages,
        effort: 'MEDIUM',
      });
    }

    // Immediate next steps
    const nextSteps = [
      {
        step: 1,
        action: 'Connect HolDex K-Score flow',
        description: 'Wire client.submitKScore() to live HolDex API',
      },
      {
        step: 2,
        action: 'Implement GASdf gasless transactions',
        description: 'Fee delegation for token burns',
      },
      {
        step: 3,
        action: 'Test multi-node consensus',
        description: 'Deploy 3+ nodes and verify φ-BFT consensus',
      },
      {
        step: 4,
        action: 'Add ZK privacy layer',
        description: 'Light Protocol integration for E-Score privacy',
      },
    ];

    return {
      priorities,
      nextSteps,
      philosophy: 'Small steps, verified at each stage. φ distrusts φ.',
    };
  }

  /**
   * Generate ASCII dashboard
   */
  toAscii() {
    const analysis = this.analyze();
    const lines = [];

    lines.push('');
    lines.push('═══════════════════════════════════════════════════════════════════════════');
    lines.push('                    CYNIC META DASHBOARD - Self Analysis');
    lines.push('                    "φ distrusts φ" - Max confidence 61.8%');
    lines.push('═══════════════════════════════════════════════════════════════════════════');
    lines.push('');

    // Identity
    lines.push('┌─ IDENTITY ───────────────────────────────────────────────────────────────┐');
    lines.push(`│  Name: ${analysis.identity.name} (${analysis.identity.etymology})`);
    lines.push(`│  Version: ${analysis.identity.version}`);
    lines.push(`│  Max Confidence: ${(analysis.identity.maxConfidence * 100).toFixed(1)}%`);
    lines.push(`│  Axioms: ${analysis.identity.axioms.join(' · ')}`);
    lines.push('└──────────────────────────────────────────────────────────────────────────┘');
    lines.push('');

    // Packages
    lines.push('┌─ PACKAGES ───────────────────────────────────────────────────────────────┐');
    for (const [name, pkg] of Object.entries(analysis.packages.packages)) {
      const bar = this._progressBar(pkg.completeness, 10);
      const status = pkg.status === 'COMPLETE' ? '✅' : pkg.status === 'ACTIVE_DEVELOPMENT' ? '🔄' : '⚠️';
      lines.push(`│  ${status} ${name.padEnd(12)} ${bar} ${(pkg.completeness * 100).toFixed(0).padStart(3)}%  ${pkg.purpose.substring(0, 35)}`);
    }
    lines.push(`│  ─────────────────────────────────────────────────────────────────────────`);
    lines.push(`│  Overall: ${analysis.packages.overall.fullyComplete}/${analysis.packages.overall.total} complete, avg ${(analysis.packages.overall.avgCompleteness * 100).toFixed(0)}%`);
    lines.push('└──────────────────────────────────────────────────────────────────────────┘');
    lines.push('');

    // Dogs
    lines.push('┌─ THE DOGS (Agents) ──────────────────────────────────────────────────────┐');
    lines.push('│  V1 (Active):');
    for (const dog of analysis.dogs.v1.list) {
      const block = dog.blocking ? '🛡️' : '👁️';
      lines.push(`│    ${block} ${dog.name.padEnd(10)} [${dog.trigger}]`);
    }
    lines.push('│  V2 (Collective):');
    for (const dog of analysis.dogs.v2.list) {
      lines.push(`│    🐕 ${dog.name.padEnd(20)} (${dog.sephirah})`);
    }
    lines.push('│  Missing (Spec only):');
    for (const dog of analysis.dogs.missing.list) {
      lines.push(`│    ⬜ ${dog.name.padEnd(10)} - ${dog.purpose}`);
    }
    lines.push(`│  ─────────────────────────────────────────────────────────────────────────`);
    lines.push(`│  Total: ${analysis.dogs.total.implemented}/${analysis.dogs.total.planned} implemented`);
    lines.push('└──────────────────────────────────────────────────────────────────────────┘');
    lines.push('');

    // Gaps
    lines.push('┌─ GAPS ───────────────────────────────────────────────────────────────────┐');
    lines.push(`│  🔴 Critical: ${analysis.gaps.summary.critical}`);
    for (const gap of analysis.gaps.critical) {
      lines.push(`│     └─ ${gap.title}`);
    }
    lines.push(`│  🟡 Important: ${analysis.gaps.summary.important}`);
    for (const gap of analysis.gaps.important) {
      lines.push(`│     └─ ${gap.title}`);
    }
    lines.push(`│  🟢 Wishlist: ${analysis.gaps.summary.wishlist}`);
    lines.push('└──────────────────────────────────────────────────────────────────────────┘');
    lines.push('');

    // Next Steps
    lines.push('┌─ RECOMMENDED NEXT STEPS ─────────────────────────────────────────────────┐');
    for (const step of analysis.recommendations.nextSteps) {
      lines.push(`│  ${step.step}. ${step.action}`);
      lines.push(`│     └─ ${step.description}`);
    }
    lines.push('└──────────────────────────────────────────────────────────────────────────┘');
    lines.push('');
    lines.push('═══════════════════════════════════════════════════════════════════════════');
    lines.push('                         *tail wag* Analysis complete.');
    lines.push('═══════════════════════════════════════════════════════════════════════════');

    return lines.join('\n');
  }

  /**
   * Generate JSON report
   */
  toJSON() {
    return JSON.stringify(this.analyze(), null, 2);
  }

  /**
   * Helper: progress bar
   */
  _progressBar(ratio, width = 10) {
    const filled = Math.round(ratio * width);
    const empty = width - filled;
    return '[' + '█'.repeat(filled) + '░'.repeat(empty) + ']';
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// MCP TOOL FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create brain_meta tool for MCP
 */
export function createMetaTool() {
  const dashboard = new MetaDashboard();

  return {
    name: 'brain_meta',
    description:
      "CYNIC self-analysis dashboard. Returns complete architecture status, gaps, and recommendations. Use this to understand CYNIC's current state and guide development.",
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['analyze', 'packages', 'dogs', 'axioms', 'gaps', 'recommendations', 'ascii', 'json'],
          description: 'Action: analyze (full), packages, dogs, axioms, gaps, recommendations, ascii (visual), json (raw)',
        },
        verbose: {
          type: 'boolean',
          description: 'Include detailed module information',
        },
      },
    },

    handler: async ({ action = 'analyze', verbose = false }) => {
      switch (action) {
        case 'packages':
          return { content: [{ type: 'text', text: JSON.stringify(dashboard.getPackagesStatus(), null, 2) }] };

        case 'dogs':
          return { content: [{ type: 'text', text: JSON.stringify(dashboard.getDogsStatus(), null, 2) }] };

        case 'axioms':
          return { content: [{ type: 'text', text: JSON.stringify(dashboard.getAxiomsStatus(), null, 2) }] };

        case 'gaps':
          return { content: [{ type: 'text', text: JSON.stringify(dashboard.getGaps(), null, 2) }] };

        case 'recommendations':
          return { content: [{ type: 'text', text: JSON.stringify(dashboard.getRecommendations(), null, 2) }] };

        case 'ascii':
          return { content: [{ type: 'text', text: dashboard.toAscii() }] };

        case 'json':
          return { content: [{ type: 'text', text: dashboard.toJSON() }] };

        case 'analyze':
        default:
          // Return ASCII by default for readability
          const ascii = dashboard.toAscii();
          const analysis = dashboard.analyze();

          return {
            content: [
              { type: 'text', text: ascii },
              {
                type: 'text',
                text: verbose ? `\n\nDetailed JSON:\n${JSON.stringify(analysis, null, 2)}` : '',
              },
            ],
          };
      }
    },
  };
}

// Default export
export default MetaDashboard;
