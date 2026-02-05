/**
 * Codebase Indexer Service
 *
 * Auto-indexes CYNIC's own codebase and extracts facts for self-knowledge.
 * Runs at session start to populate FactsRepository with structural awareness.
 *
 * SUPERMEMORY Enhancement (v0.3):
 * - indexAll() method indexes ALL .js files (not just keystones)
 * - Dependency extraction (imports/exports)
 * - Graph queries via PostgreSQL CTEs
 * - Progress callbacks for large codebases
 *
 * "Le chien doit se connaître lui-même" - CYNIC
 *
 * @module @cynic/persistence/services/codebase-indexer
 */

'use strict';

import fs from 'fs';
import fsPromises from 'fs/promises';
import path from 'path';
import { FactType } from '../postgres/repositories/facts.js';

// Parallel processing constants (φ-aligned)
const PARALLEL_CONCURRENCY = 21; // Fibonacci number for optimal parallel reads
const PARALLEL_BATCH_SIZE = 89;  // Fibonacci batch for DB inserts

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_SQ_INV = 0.381966011250105;
const FIB_BATCH = 89; // Fibonacci batch size for processing

/**
 * Directories to ignore during indexing
 */
const IGNORE_DIRS = new Set([
  'node_modules',
  '.git',
  'dist',
  'build',
  'coverage',
  '.nyc_output',
  'tmp',
  '.cache',
]);

/**
 * File extensions to index
 */
const INDEX_EXTENSIONS = new Set(['.js', '.mjs', '.cjs']);

/**
 * Keystone files that are critical for CYNIC's operation
 * These get highest confidence facts
 */
const KEYSTONE_FILES = [
  // Hooks - consciousness injection
  { path: 'scripts/hooks/awaken.js', purpose: 'Session start context injection', importance: 1.0 },
  { path: 'scripts/hooks/sleep.js', purpose: 'Session end pattern persistence', importance: 0.9 },
  { path: 'scripts/hooks/observe.js', purpose: 'Tool observation and fact extraction', importance: 0.9 },
  { path: 'scripts/hooks/digest.js', purpose: 'Knowledge extraction from content', importance: 0.8 },

  // Core judgment
  { path: 'packages/node/src/judge/judge.js', purpose: '25-dimension judgment engine', importance: 1.0 },
  { path: 'packages/node/src/judge/scorers/', purpose: 'Dimension scoring heuristics', importance: 0.9 },
  { path: 'packages/node/src/agents/collective/', purpose: '11 Collective Dogs (Sefirot)', importance: 1.0 },

  // MCP Tools
  { path: 'packages/mcp/src/tools/domains/', purpose: 'MCP brain_* tool definitions', importance: 0.9 },
  { path: 'packages/mcp/src/server.js', purpose: 'MCP server entry point', importance: 0.8 },

  // Persistence
  { path: 'packages/persistence/src/postgres/repositories/', purpose: 'Data repositories (Total Memory)', importance: 0.9 },
  { path: 'packages/persistence/src/services/', purpose: 'Persistence services (embedder, memory)', importance: 0.8 },

  // Identity
  { path: 'CLAUDE.md', purpose: 'CYNIC identity and TUI protocol', importance: 1.0 },
  { path: '.claude/cynic-consciousness.md', purpose: 'Consciousness instructions for hooks', importance: 0.9 },

  // Philosophy
  { path: 'packages/core/src/engines/philosophy/', purpose: '73 philosophy engines', importance: 0.7 },
];

/**
 * Package purposes for CYNIC's monorepo
 */
const PACKAGE_PURPOSES = {
  'core': 'Shared utilities, constants, logging, vector operations',
  'node': 'Judgment engine, scorers, collective dogs, learning service',
  'mcp': 'MCP server, brain_* tools, ecosystem service',
  'persistence': 'PostgreSQL repositories, embedder, memory retriever',
  'protocol': 'PoJ blockchain, merkle trees, cryptographic proofs',
  'emergence': 'Consciousness monitoring, meta-cognition, self-awareness',
};

/**
 * Codebase Indexer
 *
 * Extracts structural facts from CYNIC's codebase for self-knowledge.
 */
export class CodebaseIndexer {
  /**
   * @param {Object} options
   * @param {Object} options.factsRepo - FactsRepository instance
   * @param {string} options.rootDir - Root directory to index
   * @param {string} options.userId - User ID for facts
   */
  constructor(options = {}) {
    this.factsRepo = options.factsRepo;
    this.rootDir = options.rootDir || process.cwd();
    this.userId = options.userId;
    this.sessionId = options.sessionId;
    this.projectName = options.projectName || 'CYNIC';
    this.onProgress = options.onProgress || null; // Progress callback
  }

  /**
   * Run full indexing and extract facts
   * @returns {Promise<Object>} Indexing results
   */
  async index() {
    const results = {
      keystoneFacts: 0,
      packageFacts: 0,
      structureFacts: 0,
      patternFacts: 0,
      total: 0,
      errors: [],
    };

    try {
      // 1. Index keystone files
      const keystoneFacts = await this._indexKeystoneFiles();
      results.keystoneFacts = keystoneFacts.length;

      // 2. Index package structure
      const packageFacts = await this._indexPackages();
      results.packageFacts = packageFacts.length;

      // 3. Index high-level structure
      const structureFacts = await this._indexStructure();
      results.structureFacts = structureFacts.length;

      // 4. Detect code patterns
      const patternFacts = await this._detectPatterns();
      results.patternFacts = patternFacts.length;

      // Store all facts
      const allFacts = [...keystoneFacts, ...packageFacts, ...structureFacts, ...patternFacts];
      for (const fact of allFacts) {
        try {
          await this._storeFact(fact);
          results.total++;
        } catch (e) {
          results.errors.push({ fact: fact.subject, error: e.message });
        }
      }

    } catch (e) {
      results.errors.push({ phase: 'indexing', error: e.message });
    }

    return results;
  }

  /**
   * Index ALL JavaScript files in the codebase (SUPERMEMORY enhancement)
   *
   * @param {Object} options
   * @param {number} options.maxFiles - Maximum files to index (default: unlimited)
   * @param {boolean} options.extractDeps - Extract dependencies (default: true)
   * @param {boolean} options.includeKeystone - Include keystone facts (default: true)
   * @returns {Promise<Object>} Indexing results
   */
  async indexAll(options = {}) {
    const {
      maxFiles = Infinity,
      extractDeps = true,
      includeKeystone = true,
      includeStructure = false, // Slow: rescans entire codebase
      includePatterns = false,  // Slow: reads all JS files
    } = options;

    const results = {
      filesIndexed: 0,
      factsGenerated: 0,
      dependenciesExtracted: 0,
      keystoneFacts: 0,
      packageFacts: 0,
      structureFacts: 0,
      patternFacts: 0,
      errors: [],
      timing: {
        startTime: Date.now(),
        endTime: null,
        durationMs: null,
      },
    };

    try {
      // 1. Collect all JS files
      const allFiles = this._collectAllFiles(this.rootDir);
      const filesToIndex = allFiles.slice(0, maxFiles);

      this._reportProgress('scan', { total: allFiles.length, indexing: filesToIndex.length });

      // 2. Index files in PARALLEL (21 concurrent, φ-aligned)
      const parallelResult = await this._processFilesParallel(
        filesToIndex,
        extractDeps,
        (progress) => this._reportProgress('indexing', progress)
      );

      results.filesIndexed = parallelResult.processed;
      results.errors.push(...parallelResult.errors);

      // Count dependencies and store in batches
      const allFileFacts = parallelResult.facts;
      for (const fact of allFileFacts) {
        if (fact.context?.dependencies) {
          results.dependenciesExtracted += fact.context.dependencies.imports.length;
        }
      }
      results.factsGenerated += allFileFacts.length;

      // Store facts in batches (single DB round-trip per batch)
      for (let i = 0; i < allFileFacts.length; i += PARALLEL_BATCH_SIZE) {
        const batch = allFileFacts.slice(i, i + PARALLEL_BATCH_SIZE);
        try {
          await this._storeBatch(batch);
        } catch (e) {
          results.errors.push({ batch: `file_batch_${i}`, error: e.message });
        }
      }

      // 3. Include keystone facts if requested (batch insert)
      if (includeKeystone) {
        const keystoneFacts = await this._indexKeystoneFiles();
        results.keystoneFacts = keystoneFacts.length;
        try {
          await this._storeBatch(keystoneFacts);
          results.factsGenerated += keystoneFacts.length;
        } catch (e) {
          results.errors.push({ batch: 'keystone', error: e.message });
        }
      }

      // 4. Index packages (batch insert)
      const packageFacts = await this._indexPackages();
      results.packageFacts = packageFacts.length;
      try {
        await this._storeBatch(packageFacts);
        results.factsGenerated += packageFacts.length;
      } catch (e) {
        results.errors.push({ batch: 'packages', error: e.message });
      }

      // 5. Index structure (batch insert) - optional, slow
      if (includeStructure) {
        const structureFacts = await this._indexStructure();
        results.structureFacts = structureFacts.length;
        if (structureFacts.length > 0) {
          try {
            await this._storeBatch(structureFacts);
            results.factsGenerated += structureFacts.length;
          } catch (e) {
            results.errors.push({ batch: 'structure', error: e.message });
          }
        }
      }

      // 6. Detect patterns (batch insert) - optional, slow
      if (includePatterns) {
        const patternFacts = await this._detectPatterns();
        results.patternFacts = patternFacts.length;
        if (patternFacts.length > 0) {
          try {
            await this._storeBatch(patternFacts);
            results.factsGenerated += patternFacts.length;
          } catch (e) {
            results.errors.push({ batch: 'patterns', error: e.message });
          }
        }
      }

    } catch (e) {
      results.errors.push({ phase: 'indexAll', error: e.message });
    }

    results.timing.endTime = Date.now();
    results.timing.durationMs = results.timing.endTime - results.timing.startTime;

    this._reportProgress('complete', results);

    return results;
  }

  /**
   * Collect all JavaScript files respecting ignore patterns
   * @private
   */
  _collectAllFiles(dir, files = [], depth = 0) {
    if (depth > 8) return files; // Max depth

    try {
      // Use withFileTypes to avoid separate stat() calls (10x faster)
      const items = fs.readdirSync(dir, { withFileTypes: true });

      for (const item of items) {
        if (item.name.startsWith('.') || IGNORE_DIRS.has(item.name)) continue;

        const itemPath = path.join(dir, item.name);

        if (item.isDirectory()) {
          this._collectAllFiles(itemPath, files, depth + 1);
        } else if (item.isFile()) {
          const ext = path.extname(item.name).toLowerCase();
          if (INDEX_EXTENSIONS.has(ext)) {
            files.push(itemPath);
          }
        }
      }
    } catch (e) {
      // Ignore permission errors
    }

    return files;
  }

  /**
   * Process files in parallel with concurrency limit
   * @private
   * @param {string[]} files - Files to process
   * @param {boolean} extractDeps - Extract dependencies
   * @param {Function} onProgress - Progress callback
   * @returns {Promise<{facts: Object[], errors: Object[]}>}
   */
  async _processFilesParallel(files, extractDeps = true, onProgress = null) {
    const facts = [];
    const errors = [];
    let processed = 0;

    // Process in chunks with concurrency limit
    for (let i = 0; i < files.length; i += PARALLEL_CONCURRENCY) {
      const chunk = files.slice(i, i + PARALLEL_CONCURRENCY);

      // Process chunk in parallel
      const chunkResults = await Promise.all(
        chunk.map(async (filePath) => {
          try {
            const fact = await this._indexFileAsync(filePath, extractDeps);
            return { success: true, fact };
          } catch (e) {
            return { success: false, error: { file: filePath, error: e.message } };
          }
        })
      );

      // Collect results
      for (const result of chunkResults) {
        if (result.success && result.fact) {
          facts.push(result.fact);
        } else if (!result.success) {
          errors.push(result.error);
        }
        processed++;
      }

      // Report progress
      if (onProgress && processed % PARALLEL_BATCH_SIZE === 0) {
        onProgress({ processed, total: files.length });
      }
    }

    return { facts, errors, processed };
  }

  /**
   * Index a single file asynchronously (parallel-safe)
   * @private
   */
  async _indexFileAsync(filePath, extractDeps = true) {
    const content = await fsPromises.readFile(filePath, 'utf8');
    const relativePath = path.relative(this.rootDir, filePath);
    const lineCount = content.split('\n').length;

    // Extract module documentation
    let description = '';
    const docMatch = content.match(/^\/\*\*[\s\S]*?\*\//);
    if (docMatch) {
      description = docMatch[0]
        .replace(/\/\*\*|\*\/|\*/g, '')
        .replace(/@\w+[^\n]*/g, '')
        .replace(/\n\s+/g, ' ')
        .trim()
        .substring(0, 300);
    }

    // Extract dependencies
    let dependencies = null;
    if (extractDeps) {
      dependencies = this._extractDependencies(content, filePath);
    }

    // Calculate importance
    const importCount = dependencies?.imports?.length || 0;
    const exportCount = dependencies?.exports?.length || 0;
    const importance = Math.min(PHI_INV, PHI_SQ_INV + (importCount + exportCount) / 50 * PHI_SQ_INV);

    return {
      factType: FactType.FILE_STRUCTURE,
      subject: `File: ${relativePath}`,
      content: description || `JavaScript file with ${lineCount} lines`,
      confidence: importance,
      tags: ['file', 'javascript', this._getPackageName(relativePath)].filter(Boolean),
      context: {
        path: relativePath,
        fullPath: filePath,
        lineCount,
        dependencies,
        hasExports: exportCount > 0,
        isEntryPoint: relativePath.includes('index.js') || relativePath.includes('server.js'),
      },
    };
  }

  /**
   * Index a single file with dependency extraction (sync, legacy)
   * @private
   */
  async _indexFile(filePath, extractDeps = true) {
    const content = fs.readFileSync(filePath, 'utf8');
    const relativePath = path.relative(this.rootDir, filePath);
    const lineCount = content.split('\n').length;

    // Extract module documentation
    let description = '';
    const docMatch = content.match(/^\/\*\*[\s\S]*?\*\//);
    if (docMatch) {
      description = docMatch[0]
        .replace(/\/\*\*|\*\/|\*/g, '')
        .replace(/@\w+[^\n]*/g, '') // Remove JSDoc tags
        .replace(/\n\s+/g, ' ')
        .trim()
        .substring(0, 300);
    }

    // Extract dependencies
    let dependencies = null;
    if (extractDeps) {
      dependencies = this._extractDependencies(content, filePath);
    }

    // Calculate importance based on imports (more imports = more central)
    const importCount = dependencies?.imports?.length || 0;
    const exportCount = dependencies?.exports?.length || 0;
    const importance = Math.min(PHI_INV, PHI_SQ_INV + (importCount + exportCount) / 50 * PHI_SQ_INV);

    return {
      factType: FactType.FILE_STRUCTURE,
      subject: `File: ${relativePath}`,
      content: description || `JavaScript file with ${lineCount} lines`,
      confidence: importance,
      tags: ['file', 'javascript', this._getPackageName(relativePath)].filter(Boolean),
      context: {
        path: relativePath,
        fullPath: filePath,
        lineCount,
        dependencies,
        hasExports: exportCount > 0,
        isEntryPoint: relativePath.includes('index.js') || relativePath.includes('server.js'),
      },
    };
  }

  /**
   * Extract imports and exports from JavaScript content
   * @private
   */
  _extractDependencies(content, filePath) {
    const imports = [];
    const exports = [];

    // ES6 imports: import X from 'Y', import { X } from 'Y', import * as X from 'Y'
    const importRegex = /import\s+(?:(?:\{[^}]+\}|\*\s+as\s+\w+|\w+)\s+from\s+)?['"]([^'"]+)['"]/g;
    let match;
    while ((match = importRegex.exec(content)) !== null) {
      imports.push({
        source: match[1],
        isRelative: match[1].startsWith('.'),
        isPackage: !match[1].startsWith('.') && !match[1].startsWith('/'),
      });
    }

    // CommonJS require: require('X')
    const requireRegex = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
    while ((match = requireRegex.exec(content)) !== null) {
      if (!imports.some(i => i.source === match[1])) {
        imports.push({
          source: match[1],
          isRelative: match[1].startsWith('.'),
          isPackage: !match[1].startsWith('.') && !match[1].startsWith('/'),
          isRequire: true,
        });
      }
    }

    // ES6 exports: export { X }, export default X, export class X, export function X
    const namedExportRegex = /export\s+(?:const|let|var|function|class|async\s+function)\s+(\w+)/g;
    while ((match = namedExportRegex.exec(content)) !== null) {
      exports.push({ name: match[1], type: 'named' });
    }

    // export { X, Y, Z }
    const groupExportRegex = /export\s+\{([^}]+)\}/g;
    while ((match = groupExportRegex.exec(content)) !== null) {
      const names = match[1].split(',').map(n => n.trim().split(/\s+as\s+/)[0].trim());
      for (const name of names) {
        if (name && !exports.some(e => e.name === name)) {
          exports.push({ name, type: 'named' });
        }
      }
    }

    // export default
    if (/export\s+default/.test(content)) {
      exports.push({ name: 'default', type: 'default' });
    }

    return { imports, exports };
  }

  /**
   * Get package name from relative path
   * @private
   */
  _getPackageName(relativePath) {
    const match = relativePath.match(/^packages\/([^/]+)/);
    return match ? match[1] : null;
  }

  /**
   * Report progress via callback
   * @private
   */
  _reportProgress(phase, data) {
    if (this.onProgress) {
      this.onProgress({ phase, ...data });
    }
  }

  /**
   * Index keystone files (critical files for CYNIC operation)
   * @private
   */
  async _indexKeystoneFiles() {
    const facts = [];

    for (const keystone of KEYSTONE_FILES) {
      const fullPath = path.join(this.rootDir, keystone.path);

      // Check if file/dir exists
      if (!fs.existsSync(fullPath)) continue;

      const stat = fs.statSync(fullPath);
      const isDir = stat.isDirectory();

      let content = keystone.purpose;
      let lineCount = 0;

      if (!isDir) {
        try {
          const fileContent = fs.readFileSync(fullPath, 'utf8');
          lineCount = fileContent.split('\n').length;

          // Extract first comment block as additional context
          const commentMatch = fileContent.match(/^\/\*\*[\s\S]*?\*\//);
          if (commentMatch) {
            const cleanComment = commentMatch[0]
              .replace(/\/\*\*|\*\/|\*/g, '')
              .replace(/\n\s+/g, ' ')
              .trim()
              .substring(0, 200);
            if (cleanComment) {
              content += `. ${cleanComment}`;
            }
          }
        } catch (e) {
          // File read failed, use purpose only
        }
      }

      facts.push({
        factType: FactType.FILE_STRUCTURE,
        subject: `Keystone: ${keystone.path}`,
        content: content + (lineCount ? ` (${lineCount} lines)` : ''),
        confidence: Math.min(PHI_INV, keystone.importance * PHI_INV),
        tags: ['keystone', 'architecture', isDir ? 'directory' : 'file'],
        context: {
          path: keystone.path,
          importance: keystone.importance,
          isDirectory: isDir,
          lineCount,
        },
      });
    }

    return facts;
  }

  /**
   * Index package structure
   * @private
   */
  async _indexPackages() {
    const facts = [];
    const packagesDir = path.join(this.rootDir, 'packages');

    if (!fs.existsSync(packagesDir)) return facts;

    const packages = fs.readdirSync(packagesDir).filter(p => {
      const pkgPath = path.join(packagesDir, p);
      return fs.statSync(pkgPath).isDirectory();
    });

    for (const pkg of packages) {
      const purpose = PACKAGE_PURPOSES[pkg] || 'Unknown package purpose';
      const pkgPath = path.join(packagesDir, pkg);

      // Count files and directories
      let fileCount = 0;
      let dirCount = 0;
      const countFiles = (dir) => {
        try {
          const items = fs.readdirSync(dir);
          for (const item of items) {
            if (item.startsWith('.') || item === 'node_modules') continue;
            const itemPath = path.join(dir, item);
            const stat = fs.statSync(itemPath);
            if (stat.isDirectory()) {
              dirCount++;
              countFiles(itemPath);
            } else if (item.endsWith('.js') || item.endsWith('.sql')) {
              fileCount++;
            }
          }
        } catch (e) { /* ignore */ }
      };
      countFiles(pkgPath);

      // Check for package.json
      const pkgJsonPath = path.join(pkgPath, 'package.json');
      let version = null;
      let dependencies = [];
      if (fs.existsSync(pkgJsonPath)) {
        try {
          const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
          version = pkgJson.version;
          dependencies = Object.keys(pkgJson.dependencies || {});
        } catch (e) { /* ignore */ }
      }

      facts.push({
        factType: FactType.FILE_STRUCTURE,
        subject: `Package: @cynic/${pkg}`,
        content: `${purpose}. Contains ${fileCount} source files in ${dirCount} directories.${version ? ` Version ${version}.` : ''}`,
        confidence: PHI_INV * 0.9,
        tags: ['package', 'architecture', pkg],
        context: {
          package: pkg,
          fileCount,
          dirCount,
          version,
          dependencies: dependencies.slice(0, 10),
        },
      });
    }

    return facts;
  }

  /**
   * Index high-level structure
   * @private
   */
  async _indexStructure() {
    const facts = [];

    // Count total stats
    let totalJs = 0;
    let totalSql = 0;
    let totalLines = 0;

    const countAll = (dir, depth = 0) => {
      if (depth > 5) return;
      try {
        const items = fs.readdirSync(dir);
        for (const item of items) {
          if (item.startsWith('.') || item === 'node_modules') continue;
          const itemPath = path.join(dir, item);
          const stat = fs.statSync(itemPath);
          if (stat.isDirectory()) {
            countAll(itemPath, depth + 1);
          } else if (item.endsWith('.js')) {
            totalJs++;
            try {
              totalLines += fs.readFileSync(itemPath, 'utf8').split('\n').length;
            } catch (e) { /* ignore */ }
          } else if (item.endsWith('.sql')) {
            totalSql++;
          }
        }
      } catch (e) { /* ignore */ }
    };

    countAll(this.rootDir);

    facts.push({
      factType: FactType.FILE_STRUCTURE,
      subject: 'CYNIC Codebase Overview',
      content: `CYNIC monorepo with ${totalJs} JavaScript files, ${totalSql} SQL migrations, ~${Math.round(totalLines / 1000)}K lines of code. Pure ESM, no TypeScript, Node.js 20+.`,
      confidence: PHI_INV,
      tags: ['overview', 'statistics', 'architecture'],
      context: {
        jsFiles: totalJs,
        sqlFiles: totalSql,
        totalLines,
        type: 'monorepo',
        language: 'javascript',
        module: 'esm',
      },
    });

    // Architecture fact
    facts.push({
      factType: FactType.CODE_PATTERN,
      subject: 'CYNIC Architecture Pattern',
      content: 'CYNIC uses a layered architecture: Core (utilities) → Node (judgment) → MCP (tools) → Persistence (storage) → Protocol (blockchain) → Emergence (consciousness). Each layer only depends on lower layers.',
      confidence: PHI_INV,
      tags: ['architecture', 'pattern', 'layers'],
      context: {
        layers: ['core', 'node', 'mcp', 'persistence', 'protocol', 'emergence'],
        pattern: 'layered-architecture',
      },
    });

    // Philosophy fact
    facts.push({
      factType: FactType.CODE_PATTERN,
      subject: 'CYNIC Philosophy Axioms',
      content: 'Four axioms guide all CYNIC code: PHI (max 61.8% confidence), VERIFY (don\'t trust, verify), CULTURE (patterns matter), BURN (simplicity wins). These are enforced through 25-dimension scoring.',
      confidence: PHI_INV,
      tags: ['philosophy', 'axioms', 'principles'],
      context: {
        axioms: ['PHI', 'VERIFY', 'CULTURE', 'BURN'],
        dimensions: 25,
        maxConfidence: PHI_INV,
      },
    });

    return facts;
  }

  /**
   * Detect code patterns
   * @private
   */
  async _detectPatterns() {
    const facts = [];
    const patterns = {};

    // Scan for common patterns
    const scanForPatterns = (dir, depth = 0) => {
      if (depth > 4) return;
      try {
        const items = fs.readdirSync(dir);
        for (const item of items) {
          if (item.startsWith('.') || item === 'node_modules') continue;
          const itemPath = path.join(dir, item);
          const stat = fs.statSync(itemPath);

          if (stat.isDirectory()) {
            // Track directory patterns
            if (item === 'repositories') patterns['Repository Pattern'] = (patterns['Repository Pattern'] || 0) + 1;
            if (item === 'services') patterns['Service Pattern'] = (patterns['Service Pattern'] || 0) + 1;
            if (item === 'domains') patterns['Domain Pattern'] = (patterns['Domain Pattern'] || 0) + 1;
            if (item === 'hooks') patterns['Hook Pattern'] = (patterns['Hook Pattern'] || 0) + 1;
            if (item === 'migrations') patterns['Migration Pattern'] = (patterns['Migration Pattern'] || 0) + 1;
            scanForPatterns(itemPath, depth + 1);
          } else if (item.endsWith('.js')) {
            try {
              const content = fs.readFileSync(itemPath, 'utf8');

              // Pattern detection
              if (content.includes('extends BaseRepository')) patterns['Repository Pattern'] = (patterns['Repository Pattern'] || 0) + 1;
              if (content.includes('createLogger')) patterns['Logger Pattern'] = (patterns['Logger Pattern'] || 0) + 1;
              if (content.includes('PHI_INV') || content.includes('0.618')) patterns['PHI Constants'] = (patterns['PHI Constants'] || 0) + 1;
              if (content.includes('async handler')) patterns['Async Handler'] = (patterns['Async Handler'] || 0) + 1;
              if (content.match(/export\s+(class|function)/g)?.length > 3) patterns['Multi-export Module'] = (patterns['Multi-export Module'] || 0) + 1;
            } catch (e) { /* ignore */ }
          }
        }
      } catch (e) { /* ignore */ }
    };

    scanForPatterns(this.rootDir);

    // Create facts for significant patterns
    for (const [pattern, count] of Object.entries(patterns)) {
      if (count >= 3) {
        facts.push({
          factType: FactType.CODE_PATTERN,
          subject: `Pattern: ${pattern}`,
          content: `The "${pattern}" pattern appears ${count} times across CYNIC codebase. This indicates a consistent architectural choice.`,
          confidence: Math.min(PHI_INV, PHI_SQ_INV + (count / 20) * PHI_SQ_INV),
          tags: ['pattern', 'architecture', pattern.toLowerCase().replace(/\s+/g, '-')],
          context: {
            pattern,
            occurrences: count,
          },
        });
      }
    }

    return facts;
  }

  /**
   * Query dependency graph for a symbol or file
   *
   * Uses PostgreSQL recursive CTE to traverse dependencies.
   *
   * @param {string} query - Symbol name or file path to search
   * @param {Object} options
   * @param {number} options.maxDepth - Maximum traversal depth (default: 5)
   * @param {string} options.direction - 'imports', 'exports', or 'both' (default: 'both')
   * @returns {Promise<Object>} Dependency graph
   */
  async queryDependencyGraph(query, options = {}) {
    const { maxDepth = 5, direction = 'both' } = options;

    if (!this.factsRepo || !this.factsRepo.db) {
      // Fallback: search locally without DB
      return this._queryDependencyGraphLocal(query, options);
    }

    // Use recursive CTE for graph traversal
    const cteQuery = `
      WITH RECURSIVE dep_graph AS (
        -- Base case: find matching facts
        SELECT
          f.fact_id,
          f.subject,
          f.context->'dependencies' as deps,
          f.context->>'path' as file_path,
          1 as depth,
          ARRAY[f.fact_id] as path
        FROM facts f
        WHERE f.fact_type = 'file_structure'
          AND (
            f.subject ILIKE $1
            OR f.context->>'path' ILIKE $1
          )

        UNION ALL

        -- Recursive case: follow import dependencies
        SELECT
          f.fact_id,
          f.subject,
          f.context->'dependencies' as deps,
          f.context->>'path' as file_path,
          dg.depth + 1,
          dg.path || f.fact_id
        FROM facts f
        JOIN dep_graph dg ON (
          -- Match if this file's path matches an import in the parent
          EXISTS (
            SELECT 1 FROM jsonb_array_elements(dg.deps->'imports') as imp
            WHERE f.context->>'path' LIKE '%' || REPLACE(imp->>'source', './', '') || '%'
          )
        )
        WHERE dg.depth < $2
          AND NOT f.fact_id = ANY(dg.path) -- Prevent cycles
          AND f.fact_type = 'file_structure'
      )
      SELECT DISTINCT ON (fact_id)
        fact_id, subject, file_path, depth, deps
      FROM dep_graph
      ORDER BY fact_id, depth
      LIMIT 100
    `;

    try {
      const { rows } = await this.factsRepo.db.query(cteQuery, [`%${query}%`, maxDepth]);

      return {
        query,
        direction,
        maxDepth,
        nodes: rows.map(r => ({
          factId: r.fact_id,
          subject: r.subject,
          path: r.file_path,
          depth: r.depth,
          imports: r.deps?.imports?.length || 0,
          exports: r.deps?.exports?.length || 0,
        })),
        edges: this._buildEdges(rows),
        stats: {
          nodeCount: rows.length,
          maxDepthReached: Math.max(...rows.map(r => r.depth), 0),
        },
      };
    } catch (e) {
      return { error: e.message, query, nodes: [], edges: [] };
    }
  }

  /**
   * Local fallback for dependency graph query (no DB)
   * @private
   */
  async _queryDependencyGraphLocal(query, options = {}) {
    const files = this._collectAllFiles(this.rootDir);
    const matching = files.filter(f =>
      f.toLowerCase().includes(query.toLowerCase())
    );

    const nodes = [];
    for (const filePath of matching.slice(0, 20)) {
      try {
        const content = fs.readFileSync(filePath, 'utf8');
        const deps = this._extractDependencies(content, filePath);
        const relativePath = path.relative(this.rootDir, filePath);

        nodes.push({
          path: relativePath,
          imports: deps.imports,
          exports: deps.exports,
          depth: 0,
        });
      } catch (e) {
        // Skip unreadable files
      }
    }

    return {
      query,
      nodes,
      edges: [],
      stats: { nodeCount: nodes.length, local: true },
    };
  }

  /**
   * Build edges from graph query results
   * @private
   */
  _buildEdges(rows) {
    const edges = [];
    const pathMap = new Map(rows.map(r => [r.file_path, r]));

    for (const row of rows) {
      if (row.deps?.imports) {
        for (const imp of row.deps.imports) {
          // Find target node
          const targetPath = Object.keys([...pathMap.keys()]).find(p =>
            p && imp.source && p.includes(imp.source.replace('./', '').replace('../', ''))
          );
          if (targetPath) {
            edges.push({
              source: row.file_path,
              target: targetPath,
              type: 'imports',
            });
          }
        }
      }
    }

    return edges;
  }

  /**
   * Get file by path or symbol
   * @param {string} query - File path or symbol
   * @returns {Promise<Object|null>}
   */
  async getFile(query) {
    const files = this._collectAllFiles(this.rootDir);
    const match = files.find(f =>
      f.toLowerCase().includes(query.toLowerCase())
    );

    if (!match) return null;

    try {
      const content = fs.readFileSync(match, 'utf8');
      const deps = this._extractDependencies(content, match);
      const relativePath = path.relative(this.rootDir, match);

      return {
        path: relativePath,
        fullPath: match,
        lineCount: content.split('\n').length,
        dependencies: deps,
        content: content.substring(0, 2000), // First 2K chars
      };
    } catch (e) {
      return null;
    }
  }

  /**
   * Get all files that import a given file
   * @param {string} filePath - Relative file path
   * @returns {Promise<Array>}
   */
  async getReverseDependencies(filePath) {
    const allFiles = this._collectAllFiles(this.rootDir);
    const importers = [];

    for (const file of allFiles) {
      try {
        const content = fs.readFileSync(file, 'utf8');
        const deps = this._extractDependencies(content, file);

        const imports = deps.imports.some(imp =>
          filePath.includes(imp.source.replace('./', '').replace('../', '')) ||
          imp.source.includes(path.basename(filePath, '.js'))
        );

        if (imports) {
          importers.push({
            path: path.relative(this.rootDir, file),
            fullPath: file,
          });
        }
      } catch (e) {
        // Skip
      }
    }

    return importers;
  }

  /**
   * Store a fact in the repository
   * @private
   */
  async _storeFact(fact) {
    if (!this.factsRepo) return;

    // Check for existing similar fact to avoid duplicates
    const existing = await this.factsRepo.search(fact.subject, {
      userId: this.userId,
      limit: 1,
      minConfidence: 0,
    });

    if (existing.length > 0 && existing[0].subject === fact.subject) {
      // Update existing fact
      await this.factsRepo.update(existing[0].factId, {
        content: fact.content,
        confidence: Math.max(existing[0].confidence, fact.confidence),
        context: { ...existing[0].context, ...fact.context },
      });
    } else {
      // Create new fact
      await this.factsRepo.create({
        userId: this.userId,
        sessionId: this.sessionId,
        factType: fact.factType,
        subject: fact.subject,
        content: fact.content,
        confidence: fact.confidence,
        relevance: fact.confidence, // Initial relevance = confidence
        tags: fact.tags || [],
        context: fact.context || {},
        sourceTool: 'codebase-indexer',
      });
    }
  }

  /**
   * Store multiple facts in a single batch insert
   * ~100x faster than individual _storeFact() calls for remote DBs
   *
   * @param {Array} facts - Facts to store
   * @returns {Promise<number>} Number of facts stored
   */
  async _storeBatch(facts) {
    if (!this.factsRepo || !facts || facts.length === 0) return 0;

    // Prepare facts for batch insert
    const preparedFacts = facts.map(fact => ({
      userId: this.userId,
      sessionId: this.sessionId,
      factType: fact.factType,
      subject: fact.subject,
      content: fact.content,
      confidence: fact.confidence,
      relevance: fact.confidence,
      tags: fact.tags || [],
      context: fact.context || {},
      sourceTool: 'codebase-indexer',
    }));

    // Use batch insert (single round-trip)
    const results = await this.factsRepo.createBatch(preparedFacts);
    return results.length;
  }
}

/**
 * Create a CodebaseIndexer instance
 * @param {Object} options
 * @returns {CodebaseIndexer}
 */
export function createCodebaseIndexer(options = {}) {
  return new CodebaseIndexer(options);
}

export default CodebaseIndexer;
