/**
 * CYNIC Code Analyzer
 *
 * Scans the codebase and extracts symbols for 3D visualization.
 * Uses regex-based parsing for fast, dependency-free extraction.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/mcp/code-analyzer
 */

'use strict';

import { readdir, readFile, stat } from 'node:fs/promises';
import { join, relative, basename, dirname } from 'node:path';

/**
 * Code Analyzer Service
 * Extracts symbols from JavaScript files for visualization
 */
export class CodeAnalyzer {
  constructor(options = {}) {
    this.rootPath = options.rootPath || process.cwd();
    this.packagesPath = options.packagesPath || join(this.rootPath, 'packages');
    this.cache = null;
    this.cacheTime = 0;
    this.cacheTTL = options.cacheTTL || 30000; // 30 seconds
  }

  /**
   * Get full codebase tree
   * @returns {Promise<Object>} Hierarchical structure
   */
  async getTree() {
    // Check cache
    if (this.cache && Date.now() - this.cacheTime < this.cacheTTL) {
      return this.cache;
    }

    const packages = await this.scanPackages();

    this.cache = {
      root: this.rootPath,
      packages,
      stats: this.computeStats(packages),
      timestamp: Date.now(),
    };
    this.cacheTime = Date.now();

    return this.cache;
  }

  /**
   * Get single package details
   * @param {string} name - Package name (e.g., "node", "mcp")
   * @returns {Promise<Object|null>} Package details
   */
  async getPackage(name) {
    const tree = await this.getTree();
    return tree.packages.find(p => p.name === name || p.name === `@cynic/${name}`) || null;
  }

  /**
   * Search for symbols by name
   * @param {string} query - Search query
   * @returns {Promise<Array>} Matching symbols
   */
  async search(query) {
    const tree = await this.getTree();
    const results = [];
    const lowerQuery = query.toLowerCase();

    for (const pkg of tree.packages) {
      for (const mod of pkg.modules || []) {
        // Search in module name
        if (mod.name.toLowerCase().includes(lowerQuery)) {
          results.push({
            type: 'module',
            name: mod.name,
            package: pkg.name,
            path: mod.path,
          });
        }

        // Search in classes
        for (const cls of mod.classes || []) {
          if (cls.name.toLowerCase().includes(lowerQuery)) {
            results.push({
              type: 'class',
              name: cls.name,
              package: pkg.name,
              module: mod.name,
              path: mod.path,
              line: cls.line,
            });
          }

          // Search in methods
          for (const method of cls.methods || []) {
            if (method.name.toLowerCase().includes(lowerQuery)) {
              results.push({
                type: 'method',
                name: method.name,
                class: cls.name,
                package: pkg.name,
                module: mod.name,
                path: mod.path,
                line: method.line,
              });
            }
          }
        }

        // Search in functions
        for (const func of mod.functions || []) {
          if (func.name.toLowerCase().includes(lowerQuery)) {
            results.push({
              type: 'function',
              name: func.name,
              package: pkg.name,
              module: mod.name,
              path: mod.path,
              line: func.line,
            });
          }
        }
      }
    }

    return results;
  }

  /**
   * Get codebase statistics
   * @returns {Promise<Object>} Stats
   */
  async getStats() {
    const tree = await this.getTree();
    return tree.stats;
  }

  /**
   * Invalidate cache
   */
  invalidateCache() {
    this.cache = null;
    this.cacheTime = 0;
  }

  // ═══════════════════════════════════════════════════════════
  // PRIVATE METHODS
  // ═══════════════════════════════════════════════════════════

  /**
   * Scan all packages
   */
  async scanPackages() {
    const packages = [];

    try {
      const entries = await readdir(this.packagesPath, { withFileTypes: true });

      for (const entry of entries) {
        if (entry.isDirectory()) {
          const pkgPath = join(this.packagesPath, entry.name);
          const pkg = await this.scanPackage(entry.name, pkgPath);
          if (pkg) {
            packages.push(pkg);
          }
        }
      }
    } catch (err) {
      console.error('Error scanning packages:', err.message);
    }

    return packages;
  }

  /**
   * Scan a single package
   */
  async scanPackage(name, pkgPath) {
    const srcPath = join(pkgPath, 'src');

    // Check if src exists
    try {
      const srcStat = await stat(srcPath);
      if (!srcStat.isDirectory()) return null;
    } catch {
      return null; // No src directory
    }

    // Read package.json for name
    let pkgName = `@cynic/${name}`;
    try {
      const pkgJson = JSON.parse(await readFile(join(pkgPath, 'package.json'), 'utf-8'));
      pkgName = pkgJson.name || pkgName;
    } catch {
      // Use default name
    }

    // Scan source files
    const modules = await this.scanDirectory(srcPath, srcPath);

    // Determine package color based on name
    const colors = {
      core: 0x00d4aa,
      protocol: 0xd4aa00,
      persistence: 0x00aad4,
      node: 0xaa00d4,
      mcp: 0xd400aa,
      client: 0xaad400,
    };

    return {
      name: pkgName,
      shortName: name,
      path: relative(this.rootPath, pkgPath),
      color: colors[name] || 0x888888,
      modules,
      stats: {
        modules: modules.length,
        classes: modules.reduce((sum, m) => sum + (m.classes?.length || 0), 0),
        functions: modules.reduce((sum, m) => sum + (m.functions?.length || 0), 0),
        methods: modules.reduce((sum, m) =>
          sum + (m.classes?.reduce((s, c) => s + (c.methods?.length || 0), 0) || 0), 0),
        lines: modules.reduce((sum, m) => sum + (m.lines || 0), 0),
      },
    };
  }

  /**
   * Scan a directory for JS files
   */
  async scanDirectory(dirPath, basePath) {
    const modules = [];

    try {
      const entries = await readdir(dirPath, { withFileTypes: true });

      for (const entry of entries) {
        const fullPath = join(dirPath, entry.name);

        if (entry.isDirectory()) {
          // Recursively scan subdirectories
          const subModules = await this.scanDirectory(fullPath, basePath);
          modules.push(...subModules);
        } else if (entry.isFile() && (entry.name.endsWith('.js') || entry.name.endsWith('.mjs'))) {
          // Parse JavaScript file
          const mod = await this.parseFile(fullPath, basePath);
          if (mod) {
            modules.push(mod);
          }
        }
      }
    } catch (err) {
      console.error(`Error scanning directory ${dirPath}:`, err.message);
    }

    return modules;
  }

  /**
   * Parse a JavaScript file and extract symbols
   */
  async parseFile(filePath, basePath) {
    try {
      const content = await readFile(filePath, 'utf-8');
      const lines = content.split('\n');
      const relativePath = relative(basePath, filePath);
      const name = basename(filePath, '.js').replace('.mjs', '');

      // Extract symbols
      const classes = this.extractClasses(content, lines);
      const functions = this.extractFunctions(content, lines);
      const exports = this.extractExports(content);

      // Get module description from JSDoc
      const description = this.extractModuleDescription(content);

      return {
        name,
        path: relativePath,
        fullPath: filePath,
        description,
        lines: lines.length,
        classes,
        functions,
        exports,
      };
    } catch (err) {
      console.error(`Error parsing file ${filePath}:`, err.message);
      return null;
    }
  }

  /**
   * Extract classes from JavaScript content
   */
  extractClasses(content, lines) {
    const classes = [];

    // Match class declarations: class Name { or export class Name {
    const classRegex = /^(?:export\s+)?class\s+(\w+)(?:\s+extends\s+\w+)?\s*\{/gm;
    let match;

    while ((match = classRegex.exec(content)) !== null) {
      const className = match[1];
      const startIndex = match.index;
      const lineNumber = this.getLineNumber(content, startIndex);

      // Find class body end
      const classEnd = this.findMatchingBrace(content, match.index + match[0].length - 1);
      const classBody = content.slice(match.index, classEnd);

      // Extract methods
      const methods = this.extractMethods(classBody, lines, lineNumber);

      // Get class description from JSDoc
      const description = this.extractPrecedingJSDoc(content, startIndex);

      classes.push({
        name: className,
        line: lineNumber,
        description,
        methods,
      });
    }

    return classes;
  }

  /**
   * Extract methods from class body
   */
  extractMethods(classBody, lines, classStartLine) {
    const methods = [];

    // Match method declarations (including async, static, get, set)
    const methodRegex = /^\s*(?:async\s+)?(?:static\s+)?(?:get\s+|set\s+)?(\w+)\s*\([^)]*\)\s*\{/gm;
    let match;

    while ((match = methodRegex.exec(classBody)) !== null) {
      const methodName = match[1];

      // Skip constructor for some visualizations
      if (methodName === 'constructor') continue;

      const lineNumber = classStartLine + this.getLineNumber(classBody, match.index) - 1;

      // Extract parameters
      const paramsMatch = match[0].match(/\(([^)]*)\)/);
      const params = paramsMatch ?
        paramsMatch[1].split(',').map(p => p.trim().split('=')[0].trim()).filter(Boolean) :
        [];

      // Get description from JSDoc
      const description = this.extractPrecedingJSDoc(classBody, match.index);

      // Determine visibility
      const isPrivate = methodName.startsWith('_');
      const isStatic = match[0].includes('static');
      const isAsync = match[0].includes('async');
      const isGetter = match[0].includes('get ');
      const isSetter = match[0].includes('set ');

      methods.push({
        name: methodName,
        line: lineNumber,
        params,
        description,
        visibility: isPrivate ? 'private' : 'public',
        static: isStatic,
        async: isAsync,
        getter: isGetter,
        setter: isSetter,
      });
    }

    return methods;
  }

  /**
   * Extract standalone functions
   */
  extractFunctions(content, lines) {
    const functions = [];

    // Match function declarations and exports
    const funcRegex = /^(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\([^)]*\)/gm;
    let match;

    while ((match = funcRegex.exec(content)) !== null) {
      const funcName = match[1];
      const lineNumber = this.getLineNumber(content, match.index);

      // Extract parameters
      const paramsMatch = match[0].match(/\(([^)]*)\)/);
      const params = paramsMatch ?
        paramsMatch[1].split(',').map(p => p.trim().split('=')[0].trim()).filter(Boolean) :
        [];

      // Get description from JSDoc
      const description = this.extractPrecedingJSDoc(content, match.index);

      const isAsync = match[0].includes('async');
      const isExported = match[0].includes('export');

      functions.push({
        name: funcName,
        line: lineNumber,
        params,
        description,
        async: isAsync,
        exported: isExported,
      });
    }

    // Also match arrow function exports
    const arrowRegex = /^export\s+const\s+(\w+)\s*=\s*(?:async\s+)?\([^)]*\)\s*=>/gm;
    while ((match = arrowRegex.exec(content)) !== null) {
      const funcName = match[1];
      const lineNumber = this.getLineNumber(content, match.index);

      const paramsMatch = match[0].match(/\(([^)]*)\)/);
      const params = paramsMatch ?
        paramsMatch[1].split(',').map(p => p.trim().split('=')[0].trim()).filter(Boolean) :
        [];

      const description = this.extractPrecedingJSDoc(content, match.index);

      functions.push({
        name: funcName,
        line: lineNumber,
        params,
        description,
        async: match[0].includes('async'),
        exported: true,
        arrow: true,
      });
    }

    return functions;
  }

  /**
   * Extract exports
   */
  extractExports(content) {
    const exports = [];

    // Named exports
    const namedExportRegex = /export\s+\{([^}]+)\}/g;
    let match;
    while ((match = namedExportRegex.exec(content)) !== null) {
      const names = match[1].split(',').map(n => n.trim().split(' as ')[0].trim());
      exports.push(...names);
    }

    // Default export
    if (/export\s+default/.test(content)) {
      exports.push('default');
    }

    return [...new Set(exports)];
  }

  /**
   * Extract module description from JSDoc at top of file
   */
  extractModuleDescription(content) {
    const match = content.match(/^\/\*\*[\s\S]*?\*\//);
    if (match) {
      const desc = match[0]
        .replace(/^\/\*\*\s*/, '')
        .replace(/\s*\*\/$/, '')
        .split('\n')
        .map(l => l.replace(/^\s*\*\s?/, '').trim())
        .filter(l => !l.startsWith('@'))
        .join(' ')
        .trim();
      return desc.slice(0, 200);
    }
    return null;
  }

  /**
   * Extract JSDoc comment preceding a position
   */
  extractPrecedingJSDoc(content, position) {
    // Look back from position for /** ... */
    const before = content.slice(Math.max(0, position - 500), position);
    const match = before.match(/\/\*\*[\s\S]*?\*\/\s*$/);

    if (match) {
      const desc = match[0]
        .replace(/^\/\*\*\s*/, '')
        .replace(/\s*\*\/$/, '')
        .split('\n')
        .map(l => l.replace(/^\s*\*\s?/, '').trim())
        .filter(l => !l.startsWith('@'))
        .join(' ')
        .trim();
      return desc.slice(0, 200);
    }
    return null;
  }

  /**
   * Get line number for a character position
   */
  getLineNumber(content, position) {
    return content.slice(0, position).split('\n').length;
  }

  /**
   * Find matching closing brace
   */
  findMatchingBrace(content, startPos) {
    let depth = 1;
    let pos = startPos + 1;

    while (pos < content.length && depth > 0) {
      const char = content[pos];
      if (char === '{') depth++;
      else if (char === '}') depth--;
      pos++;
    }

    return pos;
  }

  /**
   * Compute overall statistics
   */
  computeStats(packages) {
    return {
      packages: packages.length,
      modules: packages.reduce((sum, p) => sum + (p.modules?.length || 0), 0),
      classes: packages.reduce((sum, p) => sum + (p.stats?.classes || 0), 0),
      functions: packages.reduce((sum, p) => sum + (p.stats?.functions || 0), 0),
      methods: packages.reduce((sum, p) => sum + (p.stats?.methods || 0), 0),
      lines: packages.reduce((sum, p) => sum + (p.stats?.lines || 0), 0),
    };
  }
}

/**
 * Create code analyzer instance
 * @param {Object} options - Options
 * @returns {CodeAnalyzer} Analyzer instance
 */
export function createCodeAnalyzer(options = {}) {
  return new CodeAnalyzer(options);
}

export default CodeAnalyzer;
