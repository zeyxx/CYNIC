#!/usr/bin/env node
/**
 * RALPH: Harmonious Audit
 *
 * Deep audit of orphan code to inform burn decisions.
 * Creates a human-readable report with:
 * - Purpose of each orphan file
 * - Future potential assessment
 * - Recommended action (BURN / PRESERVE / INTEGRATE)
 *
 * @module scripts/ralph-loops/harmonious-audit
 */

import fs from 'fs';
import path from 'path';

const C = {
  reset: '\x1b[0m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
};

// Load omniscient report
const report = JSON.parse(fs.readFileSync('./ralph-omniscient-report.json', 'utf8'));
const orphans = report.issues.find(i => i.type === 'ORPHAN_FILES')?.items || [];

/**
 * Extract file purpose from content
 */
function extractPurpose(filePath) {
  try {
    const fullPath = path.join(process.cwd(), filePath);
    const content = fs.readFileSync(fullPath, 'utf8');
    const lines = content.split('\n');

    // Find module description
    let purpose = '';
    let inComment = false;

    for (const line of lines.slice(0, 30)) {
      if (line.includes('/**')) inComment = true;
      if (inComment) {
        const clean = line.replace(/^\s*\*?\s*/, '').replace(/\*\/.*/, '').trim();
        if (clean && !clean.startsWith('@') && !clean.startsWith('*')) {
          purpose += clean + ' ';
        }
      }
      if (line.includes('*/')) {
        inComment = false;
        if (purpose) break;
      }
    }

    // Fallback: first meaningful line
    if (!purpose) {
      for (const line of lines.slice(0, 20)) {
        if (line.startsWith('//') && !line.includes('eslint') && !line.includes('prettier')) {
          purpose = line.replace(/^\/\/\s*/, '');
          break;
        }
      }
    }

    // Extract exports
    const exports = [];
    const exportRegex = /export\s+(?:const|let|var|function|class|async\s+function)\s+(\w+)/g;
    let match;
    while ((match = exportRegex.exec(content)) !== null) {
      exports.push(match[1]);
    }
    if (/export\s+default/.test(content)) exports.push('default');

    // Count lines
    const lineCount = lines.length;

    // Detect patterns
    const patterns = {
      hasTests: /describe\(|it\(|test\(/.test(content),
      usesAsync: /async\s|await\s/.test(content),
      hasClasses: /class\s+\w+/.test(content),
      usesSolana: /@solana|anchor|PublicKey/.test(content),
      usesPostgres: /pg\.|Pool|query\(/.test(content),
      usesWebSocket: /WebSocket|ws\.|socket/.test(content),
      hasCrypto: /crypto|hash|sign|verify/.test(content),
      isService: /Service|Manager|Handler/.test(content),
    };

    return {
      purpose: purpose.trim().substring(0, 200) || 'No description found',
      exports,
      lines: lineCount,
      patterns,
    };
  } catch (e) {
    return { purpose: 'Error reading file', exports: [], lines: 0, patterns: {} };
  }
}

/**
 * Assess future potential
 */
function assessPotential(info, filePath) {
  let score = 0;
  const reasons = [];

  // Positive signals
  if (info.patterns.usesSolana) {
    score += 3;
    reasons.push('Solana integration (future roadmap)');
  }
  if (info.patterns.hasCrypto) {
    score += 2;
    reasons.push('Cryptographic functionality');
  }
  if (info.exports.length > 3) {
    score += 1;
    reasons.push('Rich API surface');
  }
  if (info.lines > 200 && info.lines < 500) {
    score += 1;
    reasons.push('Well-sized module');
  }
  if (info.patterns.isService) {
    score += 1;
    reasons.push('Service architecture');
  }

  // Negative signals
  if (info.lines > 800) {
    score -= 1;
    reasons.push('Too large, needs refactor');
  }
  if (info.lines < 30) {
    score -= 1;
    reasons.push('Minimal content');
  }
  if (filePath.includes('test') || filePath.includes('mock')) {
    score -= 2;
    reasons.push('Test/mock file');
  }
  if (!info.purpose || info.purpose === 'No description found') {
    score -= 1;
    reasons.push('Undocumented');
  }

  return { score, reasons };
}

/**
 * Determine recommendation
 */
function recommend(info, potential, filePath) {
  // Always preserve Solana/anchor code (future roadmap)
  if (info.patterns.usesSolana || filePath.includes('anchor')) {
    return { action: 'PRESERVE', reason: 'Solana roadmap' };
  }

  // Preserve ZK stuff
  if (filePath.includes('zk') || info.patterns.hasCrypto && filePath.includes('verif')) {
    return { action: 'PRESERVE', reason: 'ZK/crypto roadmap' };
  }

  // Preserve emergence/consciousness
  if (filePath.includes('emergence') || filePath.includes('consciousness')) {
    return { action: 'PRESERVE', reason: 'Consciousness architecture' };
  }

  // High potential = integrate
  if (potential.score >= 3) {
    return { action: 'INTEGRATE', reason: 'High potential, needs wiring' };
  }

  // Low potential = burn
  if (potential.score <= -1) {
    return { action: 'BURN', reason: 'Low value, safe to remove' };
  }

  // Default: review
  return { action: 'REVIEW', reason: 'Needs manual assessment' };
}

// === MAIN ===
console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log(`${C.magenta}🔍 HARMONIOUS AUDIT - Orphan Code Assessment${C.reset}`);
console.log('═══════════════════════════════════════════════════════════════');
console.log('');

// Group orphans by package
const groups = {};
for (const filePath of orphans) {
  const parts = filePath.split('/');
  const group = parts[0] === 'packages' ? `packages/${parts[1]}` : parts[0];
  if (!groups[group]) groups[group] = [];
  groups[group].push(filePath);
}

const auditResults = {
  timestamp: new Date().toISOString(),
  summary: {
    total: orphans.length,
    burn: 0,
    preserve: 0,
    integrate: 0,
    review: 0,
    totalLines: 0,
    burnLines: 0,
    preserveLines: 0,
  },
  packages: {},
};

// Process each group
for (const [group, files] of Object.entries(groups).sort((a, b) => b[1].length - a[1].length)) {
  console.log(`${C.cyan}── ${group} (${files.length} orphans) ──────────────────────────────────────${C.reset}`);
  console.log('');

  auditResults.packages[group] = {
    files: [],
    summary: { burn: 0, preserve: 0, integrate: 0, review: 0, lines: 0 },
  };

  for (const filePath of files) {
    const info = extractPurpose(filePath);
    const potential = assessPotential(info, filePath);
    const rec = recommend(info, potential, filePath);

    // Update counters
    auditResults.summary[rec.action.toLowerCase()]++;
    auditResults.summary.totalLines += info.lines;
    auditResults.packages[group].summary[rec.action.toLowerCase()]++;
    auditResults.packages[group].summary.lines += info.lines;

    if (rec.action === 'BURN') {
      auditResults.summary.burnLines += info.lines;
    } else if (rec.action === 'PRESERVE' || rec.action === 'INTEGRATE') {
      auditResults.summary.preserveLines += info.lines;
    }

    // Store result
    auditResults.packages[group].files.push({
      path: filePath,
      lines: info.lines,
      purpose: info.purpose,
      exports: info.exports.slice(0, 5),
      potential: potential.score,
      potentialReasons: potential.reasons,
      recommendation: rec.action,
      recommendationReason: rec.reason,
    });

    // Display
    const actionColor = rec.action === 'BURN' ? C.red :
                        rec.action === 'PRESERVE' ? C.green :
                        rec.action === 'INTEGRATE' ? C.blue : C.yellow;

    const shortPath = filePath.replace(group + '/', '');
    console.log(`   ${actionColor}[${rec.action}]${C.reset} ${shortPath}`);
    console.log(`   ${C.dim}${info.lines} lines | ${info.purpose.substring(0, 60)}...${C.reset}`);
    console.log(`   ${C.dim}Exports: ${info.exports.slice(0, 4).join(', ') || 'none'}${C.reset}`);
    console.log(`   ${C.dim}→ ${rec.reason}${C.reset}`);
    console.log('');
  }
}

// Save detailed audit
fs.writeFileSync('./ralph-harmonious-audit.json', JSON.stringify(auditResults, null, 2));

// Summary
console.log('═══════════════════════════════════════════════════════════════');
console.log(`${C.magenta}📊 AUDIT SUMMARY${C.reset}`);
console.log('═══════════════════════════════════════════════════════════════');
console.log('');
console.log(`   Total orphans:  ${auditResults.summary.total} files (${auditResults.summary.totalLines.toLocaleString()} lines)`);
console.log('');
console.log(`   ${C.red}BURN${C.reset}:       ${auditResults.summary.burn} files (${auditResults.summary.burnLines.toLocaleString()} lines)`);
console.log(`   ${C.green}PRESERVE${C.reset}:  ${auditResults.summary.preserve} files`);
console.log(`   ${C.blue}INTEGRATE${C.reset}: ${auditResults.summary.integrate} files`);
console.log(`   ${C.yellow}REVIEW${C.reset}:    ${auditResults.summary.review} files`);
console.log('');

// Package-level summary
console.log(`${C.cyan}── BY PACKAGE ─────────────────────────────────────────────────${C.reset}`);
for (const [pkg, data] of Object.entries(auditResults.packages)) {
  const s = data.summary;
  console.log(`   ${pkg.padEnd(25)} ${C.red}B:${s.burn}${C.reset} ${C.green}P:${s.preserve}${C.reset} ${C.blue}I:${s.integrate}${C.reset} ${C.yellow}R:${s.review}${C.reset} (${s.lines} lines)`);
}

console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log(`   ${C.dim}Detailed audit saved: ralph-harmonious-audit.json${C.reset}`);
console.log('═══════════════════════════════════════════════════════════════');
console.log('');
