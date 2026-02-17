#!/usr/bin/env node
/**
 * CYNIC Thin Sleep Hook — SessionEnd
 *
 * Delegates to daemon via HTTP. Daemon stays running.
 * Also notifies Python kernel (fire-and-forget) — session close signal.
 *
 * "Le chien s'endort mais le daemon veille" - CYNIC
 *
 * @event SessionEnd
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('SessionEnd', input);

const result = await callDaemon('SessionEnd', input, { timeout: 8000 });
safeOutput(result);
