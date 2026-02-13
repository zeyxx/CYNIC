import { MetaCognitionController, _resetForTesting as resetMCC } from './packages/node/src/learning/meta-cognition-controller.js';
import { DataPipeline, _resetForTesting as resetDP } from './packages/node/src/orchestration/data-pipeline.js';
import { ResearchRunner, ResearchStep, _resetForTesting as resetRR } from './packages/node/src/orchestration/research-runner.js';
import { createOllamaValidator, createLMStudioValidator, createOpenAIValidator } from './packages/llm/src/adapters/oss-llm.js';
import { ChaosGenerator, ChaosEvent, ChaosType, _resetForTesting as resetCG } from './packages/node/src/testing/chaos-generator.js';
import { DaemonServer } from './packages/node/src/daemon/index.js';

let pass = 0, fail = 0;

function test(name, fn) {
  process.stdout.write(`Testing ${name}... `);
  try {
    fn();
    console.log('PASS');
    pass++;
  } catch (e) {
    console.log(`FAIL: ${e.message}`);
    fail++;
  }
}

// Test 1: MetaCognitionController
test('MetaCognitionController', () => {
  const c = new MetaCognitionController();
  c.start();
  const p = c.getParameters();
  if (!p.learningRate) throw new Error('learningRate missing');
  c.stop();
  resetMCC();
});

// Test 2: DataPipeline
test('DataPipeline', async () => {
  const dp = new DataPipeline();
  const item = await dp.process('test', {key: 'test'});
  if (!item.compressed) throw new Error('not compressed');
  dp.destroy();
  resetDP();
});

// Test 3: ResearchRunner
test('ResearchRunner', async () => {
  const r = new ResearchRunner();
  const proto = r.createProtocol({
    type: 'exploratory',
    question: 'Does the system work?',
    steps: [new ResearchStep({
      method: async () => ({ observation: 'test', confidence: 0.5 })
    })]
  });
  const report = await r.execute(proto);
  if (report.confidence === undefined) throw new Error('no confidence');
  resetRR();
});

// Test 4: OSSLLMAdapters
test('OSSLLMAdapters', () => {
  const o = createOllamaValidator();
  const l = createLMStudioValidator();
  const c = createOpenAIValidator({endpoint: 'http://test:8000'});
  if (o.provider !== 'ollama') throw new Error('ollama wrong');
  if (l.provider !== 'lm-studio') throw new Error('lm-studio wrong');
  if (c.provider !== 'openai-compat') throw new Error('openai-compat wrong');
});

// Test 5: ChaosGenerator
test('ChaosGenerator', async () => {
  const cg = new ChaosGenerator({enabled: true});
  const event = new ChaosEvent({type: ChaosType.MEMORY_SPIKE, duration: 50});
  const exp = cg.createExperiment({name: 'test', events: [event]});
  const result = await cg.runExperiment(exp);
  if (result.results.resilienceScore === undefined) throw new Error('no resilience score');
  await cg.stopAll();
  resetCG();
});

// Test 6: DaemonServer
test('DaemonServer', () => {
  const d = new DaemonServer({port: 39999});
  if (!d.app || typeof d.app.use !== 'function') throw new Error('Express not initialized');
});

console.log(`\n${Array(60).fill('=').join('')}`);
console.log(`RESULTS: ${pass}/6 passed, ${fail}/6 failed`);
process.exit(fail > 0 ? 1 : 0);
