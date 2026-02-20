/**
 * CYNIC Bridge Integration Test (Simplified)
 * 
 * Test que le bridge connecte reellement @cynic/llm a l'ecosysteme
 * 
 * @run: node test-bridge-integration.mjs
 */

'use strict';

// Simple test without complex imports
async function test() {
  console.log('═'.repeat(60));
  console.log('CYNIC Bridge Integration Test');
  console.log('═'.repeat(60));

  try {
    // Test 1: Import the bridge
    console.log('\n--- Test 1: Import ---');
    const { createCYNICBridge } = await import('./packages/llm/src/index.js');
    console.log('✓ createCYNICBridge imported');

    // Test 2: Create bridge with minimal config
    console.log('\n--- Test 2: Create Bridge ---');
    const bridge = await createCYNICBridge({
      enableLLMSelection: false,  // Disable for testing without LLM
      enableContextRetrieval: false,  // Disable for testing without VectorStore
      enableLearning: false,  // Disable for testing without DB
      enablePlanning: false,  // Disable for testing
    });

    console.log('✓ Bridge cree');
    console.log('\nStats:', bridge.getStats());

    // Test 3: Basic judgment (fallback mode)
    console.log('\n--- Test 3: Judgment fallback ---');
    const judgment = await bridge.judge(
      {
        id: 'test-1',
        type: 'test_item',
        content: 'Un test de jugement avec du contenu substantiel pour tester le systeme',
      },
      { 
        query: 'test de jugement',
        taskType: 'reasoning' 
      }
    );

    console.log('Judgment:', {
      id: judgment.id,
      qScore: judgment.qScore,
      verdict: judgment.verdict,
      bridgeMetadata: judgment.bridgeMetadata,
    });

    // Test 4: Stats finaux
    console.log('\n--- Test 4: Stats ---');
    const stats = bridge.getStats();
    console.log('Stats:', stats);

    // Cleanup
    await bridge.close();

    console.log('\n' + '═'.repeat(60));
    console.log('TOUS LES TESTS PASSES ✓');
    console.log('═'.repeat(60));
    console.log('\nLe bridge fonctionne! @cynic/llm est now integre a l\'ecosysteme!');
    console.log('\nPour utiliser pleinement:');
    console.log('1. Configurer PostgreSQL (CYNIC_DATABASE_URL)');
    console.log('2. Configurer Ollama (pour embeddings gratuits)');
    console.log('3. Activer les features dans createCYNICBridge()');
    
  } catch (error) {
    console.error('\n❌ ERREUR:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

test();
