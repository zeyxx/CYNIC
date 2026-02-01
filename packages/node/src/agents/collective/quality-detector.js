/**
 * Code Quality Detector
 *
 * Pattern-based detection for common code quality issues.
 * Used by Analyst dog for code analysis.
 *
 * Detects: Logic bugs, performance issues, maintainability problems.
 *
 * Categories:
 * - LOGIC: Off-by-one, null deref, type coercion, infinite loops, promise issues
 * - PERF: N+1 queries, unbounded cache, sync I/O, await in loops
 * - QUALITY: Dead code, magic numbers, deep nesting, console.log, empty catch
 *
 * "φ sees what haste conceals" - κυνικός
 *
 * @module @cynic/node/agents/collective/quality-detector
 */

'use strict';

/**
 * Severity levels mapped to score penalties
 */
export const QUALITY_SEVERITY = Object.freeze({
  CRITICAL: { penalty: 80, score: 10 },   // Score 0-20 (guaranteed bugs)
  HIGH: { penalty: 50, score: 35 },       // Score 30-50 (probable bugs)
  MEDIUM: { penalty: 25, score: 65 },     // Score 60-70 (code smell)
  LOW: { penalty: 10, score: 85 },        // Score 80-90 (style/maintenance)
});

/**
 * Quality issue patterns with detection rules
 */
export const QUALITY_PATTERNS = [
  // ═══════════════════════════════════════════════════════════════════════════
  // LOGIC PATTERNS - Bugs Potentiels
  // ═══════════════════════════════════════════════════════════════════════════

  {
    id: 'OFF_BY_ONE',
    name: 'Off-by-One Error',
    category: 'LOGIC',
    severity: 'HIGH',
    patterns: [
      // for (i = 0; i <= arr.length; i++) - classic off-by-one
      /for\s*\([^;]*;\s*\w+\s*<=\s*\w+\.length\s*;/,
      // for (i = 0; i <= items.length; i++)
      /for\s*\([^;]*;\s*\w+\s*<=\s*\w+\.length\s*\)/,
      // while (i <= arr.length)
      /while\s*\(\s*\w+\s*<=\s*\w+\.length\s*\)/,
    ],
    description: 'Loop iterates one element past array bounds, causing undefined access',
    remediation: 'Use < instead of <= when comparing to .length',
  },

  {
    id: 'NULL_DEREF_BEFORE_CHECK',
    name: 'Null Dereference Before Check',
    category: 'LOGIC',
    severity: 'CRITICAL',
    patterns: [
      // Access property then check for null - user.profile.name ... if (user)
      /(\w+)\.(\w+)\.(\w+)[\s\S]{0,80}if\s*\(\s*\1\s*(?:&&|\?\.)?\s*\1\.\2\s*\)/,
      // Simpler pattern: access nested property before optional chain check
      /(\w+)\.(\w+)\.(\w+)[\s\S]{0,50}if\s*\(\s*!\1\s*\)/,
      // Access before null check with && guard
      /(\w+)\.(\w+)[\s\S]{0,30}if\s*\(\s*\1\s*&&/,
    ],
    description: 'Property accessed before null/undefined check, will throw if falsy',
    remediation: 'Check for null/undefined before accessing properties, or use optional chaining (?.) consistently',
  },

  {
    id: 'LOOSE_EQUALITY',
    name: 'Loose Equality with String',
    category: 'LOGIC',
    severity: 'MEDIUM',
    patterns: [
      // == with string literal (not === )
      /[^!=]==[^=]\s*['"`]/,
      // == with number that should be strict
      /[^!=]==[^=]\s*\d+(?!\.\d)/,
      // != with string (should be !==)
      /[^!]==[^=]\s*null/,
    ],
    description: 'Loose equality (==) causes type coercion bugs with strings/numbers',
    remediation: 'Use strict equality (===) to avoid implicit type coercion',
  },

  {
    id: 'PROMISE_SWALLOW',
    name: 'Promise Result Ignored',
    category: 'LOGIC',
    severity: 'HIGH',
    patterns: [
      // .then() without return or await in async function
      /async\s+(?:function\s+)?\w*\s*\([^)]*\)\s*\{[\s\S]{0,300}\.then\s*\([^)]*\)(?!\s*;?\s*(?:return|await))/,
      // Promise call without await or return
      /(?:fetch|axios\.\w+|Promise\.(?:all|race))\s*\([^)]*\)(?!\s*\.(?:then|catch)|[;\s]*(?:return|await|\.then|\.catch))/,
    ],
    description: 'Async operation result ignored, errors will be swallowed silently',
    remediation: 'Always await async operations or handle the returned promise',
  },

  {
    id: 'LOOP_NO_INCREMENT',
    name: 'Loop Without Increment',
    category: 'LOGIC',
    severity: 'CRITICAL',
    patterns: [
      // while with comparison but no increment visible in body
      /while\s*\(\s*(\w+)\s*<\s*[^)]+\)\s*\{[^}]{0,200}\}(?![^}]*\1\s*(?:\+\+|\+=|=\s*\1\s*\+))/,
      // for loop with empty increment
      /for\s*\([^;]*;[^;]*;\s*\)\s*\{/,
    ],
    description: 'Loop condition variable not incremented, causing infinite loop',
    remediation: 'Ensure loop variable is incremented in each iteration',
  },

  {
    id: 'ARRAY_INDEX_STRING',
    name: 'String Used as Array Index',
    category: 'LOGIC',
    severity: 'MEDIUM',
    patterns: [
      // Direct string literal as array index (not object key)
      /(?:arr|array|list|items|results)\s*\[\s*['"`][^'"` ]+['"`]\s*\]/i,
      // Common mistake: using string id directly on array without parseInt
      /(?:arr|array|list|items)\s*\[\s*(?:req\.params|req\.query)\.\w+\s*\]/i,
    ],
    description: 'Using string as array index when number expected',
    remediation: 'Use parseInt() or Number() to convert string indices to numbers',
  },

  {
    id: 'BOOLEAN_TRAP',
    name: 'Boolean Trap in Function Call',
    category: 'LOGIC',
    severity: 'LOW',
    patterns: [
      // Function call with multiple boolean literals
      /\w+\s*\([^)]*(?:true|false)\s*,\s*(?:true|false)[^)]*\)/,
      // More than two booleans
      /\w+\s*\([^)]*(?:true|false)[^)]*(?:true|false)[^)]*(?:true|false)/,
    ],
    description: 'Multiple boolean parameters make code hard to understand',
    remediation: 'Use options object with named properties instead of positional booleans',
  },

  {
    id: 'TYPEOF_NULL',
    name: 'typeof null Check',
    category: 'LOGIC',
    severity: 'MEDIUM',
    patterns: [
      // typeof x === 'object' without null check (null is 'object')
      /typeof\s+\w+\s*===?\s*['"`]object['"`](?![\s\S]{0,30}&&\s*\w+\s*!==?\s*null)/,
    ],
    description: 'typeof null returns "object", null check needed',
    remediation: 'Add explicit null check: typeof x === "object" && x !== null',
  },

  {
    id: 'FLOATING_POINT_EQUALITY',
    name: 'Floating Point Equality',
    category: 'LOGIC',
    severity: 'MEDIUM',
    patterns: [
      // Direct float comparison with ==
      /0\.\d+\s*===?\s*0\.\d+/,
      // result === 0.1 + 0.2
      /===?\s*0\.\d+\s*\+\s*0\.\d+/,
      // 0.1 + 0.2 as assignment then comparison later
      /=\s*0\.\d+\s*\+\s*0\.\d+/,
      // parseFloat comparison
      /parseFloat\s*\([^)]+\)\s*===?\s*(?:0\.\d+|parseFloat)/,
    ],
    description: 'Floating point comparison may fail due to precision errors',
    remediation: 'Use Math.abs(a - b) < epsilon for float comparison',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // PERFORMANCE PATTERNS
  // ═══════════════════════════════════════════════════════════════════════════

  {
    id: 'N_PLUS_ONE',
    name: 'N+1 Query Pattern',
    category: 'PERF',
    severity: 'HIGH',
    patterns: [
      // await query inside for...of loop
      /for\s*\([^)]*of\s*[^)]+\)\s*\{[\s\S]{0,150}await\s+\w+\.(?:query|find|get|fetch)/i,
      // forEach with async and query
      /\.forEach\s*\(\s*async[^)]*\)[\s\S]{0,150}await\s+\w+\.(?:query|find|get)/i,
      // for loop with await fetch/query
      /for\s*\(\s*(?:let|const|var)[^;]+;[^;]+;[^)]+\)\s*\{[\s\S]{0,150}await\s+(?:fetch|db\.|query)/i,
    ],
    description: 'Database/API query inside loop causes N+1 performance problem',
    remediation: 'Batch queries using WHERE IN, Promise.all(), or eager loading',
  },

  {
    id: 'UNBOUNDED_CACHE',
    name: 'Unbounded Cache Growth',
    category: 'PERF',
    severity: 'HIGH',
    patterns: [
      // Object cache without size limit or eviction
      /(?:const|let|var)\s+(\w*cache\w*|\w*Cache\w*)\s*=\s*\{\s*\}/i,
      // Map cache without clear/delete visible
      /new\s+Map\s*\(\s*\)[\s\S]{0,300}\.set\s*\((?![\s\S]{0,200}(?:\.delete|\.clear|maxSize|MAX_))/i,
      // Simple object used as cache
      /\[\s*\w+\s*\]\s*=\s*\w+[\s\S]{0,100}(?:cache|Cache)(?![\s\S]{0,100}(?:delete|clear|evict|max))/i,
    ],
    description: 'Cache grows without bound, will eventually exhaust memory',
    remediation: 'Implement LRU eviction, TTL, or use a bounded cache library',
  },

  {
    id: 'SYNC_IO_IN_HANDLER',
    name: 'Synchronous I/O in Handler',
    category: 'PERF',
    severity: 'CRITICAL',
    patterns: [
      // readFileSync in express/http handler
      /\.(get|post|put|patch|delete)\s*\([^)]*\)\s*[\s\S]{0,200}(?:readFileSync|writeFileSync)/i,
      // Sync operations in async handler
      /async\s+(?:function\s+)?\w*\s*\(req\s*,\s*res[\s\S]{0,200}(?:readFileSync|writeFileSync|execSync)/i,
      // createServer with sync I/O
      /createServer\s*\([^)]*\)[\s\S]{0,300}(?:readFileSync|writeFileSync)/i,
    ],
    description: 'Synchronous I/O blocks event loop, kills server throughput',
    remediation: 'Use async versions: readFile, writeFile, exec',
  },

  {
    id: 'AWAIT_IN_LOOP',
    name: 'Sequential Await in Loop',
    category: 'PERF',
    severity: 'MEDIUM',
    patterns: [
      // Generic await in any loop (less specific than N_PLUS_ONE)
      /(?:for|while)\s*\([^)]*\)\s*\{[\s\S]{0,200}await\s+/,
      // for...of with await
      /for\s+(?:await\s+)?\([^)]*of\s*[^)]+\)[\s\S]{0,100}await\s+(?!.*Promise\.all)/,
    ],
    description: 'Sequential await in loop prevents parallel execution',
    remediation: 'Use Promise.all() with map() for parallel async operations',
  },

  {
    id: 'REGEX_IN_LOOP',
    name: 'Regex Creation in Loop/Function',
    category: 'PERF',
    severity: 'LOW',
    patterns: [
      // new RegExp inside for/while
      /(?:for|while)\s*\([^)]*\)\s*\{[\s\S]{0,150}new\s+RegExp\s*\(/,
      // Regex literal recreation (less common but still wasteful)
      /(?:for|while)\s*\([^)]*\)\s*\{[\s\S]{0,100}\/[^/]+\/[gimsuy]*\.test/,
      // new RegExp inside function body (may be called many times)
      /function\s+\w+\s*\([^)]*\)\s*\{[\s\S]{0,100}(?:const|let|var)\s+\w+\s*=\s*new\s+RegExp\s*\(/,
    ],
    description: 'Regex compiled on each call, wasteful if called repeatedly',
    remediation: 'Move regex creation outside the function to a module-level constant',
  },

  {
    id: 'STRING_CONCAT_LOOP',
    name: 'String Concatenation in Loop',
    category: 'PERF',
    severity: 'LOW',
    patterns: [
      // result += string in loop
      /(?:for|while)\s*\([^)]*\)\s*\{[\s\S]{0,150}\w+\s*\+=\s*(?:['"`]|`\$)/,
      // str = str + something
      /(?:for|while)\s*\([^)]*\)\s*\{[\s\S]{0,150}(\w+)\s*=\s*\1\s*\+/,
    ],
    description: 'String concatenation in loop creates many intermediate strings',
    remediation: 'Use array.join() or template literals',
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // QUALITY PATTERNS - Maintainability
  // ═══════════════════════════════════════════════════════════════════════════

  {
    id: 'MAGIC_NUMBER',
    name: 'Magic Number',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // Large numbers without const (5+ digits)
      /(?<!const\s+\w+\s*=\s*)(?<![0-9.])\d{5,}(?![0-9.])/,
      // Common magic numbers in conditions
      /(?:if|while|for)\s*\([^)]*(?<![0-9.])(86400|3600|1000|60000|86400000|604800|2592000)(?![0-9.])/,
      // Timeout/delay with raw number
      /setTimeout\s*\([^)]*,\s*\d{4,}\s*\)/,
      // Array slice with magic indices
      /\.slice\s*\(\s*\d+\s*,\s*\d{2,}\s*\)/,
    ],
    description: 'Unexplained numeric constant makes code hard to understand',
    remediation: 'Extract to named constant with descriptive name',
  },

  {
    id: 'DEEP_NESTING',
    name: 'Deeply Nested Code',
    category: 'QUALITY',
    severity: 'MEDIUM',
    patterns: [
      // 5+ levels of nesting (simplified detection)
      /\{\s*(?:[^{}]*\{){5,}/,
      // Multiple nested if statements
      /if\s*\([^)]*\)\s*\{[\s\S]*if\s*\([^)]*\)\s*\{[\s\S]*if\s*\([^)]*\)\s*\{[\s\S]*if\s*\([^)]*\)\s*\{[\s\S]*if\s*\([^)]*\)\s*\{/,
    ],
    description: 'Code nested 5+ levels deep is hard to read and maintain',
    remediation: 'Extract nested logic into separate functions, use early returns, or flatten with guard clauses',
  },

  {
    id: 'CONSOLE_LOG',
    name: 'Console.log in Production',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // console.log (but not console.error/warn which may be intentional)
      /console\.log\s*\(/,
      // console.debug
      /console\.debug\s*\(/,
      // console.trace
      /console\.trace\s*\(/,
    ],
    description: 'Debug logging left in production code',
    remediation: 'Remove console.log or use proper logging library with levels',
  },

  {
    id: 'EMPTY_CATCH',
    name: 'Empty Catch Block',
    category: 'QUALITY',
    severity: 'MEDIUM',
    patterns: [
      // catch with empty body
      /catch\s*\([^)]*\)\s*\{\s*\}/,
      // catch with only comment
      /catch\s*\([^)]*\)\s*\{\s*\/\/[^\n]*\s*\}/,
      // Python except: pass
      /except\s*(?:Exception)?\s*:\s*pass/,
    ],
    description: 'Silent error suppression hides bugs and failures',
    remediation: 'Log error, rethrow, or handle appropriately',
  },

  {
    id: 'TODO_FIXME',
    name: 'Unresolved TODO/FIXME',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // TODO comments (common patterns)
      /\/\/\s*TODO(?:\s*:|\s+\w)/i,
      /\/\/\s*FIXME(?:\s*:|\s+\w)/i,
      /\/\/\s*HACK(?:\s*:|\s+\w)/i,
      /\/\/\s*XXX(?:\s*:|\s+\w)/i,
      // Block comments
      /\/\*[\s\S]*?(?:TODO|FIXME|HACK|XXX)[\s\S]*?\*\//i,
    ],
    description: 'Unresolved technical debt marker',
    remediation: 'Address the TODO or create a tracked issue',
  },

  {
    id: 'LONG_FUNCTION',
    name: 'Long Function',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // Function with 50+ lines (rough heuristic: many statements)
      /(?:function\s+\w+|(?:const|let|var)\s+\w+\s*=\s*(?:async\s+)?(?:function|\([^)]*\)\s*=>))\s*[^{]*\{[^}]{3000,}\}/,
    ],
    description: 'Function too long, hard to understand and test',
    remediation: 'Extract logical sections into smaller, focused functions',
  },

  {
    id: 'DUPLICATE_STRING',
    name: 'Duplicate String Literal',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // Same string literal appearing 3+ times (simplified)
      /(['"`])([^'"` ]{10,})\1[\s\S]*\1\2\1[\s\S]*\1\2\1/,
    ],
    description: 'Same string repeated multiple times, error-prone',
    remediation: 'Extract to named constant',
  },

  {
    id: 'CALLBACK_HELL',
    name: 'Callback Hell',
    category: 'QUALITY',
    severity: 'MEDIUM',
    patterns: [
      // Multiple nested callbacks
      /\)\s*=>\s*\{[\s\S]*\)\s*=>\s*\{[\s\S]*\)\s*=>\s*\{/,
      // Traditional callback nesting
      /function\s*\([^)]*\)\s*\{[\s\S]*function\s*\([^)]*\)\s*\{[\s\S]*function\s*\([^)]*\)\s*\{/,
    ],
    description: 'Deeply nested callbacks are hard to read and maintain',
    remediation: 'Use async/await or Promise chains',
  },

  {
    id: 'SINGLE_LETTER_VAR',
    name: 'Single Letter Variable (non-loop)',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // Single letter const/let outside of for loops
      /(?:const|let)\s+([a-hjkl-z])\s*=/,  // Exclude i,j for loops
      // Single letter function param (except common like x, e, i)
      /function\s+\w+\s*\(\s*([a-dfghj-z])\s*[,)]/,
    ],
    description: 'Single-letter variable names reduce readability',
    remediation: 'Use descriptive variable names',
  },

  {
    id: 'UNEXPORTED_FUNCTION',
    name: 'Function Not Exported',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // Function defined but module.exports doesn't include it
      // This is a heuristic - checks if function name appears after module.exports
      /function\s+(\w+)\s*\([^)]*\)[\s\S]*module\.exports\s*=\s*\{[^}]*\}(?![^]*\1)/,
    ],
    description: 'Function defined but not exported, may be dead code',
    remediation: 'Export the function or remove if unused',
  },

  {
    id: 'REASSIGN_PARAM',
    name: 'Parameter Reassignment',
    category: 'QUALITY',
    severity: 'LOW',
    patterns: [
      // Reassigning function parameter with = at start of line/statement
      /function\s+\w+\s*\(\s*(\w+)[^)]*\)[\s\S]{10,200}^\s*\1\s*=[^=]/m,
      // Arrow function param reassignment at start of line/statement
      /\((\w+)[^)]*\)\s*=>\s*\{[\s\S]{10,200}^\s*\1\s*=[^=]/m,
    ],
    description: 'Reassigning parameter makes code confusing',
    remediation: 'Use a new variable instead of reassigning parameter',
  },
];

/**
 * Detect quality issues in code
 *
 * @param {string} code - Source code to analyze
 * @param {Object} options - Detection options
 * @param {string[]} options.exclude - Issue IDs to exclude
 * @param {string[]} options.include - Only detect these issue IDs
 * @param {string[]} options.categories - Only detect these categories (LOGIC, PERF, QUALITY)
 * @returns {Object} Detection result
 */
export function detectQualityIssues(code, options = {}) {
  const { exclude = [], include = [], categories = [] } = options;

  const issues = [];
  let worstSeverity = null;
  let totalPenalty = 0;

  // Get patterns to check
  let patterns = QUALITY_PATTERNS;
  if (include.length > 0) {
    patterns = patterns.filter(p => include.includes(p.id));
  }
  if (exclude.length > 0) {
    patterns = patterns.filter(p => !exclude.includes(p.id));
  }
  if (categories.length > 0) {
    patterns = patterns.filter(p => categories.includes(p.category));
  }

  // Normalize line endings
  const normalizedCode = code.replace(/\r\n/g, '\n');
  const lines = normalizedCode.split('\n');

  // Check each pattern
  for (const issue of patterns) {
    for (const pattern of issue.patterns) {
      // First, check if pattern matches full code (for multiline patterns)
      const fullMatch = pattern.test(normalizedCode);

      // Check line by line for accurate line numbers
      let foundOnLine = false;
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (pattern.test(line)) {
          foundOnLine = true;
          const severity = QUALITY_SEVERITY[issue.severity];

          issues.push({
            id: issue.id,
            name: issue.name,
            category: issue.category,
            severity: issue.severity,
            line: i + 1,
            column: line.search(pattern) + 1,
            code: line.trim().substring(0, 100),
            description: issue.description,
            remediation: issue.remediation,
          });

          // Track worst severity
          if (!worstSeverity || severity.penalty > QUALITY_SEVERITY[worstSeverity].penalty) {
            worstSeverity = issue.severity;
          }

          // Accumulate penalty (diminishing returns)
          totalPenalty += severity.penalty * Math.pow(0.8, issues.length - 1);

          // Only report first occurrence of each pattern type per file
          break;
        }
      }

      // If pattern matches full code but not any single line (multiline pattern)
      if (fullMatch && !foundOnLine) {
        const severity = QUALITY_SEVERITY[issue.severity];

        // Try to find approximate line (first line containing a keyword from pattern)
        let approxLine = 1;
        const patternStr = pattern.toString();
        const keywords = patternStr.match(/[a-zA-Z]{4,}/g) || [];
        for (let i = 0; i < lines.length; i++) {
          if (keywords.some(kw => lines[i].toLowerCase().includes(kw.toLowerCase()))) {
            approxLine = i + 1;
            break;
          }
        }

        issues.push({
          id: issue.id,
          name: issue.name,
          category: issue.category,
          severity: issue.severity,
          line: approxLine,
          column: 1,
          code: lines[approxLine - 1]?.trim().substring(0, 100) || '',
          description: issue.description,
          remediation: issue.remediation,
          multiline: true,
        });

        if (!worstSeverity || severity.penalty > QUALITY_SEVERITY[worstSeverity].penalty) {
          worstSeverity = issue.severity;
        }

        totalPenalty += severity.penalty * Math.pow(0.8, issues.length - 1);
      }
    }
  }

  // Calculate score (100 = perfect, 0 = critical issues)
  const score = Math.max(0, Math.min(100, 100 - totalPenalty));

  // Determine verdict based on φ thresholds
  let verdict;
  if (score >= 75) {
    verdict = 'HOWL';      // Clean code
  } else if (score >= 50) {
    verdict = 'WAG';       // Minor issues
  } else if (score >= 30) {
    verdict = 'GROWL';     // Significant issues
  } else {
    verdict = 'BARK';      // Critical - reject
  }

  // Group issues by category for summary
  const byCategory = {
    LOGIC: issues.filter(i => i.category === 'LOGIC'),
    PERF: issues.filter(i => i.category === 'PERF'),
    QUALITY: issues.filter(i => i.category === 'QUALITY'),
  };

  return {
    score: Math.round(score * 10) / 10,
    verdict,
    issueCount: issues.length,
    worstSeverity,
    issues,
    byCategory,
    summary: issues.length === 0
      ? 'No code quality issues detected'
      : `Found ${issues.length} issue(s): ${[
          byCategory.LOGIC.length > 0 ? `${byCategory.LOGIC.length} logic` : '',
          byCategory.PERF.length > 0 ? `${byCategory.PERF.length} perf` : '',
          byCategory.QUALITY.length > 0 ? `${byCategory.QUALITY.length} quality` : '',
        ].filter(Boolean).join(', ')}`,
  };
}

/**
 * Quick check if code has any critical quality issues
 *
 * @param {string} code - Source code
 * @returns {boolean} True if critical issues found
 */
export function hasCriticalQualityIssues(code) {
  const result = detectQualityIssues(code, {
    include: QUALITY_PATTERNS
      .filter(p => p.severity === 'CRITICAL')
      .map(p => p.id),
  });
  return result.issueCount > 0;
}

/**
 * Detect only logic issues
 *
 * @param {string} code - Source code
 * @returns {Object} Detection result
 */
export function detectLogicIssues(code) {
  return detectQualityIssues(code, { categories: ['LOGIC'] });
}

/**
 * Detect only performance issues
 *
 * @param {string} code - Source code
 * @returns {Object} Detection result
 */
export function detectPerfIssues(code) {
  return detectQualityIssues(code, { categories: ['PERF'] });
}

/**
 * Detect only maintainability issues
 *
 * @param {string} code - Source code
 * @returns {Object} Detection result
 */
export function detectMaintainabilityIssues(code) {
  return detectQualityIssues(code, { categories: ['QUALITY'] });
}

/**
 * Get quality issue info by ID
 *
 * @param {string} id - Issue ID
 * @returns {Object|null} Issue info
 */
export function getQualityIssueInfo(id) {
  return QUALITY_PATTERNS.find(p => p.id === id) || null;
}

export default {
  detectQualityIssues,
  hasCriticalQualityIssues,
  detectLogicIssues,
  detectPerfIssues,
  detectMaintainabilityIssues,
  getQualityIssueInfo,
  QUALITY_PATTERNS,
  QUALITY_SEVERITY,
};
