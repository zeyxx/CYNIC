#!/usr/bin/env node
/**
 * CYNIC Thin Error Hook — Error
 *
 * Delegates to daemon via HTTP. Error history persists in daemon RAM.
 *
 * "Le chien apprend de ses erreurs" - CYNIC
 *
 * @event Error
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';

const input = await readHookInput();
const result = await callDaemon('Error', input, { timeout: 5000 });
safeOutput(result);
