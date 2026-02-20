#!/usr/bin/env node
/**
 * Test LLM Router
 * Quick test to verify routing logic works
 */

import { routePrompt, classifyTask, chooseModel } from './hooks/llm-router.js';

const testCases = [
  // Simple tasks → should route to Ollama (qwen2.5)
  {
    prompt: "What is 2+2?",
    expected: { provider: 'ollama', model: 'qwen2.5:1.5b' }
  },
  {
    prompt: "List the files in the current directory",
    expected: { provider: 'ollama', model: 'qwen2.5:1.5b' }
  },

  // Moderate tasks → should route to Ollama (mistral)
  {
    prompt: "Write a function to sort an array of numbers in JavaScript",
    expected: { provider: 'ollama', model: 'mistral:7b-instruct-q4_0' }
  },

  // Complex tasks → should route to Claude
  {
    prompt: "Design the architecture for a distributed consensus system using Raft protocol. Explain the leader election process and how to handle network partitions.",
    expected: { provider: 'claude' }
  },

  // Critical tasks → should route to Claude
  {
    prompt: "Create a Solana transaction to transfer 100 SOL to mainnet address",
    expected: { provider: 'claude' }
  }
];

console.log('🧪 Testing LLM Router...\n');

for (const test of testCases) {
  console.log(`📝 Prompt: "${test.prompt.substring(0, 60)}..."`);

  // Classify
  const classification = classifyTask(test.prompt);
  console.log(`   Classification: ${classification.complexity} (${classification.reason})`);

  // Choose model
  const choice = chooseModel(classification);
  console.log(`   Choice: ${choice.provider} / ${choice.model || 'default'}`);
  console.log(`   Reason: ${choice.reason}`);

  // Verify
  const match = choice.provider === test.expected.provider &&
    (!test.expected.model || choice.model === test.expected.model);

  console.log(`   Result: ${match ? '✅ PASS' : '❌ FAIL'}\n`);
}

console.log('\n🚀 Now testing actual Ollama call...\n');

// Test real Ollama call
const simplePrompt = "What is the capital of France? Answer in one word.";
console.log(`Prompt: "${simplePrompt}"`);

try {
  const result = await routePrompt(simplePrompt);

  if (result.passthrough) {
    console.log(`⚠️  Routed to Claude: ${result.reason}`);
  } else {
    console.log(`✅ Ollama response: "${result.response}"`);
    console.log(`   Model: ${result.model}`);
    console.log(`   Latency: ${result.latency}ms`);
  }
} catch (error) {
  console.error(`❌ Error: ${error.message}`);
}
