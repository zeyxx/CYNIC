/**
 * CYNIC LSP Service
 *
 * Provides Language Server Protocol-style operations for code intelligence:
 * - Symbol extraction (functions, classes, methods)
 * - Reference finding
 * - Refactoring support
 * - Call graph analysis
 *
 * Uses regex + simple AST patterns for dependency-free operation.
 * Inspired by oh-my-opencode's LSP integration approach.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/mcp/lsp-service
 */

'use strict';

import { readFile, readdir, stat } from 'node:fs/promises';
import { join, relative, extname, basename, dirname } from 'node:path';
import { createLogger } from '@cynic/core';

const log = createLogger('LspService');

// φ-derived constants
const PHI = 1.618033988749895;
const MAX_FILE_SIZE = Math.round(1000 * PHI * PHI); // ~2618 KB
const MAX_SYMBOLS_PER_FILE = Math.round(100 * PHI); // ~162

/**
 * Symbol kinds (LSP-compatible)
 */
export const SymbolKind = {
  File: 1,
  Module: 2,
  Namespace: 3,
  Package: 4,
  Class: 5,
  Method: 6,
  Property: 7,
  Field: 8,
  Constructor: 9,
  Enum: 10,
  Interface: 11,
  Function: 12,
  Variable: 13,
  Constant: 14,
  String: 15,
  Number: 16,
  Boolean: 17,
  Array: 18,
  Object: 19,
  Key: 20,
  Null: 21,
  EnumMember: 22,
  Struct: 23,
  Event: 24,
  Operator: 25,
  TypeParameter: 26,
};

/**
 * LSP Service for code intelligence
 */
export class LSPService {
  constructor(options = {}) {
    this.rootPath = options.rootPath || process.cwd();
    this.supportedExtensions = options.extensions || ['.js', '.mjs', '.ts', '.tsx', '.jsx'];
    this.cache = new Map();
    this.cacheTTL = options.cacheTTL || 60000; // 1 minute
    this.stats = {
      filesAnalyzed: 0,
      symbolsExtracted: 0,
      referencesFound: 0,
      cacheHits: 0,
      cacheMisses: 0,
    };
  }

  /**
   * Extract symbols from a file
   * @param {string} filePath - Path to file
   * @returns {Promise<Array>} Array of symbols
   */
  async getSymbols(filePath) {
    const cacheKey = `symbols:${filePath}`;
    const cached = this._getCache(cacheKey);
    if (cached) {
      this.stats.cacheHits++;
      return cached;
    }
    this.stats.cacheMisses++;

    try {
      const content = await readFile(filePath, 'utf-8');
      const symbols = this._extractSymbols(content, filePath);
      this._setCache(cacheKey, symbols);
      this.stats.filesAnalyzed++;
      this.stats.symbolsExtracted += symbols.length;
      return symbols;
    } catch (err) {
      return [];
    }
  }

  /**
   * Find all references to a symbol
   * @param {string} symbolName - Name of symbol to find
   * @param {string} [directory] - Directory to search in
   * @returns {Promise<Array>} Array of references
   */
  async findReferences(symbolName, directory = null) {
    const searchPath = directory || this.rootPath;
    const references = [];
    const files = await this._getFiles(searchPath);

    for (const file of files) {
      try {
        const content = await readFile(file, 'utf-8');
        const refs = this._findReferencesInContent(content, symbolName, file);
        references.push(...refs);
      } catch (err) {
        // Skip unreadable files
      }
    }

    this.stats.referencesFound += references.length;
    return references;
  }

  /**
   * Get call graph for a function/method
   * @param {string} symbolName - Name of function/method
   * @param {string} filePath - File containing the symbol
   * @returns {Promise<Object>} Call graph
   */
  async getCallGraph(symbolName, filePath) {
    const content = await readFile(filePath, 'utf-8');
    const symbols = this._extractSymbols(content, filePath);
    const symbol = symbols.find(s => s.name === symbolName);

    if (!symbol || !symbol.body) {
      return { symbol: symbolName, calls: [], calledBy: [] };
    }

    // Extract function calls from the body
    const calls = this._extractCalls(symbol.body);

    // Find who calls this function
    const references = await this.findReferences(symbolName);
    const calledBy = references
      .filter(ref => ref.context?.includes('('))
      .map(ref => ({
        file: ref.file,
        line: ref.line,
        caller: ref.containerSymbol || 'unknown',
      }));

    return {
      symbol: symbolName,
      file: filePath,
      calls,
      calledBy,
    };
  }

  /**
   * Suggest refactoring for a symbol rename
   * @param {string} oldName - Current name
   * @param {string} newName - New name
   * @param {string} [directory] - Directory to search
   * @returns {Promise<Object>} Refactoring plan
   */
  async planRename(oldName, newName, directory = null) {
    const references = await this.findReferences(oldName, directory);

    const changes = references.map(ref => ({
      file: ref.file,
      line: ref.line,
      column: ref.column,
      oldText: oldName,
      newText: newName,
      context: ref.context,
    }));

    return {
      oldName,
      newName,
      totalChanges: changes.length,
      filesAffected: [...new Set(changes.map(c => c.file))].length,
      changes,
      safe: this._isRenameSafe(oldName, newName, references),
    };
  }

  /**
   * Get document outline (hierarchical symbols)
   * @param {string} filePath - Path to file
   * @returns {Promise<Array>} Hierarchical symbol tree
   */
  async getOutline(filePath) {
    const symbols = await this.getSymbols(filePath);
    return this._buildOutlineTree(symbols);
  }

  /**
   * Analyze imports/dependencies
   * @param {string} filePath - Path to file
   * @returns {Promise<Object>} Import analysis
   */
  async analyzeImports(filePath) {
    try {
      const content = await readFile(filePath, 'utf-8');
      return this._extractImports(content, filePath);
    } catch (err) {
      return { imports: [], exports: [], errors: [err.message] };
    }
  }

  /**
   * Get hover information for a symbol
   * @param {string} symbolName - Symbol name
   * @param {string} filePath - File path
   * @returns {Promise<Object>} Hover info
   */
  async getHoverInfo(symbolName, filePath) {
    const symbols = await this.getSymbols(filePath);
    const symbol = symbols.find(s => s.name === symbolName);

    if (!symbol) {
      return null;
    }

    return {
      name: symbol.name,
      kind: symbol.kindName,
      signature: symbol.signature || symbol.name,
      documentation: symbol.documentation || null,
      file: relative(this.rootPath, filePath),
      line: symbol.line,
      exported: symbol.exported || false,
    };
  }

  /**
   * Get service statistics
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this.stats,
      cacheSize: this.cache.size,
      hitRate: this.stats.cacheHits / (this.stats.cacheHits + this.stats.cacheMisses) || 0,
    };
  }

  // ============ Private Methods ============

  /**
   * Extract symbols from content using regex patterns
   * @private
   */
  _extractSymbols(content, filePath) {
    const symbols = [];
    const lines = content.split('\n');

    // Patterns for different symbol types
    const patterns = [
      // ES6 class
      {
        regex: /^(\s*)(?:export\s+)?(?:default\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?/gm,
        kind: SymbolKind.Class,
        extract: (match) => ({
          name: match[2],
          extends: match[3] || null,
          exported: match[0].includes('export'),
        }),
      },
      // Function declaration
      {
        regex: /^(\s*)(?:export\s+)?(?:async\s+)?function\s*(\*?)\s*(\w+)\s*\(([^)]*)\)/gm,
        kind: SymbolKind.Function,
        extract: (match) => ({
          name: match[3],
          async: match[0].includes('async'),
          generator: match[2] === '*',
          params: match[4],
          exported: match[0].includes('export'),
        }),
      },
      // Arrow function (const/let/var)
      {
        regex: /^(\s*)(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\([^)]*\)\s*=>/gm,
        kind: SymbolKind.Function,
        extract: (match) => ({
          name: match[2],
          async: match[0].includes('async'),
          arrow: true,
          exported: match[0].includes('export'),
        }),
      },
      // Method in class (captures the method with its body start)
      {
        regex: /^\s+(?:static\s+)?(?:async\s+)?(?:get\s+|set\s+)?(\w+)\s*\([^)]*\)\s*\{/gm,
        kind: SymbolKind.Method,
        extract: (match) => ({
          name: match[1],
          static: match[0].includes('static'),
          async: match[0].includes('async'),
          getter: match[0].includes('get '),
          setter: match[0].includes('set '),
        }),
      },
      // Constant export
      {
        regex: /^(?:export\s+)?const\s+(\w+)\s*=/gm,
        kind: SymbolKind.Constant,
        extract: (match) => ({
          name: match[1],
          exported: match[0].includes('export'),
        }),
      },
    ];

    for (const pattern of patterns) {
      let match;
      pattern.regex.lastIndex = 0;

      while ((match = pattern.regex.exec(content)) !== null) {
        const lineNumber = content.slice(0, match.index).split('\n').length;
        const extracted = pattern.extract(match);

        // Skip constructor as separate symbol (it's part of class)
        if (extracted.name === 'constructor' && pattern.kind === SymbolKind.Method) {
          continue;
        }

        // Extract body for functions/methods
        let body = null;
        if (pattern.kind === SymbolKind.Function || pattern.kind === SymbolKind.Method) {
          body = this._extractBody(content, match.index);
        }

        // Extract JSDoc if present
        const documentation = this._extractJSDoc(content, match.index);

        symbols.push({
          name: extracted.name,
          kind: pattern.kind,
          kindName: Object.keys(SymbolKind).find(k => SymbolKind[k] === pattern.kind),
          line: lineNumber,
          column: match[1]?.length || 0,
          ...extracted,
          body,
          documentation,
          file: relative(this.rootPath, filePath),
        });

        if (symbols.length >= MAX_SYMBOLS_PER_FILE) break;
      }
    }

    return symbols;
  }

  /**
   * Extract function/method body
   * @private
   */
  _extractBody(content, startIndex) {
    let braceCount = 0;
    let started = false;
    let bodyStart = -1;

    for (let i = startIndex; i < content.length && i < startIndex + 10000; i++) {
      if (content[i] === '{') {
        if (!started) {
          started = true;
          bodyStart = i;
        }
        braceCount++;
      } else if (content[i] === '}') {
        braceCount--;
        if (started && braceCount === 0) {
          return content.slice(bodyStart, i + 1);
        }
      }
    }
    return null;
  }

  /**
   * Extract JSDoc comment before a symbol
   * @private
   */
  _extractJSDoc(content, symbolIndex) {
    // Look backwards for comment block
    const beforeSymbol = content.slice(Math.max(0, symbolIndex - 1000), symbolIndex);
    const jsdocMatch = beforeSymbol.match(/\/\*\*[\s\S]*?\*\/\s*$/);
    if (jsdocMatch) {
      return jsdocMatch[0]
        .replace(/^\/\*\*\s*/, '')
        .replace(/\s*\*\/$/, '')
        .replace(/^\s*\*\s?/gm, '')
        .trim();
    }
    return null;
  }

  /**
   * Find references to a symbol in content
   * @private
   */
  _findReferencesInContent(content, symbolName, filePath) {
    const references = [];
    const lines = content.split('\n');
    const regex = new RegExp(`\\b${this._escapeRegex(symbolName)}\\b`, 'g');

    lines.forEach((line, index) => {
      let match;
      while ((match = regex.exec(line)) !== null) {
        references.push({
          file: relative(this.rootPath, filePath),
          line: index + 1,
          column: match.index,
          context: line.trim().slice(0, 100),
          isDefinition: this._isDefinition(line, symbolName),
          isImport: line.includes('import') && line.includes(symbolName),
        });
      }
    });

    return references;
  }

  /**
   * Check if a line is a definition of the symbol
   * @private
   */
  _isDefinition(line, symbolName) {
    const defPatterns = [
      new RegExp(`class\\s+${symbolName}\\b`),
      new RegExp(`function\\s+${symbolName}\\b`),
      new RegExp(`(?:const|let|var)\\s+${symbolName}\\s*=`),
      new RegExp(`^\\s*${symbolName}\\s*\\(`), // Method definition
    ];
    return defPatterns.some(p => p.test(line));
  }

  /**
   * Extract function calls from body
   * @private
   */
  _extractCalls(body) {
    if (!body) return [];

    const calls = new Set();
    // Match function calls: name(
    const callRegex = /\b([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(/g;
    let match;

    while ((match = callRegex.exec(body)) !== null) {
      const name = match[1];
      // Filter out keywords
      if (!['if', 'for', 'while', 'switch', 'catch', 'function', 'return'].includes(name)) {
        calls.add(name);
      }
    }

    // Match method calls: this.name( or object.name(
    const methodRegex = /(?:this|[a-zA-Z_$][a-zA-Z0-9_$]*)\.([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(/g;
    while ((match = methodRegex.exec(body)) !== null) {
      calls.add(match[1]);
    }

    return [...calls];
  }

  /**
   * Extract imports from content
   * @private
   */
  _extractImports(content, filePath) {
    const imports = [];
    const exports = [];

    // ES6 imports
    const importRegex = /import\s+(?:(\{[^}]+\})|(\*\s+as\s+\w+)|(\w+))?\s*(?:,\s*(?:(\{[^}]+\})|(\w+)))?\s*from\s*['"]([^'"]+)['"]/g;
    let match;

    while ((match = importRegex.exec(content)) !== null) {
      const named = match[1] || match[4];
      const namespace = match[2];
      const defaultImport = match[3] || match[5];
      const source = match[6];

      imports.push({
        source,
        default: defaultImport || null,
        named: named ? named.replace(/[{}]/g, '').split(',').map(s => s.trim()) : [],
        namespace: namespace ? namespace.replace('* as ', '') : null,
        line: content.slice(0, match.index).split('\n').length,
      });
    }

    // ES6 exports
    const exportRegex = /export\s+(?:(default)\s+)?(?:(class|function|const|let|var)\s+)?(\w+)?/g;
    while ((match = exportRegex.exec(content)) !== null) {
      if (match[3]) {
        exports.push({
          name: match[3],
          default: match[1] === 'default',
          kind: match[2] || 'value',
          line: content.slice(0, match.index).split('\n').length,
        });
      }
    }

    return { imports, exports };
  }

  /**
   * Build hierarchical outline tree
   * @private
   */
  _buildOutlineTree(symbols) {
    const tree = [];
    let currentClass = null;

    for (const symbol of symbols) {
      if (symbol.kind === SymbolKind.Class) {
        currentClass = { ...symbol, children: [] };
        tree.push(currentClass);
      } else if (symbol.kind === SymbolKind.Method && currentClass) {
        currentClass.children.push(symbol);
      } else {
        tree.push(symbol);
        currentClass = null;
      }
    }

    return tree;
  }

  /**
   * Check if rename is safe
   * @private
   */
  _isRenameSafe(oldName, newName, references) {
    // Check for naming conflicts
    if (/^[0-9]/.test(newName)) return false;
    if (/\s/.test(newName)) return false;
    if (['if', 'for', 'while', 'class', 'function', 'const', 'let', 'var', 'return'].includes(newName)) {
      return false;
    }
    return true;
  }

  /**
   * Get all files in directory
   * @private
   */
  async _getFiles(directory) {
    const files = [];

    const scan = async (dir) => {
      try {
        const entries = await readdir(dir, { withFileTypes: true });
        for (const entry of entries) {
          const path = join(dir, entry.name);
          if (entry.isDirectory()) {
            if (!entry.name.startsWith('.') && entry.name !== 'node_modules') {
              await scan(path);
            }
          } else if (this.supportedExtensions.includes(extname(entry.name))) {
            files.push(path);
          }
        }
      } catch (err) {
        log.debug('Skipping inaccessible directory', { dir, error: err.code || err.message });
      }
    };

    await scan(directory);
    return files;
  }

  /**
   * Get cached value
   * @private
   */
  _getCache(key) {
    const entry = this.cache.get(key);
    if (entry && Date.now() - entry.time < this.cacheTTL) {
      return entry.value;
    }
    this.cache.delete(key);
    return null;
  }

  /**
   * Set cache value
   * @private
   */
  _setCache(key, value) {
    this.cache.set(key, { value, time: Date.now() });
  }

  /**
   * Escape regex special characters
   * @private
   */
  _escapeRegex(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }
}

/**
 * Create LSP tools for MCP
 * @param {LSPService} lsp - LSP service instance
 * @returns {Array} Array of tool definitions
 */
export function createLSPTools(lsp) {
  return [
    {
      name: 'brain_lsp_symbols',
      description: 'Extract symbols (functions, classes, methods) from a file. Returns structured symbol information with line numbers, documentation, and exports.',
      inputSchema: {
        type: 'object',
        properties: {
          file: { type: 'string', description: 'Relative path to file' },
        },
        required: ['file'],
      },
      handler: async (params) => {
        const filePath = join(lsp.rootPath, params.file);
        const symbols = await lsp.getSymbols(filePath);
        return {
          file: params.file,
          symbolCount: symbols.length,
          symbols: symbols.map(s => ({
            name: s.name,
            kind: s.kindName,
            line: s.line,
            exported: s.exported,
            signature: s.params ? `${s.name}(${s.params})` : s.name,
            doc: s.documentation?.slice(0, 100),
          })),
        };
      },
    },
    {
      name: 'brain_lsp_references',
      description: 'Find all references to a symbol across the codebase. Useful for understanding usage patterns and impact of changes.',
      inputSchema: {
        type: 'object',
        properties: {
          symbol: { type: 'string', description: 'Symbol name to find' },
          directory: { type: 'string', description: 'Directory to search (optional, defaults to project root)' },
        },
        required: ['symbol'],
      },
      handler: async (params) => {
        const refs = await lsp.findReferences(params.symbol, params.directory ? join(lsp.rootPath, params.directory) : null);
        return {
          symbol: params.symbol,
          totalReferences: refs.length,
          definitions: refs.filter(r => r.isDefinition).length,
          usages: refs.filter(r => !r.isDefinition && !r.isImport).length,
          imports: refs.filter(r => r.isImport).length,
          files: [...new Set(refs.map(r => r.file))],
          references: refs.slice(0, 50), // Limit to 50 for response size
        };
      },
    },
    {
      name: 'brain_lsp_callgraph',
      description: 'Get call graph for a function showing what it calls and what calls it. Useful for understanding code flow.',
      inputSchema: {
        type: 'object',
        properties: {
          symbol: { type: 'string', description: 'Function/method name' },
          file: { type: 'string', description: 'File containing the symbol' },
        },
        required: ['symbol', 'file'],
      },
      handler: async (params) => {
        const filePath = join(lsp.rootPath, params.file);
        return await lsp.getCallGraph(params.symbol, filePath);
      },
    },
    {
      name: 'brain_lsp_rename',
      description: 'Plan a symbol rename refactoring. Returns all locations that would need to change. Does NOT perform the rename.',
      inputSchema: {
        type: 'object',
        properties: {
          oldName: { type: 'string', description: 'Current symbol name' },
          newName: { type: 'string', description: 'New symbol name' },
          directory: { type: 'string', description: 'Directory to search (optional)' },
        },
        required: ['oldName', 'newName'],
      },
      handler: async (params) => {
        const plan = await lsp.planRename(
          params.oldName,
          params.newName,
          params.directory ? join(lsp.rootPath, params.directory) : null
        );
        return {
          ...plan,
          warning: plan.safe ? null : 'Rename may not be safe - review changes carefully',
          changes: plan.changes.slice(0, 30), // Limit for response size
        };
      },
    },
    {
      name: 'brain_lsp_outline',
      description: 'Get document outline (hierarchical symbol structure) for a file. Shows classes with their methods nested.',
      inputSchema: {
        type: 'object',
        properties: {
          file: { type: 'string', description: 'Relative path to file' },
        },
        required: ['file'],
      },
      handler: async (params) => {
        const filePath = join(lsp.rootPath, params.file);
        const outline = await lsp.getOutline(filePath);
        return {
          file: params.file,
          outline,
        };
      },
    },
    {
      name: 'brain_lsp_imports',
      description: 'Analyze imports and exports of a file. Shows dependencies and what the file exposes.',
      inputSchema: {
        type: 'object',
        properties: {
          file: { type: 'string', description: 'Relative path to file' },
        },
        required: ['file'],
      },
      handler: async (params) => {
        const filePath = join(lsp.rootPath, params.file);
        return await lsp.analyzeImports(filePath);
      },
    },
    {
      name: 'brain_lsp_stats',
      description: 'Get LSP service statistics including cache hit rate and analysis counts.',
      inputSchema: {
        type: 'object',
        properties: {},
      },
      handler: async () => {
        return lsp.getStats();
      },
    },
  ];
}

export default LSPService;
