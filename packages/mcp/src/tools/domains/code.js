/**
 * Code Domain Tools
 *
 * Tools for code analysis:
 * - Codebase: Codebase analysis
 * - VectorSearch: Semantic code search
 *
 * @module @cynic/mcp/tools/domains/code
 */

'use strict';

import { createCodeAnalyzer } from '../../code-analyzer.js';

/**
 * Create vector search tool definition
 * @param {Object} [options] - Options
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @param {Object} [options.embeddingConfig] - External embedding config (optional)
 * @returns {Object} Tool definition
 */
export function createVectorSearchTool(options = {}) {
  const { persistence, embeddingConfig } = options;

  // Lazy load semantic search
  let semanticSearch = null;
  let initialized = false;

  return {
    name: 'brain_vector_search',
    description: `Semantic vector search for finding similar content.
Uses TF-IDF embeddings by default (no external API needed).
Actions:
- search: Find similar documents by semantic meaning
- add: Add document to search index
- remove: Remove document from index
- stats: Get search engine statistics
- initialize: Initialize with existing documents`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['search', 'add', 'remove', 'stats', 'initialize'],
          description: 'Action to perform',
        },
        query: {
          type: 'string',
          description: 'Search query text (for search action)',
        },
        documentId: {
          type: 'string',
          description: 'Document ID (for add/remove actions)',
        },
        text: {
          type: 'string',
          description: 'Document text (for add action)',
        },
        metadata: {
          type: 'object',
          description: 'Document metadata (for add action)',
        },
        documents: {
          type: 'array',
          description: 'Array of { id, text, metadata } for initialize action',
        },
        limit: {
          type: 'number',
          description: 'Max results (default: 10)',
          default: 10,
        },
        threshold: {
          type: 'number',
          description: 'Minimum similarity threshold 0-1 (default: 0.382)',
          default: 0.382,
        },
        filter: {
          type: 'object',
          description: 'Metadata filter for search',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        query,
        documentId,
        text,
        metadata = {},
        documents,
        limit = 10,
        threshold = 0.382,
        filter = null,
      } = params;

      // Lazy load vector module
      if (!semanticSearch) {
        try {
          const core = await import('@cynic/core');

          // Choose embedder based on config
          let embedder;
          if (embeddingConfig?.apiKey) {
            embedder = new core.ExternalEmbedder(embeddingConfig);
          } else {
            embedder = new core.TfIdfEmbedder();
          }

          semanticSearch = new core.SemanticSearch({ embedder });
        } catch (e) {
          throw new Error(`Vector search module not available: ${e.message}`);
        }
      }

      switch (action) {
        case 'stats':
          return {
            ...semanticSearch.getStats(),
            initialized,
          };

        case 'initialize': {
          if (!documents || !Array.isArray(documents)) {
            // Try to load from persistence
            if (persistence) {
              try {
                const knowledge = await persistence.getRecentKnowledge(100);
                const docs = knowledge.map(k => ({
                  id: k.knowledge_id,
                  text: k.content || k.summary || '',
                  metadata: {
                    sourceType: k.source_type,
                    category: k.category,
                  },
                })).filter(d => d.text);

                await semanticSearch.initialize(docs);
                initialized = true;

                return {
                  action: 'initialize',
                  documentsLoaded: docs.length,
                  source: 'persistence',
                };
              } catch (e) {
                throw new Error(`Failed to load from persistence: ${e.message}`);
              }
            }
            throw new Error('documents array required for initialize action');
          }

          await semanticSearch.initialize(documents);
          initialized = true;

          return {
            action: 'initialize',
            documentsLoaded: documents.length,
            source: 'provided',
          };
        }

        case 'add': {
          if (!documentId) throw new Error('documentId required for add action');
          if (!text) throw new Error('text required for add action');

          await semanticSearch.addDocument(documentId, text, metadata);

          return {
            action: 'add',
            documentId,
            textLength: text.length,
            metadata,
          };
        }

        case 'remove': {
          if (!documentId) throw new Error('documentId required for remove action');

          semanticSearch.removeDocument(documentId);

          return {
            action: 'remove',
            documentId,
            removed: true,
          };
        }

        case 'search': {
          if (!query) throw new Error('query required for search action');

          // Auto-initialize if empty
          if (!initialized && semanticSearch.index.size() === 0) {
            // Try to initialize from persistence
            if (persistence) {
              try {
                const knowledge = await persistence.getRecentKnowledge(100);
                const docs = knowledge.map(k => ({
                  id: k.knowledge_id,
                  text: k.content || k.summary || '',
                  metadata: {
                    sourceType: k.source_type,
                    category: k.category,
                  },
                })).filter(d => d.text);

                if (docs.length > 0) {
                  await semanticSearch.initialize(docs);
                  initialized = true;
                }
              } catch (e) {
                // Non-blocking, continue with empty index
              }
            }
          }

          const results = await semanticSearch.search(query, {
            limit,
            threshold,
            filter,
          });

          return {
            action: 'search',
            query,
            results: results.map(r => ({
              id: r.id,
              similarity: r.similarity,
              confidence: r.confidence,
              textPreview: r.text ? r.text.slice(0, 200) + (r.text.length > 200 ? '...' : '') : null,
              metadata: r.metadata,
            })),
            totalResults: results.length,
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

/**
 * Create codebase analyzer tool definition
 * @param {Object} [options] - Options including rootPath
 * @returns {Object} Tool definition
 */
export function createCodebaseTool(options = {}) {
  // Lazily create analyzer on first use
  let analyzer = null;
  let codebaseIndexer = null;

  const getAnalyzer = () => {
    if (!analyzer) {
      analyzer = createCodeAnalyzer(options);
    }
    return analyzer;
  };

  // Lazy load CodebaseIndexer for graph queries
  const getIndexer = async () => {
    if (!codebaseIndexer) {
      try {
        const mod = await import('@cynic/persistence/services/codebase-indexer');
        const CodebaseIndexer = mod.CodebaseIndexer || mod.default;
        codebaseIndexer = new CodebaseIndexer({
          rootDir: options.rootPath || process.cwd(),
          userId: 'mcp',
          sessionId: `mcp-${Date.now()}`,
        });
      } catch (e) {
        throw new Error(`CodebaseIndexer not available: ${e.message}`);
      }
    }
    return codebaseIndexer;
  };

  return {
    name: 'brain_codebase',
    description: `Analyze CYNIC codebase structure. Get package hierarchy, search symbols, view metrics, and query dependency graphs.

Actions:
- tree: Full package hierarchy for visualization
- package: Single package details
- search: Find symbols by name
- stats: Codebase metrics
- graph: Query dependency graph (SUPERMEMORY) - find imports/exports
- dependents: Find what depends on a file
- invalidate: Clear cache`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['tree', 'package', 'search', 'stats', 'graph', 'dependents', 'invalidate'],
          description: 'Action to perform',
        },
        name: {
          type: 'string',
          description: 'Package name for "package" action (e.g., "node", "mcp")',
        },
        query: {
          type: 'string',
          description: 'Search query for "search" action, or file/symbol for "graph"/"dependents" actions',
        },
        maxDepth: {
          type: 'number',
          description: 'Max traversal depth for "graph" action (default: 3)',
          default: 3,
        },
      },
    },
    handler: async (params) => {
      const { action = 'tree', name, query } = params;
      const codeAnalyzer = getAnalyzer();

      switch (action) {
        case 'tree': {
          const tree = await codeAnalyzer.getTree();
          return {
            action: 'tree',
            root: tree.root,
            packages: tree.packages.map(pkg => ({
              name: pkg.name,
              shortName: pkg.shortName,
              path: pkg.path,
              color: pkg.color,
              stats: pkg.stats,
              modules: pkg.modules.map(mod => ({
                name: mod.name,
                path: mod.path,
                lines: mod.lines,
                classes: mod.classes?.map(cls => ({
                  name: cls.name,
                  line: cls.line,
                  methodCount: cls.methods?.length || 0,
                  methods: cls.methods?.map(m => ({
                    name: m.name,
                    line: m.line,
                    params: m.params,
                    visibility: m.visibility,
                  })),
                })) || [],
                functions: mod.functions?.map(fn => ({
                  name: fn.name,
                  line: fn.line,
                  params: fn.params,
                  exported: fn.exported,
                })) || [],
              })),
            })),
            stats: tree.stats,
            message: `*tail wag* Scanned ${tree.stats.packages} packages, ${tree.stats.classes} classes, ${tree.stats.methods} methods.`,
            timestamp: Date.now(),
          };
        }

        case 'package': {
          if (!name) {
            return {
              error: 'name required for package action',
              hint: 'Provide package name like "node", "mcp", "core"',
              timestamp: Date.now(),
            };
          }
          const pkg = await codeAnalyzer.getPackage(name);
          if (!pkg) {
            return {
              error: `Package "${name}" not found`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'package',
            package: pkg,
            message: `*ears perk* Package ${pkg.name}: ${pkg.stats.modules} modules, ${pkg.stats.classes} classes.`,
            timestamp: Date.now(),
          };
        }

        case 'search': {
          if (!query) {
            return {
              error: 'query required for search action',
              hint: 'Search for class, method, or function names',
              timestamp: Date.now(),
            };
          }
          const results = await codeAnalyzer.search(query);
          return {
            action: 'search',
            query,
            results: results.slice(0, 50), // Limit results
            total: results.length,
            message: results.length > 0
              ? `*sniff* Found ${results.length} symbols matching "${query}".`
              : `*head tilt* No symbols found matching "${query}".`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = await codeAnalyzer.getStats();
          return {
            action: 'stats',
            stats,
            message: `*tail wag* Codebase: ${stats.packages} packages, ${stats.modules} modules, ${stats.lines} lines.`,
            timestamp: Date.now(),
          };
        }

        case 'invalidate': {
          codeAnalyzer.invalidateCache();
          codebaseIndexer = null; // Reset indexer too
          return {
            action: 'invalidate',
            message: '*growl* Cache invalidated. Next request will rescan.',
            timestamp: Date.now(),
          };
        }

        case 'graph': {
          if (!query) {
            return {
              error: 'query required for graph action',
              hint: 'Provide file path or symbol name to trace dependencies',
              timestamp: Date.now(),
            };
          }

          try {
            const indexer = await getIndexer();
            const result = await indexer.queryDependencyGraph(query, {
              maxDepth: params.maxDepth || 3,
            });

            return {
              action: 'graph',
              query,
              nodes: result.nodes || [],
              edges: result.edges || [],
              stats: result.stats || {},
              message: result.error
                ? `*head tilt* ${result.error}`
                : `*sniff* Found ${result.nodes?.length || 0} nodes in dependency graph.`,
              timestamp: Date.now(),
            };
          } catch (e) {
            return {
              action: 'graph',
              error: e.message,
              timestamp: Date.now(),
            };
          }
        }

        case 'dependents': {
          if (!query) {
            return {
              error: 'query required for dependents action',
              hint: 'Provide file path to find what depends on it',
              timestamp: Date.now(),
            };
          }

          try {
            const indexer = await getIndexer();
            const dependents = await indexer.getReverseDependencies(query);

            return {
              action: 'dependents',
              query,
              dependents: dependents.slice(0, 50),
              total: dependents.length,
              message: dependents.length > 0
                ? `*ears perk* Found ${dependents.length} files that depend on "${query}".`
                : `*head tilt* No files depend on "${query}".`,
              timestamp: Date.now(),
            };
          } catch (e) {
            return {
              action: 'dependents',
              error: e.message,
              timestamp: Date.now(),
            };
          }
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['tree', 'package', 'search', 'stats', 'graph', 'dependents', 'invalidate'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Factory for code domain tools
 */
export const codeFactory = {
  name: 'code',
  domain: 'code',
  requires: [],

  /**
   * Create all code domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { persistence, embeddingConfig, codebaseOptions } = options;

    const tools = [];

    // Codebase analysis tool
    tools.push(createCodebaseTool(codebaseOptions || options));

    // Vector search tool
    tools.push(createVectorSearchTool({ persistence, embeddingConfig }));

    return tools;
  },
};
