/**
 * cynic judge - "CYNIC it"
 *
 * Judge anything from the command line. Outputs a shareable Judgment Card.
 *
 * Usage:
 *   cynic judge <file>              Judge a file
 *   cynic judge <text>              Judge inline text
 *   echo "code" | cynic judge -     Judge from stdin
 *   cynic judge --url <url>         Judge a URL/address
 *   cynic judge --save              Save card to .cynic/cards/
 *
 * Output formats:
 *   (default)    ASCII card (terminal)
 *   --markdown   Markdown (GitHub/Discord)
 *   --json       JSON (programmatic)
 *   --compact    One-liner
 *
 * "CYNIC it" - κυνικός
 *
 * @module @cynic/node/cli/commands/judge
 */

'use strict';

import { readFile, writeFile, mkdir } from 'fs/promises';
import { existsSync, readFileSync } from 'fs';
import { join, extname, basename, resolve } from 'path';
import chalk from 'chalk';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { CYNICJudge } from '../../judge/judge.js';
import { generateCard, toASCII, toMarkdown, toCompact } from '../../judge/judgment-card.js';

// ═══════════════════════════════════════════════════════════════════════════
// INPUT DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Detect input type and build item object
 * @param {string} input - Raw input (file path, text, URL)
 * @param {Object} options - CLI options
 * @returns {Object} { item, title, source }
 */
async function resolveInput(input, options) {
  // Explicit URL flag
  if (options.url) {
    return {
      item: {
        type: 'url',
        content: options.url,
        name: options.url,
      },
      title: options.url,
      source: 'url',
    };
  }

  // Stdin (- or piped)
  if (input === '-' || (!input && !process.stdin.isTTY)) {
    const stdin = await readStdin();
    if (!stdin.trim()) {
      throw new Error('No input received from stdin');
    }
    return {
      item: buildItemFromText(stdin, options.type || 'stdin'),
      title: options.title || 'stdin',
      source: 'stdin',
    };
  }

  // No input at all
  if (!input) {
    throw new Error('No input provided. Usage: cynic judge <file|text> or pipe stdin');
  }

  // File path
  if (existsSync(input)) {
    const content = await readFile(input, 'utf-8');
    const ext = extname(input).slice(1);
    const name = basename(input);
    const type = detectFileType(ext);

    return {
      item: {
        type,
        content,
        name,
        path: resolve(input),
        extension: ext,
        lines: content.split('\n').length,
        size: content.length,
      },
      title: options.title || name,
      source: 'file',
    };
  }

  // Solana address (base58, 32-44 chars)
  if (/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(input)) {
    return {
      item: {
        type: 'solana_address',
        content: input,
        name: input.slice(0, 8) + '...',
      },
      title: options.title || `${input.slice(0, 8)}...${input.slice(-4)}`,
      source: 'address',
    };
  }

  // URL pattern
  if (/^https?:\/\//i.test(input)) {
    return {
      item: {
        type: 'url',
        content: input,
        name: input,
      },
      title: options.title || input,
      source: 'url',
    };
  }

  // Raw text
  return {
    item: buildItemFromText(input, options.type || 'text'),
    title: options.title || input.slice(0, 40) + (input.length > 40 ? '...' : ''),
    source: 'text',
  };
}

/**
 * Build item from text content
 */
function buildItemFromText(text, type) {
  return {
    type: type || detectTextType(text),
    content: text,
    name: text.slice(0, 60),
    lines: text.split('\n').length,
    size: text.length,
  };
}

/**
 * Detect text type from content
 */
function detectTextType(text) {
  if (!text) return 'unknown';

  const codeIndicators = [
    /^(import|export|const|let|var|function|class|def|async)\s/m,
    /[{(]\s*\n/,
    /=>\s*[{(]/,
    /;\s*$/m,
  ];
  if (codeIndicators.filter(p => p.test(text)).length >= 2) return 'code';

  const decisionIndicators = [
    /\b(decide|decision|chose|choose|option|alternative)\b/i,
    /\b(should|must|recommend)\b/i,
  ];
  if (decisionIndicators.filter(p => p.test(text)).length >= 2) return 'decision';

  return 'text';
}

/**
 * Map file extension to item type
 */
function detectFileType(ext) {
  const codeExts = ['js', 'ts', 'jsx', 'tsx', 'py', 'rs', 'go', 'rb', 'java', 'c', 'cpp', 'h', 'sol'];
  const configExts = ['json', 'yaml', 'yml', 'toml', 'ini', 'env', 'xml'];
  const docExts = ['md', 'txt', 'rst', 'adoc'];

  if (codeExts.includes(ext)) return 'code';
  if (configExts.includes(ext)) return 'config';
  if (docExts.includes(ext)) return 'document';
  return 'file';
}

/**
 * Read stdin with timeout
 */
function readStdin() {
  return new Promise((resolve) => {
    let data = '';
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', (chunk) => { data += chunk; });
    process.stdin.on('end', () => resolve(data));
    process.stdin.on('error', () => resolve(''));
    process.stdin.resume();
    // Timeout after 3s
    setTimeout(() => resolve(data), 3000);
  });
}

// ═══════════════════════════════════════════════════════════════════════════
// CARD SAVE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Save judgment card to .cynic/cards/
 * @param {Object} card - Generated card
 * @param {string} format - Which format to save
 * @returns {string} File path
 */
async function saveCard(card, format) {
  const dir = join(process.cwd(), '.cynic', 'cards');
  await mkdir(dir, { recursive: true });

  const now = new Date();
  const pad = (n) => String(n).padStart(2, '0');
  const ts = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;

  let content, ext;
  if (format === 'json') {
    content = JSON.stringify(card.json, null, 2);
    ext = 'json';
  } else if (format === 'markdown') {
    content = card.markdown;
    ext = 'md';
  } else {
    content = card.markdown; // Default save format is markdown (more useful than ASCII)
    ext = 'md';
  }

  const filename = `judgment-${ts}.${ext}`;
  const filepath = join(dir, filename);
  await writeFile(filepath, content, 'utf-8');
  return filepath;
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN COMMAND
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Judge command handler
 * @param {string} input - Input to judge
 * @param {Object} options - CLI options
 */
export async function judgeCommand(input, options) {
  try {
    // Resolve input
    const { item, title, source } = await resolveInput(input, options);

    // Create judge (standalone, no persistence needed)
    const judge = new CYNICJudge({
      applySkepticism: false, // No skeptic service in CLI mode
      includeUnnameable: true,
    });

    // Judge
    const judgment = judge.judge(item, {
      type: source,
      source: 'cli',
    });

    // Normalize weaknesses: analyzeWeaknesses returns an object, card expects array
    let weaknesses = [];
    const w = judgment.weaknesses;
    if (w && typeof w === 'object' && !Array.isArray(w)) {
      // Convert weakness object to array format for the card
      if (w.weakestAxiom) {
        weaknesses.push({
          axiom: w.weakestAxiom,
          score: w.weakestScore,
          reason: w.recommendation,
        });
      }
    } else if (Array.isArray(w)) {
      weaknesses = w;
    }

    // Build card-compatible result
    const result = {
      requestId: judgment.id,
      score: judgment.qScore,
      verdict: judgment.qVerdict?.verdict || judgment.verdict,
      confidence: judgment.confidence,
      axiomScores: judgment.axiomScores,
      weaknesses,
      itemType: item.type,
      item,
      timestamp: Date.now(),
    };

    // Generate card
    const card = generateCard(result, { title });

    // Output based on format
    if (options.json) {
      console.log(JSON.stringify(card.json, null, 2));
    } else if (options.markdown) {
      console.log(card.markdown);
    } else if (options.compact) {
      console.log(card.compact);
    } else {
      // Default: colored ASCII
      const colored = colorizeASCII(card.ascii, result.verdict);
      console.log(colored);
    }

    // Save if requested
    if (options.save) {
      const format = options.json ? 'json' : options.markdown ? 'markdown' : 'markdown';
      const filepath = await saveCard(card, format);
      console.error(chalk.gray(`\nSaved to ${filepath}`));
    }
  } catch (error) {
    console.error(chalk.red(`*GROWL* ${error.message}`));
    process.exit(1);
  }
}

/**
 * Add color to ASCII card based on verdict
 */
function colorizeASCII(ascii, verdict) {
  const colors = {
    HOWL: chalk.green,
    WAG: chalk.cyan,
    GROWL: chalk.yellow,
    BARK: chalk.red,
  };
  const color = colors[verdict] || chalk.white;

  return ascii
    .split('\n')
    .map(line => {
      // Color the box borders
      if (/^[┌┤├└│┐┘┬┴┼─]/.test(line) || /[┌┤├└│┐┘┬┴┼─]$/.test(line.trim())) {
        return color(line);
      }
      // Color verdict line
      if (line.includes('Verdict:')) {
        return color(line);
      }
      // Color score bar
      if (line.includes('█') && line.includes('░')) {
        return line.replace(/█+/, (match) => chalk.green(match))
                   .replace(/░+/, (match) => chalk.gray(match));
      }
      return line;
    })
    .join('\n');
}
