#!/usr/bin/env node
/**
 * Test chokidar directly
 */

import chokidar from 'chokidar';
import { writeFileSync, unlinkSync } from 'fs';
import { join } from 'path';

console.log('Testing chokidar...\n');

const testFile = join(process.cwd(), 'chokidar-test.txt');
let eventsReceived = 0;

const watcher = chokidar.watch(process.cwd(), {
  ignored: /(^|[\/\\])\../,
  persistent: false,
  ignoreInitial: true,
  awaitWriteFinish: {
    stabilityThreshold: 100,
    pollInterval: 50,
  },
});

watcher
  .on('add', (path) => {
    console.log('✓ ADD event:', path);
    eventsReceived++;
  })
  .on('change', (path) => {
    console.log('✓ CHANGE event:', path);
    eventsReceived++;
  })
  .on('unlink', (path) => {
    console.log('✓ UNLINK event:', path);
    eventsReceived++;
  })
  .on('ready', () => {
    console.log('✓ Watcher ready\n');

    console.log('Creating test file...');
    writeFileSync(testFile, 'initial content\n');

    setTimeout(() => {
      console.log('Modifying test file...');
      writeFileSync(testFile, 'modified content\n');
    }, 500);

    setTimeout(() => {
      console.log('Deleting test file...');
      try {
        unlinkSync(testFile);
      } catch (e) {
        // May not exist
      }
    }, 1000);

    setTimeout(async () => {
      await watcher.close();
      console.log(`\nEvents received: ${eventsReceived}`);
      if (eventsReceived >= 2) {
        console.log('✓ Chokidar works correctly');
        process.exit(0);
      } else {
        console.log('✗ Chokidar not working (expected >= 2 events)');
        process.exit(1);
      }
    }, 2000);
  });
