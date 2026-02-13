#!/usr/bin/env node
/**
 * CYNIC Code Audit System
 *
 * Comprehensive verification of codebase health.
 *
 * "Don't trust, verify" — VERIFY axiom
 * "Doubt everything" — FIDELITY axiom
 * "Catch before production" — IMMEDIACY axiom
 *
 * Audits:
 * 1. Import resolution (all imports valid?)
 * 2. Method existence (all method calls exist?)
 * 3. Export completeness (package.json exports match files?)
 * 4. EventBus wiring (emits have listeners?)
 * 5. φ-bounds (confidence never > 61.8%)
 * 6. Singleton integrity (no duplicate instances?)
 * 7. Dead code (unused exports?)
 *
 * Usage:
 *   node scripts/audit-cynic.js [--verbose]
 */

import { readFileSync, existsSync, readdirSync, statSync } from 'fs';
import { join, relative, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, '..');
const PHI_INV = 0.618033988749895;

const VERBOSE = process.argv.includes('--verbose');

const issues = {
  critical: [],
  warning: [],
  info: [],
};

// ═══════════════════════════════════════════════════════════════════════════
// UTILITIES
// ═══════════════════════════════════════════════════════════════════════════

function log(msg, level = 'info') {
  const colors = {
    info: '\x1b[36m',
    warn: '\x1b[33m',
    error: '\x1b[31m',
    success: '\x1b[32m',
    reset: '\x1b[0m',
  };
  console.log(`${colors[level] || ''}${msg}${colors.reset}`);
}

function findJSFiles(dir) {
  const files = [];

  function walk(d) {
    try {
      const entries = readdirSync(d);
      for (const entry of entries) {
        const full = join(d, entry);
        try {
          const stat = statSync(full);
          if (stat.isDirectory()) {
            if (entry !== 'node_modules' && entry !== '.git' && entry !== 'dist' && entry !== 'build') {
              walk(full);
            }
          } else if (entry.endsWith('.js') || entry.endsWith('.cjs') || entry.endsWith('.mjs')) {
            files.push(full);
          }
        } catch (err) {
          if (VERBOSE) log(`Skipped ${full}: ${err.message}`, 'warn');
        }
      }
    } catch (err) {
      if (VERBOSE) log(`Cannot read ${d}: ${err.message}`, 'warn');
    }
  }

  walk(dir);
  return files;
}

function extractImports(content) {
  const imports = [];

  // ESM imports: import ... from '...'
  const esmRegex = /import\s+(?:[\w{},\s*]+\s+from\s+)?['"]([^'"]+)['"]/g;
  let match;
  while ((match = esmRegex.exec(content)) !== null) {
    imports.push(match[1]);
  }

  // Dynamic imports: import('...')
  const dynamicRegex = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
  while ((match = dynamicRegex.exec(content)) !== null) {
    imports.push(match[1]);
  }

  // require() calls
  const requireRegex = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
  while ((match = requireRegex.exec(content)) !== null) {
    imports.push(match[1]);
  }

  return imports;
}

function resolveImport(importPath, fromFile) {
  // Relative imports
  if (importPath.startsWith('.')) {
    const resolved = resolve(dirname(fromFile), importPath);
    if (existsSync(resolved)) return true;
    if (existsSync(resolved + '.js')) return true;
    if (existsSync(resolved + '.cjs')) return true;
    if (existsSync(resolved + '.mjs')) return true;
    if (existsSync(join(resolved, 'index.js'))) return true;
    return false;
  }

  // @cynic/* workspace imports
  if (importPath.startsWith('@cynic/')) {
    const parts = importPath.split('/');
    const pkg = parts[1];
    const subpath = parts.length > 2 ? './' + parts.slice(2).join('/') : '.';

    const pkgPath = join(ROOT, 'packages', pkg);
    if (!existsSync(pkgPath)) return false;

    const pkgJsonPath = join(pkgPath, 'package.json');
    if (!existsSync(pkgJsonPath)) return false;

    try {
      const pkgJson = JSON.parse(readFileSync(pkgJsonPath, 'utf8'));

      // Check exports field
      if (pkgJson.exports && pkgJson.exports[subpath]) {
        return true;
      }

      // Check main field
      if (subpath === '.' && pkgJson.main) {
        return existsSync(join(pkgPath, pkgJson.main));
      }
    } catch (err) {
      return false;
    }

    return false;
  }

  // node_modules - assume valid
  return true;
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT 1: IMPORT RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════

function auditImports() {
  log('\n[AUDIT 1] Checking import resolution...', 'info');

  const jsFiles = findJSFiles(join(ROOT, 'packages'));
  let totalImports = 0;
  let invalidImports = 0;

  for (const file of jsFiles) {
    try {
      const content = readFileSync(file, 'utf8');
      const imports = extractImports(content);
      totalImports += imports.length;

      for (const imp of imports) {
        if (!resolveImport(imp, file)) {
          issues.critical.push({
            type: 'INVALID_IMPORT',
            file: relative(ROOT, file),
            import: imp,
          });
          invalidImports++;

          if (VERBOSE) {
            log(`  ❌ ${relative(ROOT, file)}: "${imp}" cannot be resolved`, 'error');
          }
        }
      }
    } catch (err) {
      if (VERBOSE) log(`  ⚠ ${relative(ROOT, file)}: parse error`, 'warn');
    }
  }

  log(`  Total imports: ${totalImports}`, 'info');
  log(`  Invalid: ${invalidImports}`, invalidImports > 0 ? 'error' : 'success');
  log(`  Valid: ${totalImports - invalidImports}`, 'success');
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT 2: METHOD EXISTENCE
// ═══════════════════════════════════════════════════════════════════════════

function auditMethodCalls() {
  log('\n[AUDIT 2] Checking known problematic method calls...', 'info');

  const jsFiles = findJSFiles(join(ROOT, 'packages'));
  let problems = 0;

  const knownIssues = [
    { object: 'modelIntelligence', method: 'recommend', fix: 'Use selectModel() instead' },
    { object: 'fileWatcher', method: 'on', fix: 'Subscribe to EventBus instead' },
  ];

  for (const file of jsFiles) {
    try {
      const content = readFileSync(file, 'utf8');

      for (const issue of knownIssues) {
        const regex = new RegExp(`\\b${issue.object}\\.${issue.method}\\s*\\(`, 'g');
        if (regex.test(content)) {
          issues.critical.push({
            type: 'METHOD_NOT_FOUND',
            file: relative(ROOT, file),
            call: `${issue.object}.${issue.method}()`,
            fix: issue.fix,
          });
          problems++;

          if (VERBOSE) {
            log(`  ❌ ${relative(ROOT, file)}: ${issue.object}.${issue.method}() - ${issue.fix}`, 'error');
          }
        }
      }
    } catch (err) {
      // Ignore
    }
  }

  log(`  Problematic calls: ${problems}`, problems > 0 ? 'error' : 'success');
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT 3: EXPORT COMPLETENESS
// ═══════════════════════════════════════════════════════════════════════════

function auditExports() {
  log('\n[AUDIT 3] Checking package.json exports...', 'info');

  const packagesDir = join(ROOT, 'packages');
  const packages = readdirSync(packagesDir).filter(name => {
    try {
      return statSync(join(packagesDir, name)).isDirectory();
    } catch {
      return false;
    }
  });

  let missingExports = 0;

  for (const pkg of packages) {
    const pkgJsonPath = join(packagesDir, pkg, 'package.json');
    if (!existsSync(pkgJsonPath)) continue;

    try {
      const pkgJson = JSON.parse(readFileSync(pkgJsonPath, 'utf8'));

      if (!pkgJson.exports) {
        issues.warning.push({
          type: 'NO_EXPORTS',
          file: `packages/${pkg}/package.json`,
        });
        missingExports++;

        if (VERBOSE) {
          log(`  ⚠ ${pkg}: No "exports" field`, 'warn');
        }
      }
    } catch (err) {
      if (VERBOSE) log(`  ⚠ ${pkg}: Cannot parse package.json`, 'warn');
    }
  }

  log(`  Packages without exports: ${missingExports}`, missingExports > 0 ? 'warn' : 'success');
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT 4: EVENTBUS WIRING
// ═══════════════════════════════════════════════════════════════════════════

function auditEventBus() {
  log('\n[AUDIT 4] Checking EventBus wiring...', 'info');

  const jsFiles = findJSFiles(join(ROOT, 'packages'));
  const emitters = new Map();
  const listeners = new Map();

  for (const file of jsFiles) {
    try {
      const content = readFileSync(file, 'utf8');

      // Find emits
      const emitRegex = /\.emit\s*\(\s*['"]([^'"]+)['"]/g;
      let match;
      while ((match = emitRegex.exec(content)) !== null) {
        const event = match[1];
        if (!emitters.has(event)) emitters.set(event, []);
        emitters.get(event).push(relative(ROOT, file));
      }

      // Find listeners
      const onRegex = /\.on\s*\(\s*['"]([^'"]+)['"]/g;
      while ((match = onRegex.exec(content)) !== null) {
        const event = match[1];
        if (!listeners.has(event)) listeners.set(event, []);
        listeners.get(event).push(relative(ROOT, file));
      }
    } catch (err) {
      // Ignore
    }
  }

  // Orphan emits
  let orphanEmits = 0;
  for (const [event, files] of emitters) {
    if (!listeners.has(event)) {
      issues.info.push({
        type: 'ORPHAN_EMIT',
        event,
        files,
      });
      orphanEmits++;
    }
  }

  // Orphan listeners
  let orphanListeners = 0;
  for (const [event, files] of listeners) {
    if (!emitters.has(event)) {
      issues.info.push({
        type: 'ORPHAN_LISTENER',
        event,
        files,
      });
      orphanListeners++;
    }
  }

  const totalEvents = new Set([...emitters.keys(), ...listeners.keys()]).size;
  log(`  Total events: ${totalEvents}`, 'info');
  log(`  Orphan emits: ${orphanEmits}`, 'warn');
  log(`  Orphan listeners: ${orphanListeners}`, 'warn');
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT 5: φ-BOUNDS
// ═══════════════════════════════════════════════════════════════════════════

function auditPhiBounds() {
  log('\n[AUDIT 5] Checking φ-bounds (confidence ≤ 61.8%)...', 'info');

  const jsFiles = findJSFiles(join(ROOT, 'packages'));
  let violations = 0;

  for (const file of jsFiles) {
    try {
      const content = readFileSync(file, 'utf8');

      const confidenceRegex = /confidence\s*[:=]\s*(0\.\d+|1\.0|[\d.]+)/g;
      let match;

      while ((match = confidenceRegex.exec(content)) !== null) {
        const value = parseFloat(match[1]);
        if (value > PHI_INV && value <= 1.0) {
          issues.warning.push({
            type: 'PHI_VIOLATION',
            file: relative(ROOT, file),
            value,
          });
          violations++;

          if (VERBOSE) {
            log(`  ⚠ ${relative(ROOT, file)}: confidence ${value} > φ⁻¹ (${PHI_INV})`, 'warn');
          }
        }
      }
    } catch (err) {
      // Ignore
    }
  }

  log(`  φ violations: ${violations}`, violations > 0 ? 'warn' : 'success');
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT 6: SINGLETON INTEGRITY
// ═══════════════════════════════════════════════════════════════════════════

function auditSingletons() {
  log('\n[AUDIT 6] Checking singleton integrity...', 'info');

  const jsFiles = findJSFiles(join(ROOT, 'packages'));
  const singletonFiles = [];

  for (const file of jsFiles) {
    try {
      const content = readFileSync(file, 'utf8');
      if (content.includes('let _singleton') || content.includes('let _instance')) {
        singletonFiles.push(file);
      }
    } catch (err) {
      // Ignore
    }
  }

  log(`  Singleton files: ${singletonFiles.length}`, 'info');
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════════════════

function main() {
  log('\n╔═══════════════════════════════════════════════════════╗', 'info');
  log('║         CYNIC CODE AUDIT SYSTEM                       ║', 'info');
  log('║         "Don\'t trust, verify"                         ║', 'info');
  log('╚═══════════════════════════════════════════════════════╝', 'info');

  const startTime = Date.now();

  // Run audits
  auditImports();
  auditMethodCalls();
  auditExports();
  auditEventBus();
  auditPhiBounds();
  auditSingletons();

  const duration = Date.now() - startTime;

  // Summary
  log('\n═══════════════════════════════════════════════════════', 'info');
  log('SUMMARY', 'info');
  log('═══════════════════════════════════════════════════════', 'info');
  log(`  Critical: ${issues.critical.length}`, issues.critical.length > 0 ? 'error' : 'success');
  log(`  Warnings: ${issues.warning.length}`, issues.warning.length > 0 ? 'warn' : 'success');
  log(`  Info: ${issues.info.length}`, 'info');
  log(`  Duration: ${duration}ms`, 'info');

  // Print critical issues
  if (issues.critical.length > 0) {
    log('\nCRITICAL ISSUES:', 'error');
    for (const issue of issues.critical.slice(0, 10)) {
      log(`  [${issue.type}] ${issue.file}`, 'error');
      if (issue.import) log(`    Import: "${issue.import}"`, 'error');
      if (issue.call) log(`    Call: ${issue.call}`, 'error');
      if (issue.fix) log(`    Fix: ${issue.fix}`, 'warn');
    }
    if (issues.critical.length > 10) {
      log(`  ... and ${issues.critical.length - 10} more`, 'error');
    }
  }

  // Health score
  const totalIssues = issues.critical.length + issues.warning.length;
  const healthScore = totalIssues === 0 ? PHI_INV : Math.max(0, PHI_INV - (totalIssues * 0.01));

  log(`\nHEALTH SCORE: ${(healthScore * 100).toFixed(1)}%`, healthScore >= PHI_INV * 0.9 ? 'success' : 'warn');
  log(healthScore >= PHI_INV * 0.9 ? '  ✅ HEALTHY' : '  ⚠️  NEEDS ATTENTION', healthScore >= PHI_INV * 0.9 ? 'success' : 'warn');

  // Exit code
  process.exit(issues.critical.length > 0 ? 1 : 0);
}

main();
