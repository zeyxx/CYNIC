#!/usr/bin/env node
/**
 * Test EventBus publish/subscribe
 */

import { globalEventBus } from '@cynic/core';

console.log('Testing globalEventBus...\n');

let received = false;

// Subscribe
globalEventBus.on('test:event', (data) => {
  console.log('✓ Event received:', data);
  received = true;
});

// Publish
console.log('Publishing test event...');
globalEventBus.publish('test:event', { message: 'Hello' }, { source: 'test' });

setTimeout(() => {
  if (received) {
    console.log('\n✓ EventBus works correctly');
    process.exit(0);
  } else {
    console.log('\n✗ EventBus not working (event not received)');
    process.exit(1);
  }
}, 100);
