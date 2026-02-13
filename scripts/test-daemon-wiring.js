#!/usr/bin/env node
/**
 * Test that daemon entry.js loads and has all wiring functions
 */

async function test() {
  console.log('Testing daemon wiring functions...\n');

  try {
    const wiring = await import('../packages/node/src/daemon/service-wiring.js');

    const functions = [
      'wireDaemonServices',
      'wireLearningSystem',
      'wireOrchestrator',
      'wireWatchers',
      'wireConsciousnessReflection',
      'cleanupDaemonServices',
    ];

    let allPresent = true;
    for (const fn of functions) {
      const present = typeof wiring[fn] === 'function';
      console.log(`${present ? '✓' : '✗'} ${fn}: ${present ? 'exported' : 'MISSING'}`);
      if (!present) allPresent = false;
    }

    if (allPresent) {
      console.log('\n✓ All daemon wiring functions present');
      console.log('\nDaemon boot sequence will call:');
      console.log('  1. wireDaemonServices()');
      console.log('  2. wireLearningSystem()');
      console.log('  3. wireOrchestrator()');
      console.log('  4. wireWatchers()');
      console.log('  5. wireConsciousnessReflection()');
      process.exit(0);
    } else {
      console.log('\n✗ Some wiring functions missing');
      process.exit(1);
    }
  } catch (err) {
    console.error('✗ Failed to load daemon wiring:', err.message);
    process.exit(1);
  }
}

test();
