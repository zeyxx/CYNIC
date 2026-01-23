/**
 * Ecosystem Domain Tools
 *
 * Tools for ecosystem management:
 * - Ecosystem: Repository tracking and updates
 * - EcosystemMonitor: Real-time monitoring
 * - Integrator: Cross-project synchronization
 * - Discovery: Repository discovery
 *
 * @module @cynic/mcp/tools/domains/ecosystem
 */

'use strict';

import { EcosystemMonitor, summarizeUpdates } from '@cynic/core';

/**
 * Create ecosystem monitor tool definition
 * Tracks external sources: GitHub, Twitter (TODO), Web
 * @param {Object} [options] - Options
 * @param {Object} [options.judge] - CYNICJudge instance for commit analysis
 * @param {Object} [options.persistence] - PersistenceManager for storing digests
 * @returns {Object} Tool definition
 */
export function createEcosystemMonitorTool(options = {}) {
  const { judge = null, persistence = null } = options;

  // Singleton monitor instance (created on first use)
  let monitorInstance;

  const getMonitor = () => {
    if (!monitorInstance) {
      monitorInstance = new EcosystemMonitor();
    }
    return monitorInstance;
  };

  /**
   * Digest a single commit/update for learning
   * @param {Object} update - Update from ecosystem monitor
   * @returns {Promise<Object>} Digest result
   */
  const digestUpdate = async (update) => {
    const content = `${update.title || ''}\n\n${update.description || ''}`;
    const source = update.url || `${update.source}:${update.id}`;

    // Extract patterns from commit message
    const patterns = [];
    const commitType = update.title?.match(/^(feat|fix|docs|chore|refactor|test|style|perf|ci|build)(\(.*?\))?:/i);
    if (commitType) {
      patterns.push({ type: 'commit_convention', value: commitType[1].toLowerCase() });
    }

    // Extract insights
    const insights = [];
    if (update.priority === 'CRITICAL' || update.priority === 'HIGH') {
      insights.push({ importance: 'high', reason: `Priority: ${update.priority}` });
    }
    if (update.type === 'RELEASE') {
      insights.push({ type: 'release', version: update.id });
    }

    const digest = {
      id: `dig_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      source,
      sourceType: 'commit',
      content: content.slice(0, 500),
      patterns,
      insights,
      metadata: {
        updateType: update.type,
        priority: update.priority,
        author: update.author,
        repo: update.meta?.repo,
        sha: update.meta?.sha,
        timestamp: update.timestamp,
      },
      digestedAt: Date.now(),
    };

    // Store if persistence available
    if (persistence?.storeDigest) {
      try {
        await persistence.storeDigest(digest);
      } catch (e) {
        // Non-blocking
      }
    }

    // If judge available, create a quick judgment for pattern extraction
    if (judge) {
      try {
        const judgment = await judge.judge({
          type: 'commit',
          content: content.slice(0, 200),
          sources: [source],
        });
        digest.qScore = judgment.Q;
        digest.verdict = judgment.verdict;
      } catch (e) {
        // Non-blocking
      }
    }

    return digest;
  };

  return {
    name: 'brain_ecosystem_monitor',
    description: `Monitor external ecosystem sources for updates.
Actions:
- track: Add a GitHub repo to track (owner, repo required)
- untrack: Remove a source (sourceId required)
- sources: List all tracked sources
- fetch: Fetch updates from one source (sourceId) or all sources
- updates: Get recent updates from cache
- defaults: Register default Solana ecosystem sources
- discover: Discover new relevant sources
- status: Get monitor status
- analyze: Analyze a specific update (updateIndex required)
- analyze_all: Analyze all unanalyzed updates (batch learning)`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['track', 'untrack', 'sources', 'fetch', 'updates', 'defaults', 'discover', 'status', 'analyze', 'analyze_all'],
          description: 'Action to perform',
        },
        owner: {
          type: 'string',
          description: 'GitHub repo owner (for track action)',
        },
        repo: {
          type: 'string',
          description: 'GitHub repo name (for track action)',
        },
        sourceId: {
          type: 'string',
          description: 'Source ID (for untrack, fetch actions)',
        },
        trackReleases: {
          type: 'boolean',
          description: 'Track releases (default true)',
        },
        trackCommits: {
          type: 'boolean',
          description: 'Track commits (default true)',
        },
        limit: {
          type: 'number',
          description: 'Max results (for updates action)',
        },
        minPriority: {
          type: 'string',
          enum: ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'INFO'],
          description: 'Minimum priority filter (for updates action)',
        },
        updateIndex: {
          type: 'number',
          description: 'Index of update to analyze (for analyze action)',
        },
        autoAnalyze: {
          type: 'boolean',
          description: 'Automatically analyze updates after fetch (default false)',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        owner,
        repo,
        sourceId,
        trackReleases = true,
        trackCommits = true,
        limit = 20,
        minPriority,
        updateIndex,
        autoAnalyze = false,
      } = params;

      const monitor = getMonitor();

      switch (action) {
        case 'track': {
          if (!owner || !repo) {
            throw new Error('owner and repo required for track action');
          }

          const id = monitor.trackGitHubRepo(owner, repo, {
            trackReleases,
            trackCommits,
          });

          return {
            success: true,
            sourceId: id,
            message: `*ears perk* Now tracking ${owner}/${repo}`,
            timestamp: Date.now(),
          };
        }

        case 'untrack': {
          if (!sourceId) {
            throw new Error('sourceId required for untrack action');
          }

          const success = monitor.unregisterSource(sourceId);

          return {
            success,
            sourceId,
            message: success ? '*nod* Source removed' : 'Source not found',
            timestamp: Date.now(),
          };
        }

        case 'sources': {
          const sources = monitor.listSources();

          return {
            sources,
            total: sources.length,
            message: `*sniff* Tracking ${sources.length} sources`,
            timestamp: Date.now(),
          };
        }

        case 'fetch': {
          if (sourceId) {
            // Fetch single source
            const result = await monitor.fetchSource(sourceId);

            // Auto-analyze if enabled
            let analyzed = 0;
            if (autoAnalyze && result.success && result.updates?.length > 0) {
              for (const update of result.updates.slice(0, 10)) {
                try {
                  await digestUpdate(update);
                  analyzed++;
                } catch (e) {
                  // Non-blocking
                }
              }
            }

            return {
              ...result,
              analyzed: autoAnalyze ? analyzed : undefined,
              message: result.success
                ? `*tail wag* Fetched ${result.updates?.length || 0} updates${autoAnalyze ? `, analyzed ${analyzed}` : ''}`
                : `*head tilt* ${result.reason || result.error}`,
              timestamp: Date.now(),
            };
          } else {
            // Fetch all
            const results = await monitor.fetchAll();
            const summary = summarizeUpdates(results.updates);

            // Auto-analyze if enabled
            let analyzed = 0;
            if (autoAnalyze && results.updates.length > 0) {
              for (const update of results.updates.slice(0, 20)) {
                try {
                  await digestUpdate(update);
                  analyzed++;
                } catch (e) {
                  // Non-blocking
                }
              }
            }

            return {
              fetched: results.fetched,
              skipped: results.skipped,
              errors: results.errors,
              totalUpdates: results.updates.length,
              analyzed: autoAnalyze ? analyzed : undefined,
              summary,
              message: `*sniff* Fetched ${results.updates.length} updates from ${results.fetched} sources${autoAnalyze ? `, analyzed ${analyzed}` : ''}`,
              timestamp: Date.now(),
            };
          }
        }

        case 'updates': {
          const updates = monitor.getRecentUpdates({
            limit,
            minPriority,
          });

          const summary = summarizeUpdates(updates);

          return {
            updates: updates.map(u => ({
              type: u.type,
              title: u.title,
              url: u.url,
              priority: u.priority,
              source: u.source,
              timestamp: u.timestamp,
            })),
            summary,
            total: updates.length,
            message: `*ears perk* ${updates.length} recent updates`,
            timestamp: Date.now(),
          };
        }

        case 'defaults': {
          const sources = monitor.registerSolanaDefaults();

          return {
            sources,
            total: sources.length,
            message: '*tail wag* Solana ecosystem defaults registered',
            timestamp: Date.now(),
          };
        }

        case 'discover': {
          const suggestions = await monitor.discoverSources();

          return {
            suggestions,
            total: suggestions.length,
            message: `*sniff* Found ${suggestions.length} potential sources`,
            timestamp: Date.now(),
          };
        }

        case 'status': {
          const status = monitor.getStatus();

          return {
            ...status,
            message: `*nod* Monitoring ${status.sources.length} sources`,
            timestamp: Date.now(),
          };
        }

        case 'analyze': {
          if (updateIndex === undefined) {
            throw new Error('updateIndex required for analyze action');
          }

          const updates = monitor.getRecentUpdates({ limit: 100 });
          if (updateIndex < 0 || updateIndex >= updates.length) {
            throw new Error(`Invalid updateIndex: ${updateIndex} (have ${updates.length} updates)`);
          }

          const update = updates[updateIndex];
          const digest = await digestUpdate(update);

          return {
            digest,
            update: {
              type: update.type,
              title: update.title,
              url: update.url,
            },
            message: `*sniff* Analyzed commit: ${digest.verdict || 'digested'}`,
            timestamp: Date.now(),
          };
        }

        case 'analyze_all': {
          const updates = monitor.getRecentUpdates({ limit: limit || 50 });
          const results = {
            analyzed: 0,
            skipped: 0,
            digests: [],
            byVerdict: {},
            byType: {},
          };

          for (const update of updates) {
            try {
              const digest = await digestUpdate(update);
              results.analyzed++;
              results.digests.push({
                id: digest.id,
                source: digest.source,
                qScore: digest.qScore,
                verdict: digest.verdict,
              });

              // Track verdicts
              if (digest.verdict) {
                results.byVerdict[digest.verdict] = (results.byVerdict[digest.verdict] || 0) + 1;
              }

              // Track types
              const commitType = digest.patterns?.find(p => p.type === 'commit_convention')?.value || 'other';
              results.byType[commitType] = (results.byType[commitType] || 0) + 1;
            } catch (e) {
              results.skipped++;
            }
          }

          return {
            ...results,
            total: updates.length,
            message: `*tail wag* Analyzed ${results.analyzed} commits, learned ${Object.keys(results.byType).length} patterns`,
            timestamp: Date.now(),
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

/**
 * Create ecosystem docs tool definition
 * @param {Object} ecosystem - EcosystemService instance
 * @returns {Object} Tool definition
 */
export function createEcosystemTool(ecosystem) {
  return {
    name: 'brain_ecosystem',
    description: 'Access pre-loaded ecosystem documentation (CLAUDE.md, API docs, architecture). Provides context for the $ASDFASDFA ecosystem projects.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['get', 'search', 'list', 'context', 'refresh', 'stats'],
          description: 'Action: get (specific doc), search (query), list (all docs), context (relevant docs for task), refresh (reload), stats',
        },
        project: {
          type: 'string',
          description: 'Project name (cynic, holdex, gasdf, ecosystem, asdf-brain)',
        },
        docType: {
          type: 'string',
          description: 'Document type (claude_md, api_readme, architecture, harmony)',
        },
        query: {
          type: 'string',
          description: 'Search query or context description',
        },
        limit: {
          type: 'number',
          description: 'Maximum results (default 10)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'list', project, docType, query, limit = 10 } = params;

      if (!ecosystem) {
        return {
          error: 'Ecosystem service not available',
          hint: 'Ecosystem docs provide pre-loaded project context',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'get': {
          if (!project || !docType) {
            return {
              error: 'Both project and docType required for get action',
              hint: 'Use action="list" to see available documents',
              timestamp: Date.now(),
            };
          }
          const doc = await ecosystem.get(project, docType);
          if (!doc) {
            return {
              error: `Document not found: ${project}/${docType}`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'get',
            project: doc.project,
            docType: doc.doc_type || doc.docType,
            content: doc.content,
            digest: doc.digest,
            hasDigest: !!doc.digest,
            message: `*ears perk* Found ${project}/${docType}.`,
            timestamp: Date.now(),
          };
        }

        case 'search': {
          if (!query) {
            return {
              error: 'query required for search action',
              timestamp: Date.now(),
            };
          }
          const results = await ecosystem.search(query, { limit });
          return {
            action: 'search',
            query,
            results,
            total: results.length,
            message: `*sniff* Found ${results.length} matches.`,
            timestamp: Date.now(),
          };
        }

        case 'list': {
          const docs = await ecosystem.list();
          return {
            action: 'list',
            documents: docs.map(d => ({
              project: d.project,
              docType: d.doc_type || d.docType,
              filePath: d.file_path || d.filePath,
              hasDigest: !!d.digest,
            })),
            total: docs.length,
            message: `*tail wag* ${docs.length} ecosystem docs loaded.`,
            timestamp: Date.now(),
          };
        }

        case 'context': {
          if (!query) {
            return {
              error: 'query required for context action (describe your task)',
              timestamp: Date.now(),
            };
          }
          const context = await ecosystem.getContextFor(query, { maxDocs: limit });
          return {
            action: 'context',
            query,
            documents: context.documents.map(d => ({
              project: d.project,
              docType: d.docType,
              contentLength: d.content?.length || 0,
              hasDigest: !!d.digest,
            })),
            totalLength: context.totalLength,
            count: context.count,
            message: `*head tilt* Selected ${context.count} relevant docs (${context.totalLength} chars).`,
            timestamp: Date.now(),
          };
        }

        case 'refresh': {
          const results = await ecosystem.refresh();
          return {
            action: 'refresh',
            ...results,
            message: `*sniff* Refreshed: ${results.loaded} loaded, ${results.skipped} unchanged, ${results.failed} failed.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = await ecosystem.getStats();
          return {
            action: 'stats',
            ...stats,
            message: `*tail wag* ${stats.total_docs || stats.loadCount || 0} docs, ${stats.searchCount || 0} searches, ${stats.hitCount || 0} hits.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['get', 'search', 'list', 'context', 'refresh', 'stats'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create integrator tool definition
 * @param {Object} integrator - IntegratorService instance
 * @returns {Object} Tool definition
 */
export function createIntegratorTool(integrator) {
  return {
    name: 'brain_integrator',
    description: 'Cross-project integration and synchronization. Check sync status, detect drift, get suggestions for keeping shared modules aligned.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['check', 'drifts', 'suggest', 'projects', 'modules', 'stats'],
          description: 'Action: check (sync status), drifts (current drifts), suggest (sync suggestions), projects (project status), modules (shared modules), stats (service stats)',
        },
        project: {
          type: 'string',
          description: 'Project name (for projects action)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'check', project } = params;

      if (!integrator) {
        return {
          error: 'Integrator service not available',
          hint: 'IntegratorService tracks shared modules across projects',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'check': {
          const report = await integrator.checkSync();
          return {
            action: 'check',
            allSynced: report.allSynced,
            modulesChecked: report.modules.length,
            driftsFound: report.drifts.length,
            modules: report.modules.map(m => ({
              name: m.name,
              synced: m.synced,
              mirrorsCount: m.mirrors?.length || 0,
              driftsCount: m.drifts?.length || 0,
            })),
            message: report.allSynced
              ? `*tail wag* All ${report.modules.length} modules in sync!`
              : `*GROWL* Found ${report.drifts.length} drifts across ${report.modules.filter(m => !m.synced).length} modules.`,
            timestamp: Date.now(),
          };
        }

        case 'drifts': {
          const drifts = integrator.getDrifts();
          return {
            action: 'drifts',
            drifts: drifts.map(d => ({
              type: d.type,
              module: d.module,
              canonical: d.canonical,
              drifted: d.drifted,
              critical: d.critical,
            })),
            total: drifts.length,
            critical: drifts.filter(d => d.critical).length,
            message: drifts.length === 0
              ? `*tail wag* No drifts detected.`
              : `*sniff* ${drifts.length} drifts (${drifts.filter(d => d.critical).length} critical).`,
            timestamp: Date.now(),
          };
        }

        case 'suggest': {
          const suggestions = integrator.getSyncSuggestions();
          return {
            action: 'suggest',
            suggestions: suggestions.map(s => ({
              action: s.action,
              priority: s.priority,
              from: s.from,
              to: s.to,
              reason: s.reason,
              command: s.command,
            })),
            total: suggestions.length,
            highPriority: suggestions.filter(s => s.priority === 'high').length,
            message: suggestions.length === 0
              ? `*yawn* No sync actions needed.`
              : `*ears perk* ${suggestions.length} sync actions suggested (${suggestions.filter(s => s.priority === 'high').length} high priority).`,
            timestamp: Date.now(),
          };
        }

        case 'projects': {
          const status = await integrator.getProjectStatus(project);
          return {
            action: 'projects',
            ...status,
            message: `*sniff* ${status.available}/${status.total} projects available.`,
            timestamp: Date.now(),
          };
        }

        case 'modules': {
          const modules = integrator.getSharedModules();
          return {
            action: 'modules',
            modules,
            total: modules.length,
            critical: modules.filter(m => m.critical).length,
            message: `*tail wag* Tracking ${modules.length} shared modules (${modules.filter(m => m.critical).length} critical).`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = integrator.getStats();
          return {
            action: 'stats',
            ...stats,
            message: `*ears perk* ${stats.checksPerformed} checks, ${stats.driftsDetected} drifts detected.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['check', 'drifts', 'suggest', 'projects', 'modules', 'stats'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create discovery tool definition
 * Discovers MCP servers, Claude Code plugins, and CYNIC nodes
 * @param {Object} discovery - DiscoveryService instance
 * @returns {Object} Tool definition
 */
export function createDiscoveryTool(discovery) {
  return {
    name: 'brain_discovery',
    description: `Discover MCP servers, Claude Code plugins, and CYNIC nodes from repositories or endpoints.
Actions:
- scan_repo: Scan a GitHub repo for .mcp.json and plugin.json
- mcp_servers: List discovered MCP servers
- plugins: List discovered plugins
- nodes: List discovered CYNIC nodes
- register_node: Register a new CYNIC node
- discover_node: Probe an endpoint to discover a node
- health_check: Run health checks on all nodes
- stats: Get discovery statistics`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['scan_repo', 'mcp_servers', 'plugins', 'nodes', 'register_node', 'discover_node', 'health_check', 'stats'],
          description: 'Action to perform',
        },
        owner: { type: 'string', description: 'GitHub repo owner (for scan_repo)' },
        repo: { type: 'string', description: 'GitHub repo name (for scan_repo)' },
        endpoint: { type: 'string', description: 'Node endpoint URL (for register_node, discover_node)' },
        nodeName: { type: 'string', description: 'Node name (for register_node)' },
        status: { type: 'string', description: 'Filter by status' },
        limit: { type: 'number', description: 'Max results (default 50)' },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action, owner, repo, endpoint, nodeName, status, limit = 50 } = params;

      if (!discovery) {
        return {
          error: 'Discovery service not available',
          hint: 'Ensure DiscoveryService is initialized',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'scan_repo': {
          if (!owner || !repo) {
            return { error: 'owner and repo required for scan_repo' };
          }
          const results = await discovery.scanRepo(owner, repo);
          return {
            action: 'scan_repo',
            ...results,
            message: `*sniff* Scanned ${owner}/${repo}: ${results.mcpServers.length} MCP servers, ${results.plugin ? 1 : 0} plugin.`,
            timestamp: Date.now(),
          };
        }

        case 'mcp_servers': {
          const servers = await discovery.getMcpServers({ status, limit });
          return {
            action: 'mcp_servers',
            servers,
            total: servers.length,
            message: `*ears perk* Found ${servers.length} MCP servers.`,
            timestamp: Date.now(),
          };
        }

        case 'plugins': {
          const plugins = await discovery.getPlugins({ status, limit });
          return {
            action: 'plugins',
            plugins,
            total: plugins.length,
            message: `*tail wag* Found ${plugins.length} plugins.`,
            timestamp: Date.now(),
          };
        }

        case 'nodes': {
          const nodes = await discovery.getNodes({ status, limit });
          return {
            action: 'nodes',
            nodes,
            total: nodes.length,
            message: `*nod* Found ${nodes.length} CYNIC nodes.`,
            timestamp: Date.now(),
          };
        }

        case 'register_node': {
          if (!endpoint) {
            return { error: 'endpoint required for register_node' };
          }
          const node = await discovery.registerNode({ endpoint, nodeName });
          return {
            action: 'register_node',
            node,
            message: `*tail wag* Registered node at ${endpoint}.`,
            timestamp: Date.now(),
          };
        }

        case 'discover_node': {
          if (!endpoint) {
            return { error: 'endpoint required for discover_node' };
          }
          const node = await discovery.discoverNode(endpoint);
          if (node) {
            return {
              action: 'discover_node',
              node,
              message: `*ears perk* Discovered node at ${endpoint}.`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'discover_node',
            error: 'Node not reachable or not a CYNIC node',
            endpoint,
            timestamp: Date.now(),
          };
        }

        case 'health_check': {
          const results = await discovery.runNodeHealthChecks();
          return {
            action: 'health_check',
            ...results,
            message: `*sniff* Health check: ${results.healthy}/${results.checked} healthy.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = await discovery.getStats();
          return {
            action: 'stats',
            stats,
            message: `*nod* ${stats.mcp_servers || 0} MCP servers, ${stats.plugins || 0} plugins, ${stats.nodes || 0} nodes.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['scan_repo', 'mcp_servers', 'plugins', 'nodes', 'register_node', 'discover_node', 'health_check', 'stats'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Factory for ecosystem domain tools
 */
export const ecosystemFactory = {
  name: 'ecosystem',
  domain: 'ecosystem',
  requires: [],

  /**
   * Create all ecosystem domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      ecosystem,
      integrator,
      discovery,
      persistence,
      judge,
    } = options;

    const tools = [];

    // Ecosystem tool
    if (ecosystem) {
      tools.push(createEcosystemTool(ecosystem));
    }

    // Ecosystem monitor tool
    tools.push(createEcosystemMonitorTool({ persistence, judge }));

    // Integrator tool
    if (integrator) {
      tools.push(createIntegratorTool(integrator));
    }

    // Discovery tool
    if (discovery) {
      tools.push(createDiscoveryTool(discovery));
    }

    return tools;
  },
};
