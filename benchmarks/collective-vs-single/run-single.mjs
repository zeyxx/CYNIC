#!/usr/bin/env node
/**
 * Single-Agent Baseline Runner
 *
 * Runs a single Claude call per code sample.
 * This is the baseline to compare against CYNIC collective.
 */

import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import Anthropic from '@anthropic-ai/sdk';

const client = new Anthropic();

// Load samples
const dataset = JSON.parse(
  readFileSync('benchmarks/collective-vs-single/dataset/samples.json', 'utf8')
);

const SYSTEM_PROMPT = `You are a senior security engineer reviewing code for vulnerabilities and issues.

For each code snippet, you must:
1. Identify ALL security vulnerabilities and bugs
2. Classify each issue by type (SQL_INJECTION, XSS, COMMAND_INJECTION, etc.)
3. Rate severity (critical, high, medium, low)
4. Provide a verdict: HOWL (excellent), WAG (acceptable), GROWL (concerning), BARK (reject)
5. Give a Q-Score from 0-100 (100 = perfect code)

Respond in JSON format:
{
  "issues": [
    {
      "type": "ISSUE_TYPE",
      "line": 1,
      "description": "What's wrong",
      "severity": "critical|high|medium|low"
    }
  ],
  "verdict": "HOWL|WAG|GROWL|BARK",
  "score": 0-100,
  "reasoning": "Brief explanation"
}`;

async function runSingle(sample) {
  const start = performance.now();

  try {
    const response = await client.messages.create({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 1024,
      system: SYSTEM_PROMPT,
      messages: [
        {
          role: 'user',
          content: `Review this ${sample.language} code:\n\n\`\`\`${sample.language}\n${sample.code}\n\`\`\``
        }
      ]
    });

    const elapsed = performance.now() - start;
    const text = response.content[0].text;

    // Parse JSON from response
    let result;
    try {
      // Extract JSON from markdown code block if present
      const jsonMatch = text.match(/```json\n?([\s\S]*?)\n?```/) ||
                        text.match(/```\n?([\s\S]*?)\n?```/) ||
                        [null, text];
      result = JSON.parse(jsonMatch[1] || text);
    } catch (e) {
      result = {
        issues: [],
        verdict: 'UNKNOWN',
        score: 50,
        reasoning: 'Failed to parse response',
        raw: text
      };
    }

    return {
      sampleId: sample.id,
      system: 'single',
      latencyMs: Math.round(elapsed),
      tokensIn: response.usage.input_tokens,
      tokensOut: response.usage.output_tokens,
      result,
      success: true
    };

  } catch (error) {
    return {
      sampleId: sample.id,
      system: 'single',
      latencyMs: Math.round(performance.now() - start),
      error: error.message,
      success: false
    };
  }
}

async function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  SINGLE-AGENT BASELINE');
  console.log('  Model: claude-sonnet-4-20250514');
  console.log(`  Samples: ${dataset.samples.length}`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');

  const results = [];
  let totalLatency = 0;
  let totalTokensIn = 0;
  let totalTokensOut = 0;

  for (let i = 0; i < dataset.samples.length; i++) {
    const sample = dataset.samples[i];
    process.stdout.write(`  [${i + 1}/${dataset.samples.length}] ${sample.id}... `);

    const result = await runSingle(sample);
    results.push(result);

    if (result.success) {
      totalLatency += result.latencyMs;
      totalTokensIn += result.tokensIn;
      totalTokensOut += result.tokensOut;
      console.log(`${result.latencyMs}ms | ${result.result.verdict} | Score: ${result.result.score}`);
    } else {
      console.log(`ERROR: ${result.error}`);
    }

    // Rate limiting - wait 500ms between calls
    await new Promise(r => setTimeout(r, 500));
  }

  // Summary
  console.log('');
  console.log('───────────────────────────────────────────────────────────────');
  console.log('  SUMMARY');
  console.log('───────────────────────────────────────────────────────────────');
  console.log(`  Total samples:     ${dataset.samples.length}`);
  console.log(`  Successful:        ${results.filter(r => r.success).length}`);
  console.log(`  Failed:            ${results.filter(r => !r.success).length}`);
  console.log(`  Total latency:     ${totalLatency}ms`);
  console.log(`  Avg latency:       ${Math.round(totalLatency / results.filter(r => r.success).length)}ms`);
  console.log(`  Total tokens in:   ${totalTokensIn}`);
  console.log(`  Total tokens out:  ${totalTokensOut}`);
  console.log('');

  // Save results
  mkdirSync('benchmarks/collective-vs-single/results', { recursive: true });
  const outputPath = `benchmarks/collective-vs-single/results/single-${Date.now()}.json`;
  writeFileSync(outputPath, JSON.stringify({
    system: 'single',
    model: 'claude-sonnet-4-20250514',
    timestamp: new Date().toISOString(),
    summary: {
      totalSamples: dataset.samples.length,
      successful: results.filter(r => r.success).length,
      failed: results.filter(r => !r.success).length,
      totalLatencyMs: totalLatency,
      avgLatencyMs: Math.round(totalLatency / results.filter(r => r.success).length),
      totalTokensIn,
      totalTokensOut
    },
    results
  }, null, 2));

  console.log(`  Results saved to: ${outputPath}`);
  console.log('');
}

main().catch(console.error);
