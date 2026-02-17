#!/usr/bin/env node
/**
 * CYNIC Thin Perceive Hook — UserPromptSubmit
 *
 * Delegates to daemon via HTTP. Auto-starts daemon if needed.
 * Also notifies Python kernel (fire-and-forget) for learning.
 * "Le chien délègue" - CYNIC
 *
 * @event UserPromptSubmit
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel, readKernelGuidance } from './kernel-client.js';
import fs from 'fs';
import os from 'os';
import path from 'path';

const _ERR_LOG = path.join(os.homedir(), '.cynic', 'hook-error.log');

try {
  const input = await readHookInput();

  // Notify Python kernel (fire-and-forget — never blocks)
  notifyKernel('UserPromptSubmit', input);

  const result = await callDaemon('UserPromptSubmit', input, { timeout: 8000 });

  // Inject kernel guidance from last judgment — this is the feedback loop closing.
  // guidance.json was written by the Python kernel after judging the PREVIOUS interaction.
  // CYNIC sees its own past judgment as context for the current response.
  const guidance = readKernelGuidance();
  if (guidance) {
    const verdictSymbol = { HOWL: '🟢', WAG: '🟡', GROWL: '🟠', BARK: '🔴' }[guidance.verdict] || '⚪';
    const bar = (score) => {
      const filled = Math.round((score / 61.803) * 10);
      return '[' + '█'.repeat(Math.min(filled, 10)) + '░'.repeat(Math.max(10 - filled, 0)) + ']';
    };
    const dogs = Object.entries(guidance.dog_votes || {})
      .map(([dog, score]) => `${dog} ${bar(score)}`)
      .join(' · ');
    const kernelMsg = [
      `*sniff* 🧠 Kernel (${guidance.state_key}): ${verdictSymbol} ${guidance.verdict} Q=${guidance.q_score.toFixed(1)} conf=${Math.round(guidance.confidence * 100)}%`,
      dogs ? `  ${dogs}` : '',
    ].filter(Boolean).join('\n');

    // additionalContext must be at the TOP LEVEL of hook output — Claude Code reads it there.
    // NOT inside hookSpecificOutput (that's a daemon-internal field, not a Claude Code field).
    const prev = result.additionalContext || '';
    result.additionalContext = prev ? `${prev}\n\n${kernelMsg}` : kernelMsg;
  }

  safeOutput(result);
} catch (err) {
  // Log error to file for diagnosis, then degrade gracefully
  try {
    fs.appendFileSync(_ERR_LOG, `[${new Date().toISOString()}] ${err.stack || err.message}\n`);
  } catch { /* ignore */ }
  // Always output valid JSON so Claude Code doesn't show a hook error
  process.stdout.write(JSON.stringify({ continue: true }) + '\n');
}
