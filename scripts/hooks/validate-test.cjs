#!/usr/bin/env node
/**
 * CYNIC Test Validation Hook
 *
 * "Le chien valide" - External validation via test results
 *
 * This hook is called after test runs to provide feedback to the learning service.
 * Can be triggered:
 * - From observe.cjs when Bash runs npm test/jest/vitest
 * - Directly from git hooks or CI
 * - Manually for debugging
 *
 * Usage:
 *   echo '{"passed": true, "passCount": 10, "failCount": 0}' | node validate-test.cjs
 *
 * @behavior non-blocking (feedback only)
 */

'use strict';

const path = require('path');

// Load core library
const libPath = path.join(__dirname, '..', 'lib', 'cynic-core.cjs');
const cynic = require(libPath);

/**
 * Parse test output to extract pass/fail counts
 * Supports: npm test, jest, vitest, mocha
 */
function parseTestOutput(output) {
  let passed = false;
  let passCount = 0;
  let failCount = 0;
  let testSuite = 'unknown';

  // Jest/Vitest format: "Tests: X passed, Y failed"
  const jestMatch = output.match(/Tests?:\s*(\d+)\s*passed(?:,\s*(\d+)\s*failed)?/i);
  if (jestMatch) {
    passCount = parseInt(jestMatch[1], 10) || 0;
    failCount = parseInt(jestMatch[2], 10) || 0;
    testSuite = output.includes('vitest') ? 'vitest' : 'jest';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // Mocha format: "X passing, Y failing"
  const mochaMatch = output.match(/(\d+)\s*passing(?:.*?(\d+)\s*failing)?/i);
  if (mochaMatch) {
    passCount = parseInt(mochaMatch[1], 10) || 0;
    failCount = parseInt(mochaMatch[2], 10) || 0;
    testSuite = 'mocha';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // npm test exit code detection
  if (output.includes('npm ERR!') || output.includes('FAIL')) {
    passed = false;
    failCount = 1;
    testSuite = 'npm';
  } else if (output.includes('passed') || output.includes('PASS') || output.includes('ok')) {
    passed = true;
    passCount = 1;
    testSuite = 'npm';
  }

  return { passed, passCount, failCount, testSuite };
}

async function main() {
  try {
    // Read input from stdin (JSON or raw test output)
    let input = '';
    for await (const chunk of process.stdin) {
      input += chunk;
    }

    let testResult;

    // Try to parse as JSON first
    try {
      testResult = JSON.parse(input);
    } catch (e) {
      // Parse as raw test output
      testResult = parseTestOutput(input);
    }

    // Ensure required fields
    const params = {
      passed: testResult.passed ?? false,
      passCount: testResult.passCount ?? 0,
      failCount: testResult.failCount ?? 0,
      testSuite: testResult.testSuite || 'unknown',
      judgmentId: testResult.judgmentId || null,
    };

    // Send feedback to learning service
    const result = await cynic.sendTestFeedback(params);

    // Also notify collective
    cynic.sendHookToCollectiveSync('ExternalValidation', {
      type: 'test_result',
      ...params,
      timestamp: Date.now(),
    });

    // Output result
    console.log(JSON.stringify({
      success: true,
      feedback: 'test_result',
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
