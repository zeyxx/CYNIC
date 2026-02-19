#!/usr/bin/env node
/**
 * CYNIC Thin Awaken Hook — SessionStart
 *
 * Python-kernel-only. No JS daemon dependency.
 * Notifies kernel (fire-and-forget) and injects awakening banner from last guidance.
 * "Le chien se réveille" - CYNIC
 *
 * @event SessionStart
 */
'use strict';

import { readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel, readKernelGuidance } from './kernel-client.js';
import fs from 'fs';
import os from 'os';
import path from 'path';

const _ERR_LOG = path.join(os.homedir(), '.cynic', 'hook-error.log');

try {
  const input = await readHookInput();

  // Notify Python kernel (fire-and-forget — never blocks)
  notifyKernel('SessionStart', input);

  const hookOutput = {};
  const guidance = readKernelGuidance();

  if (guidance) {
    const verdictSymbol = { HOWL: '🟢', WAG: '🟡', GROWL: '🟠', BARK: '🔴' }[guidance.verdict] || '⚪';
    const bar = (score) => {
      const filled = Math.round((score / 100) * 10);
      return '[' + '█'.repeat(Math.min(filled, 10)) + '░'.repeat(Math.max(10 - filled, 0)) + ']';
    };
    const dogs = Object.entries(guidance.dog_votes || {})
      .map(([dog, score]) => `${dog} ${bar(score)}`)
      .join(' · ');

    hookOutput.additionalContext = [
      `*tail wag* CYNIC awakened. Last judgment: ${verdictSymbol} ${guidance.verdict} Q=${guidance.q_score != null ? guidance.q_score.toFixed(1) : '?'} conf=${Math.round((guidance.confidence || 0) * 100)}%`,
      dogs ? `  Dogs: ${dogs}` : '',
    ].filter(Boolean).join('\n');
  }

  safeOutput(hookOutput);
} catch (err) {
  try {
    fs.appendFileSync(_ERR_LOG, `[${new Date().toISOString()}] awaken.js: ${err.stack || err.message}\n`);
  } catch { /* ignore */ }
  process.stdout.write(JSON.stringify({}) + '\n');
}
