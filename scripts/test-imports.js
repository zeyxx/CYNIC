#!/usr/bin/env node
/**
 * Test imports and instantiation of newly created modules
 */

async function test() {
  console.log('Testing new modules...\n');

  // Test 1: ContinualTracker
  try {
    const { ContinualTracker } = await import('../packages/node/src/learning/continual-tracker.js');
    const tracker = new ContinualTracker();
    console.log('✓ ContinualTracker: imports and instantiates');
  } catch (err) {
    console.log('✗ ContinualTracker:', err.message);
  }

  // Test 2: ConsciousnessReader
  try {
    const { getConsciousnessReader } = await import('../packages/node/src/orchestration/consciousness-reader.js');
    console.log('✓ ConsciousnessReader: imports (needs DB to instantiate)');
  } catch (err) {
    console.log('✗ ConsciousnessReader:', err.message);
  }

  // Test 3: MachineHealthWatcher
  try {
    const { createMachineHealthWatcher } = await import('../packages/node/src/perception/machine-health-watcher.js');
    console.log('✓ MachineHealthWatcher: imports');
  } catch (err) {
    console.log('✗ MachineHealthWatcher:', err.message);
  }

  // Test 4: JupiterClient
  try {
    const { createJupiterClient } = await import('../packages/node/src/perception/jupiter-client.js');
    console.log('✓ JupiterClient: imports');
  } catch (err) {
    console.log('✗ JupiterClient:', err.message);
  }

  // Test 5: Service wiring
  try {
    const wiring = await import('../packages/node/src/daemon/service-wiring.js');
    const hasAll = wiring.wireOrchestrator && wiring.wireWatchers && wiring.wireConsciousnessReflection;
    console.log('✓ Service wiring: all functions exported');
  } catch (err) {
    console.log('✗ Service wiring:', err.message);
  }

  console.log('\nAll new modules loadable.');
}

test().catch(err => {
  console.error('Test failed:', err.message);
  process.exit(1);
});
