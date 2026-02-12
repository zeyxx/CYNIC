/**
 * Real-time memory tracking
 *
 * Connects to daemon and polls /debug/memory every second to see what grows
 */

const DAEMON_URL = 'http://localhost:6180';
const POLL_INTERVAL_MS = 1000;
const DURATION_S = 120;

async function fetchMemory() {
  const res = await fetch(`${DAEMON_URL}/debug/memory`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return await res.json();
}

async function track() {
  console.log('Starting memory tracker...\n');
  console.log('Time(s)'.padStart(8), 'RSS(MB)'.padStart(10), 'Heap Used'.padStart(12), 'Heap Total'.padStart(12), 'External'.padStart(10), 'Δ Heap'.padStart(10), 'Δ RSS'.padStart(10));
  console.log('-'.repeat(90));

  let samples = [];
  let startTime = Date.now();

  for (let i = 0; i < DURATION_S; i++) {
    try {
      const stats = await fetchMemory();
      const elapsed = Math.round((stats.timestamp - startTime) / 1000);

      const prevSample = samples[samples.length - 1];
      const deltaHeap = prevSample ? stats.heapUsed - prevSample.heapUsed : 0;
      const deltaRSS = prevSample ? stats.rss - prevSample.rss : 0;

      console.log(
        elapsed.toString().padStart(8),
        stats.rss.toString().padStart(10),
        stats.heapUsed.toString().padStart(12),
        stats.heapTotal.toString().padStart(12),
        stats.external.toString().padStart(10),
        (deltaHeap > 0 ? '+' : '') + deltaHeap.toString().padStart(9),
        (deltaRSS > 0 ? '+' : '') + deltaRSS.toString().padStart(9)
      );

      samples.push(stats);

      if (samples.length > 1) {
        // Check for rapid growth
        if (deltaHeap > 10) {
          console.log(`  ⚠️  Rapid heap growth: +${deltaHeap}MB in 1s`);
        }
        if (deltaRSS > 20) {
          console.log(`  ⚠️  Rapid RSS growth: +${deltaRSS}MB in 1s`);
        }
      }

      await new Promise(r => setTimeout(r, POLL_INTERVAL_MS));
    } catch (err) {
      console.error(`\n❌ Daemon died: ${err.message}`);
      break;
    }
  }

  // Summary
  if (samples.length > 1) {
    const first = samples[0];
    const last = samples[samples.length - 1];
    const elapsed = Math.round((last.timestamp - first.timestamp) / 1000);

    console.log('\n=== SUMMARY ===');
    console.log(`Duration: ${elapsed}s`);
    console.log(`RSS growth: ${first.rss}MB → ${last.rss}MB (+${last.rss - first.rss}MB)`);
    console.log(`Heap used growth: ${first.heapUsed}MB → ${last.heapUsed}MB (+${last.heapUsed - first.heapUsed}MB)`);
    console.log(`Heap total growth: ${first.heapTotal}MB → ${last.heapTotal}MB (+${last.heapTotal - first.heapTotal}MB)`);
    console.log(`External growth: ${first.external}MB → ${last.external}MB (+${last.external - first.external}MB)`);
    console.log(`\nLeak rate:`);
    console.log(`  RSS: ${((last.rss - first.rss) / elapsed).toFixed(2)} MB/s`);
    console.log(`  Heap: ${((last.heapUsed - first.heapUsed) / elapsed).toFixed(2)} MB/s`);

    // Diagnosis
    const heapGrowth = last.heapUsed - first.heapUsed;
    const rssGrowth = last.rss - first.rss;
    const externalGrowth = last.external - first.external;

    console.log(`\n=== DIAGNOSIS ===`);
    if (heapGrowth > rssGrowth * 0.5) {
      console.log(`🔍 PRIMARY LEAK: JS heap objects (${heapGrowth}MB of ${rssGrowth}MB total)`);
    } else if (externalGrowth > rssGrowth * 0.3) {
      console.log(`🔍 PRIMARY LEAK: External memory (ArrayBuffers, etc.)`);
    } else {
      console.log(`🔍 PRIMARY LEAK: Native modules or fragmentation`);
    }
  }
}

track().catch(err => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
