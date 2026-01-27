#!/usr/bin/env node
/**
 * Ralph Loop Setup Script - CYNIC Implementation
 *
 * Cross-platform Node.js version of the Ralph Loop setup script.
 * Creates state file for in-session Ralph loop.
 */

import { writeFileSync, mkdirSync, existsSync } from 'fs';

const STATE_FILE = '.claude/ralph-loop.local.md';

function showHelp() {
  console.log(`Ralph Loop - Interactive self-referential development loop

USAGE:
  node setup-ralph-loop.js [PROMPT...] [OPTIONS]

ARGUMENTS:
  PROMPT...    Initial prompt to start the loop (can be multiple words)

OPTIONS:
  --max-iterations <n>           Maximum iterations before auto-stop (default: unlimited)
  --completion-promise '<text>'  Promise phrase to end the loop
  -h, --help                     Show this help message

DESCRIPTION:
  Starts a Ralph Loop in your CURRENT session. The stop hook prevents
  exit and feeds your output back as input until completion or iteration limit.

  To signal completion, you must output: <promise>YOUR_PHRASE</promise>

EXAMPLES:
  node setup-ralph-loop.js Build a todo API --completion-promise "DONE" --max-iterations 20
  node setup-ralph-loop.js --max-iterations 10 Fix the auth bug
  node setup-ralph-loop.js Refactor cache layer  (runs forever)

STOPPING:
  Only by reaching --max-iterations or detecting --completion-promise
  No manual stop - Ralph runs infinitely by default!`);
  process.exit(0);
}

function parseArgs(args) {
  const promptParts = [];
  let maxIterations = 0;
  let completionPromise = 'null';

  let i = 0;
  while (i < args.length) {
    const arg = args[i];

    if (arg === '-h' || arg === '--help') {
      showHelp();
    } else if (arg === '--max-iterations') {
      i++;
      if (!args[i] || isNaN(parseInt(args[i], 10))) {
        console.error('❌ Error: --max-iterations requires a number argument');
        process.exit(1);
      }
      maxIterations = parseInt(args[i], 10);
    } else if (arg === '--completion-promise') {
      i++;
      if (!args[i]) {
        console.error('❌ Error: --completion-promise requires a text argument');
        process.exit(1);
      }
      completionPromise = args[i];
    } else {
      promptParts.push(arg);
    }
    i++;
  }

  return {
    prompt: promptParts.join(' '),
    maxIterations,
    completionPromise
  };
}

function main() {
  const args = process.argv.slice(2);
  const { prompt, maxIterations, completionPromise } = parseArgs(args);

  if (!prompt) {
    console.error(`❌ Error: No prompt provided

   Ralph needs a task description to work on.

   Examples:
     /ralph-loop Build a REST API for todos
     /ralph-loop Fix the auth bug --max-iterations 20
     /ralph-loop --completion-promise "DONE" Refactor code

   For all options: /ralph-loop --help`);
    process.exit(1);
  }

  // Create .claude directory if needed
  if (!existsSync('.claude')) {
    mkdirSync('.claude', { recursive: true });
  }

  // Format completion promise for YAML
  const completionPromiseYaml = completionPromise !== 'null'
    ? `"${completionPromise}"`
    : 'null';

  // Create state file
  const now = new Date().toISOString();
  const stateContent = `---
active: true
iteration: 1
max_iterations: ${maxIterations}
completion_promise: ${completionPromiseYaml}
started_at: "${now}"
---

${prompt}
`;

  writeFileSync(STATE_FILE, stateContent, 'utf-8');

  // Output setup message
  const maxIterDisplay = maxIterations > 0 ? maxIterations : 'unlimited';
  const promiseDisplay = completionPromise !== 'null'
    ? `${completionPromise} (ONLY output when TRUE - do not lie!)`
    : 'none (runs forever)';

  console.log(`🔄 Ralph loop activated in this session!

Iteration: 1
Max iterations: ${maxIterDisplay}
Completion promise: ${promiseDisplay}

The stop hook is now active. When you try to exit, the SAME PROMPT will be
fed back to you. You'll see your previous work in files, creating a
self-referential loop where you iteratively improve on the same task.

To monitor: head -10 .claude/ralph-loop.local.md

⚠️  WARNING: This loop cannot be stopped manually! It will run infinitely
    unless you set --max-iterations or --completion-promise.

🔄

${prompt}`);

  if (completionPromise !== 'null') {
    console.log(`
═══════════════════════════════════════════════════════════
CRITICAL - Ralph Loop Completion Promise
═══════════════════════════════════════════════════════════

To complete this loop, output this EXACT text:
  <promise>${completionPromise}</promise>

STRICT REQUIREMENTS (DO NOT VIOLATE):
  ✓ Use <promise> XML tags EXACTLY as shown above
  ✓ The statement MUST be completely and unequivocally TRUE
  ✓ Do NOT output false statements to exit the loop
  ✓ Do NOT lie even if you think you should exit

IMPORTANT - Do not circumvent the loop:
  Even if you believe you're stuck, the task is impossible,
  or you've been running too long - you MUST NOT output a
  false promise statement. The loop is designed to continue
  until the promise is GENUINELY TRUE. Trust the process.
═══════════════════════════════════════════════════════════`);
  }
}

main();
