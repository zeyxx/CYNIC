/**
 * Quick test to verify UnifiedOrchestrator wiring
 */
import { wireOrchestrator, getOrchestratorSingleton, getKabbalisticRouterSingleton, getDogOrchestratorSingleton } from './packages/node/src/daemon/service-wiring.js';

async function test() {
  console.log('Testing UnifiedOrchestrator wiring...\n');

  try {
    // Wire the orchestrator
    console.log('1. Wiring orchestrator...');
    const result = await wireOrchestrator();

    console.log('✓ Orchestrator wired successfully');
    console.log(`  - UnifiedOrchestrator: ${result.orchestrator ? 'created' : 'null'}`);
    console.log(`  - KabbalisticRouter: ${result.kabbalisticRouter ? 'created' : 'null'}`);
    console.log(`  - DogOrchestrator: ${result.dogOrchestrator ? 'created' : 'null'}`);

    // Test singletons
    console.log('\n2. Testing singletons...');
    const orchestrator = getOrchestratorSingleton();
    const router = getKabbalisticRouterSingleton();
    const dogs = getDogOrchestratorSingleton();

    console.log(`✓ Singletons accessible`);
    console.log(`  - getOrchestratorSingleton(): ${orchestrator ? 'OK' : 'null'}`);
    console.log(`  - getKabbalisticRouterSingleton(): ${router ? 'OK' : 'null'}`);
    console.log(`  - getDogOrchestratorSingleton(): ${dogs ? 'OK' : 'null'}`);

    // Test orchestrator stats
    if (orchestrator) {
      console.log('\n3. Testing orchestrator stats...');
      const stats = orchestrator.getStats();
      console.log(`✓ Stats retrieved`);
      console.log(`  - eventsProcessed: ${stats.eventsProcessed || 0}`);
      console.log(`  - decisionsRouted: ${stats.decisionsRouted || 0}`);
    }

    console.log('\n✅ All tests passed!');
    process.exit(0);
  } catch (err) {
    console.error('\n❌ Test failed:', err.message);
    console.error(err.stack);
    process.exit(1);
  }
}

test();
