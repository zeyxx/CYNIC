/**
 * Safe command execution utility
 *
 * Uses execFile (not exec) to prevent shell injection attacks.
 * Handles Windows compatibility and provides structured error handling.
 *
 * @module @cynic/core/utils/execFileNoThrow
 */

'use strict';

import { execFile } from 'child_process';
import { promisify } from 'util';

const execFileAsync = promisify(execFile);

/**
 * Execute a command safely without throwing
 *
 * @param {string} command - Command to execute (no shell interpolation)
 * @param {string[]} args - Arguments (passed safely, no injection risk)
 * @param {object} options - Options for execFile
 * @returns {Promise<{ stdout: string, stderr: string, code: number, error: Error|null }>}
 */
export async function execFileNoThrow(command, args = [], options = {}) {
  const defaults = {
    encoding: 'utf8',
    timeout: 10000, // 10 seconds
    maxBuffer: 1024 * 1024, // 1 MB
    windowsHide: true, // Don't flash window on Windows
  };

  const opts = { ...defaults, ...options };

  try {
    const { stdout, stderr } = await execFileAsync(command, args, opts);
    return {
      stdout: stdout || '',
      stderr: stderr || '',
      code: 0,
      error: null,
    };
  } catch (error) {
    return {
      stdout: error.stdout || '',
      stderr: error.stderr || '',
      code: error.code || 1,
      error,
    };
  }
}

/**
 * Execute PowerShell command safely (Windows only)
 *
 * @param {string} scriptContent - PowerShell script content
 * @param {object} options - Options for execFile
 * @returns {Promise<{ stdout: string, stderr: string, code: number, error: Error|null }>}
 */
export async function execPowerShellNoThrow(scriptContent, options = {}) {
  // Use -Command with properly escaped content
  // PowerShell handles quoting internally, no shell interpolation
  return execFileNoThrow('powershell.exe', [
    '-NoProfile',
    '-NonInteractive',
    '-Command',
    scriptContent,
  ], options);
}

/**
 * Check if command exists (safe alternative to `which`)
 *
 * @param {string} command - Command name to check
 * @returns {Promise<boolean>}
 */
export async function commandExists(command) {
  const isWindows = process.platform === 'win32';

  if (isWindows) {
    const result = await execFileNoThrow('where', [command]);
    return result.code === 0;
  } else {
    const result = await execFileNoThrow('which', [command]);
    return result.code === 0;
  }
}
