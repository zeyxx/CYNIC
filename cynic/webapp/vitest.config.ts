import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    globals: true,
    setupFiles: ['tests/setup.ts'],
    testTimeout: 15000, // Increase from default 5000ms to handle concurrent/stress tests
  },
});
