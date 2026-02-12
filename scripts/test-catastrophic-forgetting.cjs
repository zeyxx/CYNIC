/**
 * Test: Catastrophic Forgetting Detection (LV-4)
 * 
 * Simulates continual learning and detects forgetting via BWT/FWT metrics
 */

const { CatastrophicForgettingTracker, FORGETTING_THRESHOLD } = require('../packages/node/src/judge/catastrophic-forgetting-tracker.js');

// Mock database for testing
class MockDB {
  constructor() {
    this.queries = [];
  }
  
  async query(sql, params) {
    this.queries.push({ sql, params });
    
    if (sql.includes('SELECT') && sql.includes('forgetting_baselines')) {
      return { rows: [] };
    }
    
    return { rows: [] };
  }
}

async function simulateContinualLearning() {
  const db = new MockDB();
  const tracker = new CatastrophicForgettingTracker({ 
    db, 
    logger: console 
  });
  
  await tracker.initialize();
  
  console.log('\n=== Simulating Continual Learning ===\n');
  
  // Phase 1: Learn task A (code quality)
  console.log('Phase 1: Learning task A (code_quality)...');
  for (let i = 0; i < 10; i++) {
    await tracker.recordJudgment('code_quality', 0.9, `judgment_a_${i}`);
  }
  
  const bwtA1 = await tracker.calculateBWT('code_quality');
  console.log(`  BWT after training: ${bwtA1.bwt.toFixed(3)} (baseline: ${bwtA1.baseline.toFixed(3)})`);
  
  // Phase 2: Learn task B (security)
  console.log('\nPhase 2: Learning task B (security)...');
  for (let i = 0; i < 10; i++) {
    await tracker.recordJudgment('security', 0.85, `judgment_b_${i}`);
  }
  
  const bwtB1 = await tracker.calculateBWT('security');
  console.log(`  BWT after training: ${bwtB1.bwt.toFixed(3)} (baseline: ${bwtB1.baseline.toFixed(3)})`);
  
  // Phase 3: Catastrophic forgetting - performance on A drops
  console.log('\nPhase 3: Catastrophic forgetting on task A...');
  for (let i = 0; i < 10; i++) {
    await tracker.recordJudgment('code_quality', 0.6, `judgment_a_forget_${i}`);
  }
  
  const bwtA2 = await tracker.calculateBWT('code_quality');
  console.log(`  BWT after forgetting: ${bwtA2.bwt.toFixed(3)} (dropped from ${bwtA2.baseline.toFixed(3)} to ${bwtA2.current.toFixed(3)})`);
  console.log(`  Catastrophic? ${bwtA2.is_catastrophic ? 'YES *GROWL*' : 'NO'}`);
  
  // Phase 4: Learn task C (performance)
  console.log('\nPhase 4: Learning task C (performance)...');
  for (let i = 0; i < 10; i++) {
    await tracker.recordJudgment('performance', 0.88, `judgment_c_${i}`);
  }
  
  // Overall metrics
  console.log('\n=== Overall BWT Metrics ===');
  const overall = await tracker.calculateOverallBWT();
  
  if (overall) {
    console.log(`  Average BWT: ${overall.average_bwt.toFixed(3)}`);
    console.log(`  Tasks tracked: ${overall.task_count}`);
    console.log(`  Catastrophic tasks: ${overall.catastrophic_count}`);
    console.log(`  Affected tasks: ${overall.catastrophic_tasks.join(', ') || 'none'}`);
    console.log(`  Confidence: ${(overall.confidence * 100).toFixed(1)}%`);
  }
  
  // Full report
  console.log('\n=== Full Report ===');
  const report = await tracker.getReport();
  
  report.tasks.forEach(task => {
    const status = task.is_catastrophic ? '❌' : '✅';
    console.log(`  ${status} ${task.taskType}: BWT=${task.bwt.toFixed(3)} (${task.baseline.toFixed(2)} → ${task.current.toFixed(2)})`);
  });
  
  // FWT test - new task benefits from past learning
  console.log('\n=== Forward Transfer (FWT) ===');
  const fwtResult = await tracker.calculateFWT('new_task', 0.75, 0.5);
  console.log(`  Task: ${fwtResult.taskType}`);
  console.log(`  Initial accuracy: ${fwtResult.initial_accuracy.toFixed(3)}`);
  console.log(`  Random baseline: ${fwtResult.random_baseline.toFixed(3)}`);
  console.log(`  FWT: ${fwtResult.fwt.toFixed(3)}`);
  console.log(`  Positive transfer? ${fwtResult.is_positive_transfer ? 'YES *tail wag*' : 'NO'}`);
  
  // Test baseline reset
  console.log('\n=== Baseline Reset ===');
  await tracker.resetBaseline('code_quality');
  const bwtA3 = await tracker.calculateBWT('code_quality');
  console.log(`  New baseline: ${bwtA3.baseline.toFixed(3)}`);
  console.log(`  BWT after reset: ${bwtA3.bwt.toFixed(3)}`);
  
  console.log('\n*sniff* Test complete. Database queries executed:', db.queries.length);
}

// Run the test
simulateContinualLearning().catch(err => {
  console.error('Test failed:', err);
  process.exit(1);
});
