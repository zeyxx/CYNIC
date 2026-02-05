/**
 * CYNIC Judgment Cards
 *
 * Shareable, exportable judgment artifacts.
 * Formats: Markdown, ASCII (TUI), JSON
 *
 * "Le jugement visible" - κυνικός
 *
 * @module @cynic/node/judge/judgment-card
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// VERDICT VISUALS
// ═══════════════════════════════════════════════════════════════════════════

const VERDICT_EMOJI = {
  HOWL: '\u{1F43A}',  // 🐺
  WAG: '\u{1F436}',   // 🐶
  GROWL: '\u{1F415}', // 🐕
  BARK: '\u{1F6A8}',  // 🚨
};

const VERDICT_LABEL = {
  HOWL: 'EXCEPTIONAL',
  WAG: 'PASSES',
  GROWL: 'NEEDS WORK',
  BARK: 'CRITICAL',
};

const AXIOM_EMOJI = {
  PHI: '\u{03C6}',       // φ
  VERIFY: '\u{1F50D}',   // 🔍
  CULTURE: '\u{1F3AD}',  // 🎭
  BURN: '\u{1F525}',     // 🔥
};

// ═══════════════════════════════════════════════════════════════════════════
// PROGRESS BAR HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a progress bar string
 * @param {number} value - Value 0-100
 * @param {number} [width=20] - Bar width in chars
 * @returns {string} Progress bar
 */
function progressBar(value, width = 20) {
  const clamped = Math.max(0, Math.min(100, value));
  const filled = Math.round((clamped / 100) * width);
  const empty = width - filled;
  return '\u2588'.repeat(filled) + '\u2591'.repeat(empty);
}

/**
 * Score to color label (for markdown)
 * @param {number} score - Score 0-100
 * @returns {string} Color indicator
 */
function scoreIndicator(score) {
  if (score >= PHI_INV * 100) return '\u{1F7E2}'; // 🟢
  if (score >= PHI_INV_2 * 100) return '\u{1F7E1}'; // 🟡
  return '\u{1F534}'; // 🔴
}

// ═══════════════════════════════════════════════════════════════════════════
// CARD FORMATTERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Format a judgment result as a Markdown card
 * @param {Object} judgment - Judgment result from brain_cynic_judge
 * @param {Object} [options] - Formatting options
 * @param {string} [options.title] - Custom title (defaults to item type)
 * @param {boolean} [options.compact=false] - Compact mode (less detail)
 * @param {boolean} [options.includeAxioms=true] - Include axiom breakdown
 * @param {boolean} [options.includeWeaknesses=true] - Include weaknesses
 * @param {boolean} [options.includeThermo=false] - Include thermodynamics
 * @returns {string} Markdown formatted card
 */
export function toMarkdown(judgment, options = {}) {
  const {
    title = judgment.itemType || judgment.item?.type || 'Item',
    compact = false,
    includeAxioms = true,
    includeWeaknesses = true,
    includeThermo = false,
  } = options;

  const verdict = judgment.verdict || 'WAG';
  const score = judgment.score ?? judgment.qScore ?? 50;
  const confidence = judgment.confidence ?? PHI_INV;
  const confidencePct = Math.round(confidence * 100 * 10) / 10;
  const emoji = VERDICT_EMOJI[verdict] || VERDICT_EMOJI.WAG;
  const label = VERDICT_LABEL[verdict] || verdict;
  const id = judgment.requestId || judgment.id || `jdg_${Date.now().toString(36)}`;
  const ts = judgment.timestamp ? new Date(judgment.timestamp).toISOString() : new Date().toISOString();

  const lines = [];

  // Header
  lines.push(`## ${emoji} CYNIC Judgment: ${title}`);
  lines.push('');

  // Score block
  lines.push(`**Q-Score:** ${score}/100 | **Verdict:** ${verdict} (${label}) | **\u03C6-confidence:** ${confidencePct}%`);
  lines.push('');

  // Progress bar
  lines.push(`\`${progressBar(score)}\` ${score}%`);
  lines.push('');

  // Axiom breakdown
  if (includeAxioms && judgment.axiomScores) {
    lines.push('### Axiom Breakdown');
    lines.push('');
    lines.push('| Axiom | Score | |');
    lines.push('|-------|------:|---|');

    for (const [axiom, axiomScore] of Object.entries(judgment.axiomScores)) {
      if (axiom === 'META') continue;
      const ae = AXIOM_EMOJI[axiom] || '';
      const indicator = scoreIndicator(axiomScore);
      lines.push(`| ${ae} **${axiom}** | ${axiomScore} | ${indicator} |`);
    }

    // THE_UNNAMEABLE
    if (judgment.axiomScores.META !== undefined) {
      lines.push(`| \u2727 **THE_UNNAMEABLE** | ${judgment.axiomScores.META} | ${scoreIndicator(judgment.axiomScores.META)} |`);
    }

    lines.push('');
  }

  // Weaknesses
  if (includeWeaknesses && judgment.weaknesses?.length > 0 && !compact) {
    lines.push('### Concerns');
    lines.push('');
    for (const w of judgment.weaknesses.slice(0, 5)) {
      const axiom = w.axiom || 'UNKNOWN';
      lines.push(`- **${axiom}**: ${w.reason || w.description || 'Below threshold'} (${w.score ?? '?'})`);
    }
    lines.push('');
  }

  // Thermodynamics
  if (includeThermo && judgment.thermodynamics) {
    const t = judgment.thermodynamics;
    lines.push('### Thermodynamics');
    lines.push('');
    lines.push(`| Metric | Value |`);
    lines.push(`|--------|------:|`);
    lines.push(`| Heat (Q) | ${t.heat?.toFixed(2) ?? '?'} |`);
    lines.push(`| Work (W) | ${t.work?.toFixed(2) ?? '?'} |`);
    lines.push(`| Efficiency (\u03B7) | ${((t.efficiency ?? 0) * 100).toFixed(1)}% |`);
    lines.push(`| Temperature | ${t.temperature?.toFixed(1) ?? '?'}\u00B0 |`);
    lines.push('');
  }

  // Footer
  lines.push('---');
  lines.push(`\`${id}\` | ${ts} | \u03C6 max: ${(PHI_INV * 100).toFixed(1)}%`);
  lines.push('');
  lines.push('*Judged by [CYNIC](\u{03BA}\u{03C5}\u{03BD}\u{03B9}\u{03BA}\u{03CC}\u{03C2}) \u2014 loyal to truth, not to comfort*');

  return lines.join('\n');
}

/**
 * Format a judgment result as ASCII card (for terminal)
 * @param {Object} judgment - Judgment result
 * @param {Object} [options] - Formatting options
 * @param {string} [options.title] - Custom title
 * @returns {string} ASCII formatted card
 */
export function toASCII(judgment, options = {}) {
  const title = options.title || judgment.itemType || judgment.item?.type || 'Item';
  const verdict = judgment.verdict || 'WAG';
  const score = judgment.score ?? judgment.qScore ?? 50;
  const confidence = judgment.confidence ?? PHI_INV;
  const confidencePct = Math.round(confidence * 100 * 10) / 10;
  const label = VERDICT_LABEL[verdict] || verdict;
  const id = judgment.requestId || judgment.id || `jdg_${Date.now().toString(36)}`;

  const W = 55; // card width
  const hr = '\u2500'.repeat(W - 2);
  const dhr = '\u2550'.repeat(W - 2);

  const pad = (str, len) => {
    const s = String(str);
    return s.length >= len ? s.slice(0, len) : s + ' '.repeat(len - s.length);
  };
  const rpad = (str, len) => {
    const s = String(str);
    return s.length >= len ? s.slice(0, len) : ' '.repeat(len - s.length) + s;
  };
  const line = (content) => `\u2502 ${pad(content, W - 4)} \u2502`;

  const lines = [];

  // Top border
  lines.push(`\u250C${hr}\u2510`);

  // Header
  lines.push(line(`CYNIC JUDGMENT`));
  lines.push(`\u251C${hr}\u2524`);

  // Title
  lines.push(line(`"${title}"`));
  lines.push(line(`Q-Score: ${score}/100  Verdict: ${verdict} (${label})`));
  lines.push(line(''));

  // Score bar
  lines.push(line(`[${progressBar(score)}] ${score}%`));
  lines.push(line(''));

  // Axiom scores
  if (judgment.axiomScores) {
    lines.push(`\u251C${hr}\u2524`);
    for (const [axiom, axiomScore] of Object.entries(judgment.axiomScores)) {
      if (axiom === 'META') continue;
      const bar = progressBar(axiomScore, 10);
      lines.push(line(`${pad(axiom, 8)} [${bar}] ${rpad(String(axiomScore), 5)}`));
    }
    if (judgment.axiomScores.META !== undefined) {
      const bar = progressBar(judgment.axiomScores.META, 10);
      lines.push(line(`${pad('UNNAMED', 8)} [${bar}] ${rpad(String(judgment.axiomScores.META), 5)}`));
    }
  }

  // Weaknesses
  if (judgment.weaknesses?.length > 0) {
    lines.push(`\u251C${hr}\u2524`);
    lines.push(line('Concerns:'));
    for (const w of judgment.weaknesses.slice(0, 3)) {
      const reason = (w.reason || w.description || 'Below threshold').slice(0, W - 10);
      lines.push(line(` - ${reason}`));
    }
  }

  // Footer
  lines.push(`\u251C${hr}\u2524`);
  lines.push(line(`\u03C6-confidence: ${confidencePct}%  (max: ${(PHI_INV * 100).toFixed(1)}%)`));
  lines.push(line(`${id}  ${new Date(judgment.timestamp || Date.now()).toISOString().slice(0, 19)}Z`));
  lines.push(`\u2514${hr}\u2518`);

  return lines.join('\n');
}

/**
 * Format a judgment as a compact one-liner
 * @param {Object} judgment - Judgment result
 * @returns {string} Compact summary
 */
export function toCompact(judgment) {
  const verdict = judgment.verdict || 'WAG';
  const score = judgment.score ?? judgment.qScore ?? 50;
  const confidence = Math.round((judgment.confidence ?? PHI_INV) * 100);
  const emoji = VERDICT_EMOJI[verdict] || '';
  const title = judgment.itemType || judgment.item?.type || 'item';
  const id = judgment.requestId || judgment.id || '?';

  return `${emoji} ${verdict} ${score}/100 (\u03C6:${confidence}%) "${title}" [${id}]`;
}

/**
 * Generate all card formats for a judgment
 * @param {Object} judgment - Judgment result from brain_cynic_judge
 * @param {Object} [options] - Options passed to formatters
 * @returns {Object} { markdown, ascii, compact, json }
 */
export function generateCard(judgment, options = {}) {
  return {
    markdown: toMarkdown(judgment, options),
    ascii: toASCII(judgment, options),
    compact: toCompact(judgment),
    json: {
      cynic_judgment_card: true,
      version: '1.0.0',
      id: judgment.requestId || judgment.id,
      score: judgment.score ?? judgment.qScore,
      verdict: judgment.verdict,
      confidence: judgment.confidence,
      axiomScores: judgment.axiomScores,
      weaknesses: judgment.weaknesses?.slice(0, 5),
      timestamp: judgment.timestamp || Date.now(),
      phi: { maxConfidence: PHI_INV, minDoubt: PHI_INV_2 },
    },
  };
}

export default { toMarkdown, toASCII, toCompact, generateCard };
