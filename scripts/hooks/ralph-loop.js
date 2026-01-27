#!/usr/bin/env node
/**
 * Ralph Loop Stop Hook - CYNIC Implementation
 *
 * Cross-platform Node.js version of the Ralph Loop stop hook.
 * Prevents session exit when a ralph-loop is active and feeds
 * Claude's output back as input to continue the loop.
 *
 * Based on: claude-plugins-official/ralph-loop/hooks/stop-hook.sh
 * Fixed for: Windows path handling issues with ${CLAUDE_PLUGIN_ROOT}
 */

import { readFileSync, writeFileSync, unlinkSync, existsSync } from 'fs';
import { join } from 'path';

const STATE_FILE = '.claude/ralph-loop.local.md';

/**
 * Parse YAML frontmatter from markdown content
 */
function parseFrontmatter(content) {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return {};

  const yaml = match[1];
  const result = {};

  for (const line of yaml.split('\n')) {
    const colonIndex = line.indexOf(':');
    if (colonIndex > 0) {
      const key = line.slice(0, colonIndex).trim();
      let value = line.slice(colonIndex + 1).trim();
      // Remove surrounding quotes if present
      if ((value.startsWith('"') && value.endsWith('"')) ||
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1);
      }
      result[key] = value;
    }
  }

  return result;
}

/**
 * Extract prompt text (everything after the closing ---)
 */
function extractPrompt(content) {
  const parts = content.split(/^---$/m);
  if (parts.length >= 3) {
    // Everything after the second ---
    return parts.slice(2).join('---').trim();
  }
  return '';
}

/**
 * Extract text from <promise> tags
 */
function extractPromiseText(text) {
  const match = text.match(/<promise>([\s\S]*?)<\/promise>/);
  if (match) {
    return match[1].trim().replace(/\s+/g, ' ');
  }
  return null;
}

/**
 * Read and parse transcript to get last assistant message
 */
function getLastAssistantMessage(transcriptPath) {
  if (!existsSync(transcriptPath)) {
    return null;
  }

  const content = readFileSync(transcriptPath, 'utf-8');
  const lines = content.trim().split('\n');

  // Find last assistant message (JSONL format)
  for (let i = lines.length - 1; i >= 0; i--) {
    try {
      const entry = JSON.parse(lines[i]);
      if (entry.role === 'assistant' || entry.message?.role === 'assistant') {
        const message = entry.message || entry;
        if (message.content && Array.isArray(message.content)) {
          return message.content
            .filter(block => block.type === 'text')
            .map(block => block.text)
            .join('\n');
        }
      }
    } catch {
      // Skip invalid JSON lines
    }
  }

  return null;
}

/**
 * Clean up state file and exit allowing stop
 */
function allowStop(message) {
  if (message) {
    console.error(message);
  }
  try {
    if (existsSync(STATE_FILE)) {
      unlinkSync(STATE_FILE);
    }
  } catch {
    // Ignore cleanup errors
  }
  process.exit(0);
}

/**
 * Main hook logic
 */
async function main() {
  // Read hook input from stdin
  let hookInput = {};
  try {
    const stdin = readFileSync(0, 'utf-8');
    if (stdin.trim()) {
      hookInput = JSON.parse(stdin);
    }
  } catch {
    // No input or invalid JSON - allow stop
    process.exit(0);
  }

  // Check if ralph-loop is active
  if (!existsSync(STATE_FILE)) {
    // No active loop - allow exit
    process.exit(0);
  }

  // Read and parse state file
  let stateContent;
  try {
    stateContent = readFileSync(STATE_FILE, 'utf-8');
  } catch (err) {
    allowStop(`⚠️  Ralph loop: Cannot read state file: ${err.message}`);
    return;
  }

  const frontmatter = parseFrontmatter(stateContent);
  const iteration = parseInt(frontmatter.iteration, 10);
  const maxIterations = parseInt(frontmatter.max_iterations, 10);
  const completionPromise = frontmatter.completion_promise;

  // Validate numeric fields
  if (isNaN(iteration)) {
    allowStop(`⚠️  Ralph loop: State file corrupted
   File: ${STATE_FILE}
   Problem: 'iteration' field is not a valid number (got: '${frontmatter.iteration}')

   Ralph loop is stopping. Run /ralph-loop again to start fresh.`);
    return;
  }

  if (isNaN(maxIterations)) {
    allowStop(`⚠️  Ralph loop: State file corrupted
   File: ${STATE_FILE}
   Problem: 'max_iterations' field is not a valid number (got: '${frontmatter.max_iterations}')

   Ralph loop is stopping. Run /ralph-loop again to start fresh.`);
    return;
  }

  // Check if max iterations reached
  if (maxIterations > 0 && iteration >= maxIterations) {
    allowStop(`🛑 Ralph loop: Max iterations (${maxIterations}) reached.`);
    return;
  }

  // Get transcript path from hook input
  const transcriptPath = hookInput.transcript_path;
  if (!transcriptPath || !existsSync(transcriptPath)) {
    allowStop(`⚠️  Ralph loop: Transcript file not found
   Expected: ${transcriptPath}
   This is unusual and may indicate a Claude Code internal issue.
   Ralph loop is stopping.`);
    return;
  }

  // Read last assistant message
  const lastOutput = getLastAssistantMessage(transcriptPath);
  if (!lastOutput) {
    allowStop(`⚠️  Ralph loop: No assistant messages found in transcript
   Transcript: ${transcriptPath}
   Ralph loop is stopping.`);
    return;
  }

  // Check for completion promise
  if (completionPromise && completionPromise !== 'null') {
    const promiseText = extractPromiseText(lastOutput);
    if (promiseText && promiseText === completionPromise) {
      allowStop(`✅ Ralph loop: Detected <promise>${completionPromise}</promise>`);
      return;
    }
  }

  // Not complete - continue loop with SAME PROMPT
  const nextIteration = iteration + 1;
  const promptText = extractPrompt(stateContent);

  if (!promptText) {
    allowStop(`⚠️  Ralph loop: State file corrupted or incomplete
   File: ${STATE_FILE}
   Problem: No prompt text found

   Ralph loop is stopping. Run /ralph-loop again to start fresh.`);
    return;
  }

  // Update iteration in state file
  const updatedContent = stateContent.replace(
    /^iteration: .*/m,
    `iteration: ${nextIteration}`
  );

  try {
    writeFileSync(STATE_FILE, updatedContent, 'utf-8');
  } catch (err) {
    allowStop(`⚠️  Ralph loop: Cannot update state file: ${err.message}`);
    return;
  }

  // Build system message
  let systemMsg;
  if (completionPromise && completionPromise !== 'null') {
    systemMsg = `🔄 Ralph iteration ${nextIteration} | To stop: output <promise>${completionPromise}</promise> (ONLY when statement is TRUE - do not lie to exit!)`;
  } else {
    systemMsg = `🔄 Ralph iteration ${nextIteration} | No completion promise set - loop runs infinitely`;
  }

  // Output JSON to block the stop and feed prompt back
  const output = {
    decision: 'block',
    reason: promptText,
    systemMessage: systemMsg
  };

  console.log(JSON.stringify(output));
  process.exit(0);
}

main().catch(err => {
  console.error(`⚠️  Ralph loop error: ${err.message}`);
  process.exit(0);
});
