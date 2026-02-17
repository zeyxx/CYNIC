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

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('UserPromptSubmit', input);

const result = await callDaemon('UserPromptSubmit', input, { timeout: 8000 });

// Inject kernel guidance from last judgment — this is the feedback loop closing.
// guidance.json was written by the Python kernel after judging the PREVIOUS interaction.
// CYNIC sees its own past judgment as context for the current response.
// Claude Code reads the `message` field as system-reminder — guidance must go there.
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

  // Append to existing message or set as message
  result.kernelGuidance = guidance; // keep for structured access
  result.message = result.message ? `${result.message}\n\n${kernelMsg}` : kernelMsg;
}

safeOutput(result);
