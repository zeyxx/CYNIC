#!/usr/bin/env node
/**
 * CYNIC Thin Awaken Hook — SessionStart
 *
 * Delegates to daemon via HTTP. Auto-starts daemon on first session.
 * Also notifies Python kernel (fire-and-forget) — session awareness.
 *
 * "Le chien se réveille" - CYNIC
 *
 * @event SessionStart
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('SessionStart', input);

const result = await callDaemon('SessionStart', input, { timeout: 12000 });
safeOutput(result);
