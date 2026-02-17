#!/usr/bin/env node
/**
 * LLM Router Hook — Intelligent routing between Claude and Ollama
 *
 * Intercepts Claude Code prompts and routes to:
 * - Ollama: Simple tasks, bulk processing, code completion
 * - Claude: Complex reasoning, architecture, critical decisions
 *
 * "Le chien choisit le bon outil" - κυνικός
 *
 * @module hooks/llm-router
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';
import { execSync } from 'child_process';

const HOOK_DIR = path.join(os.homedir(), '.cynic', 'hooks');
const ROUTING_LOG = path.join(HOOK_DIR, 'routing-decisions.jsonl');

// Ensure log dir exists
if (!fs.existsSync(HOOK_DIR)) {
  fs.mkdirSync(HOOK_DIR, { recursive: true });
}

/**
 * Classify task complexity from prompt
 */
function classifyTask(prompt) {
  const lower = prompt.toLowerCase();

  // Complexity signals
  const complexSignals = [
    'architecture', 'design', 'refactor', 'implement',
    'explain why', 'analyze', 'metathinking', 'strategy'
  ];

  const simpleSignals = [
    'fix typo', 'add comment', 'format', 'lint',
    'list', 'show', 'what is', 'simple'
  ];

  // Risk signals (always use Claude)
  const riskSignals = [
    'solana', 'mainnet', 'transaction', 'deploy',
    'security', 'auth', 'crypto', 'database migration'
  ];

  // Check risk first
  if (riskSignals.some(sig => lower.includes(sig))) {
    return {
      complexity: 'critical',
      reason: 'Risk-sensitive task detected'
    };
  }

  // Check complexity
  const complexCount = complexSignals.filter(sig => lower.includes(sig)).length;
  const simpleCount = simpleSignals.filter(sig => lower.includes(sig)).length;

  if (complexCount >= 2) {
    return {
      complexity: 'complex',
      reason: `${complexCount} complex signals`
    };
  }

  if (simpleCount >= 1 && complexCount === 0) {
    return {
      complexity: 'simple',
      reason: `${simpleCount} simple signals`
    };
  }

  // Code generation signals → moderate (even if short)
  const codeSignals = ['write', 'create', 'function', 'implement', 'code', 'build'];
  if (codeSignals.some(sig => lower.includes(sig))) {
    return {
      complexity: 'moderate',
      reason: 'Code generation task'
    };
  }

  // Length-based heuristic (fallback)
  if (prompt.length < 100) {
    return {
      complexity: 'simple',
      reason: 'Short prompt (< 100 chars)'
    };
  }

  if (prompt.length > 500) {
    return {
      complexity: 'complex',
      reason: 'Long prompt (> 500 chars)'
    };
  }

  return {
    complexity: 'moderate',
    reason: 'Default moderate complexity'
  };
}

/**
 * Choose model based on task classification
 */
function chooseModel(classification) {
  const { complexity } = classification;

  // Critical/Complex → Claude
  if (complexity === 'critical' || complexity === 'complex') {
    return {
      provider: 'claude',
      model: 'claude-sonnet-4',
      reason: `${complexity} task requires Claude`
    };
  }

  // Simple → Ollama (fast model)
  if (complexity === 'simple') {
    return {
      provider: 'ollama',
      model: 'qwen2.5:1.5b',
      reason: 'Simple task, fast Ollama model'
    };
  }

  // Moderate → Ollama (capable model)
  return {
    provider: 'ollama',
    model: 'mistral:7b-instruct-q4_0',
    reason: 'Moderate task, capable Ollama model'
  };
}

/**
 * Call Ollama API
 */
async function callOllama(model, prompt) {
  const response = await fetch('http://localhost:11434/api/generate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model,
      prompt,
      stream: false,
      options: {
        temperature: 0.7,
        top_p: 0.9
      }
    })
  });

  if (!response.ok) {
    throw new Error(`Ollama API error: ${response.status}`);
  }

  const data = await response.json();
  return data.response;
}

/**
 * Log routing decision
 */
function logDecision(decision) {
  const log = JSON.stringify({
    timestamp: new Date().toISOString(),
    ...decision
  }) + '\n';

  fs.appendFileSync(ROUTING_LOG, log);
}

/**
 * Main router logic
 */
async function routePrompt(prompt) {
  const startTime = Date.now();

  // 1. Classify task
  const classification = classifyTask(prompt);

  // 2. Choose model
  const choice = chooseModel(classification);

  // 3. Execute
  let response;
  let error = null;

  try {
    if (choice.provider === 'ollama') {
      response = await callOllama(choice.model, prompt);
    } else {
      // Let Claude Code handle it (passthrough)
      return {
        passthrough: true,
        reason: choice.reason
      };
    }
  } catch (err) {
    error = err.message;
    // Fallback to Claude on error
    return {
      passthrough: true,
      reason: `Ollama failed: ${err.message}, fallback to Claude`
    };
  }

  const latency = Date.now() - startTime;

  // 4. Log decision
  logDecision({
    classification,
    choice,
    latency,
    error,
    promptLength: prompt.length,
    responseLength: response?.length || 0
  });

  return {
    response,
    model: choice.model,
    provider: choice.provider,
    latency,
    reason: choice.reason
  };
}

/**
 * Hook entry point
 */
async function main() {
  try {
    // Read prompt from stdin (Claude Code provides this)
    const prompt = process.argv[2] || '';

    if (!prompt) {
      console.error('No prompt provided');
      process.exit(1);
    }

    // Route prompt
    const result = await routePrompt(prompt);

    // Output result
    if (result.passthrough) {
      // Let Claude Code handle it
      console.log(JSON.stringify({
        action: 'passthrough',
        reason: result.reason
      }));
    } else {
      // Return Ollama response
      console.log(JSON.stringify({
        action: 'respond',
        response: result.response,
        metadata: {
          model: result.model,
          provider: result.provider,
          latency: result.latency,
          reason: result.reason
        }
      }));
    }

  } catch (error) {
    console.error(JSON.stringify({
      action: 'error',
      error: error.message
    }));
    process.exit(1);
  }
}

// Run if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}

export { routePrompt, classifyTask, chooseModel };
