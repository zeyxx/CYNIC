/**
 * CYNIC Security Watchdog
 *
 * "Le chien veille" - The dog watches
 *
 * Core security scanner for detecting sensitive data in files.
 * Uses φ-weighted scoring for mathematically harmonious security.
 *
 * @module cynic/lib/watchdog
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const {
  PHI,
  SEVERITY_WEIGHTS,
  SECRET_PATTERNS,
  SENSITIVE_FILES,
  SAFE_PATTERNS,
  getSeverityWeight,
  isSafePattern
} = require('./security-patterns.cjs');

// =============================================================================
// CONSTANTS
// =============================================================================

const PHI_INV = 1 / PHI;  // 0.618 - max confidence
const MAX_FILE_SIZE = 1024 * 1024;  // 1MB max scan size
const BINARY_EXTENSIONS = ['.png', '.jpg', '.jpeg', '.gif', '.ico', '.woff', '.woff2', '.ttf', '.eot', '.mp3', '.mp4', '.zip', '.tar', '.gz'];

// =============================================================================
// SCANNING FUNCTIONS
// =============================================================================

/**
 * Scan content for sensitive patterns
 * @param {string} content - File content to scan
 * @param {string} filePath - File path for context
 * @returns {Array} Array of findings
 */
function scanContent(content, filePath) {
  const findings = [];

  // Skip if content matches safe patterns globally
  if (isSafePattern(content)) {
    return findings;
  }

  // Check each secret pattern
  for (const pattern of SECRET_PATTERNS) {
    const matches = content.match(pattern.pattern);
    if (matches) {
      // Check if match is safe (false positive)
      const matchStr = matches[0];
      if (!isSafePattern(matchStr)) {
        // Find line number
        const lines = content.substring(0, content.indexOf(matchStr)).split('\n');
        const lineNumber = lines.length;

        findings.push({
          id: pattern.id,
          severity: pattern.severity,
          severityWeight: getSeverityWeight(pattern.severity),
          description: pattern.description,
          recommendation: pattern.recommendation,
          file: filePath,
          line: lineNumber,
          match: maskSecret(matchStr),
          confidence: calculateConfidence(pattern, matchStr)
        });
      }
    }
  }

  return findings;
}

/**
 * Check if file path is sensitive
 * @param {string} filePath - File path to check
 * @returns {Object|null} Sensitivity info or null
 */
function checkSensitiveFile(filePath) {
  const fileName = path.basename(filePath);

  for (const sensitive of SENSITIVE_FILES) {
    if (sensitive.pattern.test(fileName) || sensitive.pattern.test(filePath)) {
      // Check if it's a safe variant (.example, etc.)
      if (!isSafePattern(filePath)) {
        return {
          severity: sensitive.severity,
          description: sensitive.description,
          file: filePath
        };
      }
    }
  }

  return null;
}

/**
 * Mask a secret for safe display
 * @param {string} secret - Secret to mask
 * @returns {string} Masked secret
 */
function maskSecret(secret) {
  if (secret.length <= 8) {
    return '*'.repeat(secret.length);
  }
  const visible = Math.min(4, Math.floor(secret.length * PHI_INV * 0.1));
  return secret.substring(0, visible) + '*'.repeat(secret.length - visible * 2) + secret.substring(secret.length - visible);
}

/**
 * Calculate confidence score (max φ⁻¹ = 61.8%)
 * @param {Object} pattern - Pattern that matched
 * @param {string} match - Matched string
 * @returns {number} Confidence 0-0.618
 */
function calculateConfidence(pattern, match) {
  let confidence = 0.5;  // Base confidence

  // Longer matches = higher confidence
  if (match.length > 30) confidence += 0.1;
  if (match.length > 50) confidence += 0.05;

  // Critical patterns = higher confidence
  if (pattern.severity === 'critical') confidence += 0.1;
  else if (pattern.severity === 'high') confidence += 0.05;

  // Cap at φ⁻¹
  return Math.min(PHI_INV, confidence);
}

// =============================================================================
// FILE SCANNING
// =============================================================================

/**
 * Scan a single file
 * @param {string} filePath - Absolute path to file
 * @returns {Object} Scan result
 */
function scanFile(filePath) {
  const result = {
    file: filePath,
    findings: [],
    sensitiveFile: null,
    skipped: false,
    skipReason: null
  };

  try {
    // Check extension
    const ext = path.extname(filePath).toLowerCase();
    if (BINARY_EXTENSIONS.includes(ext)) {
      result.skipped = true;
      result.skipReason = 'binary';
      return result;
    }

    // Check file size
    const stats = fs.statSync(filePath);
    if (stats.size > MAX_FILE_SIZE) {
      result.skipped = true;
      result.skipReason = 'too_large';
      return result;
    }

    // Check if sensitive file type
    result.sensitiveFile = checkSensitiveFile(filePath);

    // Read and scan content
    const content = fs.readFileSync(filePath, 'utf-8');
    result.findings = scanContent(content, filePath);

  } catch (error) {
    result.skipped = true;
    result.skipReason = error.code || 'error';
  }

  return result;
}

/**
 * Scan git staged files
 * @returns {Array} Array of scan results
 */
function scanStagedFiles() {
  const results = [];

  try {
    // Get list of staged files
    const staged = execFileSync('git', ['diff', '--cached', '--name-only', '--diff-filter=ACMR'], {
      encoding: 'utf-8'
    }).trim();

    if (!staged) {
      return results;
    }

    const files = staged.split('\n').filter(f => f.trim());

    for (const file of files) {
      const absPath = path.resolve(process.cwd(), file);
      if (fs.existsSync(absPath)) {
        results.push(scanFile(absPath));
      }
    }
  } catch (error) {
    // Not a git repo or git error
  }

  return results;
}

/**
 * Scan git diff (uncommitted changes)
 * @returns {Array} Array of scan results
 */
function scanUncommittedChanges() {
  const results = [];

  try {
    // Get modified files
    const modified = execFileSync('git', ['diff', '--name-only'], {
      encoding: 'utf-8'
    }).trim();

    const untracked = execFileSync('git', ['ls-files', '--others', '--exclude-standard'], {
      encoding: 'utf-8'
    }).trim();

    const allFiles = [...modified.split('\n'), ...untracked.split('\n')]
      .filter(f => f.trim())
      .filter((f, i, arr) => arr.indexOf(f) === i);  // unique

    for (const file of allFiles) {
      const absPath = path.resolve(process.cwd(), file);
      if (fs.existsSync(absPath)) {
        results.push(scanFile(absPath));
      }
    }
  } catch (error) {
    // Not a git repo or git error
  }

  return results;
}

/**
 * Scan entire workspace
 * @param {string} rootDir - Root directory to scan
 * @param {Object} options - Scan options
 * @returns {Array} Array of scan results
 */
function scanWorkspace(rootDir, options = {}) {
  const results = [];
  const {
    maxFiles = 1000,
    excludeDirs = ['node_modules', '.git', 'dist', 'build', 'coverage', '__pycache__', '.venv', 'venv']
  } = options;

  let fileCount = 0;

  function walkDir(dir) {
    if (fileCount >= maxFiles) return;

    try {
      const entries = fs.readdirSync(dir, { withFileTypes: true });

      for (const entry of entries) {
        if (fileCount >= maxFiles) break;

        const fullPath = path.join(dir, entry.name);

        if (entry.isDirectory()) {
          if (!excludeDirs.includes(entry.name) && !entry.name.startsWith('.')) {
            walkDir(fullPath);
          }
        } else if (entry.isFile()) {
          fileCount++;
          results.push(scanFile(fullPath));
        }
      }
    } catch (error) {
      // Permission denied or other error
    }
  }

  walkDir(rootDir);
  return results;
}

// =============================================================================
// VERDICT CALCULATION
// =============================================================================

/**
 * Calculate φ-weighted security verdict
 * @param {Array} results - Scan results
 * @returns {Object} Verdict with score and recommendation
 */
function calculateVerdict(results) {
  const allFindings = results
    .flatMap(r => r.findings.map(f => ({
      ...f,
      severityWeight: f.severityWeight || getSeverityWeight(f.severity)
    })))
    .concat(results.filter(r => r.sensitiveFile).map(r => ({
      ...r.sensitiveFile,
      severityWeight: getSeverityWeight(r.sensitiveFile.severity)
    })));

  if (allFindings.length === 0) {
    return {
      verdict: 'WAG',
      score: 100,
      message: '*tail wag* Workspace is clean. No secrets detected.',
      findings: [],
      shouldBlock: false
    };
  }

  // Calculate weighted score
  const totalWeight = allFindings.reduce((sum, f) => sum + (f.severityWeight || 1), 0);
  const maxPossibleWeight = allFindings.length * SEVERITY_WEIGHTS.critical;
  const riskRatio = totalWeight / maxPossibleWeight;

  // φ-scaled score (inverse risk)
  const score = Math.round(100 * (1 - riskRatio * PHI_INV));

  // Determine verdict
  const hasCritical = allFindings.some(f => f.severity === 'critical');
  const hasHigh = allFindings.some(f => f.severity === 'high');

  let verdict, message, shouldBlock;

  if (hasCritical) {
    verdict = 'GROWL';
    message = `*GROWL* CRITICAL: ${allFindings.filter(f => f.severity === 'critical').length} critical secret(s) detected!`;
    shouldBlock = true;
  } else if (hasHigh) {
    verdict = 'BARK';
    message = `*bark* WARNING: ${allFindings.filter(f => f.severity === 'high').length} high-severity finding(s).`;
    shouldBlock = false;
  } else {
    verdict = 'BARK';
    message = `*sniff* ${allFindings.length} potential issue(s) found.`;
    shouldBlock = false;
  }

  return {
    verdict,
    score,
    message,
    findings: allFindings,
    shouldBlock,
    summary: {
      critical: allFindings.filter(f => f.severity === 'critical').length,
      high: allFindings.filter(f => f.severity === 'high').length,
      medium: allFindings.filter(f => f.severity === 'medium').length,
      low: allFindings.filter(f => f.severity === 'low').length
    }
  };
}

// =============================================================================
// REPORT FORMATTING
// =============================================================================

/**
 * Format security report
 * @param {Object} verdict - Verdict from calculateVerdict
 * @returns {string} Formatted report
 */
function formatReport(verdict) {
  const lines = [];

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('🔒 CYNIC SECURITY WATCHDOG - Audit Report');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('');

  // Score bar
  const scoreBar = '█'.repeat(Math.floor(verdict.score / 5)) + '░'.repeat(20 - Math.floor(verdict.score / 5));
  lines.push(`Security Score: [${scoreBar}] ${verdict.score}/100`);
  lines.push(`Verdict: ${verdict.verdict}`);
  lines.push('');

  if (verdict.findings.length === 0) {
    lines.push(verdict.message);
  } else {
    lines.push(verdict.message);
    lines.push('');
    lines.push('── FINDINGS ───────────────────────────────────────────────');

    // Group by severity
    const bySeverity = {
      critical: verdict.findings.filter(f => f.severity === 'critical'),
      high: verdict.findings.filter(f => f.severity === 'high'),
      medium: verdict.findings.filter(f => f.severity === 'medium'),
      low: verdict.findings.filter(f => f.severity === 'low')
    };

    for (const [severity, findings] of Object.entries(bySeverity)) {
      if (findings.length > 0) {
        const icon = severity === 'critical' ? '🔴' :
                     severity === 'high' ? '🟠' :
                     severity === 'medium' ? '🟡' : '🔵';
        lines.push(`\n${icon} ${severity.toUpperCase()} (${findings.length}):`);

        for (const finding of findings) {
          lines.push(`   • ${finding.description}`);
          lines.push(`     File: ${finding.file}${finding.line ? `:${finding.line}` : ''}`);
          if (finding.match) {
            lines.push(`     Match: ${finding.match}`);
          }
          if (finding.recommendation) {
            lines.push(`     → ${finding.recommendation}`);
          }
        }
      }
    }
  }

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push(`φ⁻¹ confidence: ${Math.round(PHI_INV * 100)}% max certainty`);
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Scanning functions
  scanContent,
  scanFile,
  scanStagedFiles,
  scanUncommittedChanges,
  scanWorkspace,

  // Verdict and reporting
  calculateVerdict,
  formatReport,

  // Utilities
  checkSensitiveFile,
  maskSecret,

  // Constants
  PHI,
  PHI_INV
};
