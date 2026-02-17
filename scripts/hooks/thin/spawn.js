#!/usr/bin/env node
/**
 * CYNIC Thin Spawn Hook — SubagentStart/SubagentStop
 *
 * Delegates to daemon via HTTP. Maps subagent types to Sefirot dogs.
 * Also notifies Python kernel (fire-and-forget) — dog lifecycle awareness.
 *
 * Usage: node spawn.js start  → SubagentStart
 *        node spawn.js stop   → SubagentStop
 *
 * "Le chien coordonne le collectif" - CYNIC
 *
 * @event SubagentStart, SubagentStop
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';

const action = process.argv[2] || 'start';
const event = action === 'stop' ? 'SubagentStop' : 'SubagentStart';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel(event, input);

const result = await callDaemon(event, input, { timeout: 5000 });
safeOutput(result);
