#!/usr/bin/env node
/**
 * CYNIC Commit Validation Hook
 *
 * "Le chien valide" - External validation via commit results
 *
 * This hook is called after commits to provide feedback to the learning service.
 * Can be triggered:
 * - From observe.cjs when Bash runs git commit
 * - As a git post-commit hook
 * - Manually for debugging
 *
 * Usage:
 *   echo '{"success": true, "commitHash": "abc123"}' | node validate-commit.cjs
 *   OR
 *   node validate-commit.cjs  # Auto-detects last commit
 *
 * @behavior non-blocking (feedback only)
 */

'use strict';

const path = require('path');
const { execFileSync } = require('child_process');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

/**
 * Get the last commit info from git
 */
function getLastCommit() {
  try {
    const hash = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf-8' }).trim();
    const message = execFileSync('git', ['log', '-1', '--format=%s'], { encoding: 'utf-8' }).trim();
    const author = execFileSync('git', ['log', '-1', '--format=%ae'], { encoding: 'utf-8' }).trim();

    return {
      success: true,
      commitHash: hash,
      message,
      author,
      hooksPassed: true, // If we got here, hooks passed
    };
  } catch (e) {
    return {
      success: false,
      error: e.message,
    };
  }
}

/**
 * Detect if this is being called from a git hook
 * Git hooks set certain environment variables
 */
function isGitHookContext() {
  return process.env.GIT_DIR || process.env.GIT_AUTHOR_NAME;
}

async function main() {
  try {
    let commitResult;

    // Check if input is provided via stdin
    let input = '';
    const hasStdin = !process.stdin.isTTY;

    if (hasStdin) {
      for await (const chunk of process.stdin) {
        input += chunk;
      }
    }

    if (input.trim()) {
      // Try to parse as JSON
      try {
        commitResult = JSON.parse(input);
      } catch (e) {
        // Treat as commit message
        commitResult = {
          success: true,
          message: input.trim(),
        };
      }
    } else {
      // Auto-detect from git
      commitResult = getLastCommit();
    }

    // Ensure required fields
    const params = {
      success: commitResult.success ?? true,
      commitHash: commitResult.commitHash || commitResult.hash,
      hooksPassed: commitResult.hooksPassed ?? true,
      message: commitResult.message,
      judgmentId: commitResult.judgmentId || null,
    };

    // Send feedback to learning service
    const result = await cynic.sendCommitFeedback(params);

    // Also notify collective
    cynic.sendHookToCollectiveSync('ExternalValidation', {
      type: 'commit',
      ...params,
      isGitHook: isGitHookContext(),
      timestamp: Date.now(),
    });

    // Output result
    console.log(JSON.stringify({
      success: true,
      feedback: 'commit',
      ...params,
      learningResult: result,
    }));

  } catch (error) {
    console.log(JSON.stringify({
      success: false,
      error: error.message,
    }));
  }
}

main();
