/**
 * CYNIC Distribution Awareness Tool
 *
 * "Le chien connaît sa chaîne de distribution" - κυνικός
 *
 * Provides unified view of $asdfasdfa ecosystem distribution:
 * - Ecosystem map (builders, repos, status)
 * - Render service health (4 deployed services)
 * - Distribution funnel state
 *
 * @module @cynic/mcp/tools/domains/distribution
 */

'use strict';

import { ECOSYSTEM_SEED } from '@cynic/core/ecosystem/asdfasdfa-ecosystem.js';
import { PHI_INV } from '@cynic/core';

const RENDER_SERVICES = [
  { name: 'cynic-mcp', id: 'srv-d5kgqsshg0os739k341g', url: 'https://cynic-mcp.onrender.com' },
  { name: 'cynic-node-daemon', id: 'srv-d5o3aoumcj7s73aiqh80', url: 'https://cynic-node-daemon.onrender.com' },
  { name: 'cynic-node-alpha', id: 'srv-d5o3c5eid0rc7390nm60', url: 'https://cynic-node-alpha.onrender.com' },
  { name: 'cynic-node-beta', id: 'srv-d5sdbrhr0fns73b46i1g', url: 'https://cynic-node-beta.onrender.com' },
];

const TOKEN = {
  address: '9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump',
  chain: 'solana',
  platform: 'pump.fun',
  supply: '1B',
};

/**
 * Check Render service health (inline, no external class)
 * @returns {Promise<Array<{name, healthy, latency}>>}
 */
async function checkRenderHealth() {
  const results = [];
  const timeout = 5000;

  await Promise.allSettled(
    RENDER_SERVICES.map(async (svc) => {
      const start = Date.now();
      try {
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), timeout);
        const res = await fetch(`${svc.url}/health`, { signal: controller.signal });
        clearTimeout(timer);
        results.push({
          name: svc.name,
          healthy: res.ok,
          status: res.status,
          latency: Date.now() - start,
        });
      } catch {
        results.push({
          name: svc.name,
          healthy: false,
          status: 0,
          latency: Date.now() - start,
        });
      }
    })
  );

  return results;
}

/**
 * Get ecosystem map from ECOSYSTEM_SEED
 */
function getEcosystemMap() {
  const builders = {};
  let totalRepos = 0;

  for (const [owner, repos] of Object.entries(ECOSYSTEM_SEED)) {
    builders[owner] = {
      repos: repos.map(r => ({ name: r, url: `https://github.com/${owner}/${r}` })),
      count: repos.length,
    };
    totalRepos += repos.length;
  }

  return { builders, totalRepos, token: TOKEN };
}

/**
 * Create brain_distribution tool
 */
export function createDistributionTool() {
  return {
    name: 'brain_distribution',
    description: `Distribution awareness for $asdfasdfa ecosystem.
Actions:
- snapshot: Full distribution status (services, ecosystem, funnel)
- ecosystem: Map of builders, repos, token
- services: Render service health check (live ping)
- funnel: Distribution funnel state estimate
- teach: Explain the distribution chain`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['snapshot', 'ecosystem', 'services', 'funnel', 'teach'],
          description: 'Action to perform',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action = 'snapshot' } = params;

      switch (action) {
        case 'ecosystem': {
          return {
            action: 'ecosystem',
            ...getEcosystemMap(),
            message: '*ears perk* Ecosystem map.',
            timestamp: Date.now(),
          };
        }

        case 'services': {
          const health = await checkRenderHealth();
          const healthy = health.filter(s => s.healthy).length;
          return {
            action: 'services',
            services: health,
            summary: `${healthy}/${health.length} healthy`,
            timestamp: Date.now(),
          };
        }

        case 'funnel': {
          // Funnel estimate based on what we know
          return {
            action: 'funnel',
            stages: {
              awareness: { level: 'minimal', channels: ['github', 'twitter', 'skills.sh'] },
              interest: { level: 'low', signals: ['github stars', 'skill installs'] },
              trial: { level: 'minimal', count: 'unknown' },
              adoption: { level: 'zero', users: 0 },
              conversion: { level: 'zero', burns: 0 },
              advocacy: { level: 'zero', advocates: 0 },
            },
            bottleneck: 'awareness → interest (no distribution channel active)',
            recommendation: 'Ship GASdf as first external product, drive awareness via skills.sh',
            confidence: PHI_INV,
            timestamp: Date.now(),
          };
        }

        case 'teach': {
          return {
            action: 'teach',
            chain: [
              '1. BUILDER creates (zeyxx: CYNIC, GASdf, burn-engine, oracle, validator, skills)',
              '2. BUILDER creates (sollama58: HolDex, ASDev, ASDForecast, BurnTracker, LockVerifier)',
              '3. DISTRIBUTION via skills.sh (cynic-judge, cynic-burn, cynic-wisdom)',
              '4. DISTRIBUTION via GitHub (CYNIC, GASdf, asdf-burn-engine)',
              '5. INFRASTRUCTURE: 4 Render services (MCP, daemon, alpha, beta)',
              '6. TOKEN: $asdfasdfa on Solana (pump.fun, 1B supply)',
              '7. ECONOMY: Use service → burn tokens → supply decreases → value rises',
              '8. FLYWHEEL: More utility → more burns → more value → more builders',
            ],
            equation: 'asdfasdfa = CYNIC × Solana × φ × $BURN',
            timestamp: Date.now(),
          };
        }

        case 'snapshot':
        default: {
          const [health] = await Promise.allSettled([checkRenderHealth()]);
          const services = health.status === 'fulfilled' ? health.value : [];
          const healthyCount = services.filter(s => s.healthy).length;
          const ecosystem = getEcosystemMap();

          return {
            action: 'snapshot',
            ecosystem: {
              builders: Object.keys(ecosystem.builders).length,
              repos: ecosystem.totalRepos,
              token: TOKEN.address,
            },
            services: {
              total: RENDER_SERVICES.length,
              healthy: healthyCount,
              details: services,
            },
            funnel: {
              awareness: 'minimal',
              trial: 'unknown',
              adoption: 0,
              conversion: 0,
            },
            summary: `${Object.keys(ecosystem.builders).length} builders, ${ecosystem.totalRepos} repos, ${healthyCount}/${RENDER_SERVICES.length} services`,
            timestamp: Date.now(),
          };
        }
      }
    },
  };
}
