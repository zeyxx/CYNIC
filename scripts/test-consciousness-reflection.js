/**
 * Test script for consciousness reflection loop
 * Validates that ConsciousnessReader can read state and generate reflections
 */

import { getConsciousnessReader } from '../packages/node/src/orchestration/consciousness-reader.js';
import { getDatabase } from '@cynic/persistence';

async function testConsciousnessReflection() {
  console.log("Testing consciousness reflection loop...");

  const reader = getConsciousnessReader({ reflectionWindow: 24 });
  await reader.initialize();

  console.log("Generating reflection...");
  const reflection = await reader.generateReflection();

  console.log("Reflection generated:");
  console.log(JSON.stringify(reflection, null, 2));

  console.log("\nStoring reflection...");
  const reflectionId = await reader.storeReflection(reflection);
  console.log(`Reflection stored with ID: ${reflectionId}`);

  process.exit(0);
}

testConsciousnessReflection().catch(err => {
  console.error("Test failed:", err);
  process.exit(1);
});
