/**
 * Agent Booster - Fast Code Transforms Without LLM
 *
 * Handles simple code transformations using AST-like pattern matching,
 * bypassing LLM inference entirely for routine modifications.
 *
 * Target: <1ms per transform, $0 cost
 *
 * Inspired by Claude Flow's Agent Booster (WASM-based).
 * This implementation uses JavaScript for portability, with architecture
 * ready for WASM compilation via AssemblyScript.
 *
 * "Not every bark needs the whole pack" - κυνικός
 *
 * @module @cynic/node/routing/agent-booster
 */

'use strict';

import { EventEmitter } from 'events';

/**
 * Supported transform intents
 */
export const TransformIntent = Object.freeze({
  VAR_TO_CONST: 'var-to-const',
  VAR_TO_LET: 'var-to-let',
  ADD_TYPES: 'add-types',
  ADD_ASYNC_AWAIT: 'add-async-await',
  ADD_ERROR_HANDLING: 'add-error-handling',
  ADD_LOGGING: 'add-logging',
  REMOVE_CONSOLE: 'remove-console',
  REMOVE_DEBUGGER: 'remove-debugger',
  ADD_SEMICOLONS: 'add-semicolons',
  REMOVE_UNUSED_IMPORTS: 'remove-unused-imports',
  SORT_IMPORTS: 'sort-imports',
  ADD_STRICT_MODE: 'add-strict-mode',
});

/**
 * Transform result status
 */
export const TransformStatus = Object.freeze({
  SUCCESS: 'success',
  NO_CHANGE: 'no_change',
  PARTIAL: 'partial',
  ERROR: 'error',
  UNSUPPORTED: 'unsupported',
});

/**
 * Intent detection patterns
 */
const INTENT_PATTERNS = [
  { intent: TransformIntent.VAR_TO_CONST, pattern: /\b(convert|change|replace)\s+(all\s+)?var(s)?\s+(to|with|into)\s+const/i },
  { intent: TransformIntent.VAR_TO_CONST, pattern: /\bvar\s+to\s+const/i },
  { intent: TransformIntent.VAR_TO_LET, pattern: /\b(convert|change|replace)\s+(all\s+)?var(s)?\s+(to|with|into)\s+let/i },
  { intent: TransformIntent.VAR_TO_LET, pattern: /\bvar\s+to\s+let/i },
  { intent: TransformIntent.ADD_TYPES, pattern: /\badd\s+(typescript\s+)?types?/i },
  { intent: TransformIntent.ADD_ASYNC_AWAIT, pattern: /\b(add|convert\s+to)\s+async[\s/-]?await/i },
  { intent: TransformIntent.ADD_ERROR_HANDLING, pattern: /\badd\s+(error\s+)?handl(ing|er)/i },
  { intent: TransformIntent.ADD_ERROR_HANDLING, pattern: /\bwrap\s+(in|with)\s+try[\s/-]?catch/i },
  { intent: TransformIntent.ADD_LOGGING, pattern: /\badd\s+log(ging|s)?/i },
  { intent: TransformIntent.REMOVE_CONSOLE, pattern: /\b(remove|delete|strip)\s+(all\s+)?console\s*(\.log)?/i },
  { intent: TransformIntent.REMOVE_DEBUGGER, pattern: /\b(remove|delete|strip)\s+(all\s+)?debugger/i },
  { intent: TransformIntent.ADD_SEMICOLONS, pattern: /\badd\s+semicolons?/i },
  { intent: TransformIntent.REMOVE_UNUSED_IMPORTS, pattern: /\b(remove|delete|clean)\s+(unused\s+)?imports?/i },
  { intent: TransformIntent.SORT_IMPORTS, pattern: /\bsort\s+imports?/i },
  { intent: TransformIntent.ADD_STRICT_MODE, pattern: /\badd\s+['"]?use\s+strict['"]?/i },
];

/**
 * Agent Booster - Fast code transforms
 */
export class AgentBooster extends EventEmitter {
  constructor(options = {}) {
    super();

    this.stats = {
      transforms: 0,
      byIntent: {},
      avgTimeMs: 0,
      totalSaved: 0, // Estimated LLM calls saved
    };

    // Initialize intent stats
    for (const intent of Object.values(TransformIntent)) {
      this.stats.byIntent[intent] = 0;
    }
  }

  /**
   * Check if a request can be handled by Agent Booster
   *
   * @param {string} request - User request
   * @returns {Object|null} Detected intent or null
   */
  canHandle(request) {
    if (!request || typeof request !== 'string') return null;

    for (const { intent, pattern } of INTENT_PATTERNS) {
      if (pattern.test(request)) {
        return { intent, confidence: PHI_INV }; // φ⁻¹: Pattern match ≠ certainty
      }
    }

    return null;
  }

  /**
   * Transform code based on detected intent
   *
   * @param {Object} params - Transform parameters
   * @param {string} params.code - Source code to transform
   * @param {string} params.intent - Transform intent
   * @param {Object} [params.options] - Intent-specific options
   * @returns {Object} Transform result
   */
  transform(params) {
    const startTime = performance.now();
    const { code, intent, options = {} } = params;

    if (!code || typeof code !== 'string') {
      return this._result(TransformStatus.ERROR, code, 'Invalid code input', startTime);
    }

    let result;
    try {
      switch (intent) {
        case TransformIntent.VAR_TO_CONST:
          result = this._varToConst(code, options);
          break;
        case TransformIntent.VAR_TO_LET:
          result = this._varToLet(code, options);
          break;
        case TransformIntent.ADD_TYPES:
          result = this._addTypes(code, options);
          break;
        case TransformIntent.ADD_ASYNC_AWAIT:
          result = this._addAsyncAwait(code, options);
          break;
        case TransformIntent.ADD_ERROR_HANDLING:
          result = this._addErrorHandling(code, options);
          break;
        case TransformIntent.ADD_LOGGING:
          result = this._addLogging(code, options);
          break;
        case TransformIntent.REMOVE_CONSOLE:
          result = this._removeConsole(code, options);
          break;
        case TransformIntent.REMOVE_DEBUGGER:
          result = this._removeDebugger(code, options);
          break;
        case TransformIntent.ADD_SEMICOLONS:
          result = this._addSemicolons(code, options);
          break;
        case TransformIntent.REMOVE_UNUSED_IMPORTS:
          result = this._removeUnusedImports(code, options);
          break;
        case TransformIntent.SORT_IMPORTS:
          result = this._sortImports(code, options);
          break;
        case TransformIntent.ADD_STRICT_MODE:
          result = this._addStrictMode(code, options);
          break;
        default:
          return this._result(TransformStatus.UNSUPPORTED, code, `Unknown intent: ${intent}`, startTime);
      }

      this.stats.byIntent[intent]++;
      return this._result(result.status, result.code, result.message, startTime, result.changes);

    } catch (err) {
      return this._result(TransformStatus.ERROR, code, err.message, startTime);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRANSFORM IMPLEMENTATIONS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Convert var declarations to const
   * Only converts vars that are never reassigned
   */
  _varToConst(code, options) {
    // Find all var declarations
    const varPattern = /\bvar\s+(\w+)\s*=/g;
    const vars = [];
    let match;

    while ((match = varPattern.exec(code)) !== null) {
      vars.push({ name: match[1], index: match.index });
    }

    if (vars.length === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No var declarations found' };
    }

    let result = code;
    let changes = 0;

    // Check each var for reassignment
    for (const { name } of vars) {
      // Check if variable is reassigned (simple heuristic)
      const reassignPattern = new RegExp(`\\b${name}\\s*=(?!=)`, 'g');
      const assignments = (result.match(reassignPattern) || []).length;

      // If only one assignment (the declaration), safe to convert
      if (assignments <= 1) {
        const declPattern = new RegExp(`\\bvar\\s+(${name}\\s*=)`, 'g');
        result = result.replace(declPattern, 'const $1');
        changes++;
      }
    }

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'All vars are reassigned' };
    }

    return {
      status: changes === vars.length ? TransformStatus.SUCCESS : TransformStatus.PARTIAL,
      code: result,
      message: `Converted ${changes}/${vars.length} var to const`,
      changes,
    };
  }

  /**
   * Convert var declarations to let
   */
  _varToLet(code, options) {
    const result = code.replace(/\bvar\b/g, 'let');
    const changes = (code.match(/\bvar\b/g) || []).length;

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No var declarations found' };
    }

    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: `Converted ${changes} var to let`,
      changes,
    };
  }

  /**
   * Add basic TypeScript type annotations
   */
  _addTypes(code, options) {
    let result = code;
    let changes = 0;

    // Add : string to string literals
    result = result.replace(
      /(\w+)\s*=\s*(['"`])/g,
      (match, name, quote) => {
        changes++;
        return `${name}: string = ${quote}`;
      }
    );

    // Add : number to number literals
    result = result.replace(
      /(\w+)\s*=\s*(\d+(?:\.\d+)?)\s*[;,\n]/g,
      (match, name, num) => {
        changes++;
        return `${name}: number = ${num}${match.slice(-1)}`;
      }
    );

    // Add : boolean to boolean literals
    result = result.replace(
      /(\w+)\s*=\s*(true|false)\s*[;,\n]/g,
      (match, name, bool) => {
        changes++;
        return `${name}: boolean = ${bool}${match.slice(-1)}`;
      }
    );

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No obvious types to add' };
    }

    return {
      status: TransformStatus.PARTIAL,
      code: result,
      message: `Added ${changes} type annotations (basic inference)`,
      changes,
    };
  }

  /**
   * Convert .then() chains to async/await
   */
  _addAsyncAwait(code, options) {
    let result = code;
    let changes = 0;

    // Simple .then() to await conversion
    // Pattern: somePromise.then(result => { ... })
    result = result.replace(
      /(\w+)\.then\(\s*(\w+)\s*=>\s*\{/g,
      (match, promise, param) => {
        changes++;
        return `const ${param} = await ${promise};\n{`;
      }
    );

    // Convert function to async if it contains await
    if (changes > 0 && !result.includes('async function') && !result.includes('async (')) {
      result = result.replace(
        /\bfunction\s+(\w+)\s*\(/,
        'async function $1('
      );
    }

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No .then() chains found' };
    }

    return {
      status: TransformStatus.PARTIAL,
      code: result,
      message: `Converted ${changes} .then() to await (may need review)`,
      changes,
    };
  }

  /**
   * Wrap code in try-catch
   */
  _addErrorHandling(code, options) {
    const functionName = options.functionName || 'main';

    // Check if already has try-catch
    if (code.includes('try {') || code.includes('try{')) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'Already has error handling' };
    }

    // Wrap entire code in try-catch
    const indent = '  ';
    const indentedCode = code.split('\n').map(line => indent + line).join('\n');

    const result = `try {
${indentedCode}
} catch (error) {
  console.error('Error in ${functionName}:', error);
  throw error;
}`;

    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: 'Wrapped in try-catch',
      changes: 1,
    };
  }

  /**
   * Add console.log at function entries
   */
  _addLogging(code, options) {
    let result = code;
    let changes = 0;

    // Add logging at function start
    result = result.replace(
      /(function\s+(\w+)\s*\([^)]*\)\s*\{)/g,
      (match, full, name) => {
        changes++;
        return `${full}\n  console.log('[${name}] called');`;
      }
    );

    // Add logging at arrow function start (with body)
    result = result.replace(
      /(const\s+(\w+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>\s*\{)/g,
      (match, full, name) => {
        changes++;
        return `${full}\n  console.log('[${name}] called');`;
      }
    );

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No functions found to add logging' };
    }

    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: `Added logging to ${changes} functions`,
      changes,
    };
  }

  /**
   * Remove all console.* statements
   */
  _removeConsole(code, options) {
    // Remove console.* statements including multiline
    const result = code.replace(
      /^\s*console\.\w+\([^)]*\);?\s*$/gm,
      ''
    );

    // Also remove inline console statements
    const result2 = result.replace(
      /console\.\w+\([^)]*\);?\s*/g,
      ''
    );

    const originalLines = code.split('\n').length;
    const resultLines = result2.split('\n').filter(l => l.trim()).length;
    const changes = originalLines - resultLines;

    if (result2 === code) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No console statements found' };
    }

    return {
      status: TransformStatus.SUCCESS,
      code: result2.replace(/\n\s*\n\s*\n/g, '\n\n'), // Clean up extra newlines
      message: `Removed console statements`,
      changes: Math.max(1, changes),
    };
  }

  /**
   * Remove debugger statements
   */
  _removeDebugger(code, options) {
    const result = code.replace(/^\s*debugger;?\s*$/gm, '');
    const changes = (code.match(/debugger/g) || []).length;

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No debugger statements found' };
    }

    return {
      status: TransformStatus.SUCCESS,
      code: result.replace(/\n\s*\n\s*\n/g, '\n\n'),
      message: `Removed ${changes} debugger statements`,
      changes,
    };
  }

  /**
   * Add missing semicolons
   */
  _addSemicolons(code, options) {
    // Add semicolons at end of statements (simple heuristic)
    const result = code.replace(
      /^(\s*(?:const|let|var|return|throw|import|export)\s+.+[^;{}\s])$/gm,
      '$1;'
    );

    const originalSemis = (code.match(/;/g) || []).length;
    const resultSemis = (result.match(/;/g) || []).length;
    const changes = resultSemis - originalSemis;

    if (changes === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No missing semicolons found' };
    }

    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: `Added ${changes} semicolons`,
      changes,
    };
  }

  /**
   * Remove unused imports (basic detection)
   */
  _removeUnusedImports(code, options) {
    const lines = code.split('\n');
    const importLines = [];
    const otherLines = [];

    // Separate imports from other code
    for (const line of lines) {
      if (/^\s*import\s+/.test(line)) {
        importLines.push(line);
      } else {
        otherLines.push(line);
      }
    }

    if (importLines.length === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'No imports found' };
    }

    const otherCode = otherLines.join('\n');
    const keptImports = [];
    let removed = 0;

    for (const importLine of importLines) {
      // Extract imported names
      const match = importLine.match(/import\s+(?:\{([^}]+)\}|(\w+))/);
      if (!match) {
        keptImports.push(importLine);
        continue;
      }

      const names = (match[1] || match[2]).split(',').map(n => n.trim().split(/\s+as\s+/).pop());
      const usedNames = names.filter(name => {
        const namePattern = new RegExp(`\\b${name}\\b`);
        return namePattern.test(otherCode);
      });

      if (usedNames.length === 0) {
        removed++;
      } else if (usedNames.length < names.length) {
        // Partial - rebuild import with only used names
        const newImport = importLine.replace(
          /\{[^}]+\}/,
          `{ ${usedNames.join(', ')} }`
        );
        keptImports.push(newImport);
        removed++;
      } else {
        keptImports.push(importLine);
      }
    }

    if (removed === 0) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'All imports are used' };
    }

    const result = [...keptImports, '', ...otherLines].join('\n');
    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: `Removed ${removed} unused imports`,
      changes: removed,
    };
  }

  /**
   * Sort imports alphabetically
   */
  _sortImports(code, options) {
    const lines = code.split('\n');
    const importLines = [];
    const otherLines = [];
    let inImportSection = true;

    for (const line of lines) {
      if (/^\s*import\s+/.test(line)) {
        importLines.push(line);
      } else if (line.trim() === '' && inImportSection && importLines.length > 0) {
        // Keep blank line after imports
        continue;
      } else {
        inImportSection = false;
        otherLines.push(line);
      }
    }

    if (importLines.length <= 1) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'Not enough imports to sort' };
    }

    // Sort imports
    const sortedImports = [...importLines].sort((a, b) => {
      // Extract module name for sorting
      const modA = (a.match(/from\s+['"]([^'"]+)['"]/)?.[1] || '').toLowerCase();
      const modB = (b.match(/from\s+['"]([^'"]+)['"]/)?.[1] || '').toLowerCase();
      return modA.localeCompare(modB);
    });

    // Check if already sorted
    const alreadySorted = importLines.every((line, i) => line === sortedImports[i]);
    if (alreadySorted) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'Imports already sorted' };
    }

    const result = [...sortedImports, '', ...otherLines].join('\n');
    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: `Sorted ${importLines.length} imports`,
      changes: 1,
    };
  }

  /**
   * Add 'use strict' directive
   */
  _addStrictMode(code, options) {
    if (code.includes("'use strict'") || code.includes('"use strict"')) {
      return { status: TransformStatus.NO_CHANGE, code, message: 'Already has use strict' };
    }

    const result = `'use strict';\n\n${code}`;
    return {
      status: TransformStatus.SUCCESS,
      code: result,
      message: 'Added use strict',
      changes: 1,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create result object and update stats
   * @private
   */
  _result(status, code, message, startTime, changes = 0) {
    const elapsed = performance.now() - startTime;

    this.stats.transforms++;
    this.stats.avgTimeMs =
      (this.stats.avgTimeMs * (this.stats.transforms - 1) + elapsed) /
      this.stats.transforms;

    if (status === TransformStatus.SUCCESS || status === TransformStatus.PARTIAL) {
      this.stats.totalSaved++;
    }

    this.emit('transform', { status, elapsed, changes });

    return {
      status,
      code,
      message,
      elapsed,
      changes,
    };
  }

  /**
   * Get booster statistics
   * @returns {Object} Stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      transforms: 0,
      byIntent: {},
      avgTimeMs: 0,
      totalSaved: 0,
    };
    for (const intent of Object.values(TransformIntent)) {
      this.stats.byIntent[intent] = 0;
    }
  }
}

/**
 * Create Agent Booster instance
 *
 * @param {Object} [options] - Options
 * @returns {AgentBooster}
 */
export function createAgentBooster(options = {}) {
  return new AgentBooster(options);
}

export default {
  AgentBooster,
  createAgentBooster,
  TransformIntent,
  TransformStatus,
};
